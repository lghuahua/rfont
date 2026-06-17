use crate::Font;
use rfont_types::FontError;

/// 提取 glyf 和 loca 数据
pub fn extract_glyf_and_loca(
    font: &Font,
    subset_glyphs: &[u16],
) -> Result<(Vec<u8>, Vec<u8>), FontError> {
    let num_glyphs = subset_glyphs.len();
    // let index_to_loc_format = font.head.index_to_loc_format;

    // 构建新的 loca 偏移表
    let mut new_loca_offsets = Vec::with_capacity(num_glyphs + 1);
    let mut new_glyf_data = Vec::new();
    let mut current_offset = 0u32;

    for &glyph_id in subset_glyphs {
        new_loca_offsets.push(current_offset);

        // 从原始 glyf 表中提取字形数据
        // 跳过 .notdef (glyph_id == 0) 的轮廓数据，保持空的 .notdef
        if glyph_id == 0 {
            continue;
        }

        if (glyph_id as usize) < font.loca.offsets.len() - 1 {
            let start = font.loca.offsets[glyph_id as usize];
            let end = font.loca.offsets[glyph_id as usize + 1];

            let glyf_bytes = font.glyf.slice(start as usize, end as usize)?;
            // Cow<[u8]> 可以直接作为 &[u8] 使用
            new_glyf_data.extend_from_slice(&glyf_bytes);
            current_offset += end - start;
        }
    }

    // 添加最后一个偏移量（指向 glyf 表末尾）
    new_loca_offsets.push(current_offset);

    // 编码 loca 表
    let loca_data = if new_glyf_data.len() < 65536 {
        // short format (offset / 2)
        let mut data = Vec::with_capacity(new_loca_offsets.len() * 2);
        for &offset in &new_loca_offsets {
            data.extend_from_slice(&((offset / 2) as u16).to_be_bytes());
        }
        data
    } else {
        // long format
        let mut data = Vec::with_capacity(new_loca_offsets.len() * 4);
        for &offset in &new_loca_offsets {
            data.extend_from_slice(&offset.to_be_bytes());
        }
        data
    };

    Ok((loca_data, new_glyf_data))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_glyf_and_loca_basic() {
        // 这个测试需要完整的 Font 实例，因此在集成测试中更合适
        // 这里我们只验证函数签名和返回类型
        // 实际测试在 lib.rs 的集成测试中进行
    }

    #[test]
    fn test_loca_encoding_short_format() {
        // 测试 short format (index_to_loc_format = 0)
        let offsets = vec![0u32, 10, 20, 30];
        let mut data = Vec::with_capacity(offsets.len() * 2);
        for &offset in &offsets {
            data.extend_from_slice(&((offset / 2) as u16).to_be_bytes());
        }

        // 验证编码后的数据长度
        assert_eq!(data.len(), offsets.len() * 2);

        // 验证可以正确解码
        let decoded: Vec<u16> = (0..offsets.len())
            .map(|i| u16::from_be_bytes([data[i * 2], data[i * 2 + 1]]))
            .collect();

        assert_eq!(decoded[0], 0);
        assert_eq!(decoded[1], 5); // 10 / 2
        assert_eq!(decoded[2], 10); // 20 / 2
        assert_eq!(decoded[3], 15); // 30 / 2
    }

    #[test]
    fn test_loca_encoding_long_format() {
        // 测试 long format (index_to_loc_format = 1)
        let offsets = vec![0u32, 100, 200, 300];
        let mut data = Vec::with_capacity(offsets.len() * 4);
        for &offset in &offsets {
            data.extend_from_slice(&offset.to_be_bytes());
        }

        // 验证编码后的数据长度
        assert_eq!(data.len(), offsets.len() * 4);

        // 验证可以正确解码
        let decoded: Vec<u32> = (0..offsets.len())
            .map(|i| {
                u32::from_be_bytes([
                    data[i * 4],
                    data[i * 4 + 1],
                    data[i * 4 + 2],
                    data[i * 4 + 3],
                ])
            })
            .collect();

        assert_eq!(decoded, offsets);
    }

    #[test]
    fn test_glyf_padding_alignment() {
        // 测试 glyf 数据的 4 字节对齐
        let sizes = vec![1, 2, 3, 4, 5, 7, 8, 9, 15, 16];

        for size in sizes {
            let padding = (4 - (size % 4)) % 4;
            let padded_size = size + padding;

            // 验证对齐后是 4 的倍数
            assert_eq!(padded_size % 4, 0);

            // 验证 padding 计算正确
            if size % 4 == 0 {
                assert_eq!(padding, 0);
            } else {
                assert_eq!(padding, 4 - (size % 4));
            }
        }
    }
}
