use rfont_types::{FontError, ReadBytes, Reader, TableRecord, Tag};
use std::collections::HashMap;

/// 原始字体数据和目录信息
///
/// `FontData` 封装了字体的原始二进制数据和表目录信息。
/// 目录解析阶段已校验每个表的范围，因此后续切片访问不会越界。
///
/// # 注意
/// 此结构体通常不直接创建，而是通过 `Font::load()` 方法间接使用。
pub struct FontData {
    data: Vec<u8>,
    table_map: HashMap<Tag, TableRecord>,
}

impl FontData {
    /// 从字节向量创建（拥有所有权）
    ///
    /// 解析字体目录并校验所有表的范围。
    ///
    /// # 参数
    /// - `data`: 完整的字体文件数据
    ///
    /// # 返回值
    /// - `Ok(FontData)`: 成功创建的 FontData 实例
    /// - `Err(FontError)`: 如果字体格式无效或目录解析失败
    pub fn new(data: Vec<u8>) -> Result<Self, FontError> {
        let table_map = Self::parse_directory(&data)?;
        Ok(FontData { data, table_map })
    }

    /// 解析字体目录
    ///
    /// 校验目录本身与每个表的范围，避免后续按偏移切片时越界。
    pub(crate) fn parse_directory(data: &[u8]) -> Result<HashMap<Tag, TableRecord>, FontError> {
        let mut reader = Reader::new(data);

        // 读取 Offset Table
        let _sfnt_version = reader.read_u32()?;
        let num_tables = reader.read_u16()?;
        let _search_range = reader.read_u16()?;
        let _entry_selector = reader.read_u16()?;
        let _range_shift = reader.read_u16()?;

        let directory_end = 12usize + num_tables as usize * 16;
        if directory_end > data.len() {
            return Err(FontError::InvalidOffset {
                table: "table directory".to_string(),
                offset: directory_end as u32,
                max: data.len() as u32,
            });
        }

        // 读取 Table Directory
        let mut table_map = HashMap::with_capacity(num_tables as usize);
        for _ in 0..num_tables {
            let tag_bytes = [
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
            ];
            let tag = Tag(tag_bytes);
            let record = TableRecord {
                tag,
                checksum: reader.read_u32()?,
                offset: reader.read_u32()?,
                length: reader.read_u32()?,
            };

            let start = record.offset as usize;
            let end =
                start
                    .checked_add(record.length as usize)
                    .ok_or(FontError::InvalidOffset {
                        table: tag.to_string(),
                        offset: record.offset,
                        max: data.len() as u32,
                    })?;
            if end > data.len() {
                return Err(FontError::InvalidOffset {
                    table: tag.to_string(),
                    offset: record.offset,
                    max: data.len() as u32,
                });
            }

            table_map.insert(tag, record);
        }

        Ok(table_map)
    }

    /// 获取原始字体数据
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// SFNT flavor（TrueType 为 0x00010000，CFF 为 'OTTO'）。
    pub fn sfnt_flavor(&self) -> Option<u32> {
        self.data
            .get(0..4)
            .map(|bytes| u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// 获取表的原始字节
    ///
    /// # 参数
    /// - `tag`: 表标签（如 `Tag(*b"head")`）
    ///
    /// # 返回值
    /// - `Some(&[u8])`: 表的原始字节数据
    /// - `None`: 如果表不存在
    pub fn get_table_bytes(&self, tag: Tag) -> Option<&[u8]> {
        let record = self.table_map.get(&tag)?;
        let start = record.offset as usize;
        let end = start.checked_add(record.length as usize)?;
        self.data.get(start..end)
    }

    /// 通用表解析方法
    ///
    /// 从字体数据中解析指定类型的表。
    ///
    /// # 类型参数
    /// - `T`: 要实现 `ReadBytes` trait 的表类型
    ///
    /// # 参数
    /// - `tag`: 表标签
    ///
    /// # 返回值
    /// - `Ok(T)`: 解析后的表对象
    /// - `Err(FontError)`: 如果表不存在或解析失败
    pub fn parse_table<T>(&self, tag: Tag) -> Result<T, FontError>
    where
        T: for<'a> ReadBytes<'a>,
    {
        let bytes = self
            .get_table_bytes(tag)
            .ok_or_else(|| FontError::TableNotFound {
                tag: format!("{:?}", tag),
            })?;
        T::read_from(&mut Reader::new(bytes))
    }

    /// 获取所有表记录的引用
    ///
    /// 返回字体中所有表的目录信息（标签、校验和、偏移量、长度）。
    ///
    /// # 返回值
    /// HashMap<Tag, TableRecord> 包含所有表的信息
    pub fn get_table_records(&self) -> &HashMap<Tag, TableRecord> {
        &self.table_map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造只含一张表的 SFNT 头部 + 目录，并按需补零到 total_len
    fn sfnt_with_one_table(offset: u32, length: u32, total_len: usize) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&16u16.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(b"head");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&offset.to_be_bytes());
        data.extend_from_slice(&length.to_be_bytes());
        data.resize(total_len.max(28), 0);
        data
    }

    #[test]
    fn test_parse_directory_accepts_valid_table() {
        let data = sfnt_with_one_table(28, 4, 32);
        let font_data = FontData::new(data).unwrap();

        let bytes = font_data.get_table_bytes(Tag(*b"head")).unwrap();
        assert_eq!(bytes.len(), 4);
    }

    #[test]
    fn test_parse_directory_rejects_truncated_directory() {
        // 声明 1 张表，但连表目录项都不完整
        let mut data = Vec::new();
        data.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&[0u8; 6]);
        data.extend_from_slice(b"head");

        let err = FontData::new(data).err().unwrap();
        assert!(matches!(err, FontError::InvalidOffset { .. }));
    }

    #[test]
    fn test_parse_directory_rejects_out_of_bounds_table() {
        let data = sfnt_with_one_table(100, 4, 32);
        let err = FontData::new(data).err().unwrap();
        assert!(matches!(err, FontError::InvalidOffset { .. }));
    }

    #[test]
    fn test_parse_directory_rejects_overflowing_range() {
        let data = sfnt_with_one_table(u32::MAX, 4, 32);
        let err = FontData::new(data).err().unwrap();
        assert!(matches!(err, FontError::InvalidOffset { .. }));
    }

    #[test]
    fn test_parse_directory_rejects_short_header() {
        let err = FontData::new(vec![0x00, 0x01, 0x00]).err().unwrap();
        assert!(matches!(err, FontError::UnexpectedEndOfData { .. }));
    }
}
