use once_cell::sync::OnceCell;
use rfont_types::{FontError, ReadBytes, Reader, TableRecord, Tag};
use std::collections::HashMap;

/// 原始字体数据和目录信息
pub struct FontData {
    data: Vec<u8>,
    table_map: HashMap<Tag, TableRecord>,
    // 懒加载缓存：存储已解析的表数据
    parsed_tables: HashMap<Tag, OnceCell<Vec<u8>>>,
}

impl FontData {
    /// 从字节向量创建（拥有所有权）
    pub fn new(data: Vec<u8>) -> Result<Self, FontError> {
        let table_map = Self::parse_directory(&data)?;
        let parsed_tables = table_map
            .keys()
            .map(|tag| (*tag, OnceCell::new()))
            .collect();
        Ok(FontData {
            data,
            table_map,
            parsed_tables,
        })
    }

    /// 解析字体目录
    fn parse_directory(data: &[u8]) -> Result<HashMap<Tag, TableRecord>, FontError> {
        let mut reader = Reader::new(data);

        // 读取 Offset Table
        let _sfnt_version = reader.read_u32()?;
        let num_tables = reader.read_u16()?;
        let _search_range = reader.read_u16()?;
        let _entry_selector = reader.read_u16()?;
        let _range_shift = reader.read_u16()?;

        // 读取 Table Directory
        let mut table_map = HashMap::new();
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
            table_map.insert(tag, record);
        }

        Ok(table_map)
    }

    /// 获取表的原始字节（带懒加载缓存）
    pub fn get_table_bytes(&self, tag: Tag) -> Option<&[u8]> {
        self.table_map.get(&tag).map(|record| {
            // 尝试从缓存获取
            if let Some(cell) = self.parsed_tables.get(&tag) {
                if let Some(cached) = cell.get() {
                    return cached.as_slice();
                }
            }

            // 从原始数据提取并缓存
            let start = record.offset as usize;
            let end = start + record.length as usize;
            &self.data[start..end]
        })
    }

    /// 预加载指定的表到缓存中
    pub fn preload_table(&self, tag: Tag) -> Result<(), FontError> {
        if let Some(record) = self.table_map.get(&tag) {
            if let Some(cell) = self.parsed_tables.get(&tag) {
                let start = record.offset as usize;
                let end = start + record.length as usize;
                let data = self.data[start..end].to_vec();
                cell.set(data)
                    .map_err(|_| FontError::Generic("Failed to cache table".to_string()))?;
            }
        }
        Ok(())
    }

    /// 预加载多个表
    pub fn preload_tables(&self, tags: &[Tag]) -> Result<(), FontError> {
        for &tag in tags {
            self.preload_table(tag)?;
        }
        Ok(())
    }

    /// 通用表解析方法
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
    pub fn get_table_records(&self) -> &HashMap<Tag, TableRecord> {
        &self.table_map
    }
}
