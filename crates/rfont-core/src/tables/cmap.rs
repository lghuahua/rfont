use rfont_types::{EncodingRecord, FontError, ReadBytes, Reader, WriteBytes};
use std::collections::HashMap;
use tracing::{debug, info};
/// 唯一标识一个 cmap 子表的键
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct CmapSubtableKey {
    pub platform_id: u16,
    pub encoding_id: u16,
}

impl From<&EncodingRecord> for CmapSubtableKey {
    fn from(value: &EncodingRecord) -> Self {
        Self {
            platform_id: value.platform_id,
            encoding_id: value.encoding_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cmap {
    // 核心存储：按子表键存储独立的映射
    pub subtables: HashMap<CmapSubtableKey, HashMap<u32, u16>>,
}

impl<'a> ReadBytes<'a> for Cmap {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let _version = reader.read_u16()?;
        let num_tables = reader.read_u16()?;

        let encoding_records: Vec<EncodingRecord> = reader.read_array(num_tables as usize)?;

        // 预分配 HashMap 容量，避免多次扩容
        let mut subtables = HashMap::with_capacity(num_tables as usize);

        for record in encoding_records.iter() {
            let key: CmapSubtableKey = record.into();
            // 尝试解析该子表
            if let Ok(map) = Self::parse_subtable_at(reader, record.offset as usize)
                && !map.is_empty()
            {
                debug!(
                    "[Cmap] Using platform {} encoding {}",
                    record.platform_id, record.encoding_id
                );

                subtables.insert(key, map);
            }
        }
        Ok(Self { subtables })
    }
}

impl WriteBytes for Cmap {
    fn write_to(&self, writer: &mut rfont_types::Writer) -> Result<(), FontError> {
        let mut records: Vec<(CmapSubtableKey, Vec<u8>)> = self
            .subtables
            .iter()
            .map(|(key, map)| (*key, Self::encode_subtable(map)))
            .collect();

        // 2. 按规范排序：先按 platform_id，再按 encoding_id
        records.sort_by(|a, b| match a.0.platform_id.cmp(&b.0.platform_id) {
            std::cmp::Ordering::Equal => a.0.encoding_id.cmp(&b.0.encoding_id),
            other => other,
        });

        writer.write_u16(0)?;
        writer.write_u16(records.len() as u16)?;
        let mut offset = 4_u32 + records.len() as u32 * 8;
        let mut subtable_data = Vec::new();
        for (key, data) in records.iter() {
            let record = EncodingRecord {
                platform_id: key.platform_id,
                encoding_id: key.encoding_id,
                offset,
            };
            record.write_to(writer)?;
            offset += data.len() as u32;
            subtable_data.extend_from_slice(data);
        }
        writer.write_bytes(&subtable_data)
    }
}

impl Cmap {
    fn encode_subtable(map: &HashMap<u32, u16>) -> Vec<u8> {
        // 检查是否需要 Format 12（是否有超出 BMP 的字符）
        let needs_format12 = map.keys().any(|&cp| cp > 0xFFFF);

        if needs_format12 {
            Self::encode_format12(map)
        } else {
            Self::encode_format4(map)
        }
    }

    fn encode_format4(map: &HashMap<u32, u16>) -> Vec<u8> {
        // 收集并排序所有字符码位（使用 unstable sort 更快）
        let mut unicode_map = map
            .iter()
            .map(|(cp, glyph_id)| (*cp, *glyph_id))
            .collect::<Vec<_>>();
        unicode_map.sort_unstable_by_key(|&(unicode, _)| unicode);

        // 构建段（segments）- 预分配容量
        let estimated_segments = (unicode_map.len() / 10).max(1); // 估算段数量
        let mut start_codes = Vec::with_capacity(estimated_segments + 1); // +1 for sentinel
        let mut end_codes = Vec::with_capacity(estimated_segments + 1);
        let mut id_deltas = Vec::with_capacity(estimated_segments + 1);

        let mut i = 0;
        while i < unicode_map.len() {
            if unicode_map[i].0 == 0xFFFF {
                i += 1;
                continue;
            }

            let start_code = unicode_map[i].0 as u16;
            let start_gid = unicode_map[i].1;
            let mut end_code = start_code;
            let mut end_gid = start_gid;
            let mut j = i + 1;

            while j < unicode_map.len() {
                let next_code = unicode_map[j].0 as u16;
                let next_gid = unicode_map[j].1;

                // 检查是否连续：码点连续 且 glyph ID 也连续
                // 注意：end_code 不能达到 0xFFFF，因为那是 sentinel 的值
                if next_code == end_code + 1 && next_gid == end_gid + 1 && next_code < 0xFFFF {
                    end_code = next_code;
                    end_gid = next_gid;
                    j += 1;
                } else {
                    break;
                }
            }

            let id_delta = (start_gid as i32 - start_code as i32) as i16;

            start_codes.push(start_code);
            end_codes.push(end_code);
            id_deltas.push(id_delta as u16);
            i = j;
        }

        // 添加终止段
        start_codes.push(0xFFFF);
        end_codes.push(0xFFFF);
        id_deltas.push(1);

        let n_segments = start_codes.len() as u16;
        
        // 预分配结果向量容量
        let length = 16 + (n_segments * 8); // 16是头部固定部分，8是每个段占用的字节数
        let mut result = Vec::with_capacity(length as usize);

        let seg_count_x2 = n_segments * 2;
        let max_power = if n_segments > 0 {
            (n_segments as f32).log2().floor().exp2() as usize
        } else {
            1
        };
        // searchRange = 2 * (小于等于 segCount 的最大 2 的幂)
        // 因为搜索的是 u16 数组，所以要乘以 2
        let search_range = (max_power * 2) as u16;
        let entry_selector = max_power.trailing_zeros() as u16;
        let range_shift = seg_count_x2.saturating_sub(search_range);

        // Format 4 头部
        result.extend(&0x0004u16.to_be_bytes()); // format
        result.extend(&length.to_be_bytes()); // length
        result.extend(&0u16.to_be_bytes()); // language (通常为0)

        result.extend(&seg_count_x2.to_be_bytes()); // segCountX2
        result.extend(&search_range.to_be_bytes()); // searchRange
        result.extend(&entry_selector.to_be_bytes()); // entrySelector
        result.extend(&range_shift.to_be_bytes()); // rangeShift

        // 写入 endCodes
        for &code in &end_codes {
            result.extend(&code.to_be_bytes());
        }
        result.extend(&0u16.to_be_bytes()); // reservedPad

        // 写入 startCodes
        for &code in &start_codes {
            result.extend(&code.to_be_bytes());
        }

        // 写入 idDeltas
        for &delta in &id_deltas {
            result.extend(&delta.to_be_bytes());
        }

        // 写入 idRangeOffsets (简化版本全部为0)
        for _ in 0..n_segments {
            result.extend(&0u16.to_be_bytes());
        }

        result
    }
    fn encode_format12(map: &HashMap<u32, u16>) -> Vec<u8> {
        let mut chars: Vec<u32> = map.keys().copied().collect();
        chars.sort();

        // 构建组（groups）
        let mut groups = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            let start_char_code = chars[i];
            let mut end_char_code = start_char_code;
            let start_glyph_id = map[&start_char_code];

            // 找到连续的组
            while i + 1 < chars.len() && chars[i + 1] == end_char_code + 1 {
                let next_glyph_id = map[&chars[i + 1]];
                // 检查是否连续映射（glyph ID 也连续）
                if next_glyph_id == start_glyph_id + (chars[i + 1] - start_char_code) as u16 {
                    end_char_code = chars[i + 1];
                    i += 1;
                } else {
                    break;
                }
            }

            groups.push((start_char_code, end_char_code, start_glyph_id));
            i += 1;
        }

        let n_groups = groups.len() as u32;
        let mut result = Vec::new();

        // Format 12 头部
        result.extend(&0x000Cu16.to_be_bytes()); // format
        result.extend(&0u16.to_be_bytes()); // reserved (必须为0)
        result.extend(&(16 + n_groups * 12).to_be_bytes()); // length
        result.extend(&0u32.to_be_bytes()); // language (通常为0)
        result.extend(&n_groups.to_be_bytes()); // numGroups

        // 写入每个组
        for (start, end, glyph_id) in groups {
            result.extend(&start.to_be_bytes());
            result.extend(&end.to_be_bytes());
            result.extend(&glyph_id.to_be_bytes());
        }

        result
    }
    fn parse_subtable_at(
        reader: &mut Reader,
        offset: usize,
    ) -> Result<HashMap<u32, u16>, FontError> {
        let _format = reader.read_u16_at(offset)?;

        let length = reader.read_u16_at(offset + 2)? as usize;

        // 创建该子表的专属 Reader
        match reader.slice(offset, length) {
            Ok(mut sub_reader) => Self::parse_subtable(&mut sub_reader),
            Err(e) => Err(e),
        }
    }

    fn parse_subtable(reader: &mut Reader) -> Result<HashMap<u32, u16>, FontError> {
        let format = reader.read_u16()?;

        match format {
            0 => {
                // Format 0: format(2) + length(2) + language(2)
                let _length = reader.read_u16()?;
                let _language = reader.read_u16()?;
                Self::parse_format0(reader)
            }
            4 => {
                // Format 4: format(2) + length(2) + language(2)
                let _length = reader.read_u16()?;
                let _language = reader.read_u16()?;
                Self::parse_format4(reader)
            }
            12 => {
                // Format 12: format(2) + reserved(2) + length(4) + language(4)
                let _reserved = reader.read_u16()?;
                let _length = reader.read_u32()?;
                let _language = reader.read_u32()?;
                Self::parse_format12(reader)
            }
            _ => Err(FontError::UnsupportedCmapFormat { format }),
        }
    }

    /// Format 0: 简单字节映射表（256 个条目）
    fn parse_format0(reader: &mut Reader) -> Result<HashMap<u32, u16>, FontError> {
        // Format 0 有 256 个字节的映射数组
        let glyph_id_array = reader.read_array::<u8>(256)?;

        let mut map = HashMap::new();
        for (char_code, &glyph_id) in glyph_id_array.iter().enumerate() {
            if glyph_id != 0 {
                map.insert(char_code as u32, glyph_id as u16);
            }
        }

        Ok(map)
    }

    fn parse_format4(reader: &mut Reader) -> Result<HashMap<u32, u16>, FontError> {
        let seg_count_x2 = reader.read_u16()?;
        let seg_count = seg_count_x2 / 2;

        let _search_range = reader.read_u16()?;
        let _entry_selector = reader.read_u16()?;
        let _range_shift = reader.read_u16()?;

        let end_code = reader.read_array::<u16>(seg_count as usize)?;
        let _reserved_pad = reader.read_u16()?;
        let start_code = reader.read_array::<u16>(seg_count as usize)?;

        // 注意：id_delta 在规范中是 int16，我们需要按有符号数读取
        let mut id_delta_signed = Vec::with_capacity(seg_count as usize);
        for _ in 0..seg_count {
            id_delta_signed.push(reader.read_i16()?);
        }

        let id_range_offset = reader.read_array::<u16>(seg_count as usize)?;

        // glyphIdArray 紧跟在 id_range_offset 之后
        let remaining_len = reader.data.len() - reader.offset;
        let glyph_id_array = reader.read_array::<u16>(remaining_len / 2)?;

        let mut map = HashMap::new();

        for i in 0..seg_count {
            let start = start_code[i as usize];
            let end = end_code[i as usize];
            let delta = id_delta_signed[i as usize] as i32;
            let range_offset = id_range_offset[i as usize] as usize;

            // 移除错误的提前退出逻辑
            // 根据 OpenType 规范，应该遍历所有段，而不是在遇到 0-0xFFFF 段时退出
            // 0-0xFFFF 段通常是第一个段（覆盖整个 BMP），后续可能还有更具体的段
            // println!("Segment {}: 0x{:04X} - 0x{:04X}", i, start, end);

            if range_offset == 0 {
                // 情况1：连续映射，直接使用 delta 计算
                for c in start..=end {
                    let glyph_id = (c as i32 + delta) as u16;
                    // 根据规范，如果计算结果为0，应跳过此映射吗？这里保留与原始逻辑一致
                    // if glyph_id != 0 {
                    map.insert(c as u32, glyph_id);
                    // }
                }
            } else {
                // 情况2：稀疏映射，需要从 glyphIdArray 中读取
                // 计算公式：glyphIdArrayIndex = rangeOffset/2 + (c - start)

                let range_offset_idx = range_offset / 2;
                let seg_adjustment = i.wrapping_sub(seg_count) as usize;
                let base_index = range_offset_idx.wrapping_add(seg_adjustment) as u16;

                for c in start..=end {
                    let char_offset = c - start;
                    let array_index = base_index + char_offset;

                    if let Some(&gid) = glyph_id_array.get(array_index as usize) {
                        if gid != 0 {
                            let glyph_id = (gid as i32 + delta) as u16;
                            map.insert(c as u32, glyph_id);
                        }
                        // 如果 gid == 0，表示该字符无映射，跳过
                    } else {
                        // 数组越界，说明数据损坏或段结束
                        break;
                    }
                }
            }
        }
        Ok(map)
    }

    fn parse_format12(reader: &mut Reader) -> Result<HashMap<u32, u16>, FontError> {
        // 注意：reserved、length、language 已经在 parse_subtable 中读取
        let n_groups = reader.read_u32()?;

        let mut map = HashMap::new();
        for _ in 0..n_groups {
            let start_char_code = reader.read_u32()?;
            let end_char_code = reader.read_u32()?;
            let start_glyph_id = reader.read_u32()?;

            for i in 0..=(end_char_code - start_char_code) {
                map.insert(start_char_code + i, (start_glyph_id + i) as u16);
            }
        }
        Ok(map)
    }

    /// 获取字形 ID
    pub fn get_glyph_id(&self, char_code: char) -> Option<u16> {
        let code = char_code as u32;

        self.get_glyph_id_by_code(code)
    }
    /// 获取字形 ID
    pub fn get_glyph_id_by_code(&self, code: u32) -> Option<u16> {
        // 直接查找 unicode_map
        for (_key, map) in self.subtables.iter() {
            if let Some(id) = map.get(&code)
                && *id > 0
            {
                return Some(*id);
            }
        }
        info!("No glyph found for char code: {}", code);
        None
    }
    /// 批量查询字形 ID（优化版本）
    pub fn get_glyph_ids(&self, text: &str) -> Vec<(char, Option<u16>)> {
        text.chars().map(|ch| (ch, self.get_glyph_id(ch))).collect()
    }

    pub fn supported_chars(&self) -> Vec<char> {
        let mut chars = Vec::new();
        let mut iter = self.subtables.iter();
        if let Some((_, map)) = iter.next() {
            for (char_code, _glyph_id) in map.iter() {
                chars.push(char::from_u32(*char_code).unwrap());
            }
        }

        chars
    }

    pub fn supported_chars_count(&self) -> usize {
        let mut iter = self.subtables.iter();
        if let Some((_, map)) = iter.next() {
            return map.len();
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::io::Reader;

    #[test]
    fn test_cmap_format0_parsing() {
        // Format 0: Byte encoding table
        let mut data = vec![
            0x00, 0x00, // format = 0
            0x01, 0x06, // length = 262 (2 + 2 + 256)
            0x00, 0x00, // language
        ];
        // 添加 256 字节的 glyph 索引数组
        for i in 0..256 {
            data.push(i as u8);
        }

        let mut reader = Reader::new(&data);
        let map = Cmap::parse_subtable(&mut reader).unwrap();

        // 验证一些映射（注意：glyph_id 0 不会被插入）
        assert_eq!(map.get(&0), None); // glyph_id 0 被跳过
        assert_eq!(map.get(&65), Some(&65)); // 'A'
        assert_eq!(map.get(&97), Some(&97)); // 'a'
        assert_eq!(map.len(), 255); // 256 - 1 (跳过了 0)
    }

    #[test]
    fn test_cmap_format4_simple() {
        // Format 4: Segment mapping to delta values
        // 简化版本：只有一个段，映射 'A' (U+0041) -> GlyphID 65
        let data = vec![
            0x00, 0x04, // format = 4
            0x00, 0x10, // length = 16
            0x00, 0x00, // language
            0x00, 0x02, // seg_count_x2 = 2 (1 segment)
            0x00, 0x04, // search_range = 4
            0x00, 0x00, // entry_selector = 0
            0x00, 0x04, // range_shift = 4
            0x00, 0x41, // end_code[0] = 0x41 ('A')
            0xFF, 0xFF, // reserved_pad = 0xFFFF
            0x00, 0x41, // start_code[0] = 0x41
            0x00, 0x00, // id_delta[0] = 0
            0x00, 0x00, // id_range_offset[0] = 0
        ];

        let mut reader = Reader::new(&data);
        let map = Cmap::parse_subtable(&mut reader).unwrap();

        assert_eq!(map.get(&0x41), Some(&65)); // 'A' -> 65
    }

    #[test]
    fn test_cmap_unicode_lookup() {
        // 测试常见中文字符的查找
        let mut map = HashMap::new();

        // 手动添加一些映射
        map.insert(0x963F, 6329); // '阿'
        map.insert(0x91CC, 6031); // '里'
        map.insert(0x5988, 1309); // '妈'
        let mut subtables = HashMap::new();
        subtables.insert(
            CmapSubtableKey {
                platform_id: 3,
                encoding_id: 1,
            },
            map,
        );

        let cmap = Cmap { subtables };

        assert_eq!(cmap.get_glyph_id('阿'), Some(6329));
        assert_eq!(cmap.get_glyph_id('里'), Some(6031));
        assert_eq!(cmap.get_glyph_id('妈'), Some(1309));
        assert_eq!(cmap.get_glyph_id('不'), None); // 不存在的字符
    }

    #[test]
    fn test_cmap_format12_parsing() {
        // Format 12: Segmented coverage (supports Unicode beyond BMP)
        // 注意：parse_subtable 期望从 format 字段开始读取
        let data = vec![
            0x00, 0x0C, // format = 12
            0x00, 0x00, // reserved (length field in subtable header)
            0x00, 0x00, 0x00, 0x1C, // length = 28
            0x00, 0x00, 0x00, 0x00, // language
            0x00, 0x00, 0x00, 0x01, // n_groups = 1
            // Group 1: U+0041-U+0043 -> GlyphID 65-67 ('A', 'B', 'C')
            0x00, 0x00, 0x00, 0x41, // start_char_code = 0x41
            0x00, 0x00, 0x00, 0x43, // end_char_code = 0x43
            0x00, 0x00, 0x00, 0x41, // start_glyph_id = 65
        ];

        let mut reader = Reader::new(&data);
        let map = Cmap::parse_subtable(&mut reader).unwrap();

        assert_eq!(map.get(&0x41), Some(&65)); // 'A'
        assert_eq!(map.get(&0x42), Some(&66)); // 'B'
        assert_eq!(map.get(&0x43), Some(&67)); // 'C'
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_cmap_format12_multiple_groups() {
        // Multiple groups in Format 12
        let data = vec![
            0x00, 0x0C, // format = 12
            0x00, 0x00, 0x00, 0x00, 0x00, 0x34, // length = 52 (4 + 4 + 4 + 4 + 2*12)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, // n_groups = 2
            // Group 1: U+0030-U+0039 -> GlyphID 48-57 (digits '0'-'9')
            0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x39, 0x00, 0x00, 0x00, 0x30,
            // Group 2: U+0041-U+005A -> GlyphID 65-90 (uppercase 'A'-'Z')
            0x00, 0x00, 0x00, 0x41, 0x00, 0x00, 0x00, 0x5A, 0x00, 0x00, 0x00, 0x41,
        ];

        let mut reader = Reader::new(&data);
        let map = Cmap::parse_subtable(&mut reader).unwrap();

        assert_eq!(map.get(&0x30), Some(&48)); // '0'
        assert_eq!(map.get(&0x39), Some(&57)); // '9'
        assert_eq!(map.get(&0x41), Some(&65)); // 'A'
        assert_eq!(map.get(&0x5A), Some(&90)); // 'Z'
        assert_eq!(map.len(), 36); // 10 digits + 26 letters
    }

    #[test]
    fn test_cmap_format4_with_delta() {
        // Format 4 with non-zero delta
        let data = vec![
            0x00, 0x04, // format = 4
            0x00, 0x10, 0x00, 0x00, 0x00, 0x02, // seg_count_x2 = 2
            0x00, 0x04, 0x00, 0x00, 0x00, 0x04, 0x00, 0x41, // end_code[0] = 0x41
            0xFF, 0xFF, 0x00, 0x41, // start_code[0] = 0x41
            0x00, 0x0A, // id_delta[0] = 10 (delta)
            0x00, 0x00,
        ];

        let mut reader = Reader::new(&data);
        let map = Cmap::parse_subtable(&mut reader).unwrap();

        // 'A' (0x41) + delta(10) = glyph_id 75
        assert_eq!(map.get(&0x41), Some(&75));
    }

    #[test]
    fn test_cmap_unsupported_format() {
        // Unsupported format (e.g., format 6)
        let data = vec![
            0x00, 0x06, // format = 6
            0x00, 0x0A, 0x00, 0x00,
        ];

        let mut reader = Reader::new(&data);
        let result = Cmap::parse_subtable(&mut reader);

        assert!(result.is_err());
    }

    #[test]
    fn test_cmap_batch_query() {
        let mut map = HashMap::new();

        // 手动添加一些映射
        map.insert(0x41, 65); // 'A'
        map.insert(0x42, 66); // 'B'
        map.insert(0x43, 67); // 'C'
        let mut subtables = HashMap::new();
        subtables.insert(
            CmapSubtableKey {
                platform_id: 3,
                encoding_id: 1,
            },
            map,
        );

        let cmap = Cmap { subtables };

        let results = cmap.get_glyph_ids("ABC");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], ('A', Some(65)));
        assert_eq!(results[1], ('B', Some(66)));
        assert_eq!(results[2], ('C', Some(67)));
    }
}
