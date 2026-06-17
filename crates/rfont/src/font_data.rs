use once_cell::sync::OnceCell;
use rfont_types::{FontError, ReadBytes, Reader, TableRecord, Tag};
use std::collections::HashMap;

/// 原始字体数据和目录信息
///
/// `FontData` 封装了字体的原始二进制数据和表目录信息。
/// 提供懒加载缓存机制，避免重复解析相同的表数据。
///
/// # 注意
/// 此结构体通常不直接创建，而是通过 `Font::load()` 方法间接使用。
pub struct FontData {
    data: Vec<u8>,
    table_map: HashMap<Tag, TableRecord>,
    // 懒加载缓存：存储已解析的表数据
    parsed_tables: HashMap<Tag, OnceCell<Vec<u8>>>,
}

impl FontData {
    /// 从字节向量创建（拥有所有权）
    ///
    /// 解析字体目录并初始化懒加载缓存。
    ///
    /// # 参数
    /// - `data`: 完整的字体文件数据
    ///
    /// # 返回值
    /// - `Ok(FontData)`: 成功创建的 FontData 实例
    /// - `Err(FontError)`: 如果字体格式无效或目录解析失败
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
    pub(crate) fn parse_directory(data: &[u8]) -> Result<HashMap<Tag, TableRecord>, FontError> {
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

    /// 获取原始字体数据
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// 获取表的原始字节（带懒加载缓存）
    ///
    /// 如果表数据已在缓存中，直接返回；否则从原始数据中提取。
    ///
    /// # 参数
    /// - `tag`: 表标签（如 `Tag(*b"head")`）
    ///
    /// # 返回值
    /// - `Some(&[u8])`: 表的原始字节数据
    /// - `None`: 如果表中不存在
    pub fn get_table_bytes(&self, tag: Tag) -> Option<&[u8]> {
        self.table_map.get(&tag).map(|record| {
            // 尝试从缓存获取
            if let Some(cell) = self.parsed_tables.get(&tag)
                && let Some(cached) = cell.get()
            {
                return cached.as_slice();
            }

            // 从原始数据提取并缓存
            let start = record.offset as usize;
            let end = start + record.length as usize;
            &self.data[start..end]
        })
    }

    /// 预加载指定的表到缓存中
    ///
    /// 强制将表数据加载到缓存中，适合需要多次访问同一表的场景。
    ///
    /// # 参数
    /// - `tag`: 要预加载的表标签
    ///
    /// # 返回值
    /// - `Ok(())`: 预加载成功
    /// - `Err(FontError)`: 如果表不存在或缓存失败
    pub fn preload_table(&self, tag: Tag) -> Result<(), FontError> {
        if let Some(record) = self.table_map.get(&tag)
            && let Some(cell) = self.parsed_tables.get(&tag)
        {
            let start = record.offset as usize;
            let end = start + record.length as usize;
            let data = self.data[start..end].to_vec();
            cell.set(data)
                .map_err(|_| FontError::Generic("Failed to cache table".to_string()))?;
        }
        Ok(())
    }

    /// 预加载多个表
    ///
    /// 批量预加载多个表到缓存中。如果任何一个表预加载失败，会立即返回错误。
    ///
    /// # 参数
    /// - `tags`: 要预加载的表标签列表
    ///
    /// # 返回值
    /// - `Ok(())`: 所有表预加载成功
    /// - `Err(FontError)`: 如果任何表预加载失败
    pub fn preload_tables(&self, tags: &[Tag]) -> Result<(), FontError> {
        for &tag in tags {
            self.preload_table(tag)?;
        }
        Ok(())
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
