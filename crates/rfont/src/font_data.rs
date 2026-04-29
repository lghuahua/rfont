use rfont_types::{FontError, Reader, TableRecord, Tag, ReadBytes};
use std::collections::HashMap;

/// 原始字体数据和目录信息
pub struct FontData {
    data: Vec<u8>,
    table_map: HashMap<Tag, TableRecord>,
}

impl FontData {
    /// 从字节向量创建（拥有所有权）
    pub fn new(data: Vec<u8>) -> Result<Self, FontError> {
        let table_map = Self::parse_directory(&data)?;
        Ok(FontData { data, table_map })
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
            let tag_bytes = [reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?];
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
    
    /// 获取表的原始字节
    pub fn get_table_bytes(&self, tag: Tag) -> Option<&[u8]> {
        self.table_map.get(&tag).map(|record| {
            let start = record.offset as usize;
            let end = start + record.length as usize;
            &self.data[start..end]
        })
    }
    
    /// 通用表解析方法
    pub fn parse_table<T>(&self, tag: Tag) -> Result<T, FontError> 
    where 
        T: for<'a> ReadBytes<'a>
    {
        let bytes = self.get_table_bytes(tag)
            .ok_or_else(|| FontError::TableNotFound { tag: format!("{:?}", tag) })?;
        T::read_from(&mut Reader::new(bytes))
    }
    
    /// 获取所有表记录的引用
    pub fn get_table_records(&self) -> &HashMap<Tag, TableRecord> {
        &self.table_map
    }
}
