use rfont_types::FontError;

const GLYF_HEADER_SIZE: usize = 10;

const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const WE_HAVE_A_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
const WE_HAVE_INSTRUCTIONS: u16 = 0x0100;

/// 移除单个字形的 TrueType 指令数据
///
/// 简单字形直接丢弃指令区；复合字形清除最后一个组件的
/// `WE_HAVE_INSTRUCTIONS` 标志并截断其后的指令字节。
/// 空字形或无法识别的数据原样返回。
pub fn strip_glyph_hinting(glyph: &[u8]) -> Result<Vec<u8>, FontError> {
    if glyph.len() < GLYF_HEADER_SIZE {
        return Ok(glyph.to_vec());
    }

    let num_contours = i16::from_be_bytes([glyph[0], glyph[1]]);
    if num_contours >= 0 {
        strip_simple_glyph(glyph, num_contours as usize)
    } else if num_contours == -1 {
        strip_composite_glyph(glyph)
    } else {
        Ok(glyph.to_vec())
    }
}

fn strip_simple_glyph(glyph: &[u8], num_contours: usize) -> Result<Vec<u8>, FontError> {
    let end_pts_end = GLYF_HEADER_SIZE + num_contours * 2;
    let length_field_end = end_pts_end + 2;
    if length_field_end > glyph.len() {
        return Err(FontError::TableTooShort {
            table: "glyf".to_string(),
            min_size: length_field_end,
            actual_size: glyph.len(),
        });
    }

    let instruction_len = u16::from_be_bytes([glyph[end_pts_end], glyph[end_pts_end + 1]]) as usize;
    let instructions_end = length_field_end + instruction_len;
    if instructions_end > glyph.len() {
        return Err(FontError::TableTooShort {
            table: "glyf".to_string(),
            min_size: instructions_end,
            actual_size: glyph.len(),
        });
    }

    let mut out = Vec::with_capacity(glyph.len() - instruction_len);
    out.extend_from_slice(&glyph[..end_pts_end]);
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&glyph[instructions_end..]);
    Ok(out)
}

fn strip_composite_glyph(glyph: &[u8]) -> Result<Vec<u8>, FontError> {
    let mut offset = GLYF_HEADER_SIZE;
    let mut last_flags_offset;

    loop {
        if offset + 4 > glyph.len() {
            return Err(FontError::TableTooShort {
                table: "glyf composite".to_string(),
                min_size: offset + 4,
                actual_size: glyph.len(),
            });
        }
        let flags = u16::from_be_bytes([glyph[offset], glyph[offset + 1]]);
        last_flags_offset = offset;
        offset += 4; // flags + glyphIndex

        offset += if flags & ARG_1_AND_2_ARE_WORDS != 0 {
            4
        } else {
            2
        };
        if flags & WE_HAVE_A_SCALE != 0 {
            offset += 2;
        } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            offset += 4;
        } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
            offset += 8;
        }

        if offset > glyph.len() {
            return Err(FontError::TableTooShort {
                table: "glyf composite".to_string(),
                min_size: offset,
                actual_size: glyph.len(),
            });
        }

        if flags & MORE_COMPONENTS == 0 {
            break;
        }
    }

    let mut out = glyph[..offset].to_vec();
    let flags = u16::from_be_bytes([out[last_flags_offset], out[last_flags_offset + 1]]);
    if flags & WE_HAVE_INSTRUCTIONS != 0 {
        let cleared = flags & !WE_HAVE_INSTRUCTIONS;
        out[last_flags_offset..last_flags_offset + 2].copy_from_slice(&cleared.to_be_bytes());
    }
    Ok(out)
}

/// 移除整张 glyf 表中所有字形的指令，返回新的 glyf 数据与偏移表
///
/// `offsets` 必须包含 `num_glyphs + 1` 个元素。输出偏移按 2 字节对齐，
/// 以便安全使用 short loca 格式。
pub fn strip_hinting_from_glyf(
    glyf: &[u8],
    offsets: &[u32],
) -> Result<(Vec<u8>, Vec<u32>), FontError> {
    let mut new_glyf = Vec::with_capacity(glyf.len());
    let mut new_offsets = Vec::with_capacity(offsets.len());

    for pair in offsets.windows(2) {
        let (start, end) = (pair[0] as usize, pair[1] as usize);
        new_offsets.push(new_glyf.len() as u32);

        if start >= end {
            continue;
        }
        if end > glyf.len() {
            return Err(FontError::InvalidOffset {
                table: "glyf".to_string(),
                offset: end as u32,
                max: glyf.len() as u32,
            });
        }

        let stripped = strip_glyph_hinting(&glyf[start..end])?;
        new_glyf.extend_from_slice(&stripped);
        while !new_glyf.len().is_multiple_of(2) {
            new_glyf.push(0);
        }
    }

    new_offsets.push(new_glyf.len() as u32);
    Ok((new_glyf, new_offsets))
}

/// 编码 loca 表：偏移均小于 64K 时使用 short 格式
pub fn encode_loca(offsets: &[u32]) -> Vec<u8> {
    let use_short = offsets.last().is_none_or(|&max| max < 0x1_0000)
        && offsets.iter().all(|offset| offset.is_multiple_of(2));

    let mut data = Vec::with_capacity(offsets.len() * if use_short { 2 } else { 4 });
    for &offset in offsets {
        if use_short {
            data.extend_from_slice(&((offset / 2) as u16).to_be_bytes());
        } else {
            data.extend_from_slice(&offset.to_be_bytes());
        }
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_glyph_with_instructions() -> Vec<u8> {
        vec![
            0x00, 0x01, // num_contours = 1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x0A, // bbox
            0x00, 0x00, // end_pts_of_contours: [0]
            0x00, 0x02, // instruction_length = 2
            0xAA, 0xBB, // instructions
            0x01, // flag
            0x05, // x
            0x07, // y
        ]
    }

    #[test]
    fn test_strip_simple_glyph() {
        let glyph = simple_glyph_with_instructions();
        let stripped = strip_glyph_hinting(&glyph).unwrap();

        // 头部 10 + endPts 2 + 指令长度 2 + flag 1 + 坐标 2
        assert_eq!(stripped.len(), glyph.len() - 2);
        assert_eq!(&stripped[12..14], &[0, 0]);
        assert_eq!(&stripped[14..], &glyph[16..]);
    }

    #[test]
    fn test_strip_composite_glyph() {
        let glyph = vec![
            0xFF, 0xFF, // num_contours = -1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x0A, // bbox
            0x00, 0x00, // flags：最后一个组件，带 WE_HAVE_INSTRUCTIONS
            0x00, 0x02, // glyphIndex
            0x00, 0x00, // args
            0x01, 0x02, // components 之后的指令
            0x03, // 更多指令
        ];
        // 将最后一个组件的 flags 设为 0x0100
        let mut glyph = glyph;
        glyph[10] = 0x01;

        let stripped = strip_glyph_hinting(&glyph).unwrap();
        assert_eq!(stripped.len(), 16);
        assert_eq!(&stripped[10..12], &[0, 0]);
    }

    #[test]
    fn test_strip_hinting_from_glyf_pads_offsets() {
        let glyph = simple_glyph_with_instructions();
        let offsets = vec![0, glyph.len() as u32];
        let (glyf, new_offsets) = strip_hinting_from_glyf(&glyph, &offsets).unwrap();

        assert_eq!(new_offsets.len(), 2);
        assert!(new_offsets[1] as usize <= glyf.len());
        assert!(new_offsets[1].is_multiple_of(2));
    }

    #[test]
    fn test_encode_loca_short_and_long() {
        let short = encode_loca(&[0, 2, 4]);
        assert_eq!(short.len(), 6);
        assert_eq!(u16::from_be_bytes([short[2], short[3]]), 1);

        let long = encode_loca(&[0, 0x1_0002]);
        assert_eq!(long.len(), 8);
        assert_eq!(
            u32::from_be_bytes([long[4], long[5], long[6], long[7]]),
            0x1_0002
        );
    }
}
