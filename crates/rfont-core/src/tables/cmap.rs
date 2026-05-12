use lru::LruCache;
use rfont_types::{EncodingRecord, FontError, ReadBytes, Reader};
use std::collections::HashMap;
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
pub struct Cmap {
    pub encoding_records: Vec<EncodingRecord>,
    pub unicode_map: HashMap<u32, u16>,
    // LRU 缓存：最近查询的 Unicode → GlyphID 映射
    query_cache: Option<LruCache<u32, u16>>,
}

impl Cmap {
    pub fn read_from(reader: &mut Reader) -> Result<Self, FontError> {
        
        let _version = reader.read_u16()?;
        let num_tables = reader.read_u16()?;

        let mut encoding_records = Vec::with_capacity(num_tables as usize);
        for _ in 0..num_tables {
            encoding_records.push(EncodingRecord::read_from(reader)?);
        }

        let mut unicode_map = HashMap::new();

        // 定义优先级：Microsoft Unicode > Unicode Platform > Mac Roman
        let priority_order = [
            (3u16, 1u16), // Microsoft Unicode BMP
            (0u16, 3u16), // Unicode 2.0+ Full Repertoire
            (0u16, 4u16), // Unicode Variation Sequences
            (0u16, 0u16), // Unicode Default
            (3u16, 0u16), // Microsoft Symbol
            (1u16, 0u16), // Mac Roman
        ];

        // 按优先级尝试解析
        for &(platform_id, encoding_id) in &priority_order {
            if let Some(record) = encoding_records
                .iter()
                .find(|r| r.platform_id == platform_id && r.encoding_id == encoding_id)
            {
                // 尝试解析该子表
                if let Ok(map) = Self::parse_subtable_at(reader, record.offset as usize) {
                    if !map.is_empty() {
                        println!(
                            "[Cmap] Using platform {} encoding {}",
                            platform_id, encoding_id
                        );
                        unicode_map.extend(map);
                        break; // 找到一个可用的映射就停止
                    }
                }
            }
        }

        // 如果上述都没找到，尝试任意可用的子表
        if unicode_map.is_empty() {
            for record in &encoding_records {
                if let Ok(map) = Self::parse_subtable_at(reader, record.offset as usize) {
                    if !map.is_empty() {
                        println!(
                            "[Cmap] Fallback to platform {} encoding {}",
                            record.platform_id, record.encoding_id
                        );
                        unicode_map.extend(map);
                        break;
                    }
                }
            }
        }

        Ok(Self {
            encoding_records,
            unicode_map,
            query_cache: Some(LruCache::new(NonZeroUsize::new(256).unwrap())), // 默认缓存 256 个条目
        })
    }

    fn parse_subtable_at(
        reader: &mut Reader,
        offset: usize,
    ) -> Result<HashMap<u32, u16>, FontError> {
        let format = reader.read_u16_at(offset)?;

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

            if start == 0 && end == 0xFFFF {
                break;
            }

            for c in start..=end {
                let glyph_id = if range_offset == 0 {
                    // 这里的加法结果如果是负数，转为 u16 时会回绕，符合规范
                    (c as i32 + delta) as u16
                } else {
                    // index = (rangeOffset / 2) + (c - startCode) + (i - segCount)
                    // 注意：这里的 i - segCount 是为了补偿 glyphIdArray 开头的偏移
                    let range_offset_idx = range_offset / 2;
                    let char_offset = (c - start) as usize;
                    let seg_adjustment = i.wrapping_sub(seg_count) as usize;

                    // 使用 saturating_add 防止溢出
                    let index = range_offset_idx
                        .saturating_add(char_offset)
                        .saturating_add(seg_adjustment);

                    if index < glyph_id_array.len() {
                        glyph_id_array[index]
                    } else {
                        0
                    }
                };

                map.insert(c as u32, glyph_id);
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

    /// 获取字形 ID（带缓存优化）
    pub fn get_glyph_id(&self, char_code: char) -> Option<u16> {
        let code = char_code as u32;

        // 直接查找 unicode_map
        self.unicode_map.get(&code).copied()
    }

    /// 获取字形 ID（可变版本，会更新缓存）
    pub fn get_glyph_id_mut(&mut self, char_code: char) -> Option<u16> {
        let code = char_code as u32;

        // 尝试从缓存获取
        if let Some(cache) = &mut self.query_cache {
            if let Some(&glyph_id) = cache.get(&code) {
                return Some(glyph_id);
            }
        }

        // 查找 unicode_map
        let result = self.unicode_map.get(&code).copied();

        // 更新缓存
        if let Some(glyph_id) = result {
            if let Some(cache) = &mut self.query_cache {
                cache.put(code, glyph_id);
            }
        }

        result
    }

    /// 批量查询字形 ID（优化版本）
    pub fn get_glyph_ids(&self, text: &str) -> Vec<(char, Option<u16>)> {
        text.chars().map(|ch| (ch, self.get_glyph_id(ch))).collect()
    }

    /// 清除查询缓存
    pub fn clear_cache(&mut self) {
        if let Some(cache) = &mut self.query_cache {
            cache.clear();
        }
    }

    /// 获取缓存大小
    pub fn cache_len(&self) -> usize {
        self.query_cache.as_ref().map_or(0, |cache| cache.len())
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
        let mut cmap = Cmap {
            encoding_records: vec![],
            unicode_map: HashMap::new(),
            query_cache: None, // 测试时不使用缓存
        };

        // 手动添加一些映射
        cmap.unicode_map.insert(0x963F, 6329); // '阿'
        cmap.unicode_map.insert(0x91CC, 6031); // '里'
        cmap.unicode_map.insert(0x5988, 1309); // '妈'

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
    fn test_cmap_cache_operations() {
        let mut cmap = Cmap {
            encoding_records: vec![],
            unicode_map: HashMap::new(),
            query_cache: Some(LruCache::new(NonZeroUsize::new(10).unwrap())),
        };

        cmap.unicode_map.insert(0x41, 65); // 'A'
        cmap.unicode_map.insert(0x42, 66); // 'B'

        // 首次查询（会缓存）
        assert_eq!(cmap.get_glyph_id_mut('A'), Some(65));
        assert_eq!(cmap.cache_len(), 1);

        // 第二次查询（从缓存获取）
        assert_eq!(cmap.get_glyph_id_mut('A'), Some(65));
        assert_eq!(cmap.cache_len(), 1);

        // 查询另一个字符
        assert_eq!(cmap.get_glyph_id_mut('B'), Some(66));
        assert_eq!(cmap.cache_len(), 2);

        // 清除缓存
        cmap.clear_cache();
        assert_eq!(cmap.cache_len(), 0);
    }

    #[test]
    fn test_cmap_batch_query() {
        let mut cmap = Cmap {
            encoding_records: vec![],
            unicode_map: HashMap::new(),
            query_cache: None,
        };

        cmap.unicode_map.insert(0x41, 65); // 'A'
        cmap.unicode_map.insert(0x42, 66); // 'B'
        cmap.unicode_map.insert(0x43, 67); // 'C'

        let results = cmap.get_glyph_ids("ABC");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], ('A', Some(65)));
        assert_eq!(results[1], ('B', Some(66)));
        assert_eq!(results[2], ('C', Some(67)));
    }
}
