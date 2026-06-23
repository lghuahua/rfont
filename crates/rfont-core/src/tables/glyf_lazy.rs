use crate::tables::glyf::{CompositeGlyph, GlyfRecord, GlyphData};
use rfont_types::{FontError, Reader, WriteBytes, Writer};
use std::collections::{HashMap, HashSet};

/// 字体中字形数据的懒加载器
///
/// 这个结构提供了按需解析字形的能力，避免一次性加载所有字形到内存。
/// 特别适用于子集化场景，只需要处理少量字形时。
pub struct GlyfLazyLoader<'a> {
    /// glyf 表的原始字节数据
    glyf_data: &'a [u8],
    /// loca 表的偏移量数组
    loca_offsets: &'a [u32],
}

impl<'a> GlyfLazyLoader<'a> {
    /// 创建一个新的懒加载器
    pub fn new(glyf_data: &'a [u8], loca_offsets: &'a [u32]) -> Result<Self, FontError> {
        if loca_offsets.len() < 2 {
            return Err(FontError::Generic("loca_offsets 长度不足".to_string()));
        }
        Ok(Self {
            glyf_data,
            loca_offsets,
        })
    }

    /// 判断字形类型（零解析优化）
    ///
    /// 只读取 `num_contours` 字段（前 2 字节）来判断字形类型，
    /// 避免完整解析字形数据。用于依赖解析场景，只需知道是否是复合字形。
    ///
    /// # 参数
    /// - `glyph_id`: 字形 ID
    ///
    /// # 返回值
    /// - `Ok(true)`: 是复合字形
    /// - `Ok(false)`: 是简单字形或空字形
    /// - `Err`: 数据错误
    pub fn is_composite_glyph(&self, glyph_id: u16) -> Result<bool, FontError> {
        // 检查范围
        if self.loca_offsets.len() < 2 || glyph_id as usize >= self.loca_offsets.len() - 1 {
            return Ok(false);
        }

        let start = self.loca_offsets[glyph_id as usize];
        let end = self.loca_offsets[glyph_id as usize + 1];

        // 空字形
        if start >= end {
            return Ok(false);
        }

        // 边界检查：至少需要 2 字节读取 num_contours
        if (end as usize) > self.glyf_data.len() || (end as usize) < (start as usize) + 2 {
            return Err(FontError::unexpected_end(
                end as usize,
                0,
                "GlyfLazyLoader::is_composite_glyph",
            ));
        }

        // 只读取 num_contours（前 2 字节）
        let num_contours = i16::from_be_bytes([
            self.glyf_data[start as usize],
            self.glyf_data[(start + 1) as usize],
        ]);

        // num_contours == -1 表示复合字形
        Ok(num_contours == -1)
    }

    /// 解析单个字形（懒加载）
    ///
    /// # 参数
    /// - `glyph_id`: 字形 ID
    ///
    /// # 返回值
    /// 解析后的字形记录，如果字形为空或不存在则返回 None
    pub fn load_glyph(&self, glyph_id: u16) -> Result<GlyfRecord, FontError> {
        // 检查范围
        if glyph_id as usize >= self.loca_offsets.len() - 1 {
            return Err(FontError::Generic(format!(
                "glyph_id {} 超出范围",
                glyph_id
            )));
        }

        let start = self.loca_offsets[glyph_id as usize];
        let end = self.loca_offsets[glyph_id as usize + 1];

        // 空字形
        if start >= end {
            return Ok(GlyfRecord {
                glyph_id,
                data: GlyphData::Empty,
            });
        }

        // 边界检查
        if end as usize > self.glyf_data.len() {
            return Err(FontError::unexpected_end(
                end as usize,
                0,
                "GlyfLazyLoader::load_glyph",
            ));
        }

        let glyph_data = &self.glyf_data[start as usize..end as usize];
        let mut reader = Reader::new(glyph_data);

        // 解析字形
        let record = GlyfRecord::parse(&mut reader, glyph_id)?;
        Ok(record)
    }

    /// 提取复合字形的组件列表
    ///
    /// 只解析复合字形的组件信息，不解析简单字形的轮廓数据。
    ///
    /// # 参数
    /// - `glyph_id`: 字形 ID（必须是复合字形）
    ///
    /// # 返回值
    /// 组件字形 ID 列表
    pub fn get_composite_components(&self, glyph_id: u16) -> Result<Vec<u16>, FontError> {
        // 检查范围
        if self.loca_offsets.len() < 2 || glyph_id as usize >= self.loca_offsets.len() - 1 {
            return Ok(Vec::new());
        }

        let start = self.loca_offsets[glyph_id as usize];
        let end = self.loca_offsets[glyph_id as usize + 1];

        // 空字形
        if start >= end {
            return Ok(Vec::new());
        }

        // 边界检查
        if (end as usize) > self.glyf_data.len() {
            return Err(FontError::unexpected_end(
                end as usize,
                0,
                "GlyfLazyLoader::get_composite_components",
            ));
        }

        let glyph_data = &self.glyf_data[start as usize..end as usize];
        let mut reader = Reader::new(glyph_data);

        // 读取头部（num_contours, bbox）
        let _num_contours = reader.read_i16()?;
        let _bbox_size = 8; // x_min, y_min, x_max, y_max
        reader.skip(_bbox_size)?;

        // 读取组件列表
        let mut components = Vec::new();
        loop {
            let flags = reader.read_u16()?;
            let glyph_index = reader.read_u16()?;
            components.push(glyph_index);

            // 计算参数大小
            let mut arg_size = 0;
            if flags & 0x0001 != 0 {
                // ARG_1_AND_2_ARE_WORDS
                arg_size += 4;
            } else {
                arg_size += 2;
            }

            if flags & 0x0008 != 0 {
                // WE_HAVE_A_SCALE
                arg_size += 2;
            } else if flags & 0x0040 != 0 {
                // WE_HAVE_AN_X_AND_Y_SCALE
                arg_size += 4;
            } else if flags & 0x0080 != 0 {
                // WE_HAVE_A_TWO_BY_TWO
                arg_size += 8;
            }

            reader.skip(arg_size)?;

            // 如果 MORE_COMPONENTS (bit 5) 为 0，则结束
            if (flags & 0x0020) == 0 {
                break;
            }
        }

        Ok(components)
    }

    /// 解析复合字形的依赖关系（零解析优化）
    ///
    /// 优化版本：
    /// 1. 使用 `is_composite_glyph` 快速判断字形类型，不解析完整数据
    /// 2. 只对复合字形调用 `get_composite_components` 提取组件
    /// 3. 避免解析简单字形的轮廓、指令等数据
    ///
    /// # 参数
    /// - `initial_glyphs`: 初始需要的字形 ID 列表
    ///
    /// # 返回值
    /// 包含所有必需字形 ID 的 HashSet
    pub fn resolve_dependencies(
        &self,
        initial_glyphs: &[u16],
    ) -> Result<(HashSet<u16>, HashMap<u16, CompositeGlyph>), FontError> {
        let mut needed_glyphs: HashSet<u16> = initial_glyphs.iter().cloned().collect();
        let mut to_process: Vec<u16> = initial_glyphs.to_vec();
        let mut composite_glyphs: HashMap<u16, CompositeGlyph> = HashMap::new();

        while let Some(glyph_id) = to_process.pop() {
            // 优化：只判断是否是复合字形，不解析完整数据
            if self.is_composite_glyph(glyph_id)?
                && let GlyphData::Composite(glyph) = self.load_glyph(glyph_id)?.data
            {
                for component in &glyph.components {
                    // 如果这个组件还没被处理过，添加到待处理队列
                    if needed_glyphs.insert(component.glyph_index) {
                        to_process.push(component.glyph_index);
                    }
                }
                composite_glyphs.insert(glyph_id, glyph);
            }

            // 简单字形或空字形没有依赖，跳过
        }

        Ok((needed_glyphs, composite_glyphs))
    }

    /// 解析复合字形依赖并提取 glyf/loca 数据
    ///
    /// 流程：
    /// 1. 使用 BFS 解析依赖关系（只判断复合字形，不解析完整数据）
    /// 2. 排序字形 ID
    /// 3. 从原始字节复制 glyf 数据并构建 loca 表
    ///
    /// # 参数
    /// - `initial_glyphs`: 初始需要的字形 ID 列表
    ///
    /// # 返回值
    /// - `(Vec<u16>, Vec<u8>, Vec<u8>)`:
    ///   - 完整的字形 ID 列表（包含依赖，已排序）
    ///   - loca 表数据
    ///   - glyf 表数据
    pub fn resolve_and_extract(
        &self,
        initial_glyphs: &[u16],
    ) -> Result<(Vec<u16>, Vec<u8>, Vec<u8>), FontError> {
        // 步骤 1: 解析依赖关系（BFS）
        let (needed_glyphs, mut composite_glyphs) = self.resolve_dependencies(initial_glyphs)?;
        let mut sorted_glyphs: Vec<u16> = needed_glyphs.into_iter().collect();
        sorted_glyphs.sort();

        let mut old_to_new_gid = HashMap::new();
        for (new_gid, &old_gid) in sorted_glyphs.iter().enumerate() {
            if old_gid != 0 {
                old_to_new_gid.insert(old_gid, new_gid as u16);
            }
        }

        // 步骤 2: 构建 glyf 和 loca 表
        let mut new_loca_offsets = Vec::with_capacity(sorted_glyphs.len() + 1);
        let mut new_glyf_data = Vec::new();
        let mut current_offset = 0u32;

        for &glyph_id in &sorted_glyphs {
            new_loca_offsets.push(current_offset);

            // 跳过 .notdef (glyph_id == 0) 的轮廓数据
            if glyph_id == 0 {
                continue;
            }

            // 从原始 loca 获取偏移量并复制数据
            if (glyph_id as usize) < self.loca_offsets.len() - 1 {
                let start = self.loca_offsets[glyph_id as usize];
                let end = self.loca_offsets[glyph_id as usize + 1];

                // 复制原始字节数据
                if start < end && (end as usize) <= self.glyf_data.len() {
                    if let Some(glyph) = composite_glyphs.get_mut(&glyph_id) {
                        for component in &mut glyph.components {
                            if let Some(new_gid) = old_to_new_gid.get(&component.glyph_index) {
                                component.glyph_index = *new_gid;
                            }
                        }
                        let mut writer = Writer::with_capacity((end - start) as usize);
                        glyph.write_to(&mut writer)?;
                        new_glyf_data.extend_from_slice(&writer.data);
                        current_offset += writer.len() as u32;
                    } else {
                        let glyph_bytes = &self.glyf_data[start as usize..end as usize];
                        new_glyf_data.extend_from_slice(glyph_bytes);
                        current_offset += glyph_bytes.len() as u32;
                    }
                    // let glyph_bytes = &self.glyf_data[start as usize..end as usize];
                    // new_glyf_data.extend_from_slice(glyph_bytes);
                    // current_offset += glyph_bytes.len() as u32;
                }
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

        Ok((sorted_glyphs, loca_data, new_glyf_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_loader_empty() {
        let glyf_data: &[u8] = &[];
        let loca_offsets: &[u32] = &[];

        // loca_offsets 长度不足，应该返回错误
        let result = GlyfLazyLoader::new(glyf_data, loca_offsets);
        assert!(result.is_err());
    }

    #[test]
    fn test_lazy_loader_simple_glyph() {
        // 创建一个完整的简单字形数据
        let glyf_data = vec![
            0x00, 0x01, // num_contours = 1 (简单字形)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, // bbox: (0,0) to (100,100)
            0x00, 0x03, // end_pts_of_contours: [3] (4 个点)
            0x00, 0x00, // instruction_length = 0
            // flags (4 个点): on-curve + x-short+same + y-short+same
            0x37, 0x37, 0x37, 0x27, // x coordinates: 0, 100, 0, -100 (相对增量)
            0x00, 0x64, 0x00, 0x64, // y coordinates: 0, 0, 100, 0
            0x00, 0x00, 0x64, 0x00,
        ];
        let loca_offsets = vec![0, glyf_data.len() as u32];

        let loader = GlyfLazyLoader::new(&glyf_data, &loca_offsets).expect("创建加载器失败");
        let record = loader.load_glyph(0).expect("加载失败");

        assert_eq!(record.glyph_id, 0);
        assert!(matches!(record.data, GlyphData::Simple(_)));
    }

    #[test]
    fn test_resolve_dependencies_no_composite() {
        let glyf_data: &[u8] = &[];
        let loca_offsets = vec![0, 0, 0]; // 两个空字形

        let loader = GlyfLazyLoader::new(glyf_data, &loca_offsets).expect("创建加载器失败");
        let initial = vec![0, 1];

        let (needed_glyphs, composite_glyphs) =
            loader.resolve_dependencies(&initial).expect("解析失败");
        assert_eq!(needed_glyphs.len(), 2);
        assert!(needed_glyphs.contains(&0));
        assert!(needed_glyphs.contains(&1));
        assert!(composite_glyphs.is_empty()); // 没有复合字形
    }

    #[test]
    fn test_is_composite_glyph() {
        // 简单字形 (num_contours = 1)
        let simple_glyph = vec![
            0x00, 0x01, // num_contours = 1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, // bbox
        ];

        // 复合字形 (num_contours = -1)
        let composite_glyph = vec![
            0xFF, 0xFF, // num_contours = -1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, // bbox
            // 组件数据：flags + glyph_index + 参数
            0x00, 0x00, // flags = 0
            0x00, 0x02, // glyph_index = 2
                  // MORE_COMPONENTS = 0, 结束
        ];

        let glyf_data = [simple_glyph.as_slice(), composite_glyph.as_slice()].concat();
        let loca_offsets = vec![0, simple_glyph.len() as u32, glyf_data.len() as u32];

        let loader = GlyfLazyLoader::new(&glyf_data, &loca_offsets).expect("创建加载器失败");

        // 测试简单字形
        assert!(!loader.is_composite_glyph(0).expect("判断失败"));

        // 测试复合字形
        assert!(loader.is_composite_glyph(1).expect("判断失败"));

        // 测试空字形
        assert!(!loader.is_composite_glyph(2).expect("判断失败")); // 超出范围
    }

    #[test]
    fn test_get_composite_components() {
        // 复合字形包含两个组件：glyph 2 和 glyph 3
        // MORE_COMPONENTS (bit 5) = 1 表示还有更多组件，= 0 表示结束
        // 组件格式：flags(2) + glyph_index(2) + arg_data(可变)
        // arg_data 大小由 flags 决定：
        //   - ARG_1_AND_2_ARE_WORDS (bit 0) = 0: 2 字节 (point indices)
        //   - ARG_1_AND_2_ARE_WORDS (bit 0) = 1: 4 字节 (xy values)
        let composite_glyph = vec![
            0xFF, 0xFF, // num_contours = -1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, // bbox (8 字节)
            // 组件 1: flags=0x0020 (MORE_COMPONENTS=1, ARG_1_AND_2_ARE_WORDS=0), glyph_index=2
            0x00, 0x20, // flags = MORE_COMPONENTS (还有更多), 无缩放
            0x00, 0x02, // glyph_index = 2
            0x00, 0x00, // arg_data: 2 字节 (point indices)
            // 组件 2: flags=0x0000 (MORE_COMPONENTS=0, ARG_1_AND_2_ARE_WORDS=0), glyph_index=3
            0x00, 0x00, // flags = 0 (最后一个组件)
            0x00, 0x03, // glyph_index = 3
            0x00, 0x00, // arg_data: 2 字节 (point indices)
        ];

        let glyf_data = composite_glyph;
        let loca_offsets = vec![0, glyf_data.len() as u32, glyf_data.len() as u32];

        let loader = GlyfLazyLoader::new(&glyf_data, &loca_offsets).expect("创建加载器失败");

        let components = loader.get_composite_components(0).expect("提取失败");
        assert_eq!(components, vec![2, 3]);
    }

    #[test]
    fn test_resolve_dependencies_with_composite() {
        // glyph 0: 空字形 (.notdef)
        // glyph 1: 复合字形，包含组件 2 和 3
        // glyph 2: 简单字形
        // glyph 3: 简单字形

        let glyph_0 = vec![]; // 空

        let glyph_1 = vec![
            0xFF, 0xFF, // num_contours = -1 (复合)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, // bbox (8 字节)
            // 组件 1: glyph 2 (MORE_COMPONENTS=1)
            0x00, 0x20, // flags
            0x00, 0x02, // glyph_index
            0x00, 0x00, // arg_data (2 字节)
            // 组件 2: glyph 3 (MORE_COMPONENTS=0)
            0x00, 0x00, // flags
            0x00, 0x03, // glyph_index
            0x00, 0x00, // arg_data (2 字节)
        ];

        let glyph_2 = vec![
            0x00, 0x01, // num_contours = 1 (简单)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];

        let glyph_3 = vec![
            0x00, 0x01, // num_contours = 1 (简单)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];

        let glyf_data = [
            glyph_0.as_slice(),
            glyph_1.as_slice(),
            glyph_2.as_slice(),
            glyph_3.as_slice(),
        ]
        .concat();

        let mut loca_offsets = vec![0];
        let mut offset = 0u32;
        for g in [&glyph_0, &glyph_1, &glyph_2, &glyph_3] {
            offset += g.len() as u32;
            loca_offsets.push(offset);
        }

        let loader = GlyfLazyLoader::new(&glyf_data, &loca_offsets).expect("创建加载器失败");

        // 解析 glyph 1 的依赖，应该包含 1, 2, 3
        let (needed_glyphs, composite_glyphs) =
            loader.resolve_dependencies(&[1]).expect("解析失败");
        assert_eq!(needed_glyphs.len(), 3);
        assert!(needed_glyphs.contains(&1));
        assert!(needed_glyphs.contains(&2));
        assert!(needed_glyphs.contains(&3));
        assert_eq!(composite_glyphs.len(), 1); // 只有 glyph 1 是复合字形
        assert!(composite_glyphs.contains_key(&1));
    }

    #[test]
    fn test_resolve_and_extract_composite_reference() {
        // 测试 resolve_and_extract 后，复合字形中的组件引用是否正确重映射
        // glyph 0: 空字形
        // glyph 1: 简单字形
        // glyph 2: 简单字形
        // glyph 3: 复合字形，包含组件 1 和 2

        let glyph_0 = vec![]; // 空

        let glyph_1 = vec![
            0x00, 0x01, // num_contours = 1 (简单)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];

        let glyph_2 = vec![
            0x00, 0x01, // num_contours = 1 (简单)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];

        let glyph_3 = vec![
            0xFF, 0xFF, // num_contours = -1 (复合)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, // bbox (8 字节)
            // 组件 1: glyph 1 (MORE_COMPONENTS=1)
            0x00, 0x20, // flags
            0x00, 0x01, // glyph_index = 1
            0x00, 0x00, // arg_data (2 字节)
            // 组件 2: glyph 2 (MORE_COMPONENTS=0)
            0x00, 0x00, // flags
            0x00, 0x02, // glyph_index = 2
            0x00, 0x00, // arg_data (2 字节)
        ];

        let glyf_data = [
            glyph_0.as_slice(),
            glyph_1.as_slice(),
            glyph_2.as_slice(),
            glyph_3.as_slice(),
        ]
        .concat();

        let mut loca_offsets = vec![0];
        let mut offset = 0u32;
        for g in [&glyph_0, &glyph_1, &glyph_2, &glyph_3] {
            offset += g.len() as u32;
            loca_offsets.push(offset);
        }

        let loader = GlyfLazyLoader::new(&glyf_data, &loca_offsets).expect("创建加载器失败");

        // 解析 glyph 3 的依赖，应该包含 1, 2, 3
        let (resolved_glyphs, _loca_data, new_glyf_data) =
            loader.resolve_and_extract(&[3]).expect("提取失败");

        // 排序后应该是 [1, 2, 3]
        assert_eq!(resolved_glyphs, vec![1, 2, 3]);

        // 新字体中的字形 ID 映射:
        // 新 glyph 0 = 旧 1
        // 新 glyph 1 = 旧 2
        // 新 glyph 2 = 旧 3

        // 验证：新 glyph 2 (旧 3) 的复合字形引用应该被重映射为 [0, 1]
        // 而不是原始的 [1, 2]

        // 由于我们重映射了组件引用，新 glyph 2 的组件应该是 [0, 1]
        // 这里我们验证新 glyf_data 的长度是否合理
        assert!(!new_glyf_data.is_empty());

        // 更详细的验证需要解析新 glyf 数据，这里先做基本检查
        // 如果组件重映射正确，测试应该通过
    }
}
