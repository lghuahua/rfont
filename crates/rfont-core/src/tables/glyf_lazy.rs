use crate::tables::glyf::{GlyfRecord, GlyphData};
use rfont_types::{FontError, Reader};
use std::collections::{BTreeMap, HashSet};

/// 字体中字形数据的懒加载器
///
/// 这个结构提供了按需解析字形的能力，避免一次性加载所有字形到内存。
/// 特别适用于子集化场景，只需要处理少量字形时。
pub struct GlyfLazyLoader<'a> {
    /// glyf 表的原始字节数据
    glyf_data: &'a [u8],
    /// loca 表的偏移量数组
    loca_offsets: &'a [u32],
    /// 缓存已解析的字形记录（可选）
    cache: Option<Vec<Option<GlyfRecord>>>,
}

/// 字形数据及其原始字节（用于零拷贝优化）
#[derive(Debug, Clone)]
pub struct GlyphWithRaw<'a> {
    /// 解析后的字形记录
    pub record: GlyfRecord,
    /// 原始字节数据（零拷贝引用）
    pub raw_bytes: &'a [u8],
}

impl<'a> GlyfLazyLoader<'a> {
    /// 创建一个新的懒加载器
    pub fn new(glyf_data: &'a [u8], loca_offsets: &'a [u32]) -> Self {
        Self {
            glyf_data,
            loca_offsets,
            cache: None,
        }
    }

    /// 创建带缓存的懒加载器
    ///
    /// 当需要多次访问相同字形时，使用缓存可以提高性能。
    pub fn with_cache(glyf_data: &'a [u8], loca_offsets: &'a [u32]) -> Self {
        let num_glyphs = if loca_offsets.is_empty() {
            0
        } else {
            loca_offsets.len() - 1
        };

        Self {
            glyf_data,
            loca_offsets,
            cache: Some(vec![None; num_glyphs]),
        }
    }

    /// 解析单个字形并返回原始字节（零拷贝优化）
    ///
    /// # 参数
    /// - `glyph_id`: 字形 ID
    ///
    /// # 返回值
    /// 包含解析记录和原始字节的结构体
    pub fn load_glyph_with_raw(
        &self,
        glyph_id: u16,
    ) -> Result<Option<GlyphWithRaw<'_>>, FontError> {
        // 检查范围
        if self.loca_offsets.len() < 2 || glyph_id as usize >= self.loca_offsets.len() - 1 {
            return Ok(None);
        }

        let start = self.loca_offsets[glyph_id as usize];
        let end = self.loca_offsets[glyph_id as usize + 1];

        // 空字形
        if start >= end {
            return Ok(Some(GlyphWithRaw {
                record: GlyfRecord {
                    glyph_id,
                    data: GlyphData::Empty,
                },
                raw_bytes: &[],
            }));
        }

        // 边界检查
        if end as usize > self.glyf_data.len() {
            return Err(FontError::unexpected_end(
                end as usize,
                0,
                "GlyfLazyTable::load_glyph_with_raw",
            ));
        }

        let glyph_data = &self.glyf_data[start as usize..end as usize];
        let mut reader = Reader::new(glyph_data);

        // 解析字形
        let record = GlyfRecord::parse(&mut reader, glyph_id)?;

        Ok(Some(GlyphWithRaw {
            record,
            raw_bytes: glyph_data,
        }))
    }

    /// 解析单个字形（懒加载）
    ///
    /// # 参数
    /// - `glyph_id`: 字形 ID
    ///
    /// # 返回值
    /// 解析后的字形记录，如果字形为空或不存在则返回 None
    pub fn load_glyph(&self, glyph_id: u16) -> Result<Option<GlyfRecord>, FontError> {
        // 检查范围
        if self.loca_offsets.len() < 2 || glyph_id as usize >= self.loca_offsets.len() - 1 {
            return Ok(None);
        }

        let start = self.loca_offsets[glyph_id as usize];
        let end = self.loca_offsets[glyph_id as usize + 1];

        // 空字形
        if start >= end {
            return Ok(Some(GlyfRecord {
                glyph_id,
                data: GlyphData::Empty,
            }));
        }

        // 边界检查
        if end as usize > self.glyf_data.len() {
            return Err(FontError::unexpected_end(
                end as usize,
                0,
                "GlyfLazyTable::load_glyph",
            ));
        }

        let glyph_data = &self.glyf_data[start as usize..end as usize];
        let mut reader = Reader::new(glyph_data);

        // 解析字形
        let record = GlyfRecord::parse(&mut reader, glyph_id)?;
        Ok(Some(record))
    }

    /// 解析单个字形并使用缓存
    pub fn load_glyph_cached(&mut self, glyph_id: u16) -> Result<Option<GlyfRecord>, FontError> {
        // 检查缓存是否初始化
        if self.cache.is_none() {
            return Err(FontError::new("缓存未初始化，请使用 with_cache 创建"));
        }

        let cache = self.cache.as_ref().unwrap();

        // 检查范围
        if glyph_id as usize >= cache.len() {
            return Ok(None);
        }

        // 如果缓存中已有，直接返回克隆
        if let Some(ref record) = cache[glyph_id as usize] {
            return Ok(Some(record.clone()));
        }

        // 先解析字形（不持有缓存的引用）
        let record = self.load_glyph(glyph_id)?;

        // 然后缓存结果
        if let Some(cache) = self.cache.as_mut() {
            cache[glyph_id as usize] = record.clone();
        }

        Ok(record)
    }

    /// 批量解析指定的字形
    pub fn load_glyphs(&self, glyph_ids: &[u16]) -> Result<Vec<GlyfRecord>, FontError> {
        let mut records = Vec::with_capacity(glyph_ids.len());

        for &glyph_id in glyph_ids {
            if let Some(record) = self.load_glyph(glyph_id)? {
                records.push(record);
            }
        }

        Ok(records)
    }

    /// 解析复合字形的依赖关系（使用懒加载）
    ///
    /// 这是推荐的子集化依赖解析方式：
    /// 1. 只解析需要的字形，而不是全部
    /// 2. 遇到复合字形时才递归解析其组件
    /// 3. 避免不必要的解析工作
    ///
    /// # 参数
    /// - `initial_glyphs`: 初始需要的字形 ID 列表
    ///
    /// # 返回值
    /// 包含所有必需字形 ID 的 HashSet
    pub fn resolve_dependencies(&self, initial_glyphs: &[u16]) -> Result<HashSet<u16>, FontError> {
        let mut needed_glyphs: HashSet<u16> = initial_glyphs.iter().cloned().collect();
        let mut to_process: Vec<u16> = initial_glyphs.to_vec();

        while let Some(glyph_id) = to_process.pop() {
            // 懒加载：只在需要时才解析
            let record = match self.load_glyph(glyph_id)? {
                Some(r) => r,
                None => continue,
            };

            // 如果是复合字形，提取所有组件
            if let GlyphData::Composite(composite) = &record.data {
                for component in &composite.components {
                    let component_glyph_index = component.glyph_index;

                    // 如果这个组件还没被处理过，添加到待处理队列
                    if needed_glyphs.insert(component_glyph_index) {
                        to_process.push(component_glyph_index);
                    }
                }
            }
            // 简单字形或空字形没有依赖，跳过
        }

        Ok(needed_glyphs)
    }

    /// 一次性解析依赖并提取字形数据（极致优化版本）
    ///
    /// 合并了 `resolve_dependencies` 和 `extract_glyf_and_loca` 的功能，
    /// 在一次遍历中完成所有工作：
    /// 1. 解析复合字形依赖
    /// 2. 缓存解析后的字形记录 + 原始字节
    /// 3. 直接从缓存构建 loca/glyf 表（零拷贝）
    ///
    /// # 参数
    /// - `initial_glyphs`: 初始需要的字形 ID 列表
    ///
    /// # 返回值
    /// - `(Vec<u16>, Vec<u8>, Vec<u8>)`:
    ///   - 完整的字形 ID 列表（包含依赖，已排序）
    ///   - loca 表数据
    ///   - glyf 表数据
    ///
    /// # 性能优势
    /// - **零重复 I/O**：每个字形只读取一次
    /// - **解析结果缓存**：可用于后续 WOFF2 转换
    /// - **提升缓存命中率**：连续访问同一块数据
    /// - **减少内存分配**：避免中间数据结构
    pub fn resolve_and_extract(
        &self,
        initial_glyphs: &[u16],
    ) -> Result<(Vec<u16>, Vec<u8>, Vec<u8>), FontError> {
        use std::collections::{BTreeMap, HashSet};

        let mut needed_glyphs: HashSet<u16> = initial_glyphs.iter().cloned().collect();
        let mut to_process: Vec<u16> = initial_glyphs.to_vec();

        // 使用 BTreeMap 缓存解析结果并保持有序
        let mut parsed_glyphs: BTreeMap<u16, GlyphWithRaw<'_>> = BTreeMap::new();

        // 第一阶段：解析依赖关系并缓存结果（含原始字节）
        while let Some(glyph_id) = to_process.pop() {
            // 跳过已解析的字形
            if parsed_glyphs.contains_key(&glyph_id) {
                continue;
            }

            let glyph_with_raw = match self.load_glyph_with_raw(glyph_id)? {
                Some(r) => r,
                None => continue,
            };

            // 如果是复合字形，递归添加组件
            if let GlyphData::Composite(composite) = &glyph_with_raw.record.data {
                for component in &composite.components {
                    let component_glyph_index = component.glyph_index;
                    if needed_glyphs.insert(component_glyph_index) {
                        to_process.push(component_glyph_index);
                    }
                }
            }

            // 缓存解析结果 + 原始字节（关键优化：避免二次读取）
            parsed_glyphs.insert(glyph_id, glyph_with_raw);
        }

        // 第二阶段：直接从缓存的原始字节构建 loca/glyf 表
        let sorted_glyphs: Vec<u16> = parsed_glyphs.keys().cloned().collect();
        let (loca_data, glyf_data) = Self::build_tables_from_cached(&parsed_glyphs)?;

        Ok((sorted_glyphs, loca_data, glyf_data))
    }

    /// 从缓存的字形数据构建 loca 和 glyf 表（零拷贝）
    ///
    /// # 参数
    /// - `cached_glyphs`: 缓存的字形数据（包含原始字节）
    ///
    /// # 返回值
    /// - `(Vec<u8>, Vec<u8>)`: loca 表数据和 glyf 表数据
    fn build_tables_from_cached(
        cached_glyphs: &BTreeMap<u16, GlyphWithRaw<'_>>,
    ) -> Result<(Vec<u8>, Vec<u8>), FontError> {
        let num_glyphs = cached_glyphs.len();
        let mut new_loca_offsets = Vec::with_capacity(num_glyphs + 1);
        let mut new_glyf_data = Vec::new();
        let mut current_offset = 0u32;

        for (glyph_id, glyph_with_raw) in cached_glyphs {
            new_loca_offsets.push(current_offset);

            // 跳过 .notdef (glyph_id == 0) 的轮廓数据
            if *glyph_id == 0 {
                continue;
            }

            // 直接使用缓存的原始字节（零拷贝）
            if !glyph_with_raw.raw_bytes.is_empty() {
                new_glyf_data.extend_from_slice(glyph_with_raw.raw_bytes);
                current_offset += glyph_with_raw.raw_bytes.len() as u32;
            }
        }

        // 添加最后一个偏移量
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_loader_empty() {
        let glyf_data: &[u8] = &[];
        let loca_offsets: &[u32] = &[];

        let loader = GlyfLazyLoader::new(glyf_data, loca_offsets);
        let result = loader.load_glyph(0).expect("加载失败");
        assert!(result.is_none());
    }

    #[test]
    fn test_lazy_loader_simple_glyph() {
        // 创建一个完整的简单字形数据
        let glyf_data = vec![
            0x00, 0x01, // num_contours = 1 (简单字形)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, // bbox: (0,0) to (100,100)
            0x00, 0x03, // end_pts_of_contours: [3] (4个点)
            0x00, 0x00, // instruction_length = 0
            // flags (4个点): on-curve + x-short+same + y-short+same
            0x37, 0x37, 0x37, 0x27, // x coordinates: 0, 100, 0, -100 (相对增量)
            0x00, 0x64, 0x00, 0x64, // y coordinates: 0, 0, 100, 0
            0x00, 0x00, 0x64, 0x00,
        ];
        let loca_offsets = vec![0, glyf_data.len() as u32];

        let loader = GlyfLazyLoader::new(&glyf_data, &loca_offsets);
        let result = loader.load_glyph(0).expect("加载失败");
        assert!(result.is_some());

        let record = result.unwrap();
        assert_eq!(record.glyph_id, 0);
        assert!(matches!(record.data, GlyphData::Simple(_)));
    }

    #[test]
    fn test_resolve_dependencies_no_composite() {
        let glyf_data: &[u8] = &[];
        let loca_offsets = vec![0, 0, 0]; // 两个空字形

        let loader = GlyfLazyLoader::new(glyf_data, &loca_offsets);
        let initial = vec![0, 1];

        let result = loader.resolve_dependencies(&initial).expect("解析失败");
        assert_eq!(result.len(), 2);
        assert!(result.contains(&0));
        assert!(result.contains(&1));
    }
}
