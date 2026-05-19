use crate::tables::glyf::{GlyfRecord, GlyphData};
use rfont_types::{FontError, Reader};
use std::collections::HashSet;

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
            return Err(FontError::UnexpectedEndOfData {
                offset: end as usize,
                needed: 0,
            });
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

    /// 解析复合字形的依赖关系（使用缓存版本）
    pub fn resolve_dependencies_cached(
        &mut self,
        initial_glyphs: &[u16],
    ) -> Result<HashSet<u16>, FontError> {
        let mut needed_glyphs: HashSet<u16> = initial_glyphs.iter().cloned().collect();
        let mut to_process: Vec<u16> = initial_glyphs.to_vec();
        
        while let Some(glyph_id) = to_process.pop() {
            let record = match self.load_glyph_cached(glyph_id)? {
                Some(r) => r.clone(), // 需要 clone 以避免借用问题
                None => continue,
            };
            
            if let GlyphData::Composite(composite) = &record.data {
                for component in &composite.components {
                    let component_glyph_index = component.glyph_index;
                    
                    if needed_glyphs.insert(component_glyph_index) {
                        to_process.push(component_glyph_index);
                    }
                }
            }
        }
        
        Ok(needed_glyphs)
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
            0x37, 0x37, 0x37, 0x27,
            // x coordinates: 0, 100, 0, -100 (相对增量)
            0x00, 0x64, 0x00, 0x64,
            // y coordinates: 0, 0, 100, 0
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
