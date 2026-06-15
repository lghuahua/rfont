/// WOFF2 Glyf 表转换模块
///
/// 参照 woff2 项目实现 glyf/loca 表的转换，用于提高 Brotli 压缩率。
///
/// 参考: https://github.com/google/woff2/blob/master/src/transform.cc
use crate::tables::glyf::{CompositeGlyph, GlyfRecord, GlyphData, SimpleGlyph};
use rfont_types::{FontError, WriteBytes, Writer};

// ==================== 常量定义 ====================

// ==================== 数据结构 ====================

/// Glyf 编码器
///
/// 将 glyf 表转换为多个独立的流，以提高 Brotli 压缩率。
pub struct GlyfEncoder {
    n_glyphs: u16,
    index_format: u8,
    // 7个主要流
    n_contour_stream: Writer,   // 轮廓数量流
    n_points_stream: Writer,    // 点数流
    flag_byte_stream: Writer,   // 标志位流
    glyph_stream: Writer,       // 字形数据流（坐标三元组）
    composite_stream: Writer,   // 复合字形流
    bbox_bitmap: Vec<u8>,        // BBox 位图
    bbox_stream: Writer,        // BBox 数据流
    instruction_stream: Vec<u8>, // 指令流

    // 重叠位图（可选）
    overlap_bitmap: Vec<u8>,
}

impl GlyfEncoder {
    /// 创建新的编码器
    pub fn new(n_glyphs: u16, index_format: u8) -> Self {
        let bbox_bitmap_size = ((n_glyphs + 31) >> 5) << 2;

        Self {
            n_glyphs,
            index_format,
            n_contour_stream: Writer::new(),
            n_points_stream: Writer::new(),
            flag_byte_stream: Writer::new(),
            glyph_stream: Writer::new(),
            composite_stream: Writer::new(),
            bbox_bitmap: vec![0u8; bbox_bitmap_size.into()],
            bbox_stream: Writer::new(),
            instruction_stream: Vec::new(),
            overlap_bitmap: Vec::new(),
        }
    }

    /// 编码所有字形
    pub fn encode_glyphs(&mut self, glyphs: &[GlyfRecord]) -> Result<(), FontError> {
        for record in glyphs {
            match &record.data {
                GlyphData::Empty => {
                    // 空字形：写入 n_contour = 0
                    self.n_contour_stream.write_u16(0)?;
                }
                GlyphData::Simple(simple) => {
                    self.write_simple_glyph(record.glyph_id, simple)?;
                }
                GlyphData::Composite(composite) => {
                    self.write_composite_glyph(record.glyph_id, composite)?;
                }
            }
        }
        Ok(())
    }

    /// 写入简单字形
    fn write_simple_glyph(&mut self, glyph_id: u16, glyph: &SimpleGlyph) -> Result<(), FontError> {
        // 处理重叠标志
        if glyph.flags.iter().any(|&f| f & 0x40 != 0) {
            self.ensure_overlap_bitmap();
            let byte_idx = (glyph_id >> 3) as usize;
            let bit_idx = glyph_id & 7;
            if byte_idx < self.overlap_bitmap.len() {
                self.overlap_bitmap[byte_idx] |= 0x80 >> bit_idx;
            }
        }

        let num_contours = glyph.num_contours as u16;
        self.n_contour_stream.write_u16(num_contours)?;

        // 条件写入 BBox
        if self.should_write_bbox(glyph) {
            self.write_bbox(glyph_id, glyph.x_min, glyph.y_min, glyph.x_max, glyph.y_max)?;
        }

        // 写入每个轮廓的点数
        let mut prev_end = 0u16;
        for (i, end_pt) in glyph.end_pts_of_contours.iter().enumerate() {
            let num_points = if i == 0 {
                end_pt + 1
            } else {
                end_pt - prev_end
            };
            // Self::write_255_ushort(&mut self.n_points_stream, num_points as usize);
            self.n_points_stream.write_255_ushort(num_points)?;
            prev_end = *end_pt;
        }

        // 使用三元组编码坐标
        let mut last_x = 0i32;
        let mut last_y = 0i32;

        for (flag, x, y) in glyph
            .flags
            .iter()
            .zip(glyph.x_coordinates.iter())
            .zip(glyph.y_coordinates.iter())
            .map(|((f, x), y)| (*f, *x, *y))
        {
            let on_curve = (flag & 0x01) != 0;
            let dx = x as i32 - last_x;
            let dy = y as i32 - last_y;

            self.write_triplet(on_curve, dx, dy)?;

            last_x = x as i32;
            last_y = y as i32;
        }

        // 写入指令
        if glyph.num_contours > 0 {
            self.write_instructions(&glyph.instructions)?;
        }

        Ok(())
    }

    /// 写入复合字形
    fn write_composite_glyph(
        &mut self,
        glyph_id: u16,
        glyph: &CompositeGlyph,
    ) -> Result<(), FontError> {
        // 复合字形标记为 -1
        // Self::write_ushort(&mut self.n_contour_stream, 0xFFFF);
        self.n_contour_stream.write_u16(0xFFFF)?;

        // 写入 BBox
        self.write_bbox(glyph_id, glyph.x_min, glyph.y_min, glyph.x_max, glyph.y_max)?;

        // 写入复合组件数据（需要重新构建原始字节）
        glyph.write_to(&mut self.composite_stream)?;

        Ok(())
    }

    /// 写入三元组编码（核心优化算法）
    fn write_triplet(&mut self, on_curve: bool, dx: i32, dy: i32) -> Result<(), FontError> {
        let abs_x = dx.abs();
        let abs_y = dy.abs();
        let on_curve_bit: u8 = if on_curve { 0 } else { 128 };
        let x_sign_bit: u8 = if dx < 0 { 0 } else { 1 };
        let y_sign_bit: u8 = if dy < 0 { 0 } else { 1 };
        let xy_sign_bits: u8 = x_sign_bit + 2 * y_sign_bit;

        if dx == 0 && abs_y < 1280 {
            // 情况1: X=0, Y小值 → 2字节
            self.flag_byte_stream
                .write_u8(on_curve_bit + ((abs_y & 0xf00) >> 7) as u8 + y_sign_bit)?;
            self.glyph_stream.write_u8((abs_y & 0xff) as u8)?;
        } else if dy == 0 && abs_x < 1280 {
            // 情况2: Y=0, X小值 → 2字节
            self.flag_byte_stream
                .write_u8(on_curve_bit + 10 + ((abs_x & 0xf00) >> 7) as u8 + x_sign_bit)?;
            self.glyph_stream.write_u8((abs_x & 0xff) as u8)?;
        } else if abs_x < 65 && abs_y < 65 {
            // 情况3: X,Y都很小 → 2字节
            self.flag_byte_stream.write_u8(
                on_curve_bit
                    + 20
                    + ((abs_x - 1) & 0x30) as u8
                    + (((abs_y - 1) & 0x30) >> 2) as u8
                    + xy_sign_bits,
            )?;
            self.glyph_stream
                .write_u8((((abs_x - 1) & 0xf) << 4 | ((abs_y - 1) & 0xf)) as u8)?;
        } else if abs_x < 769 && abs_y < 769 {
            // 情况4: X,Y中等 → 3字节
            self.flag_byte_stream.write_u8(
                on_curve_bit
                    + 84
                    + (12 * (((abs_x - 1) & 0x300) >> 8)) as u8
                    + (((abs_y - 1) & 0x300) >> 6) as u8
                    + xy_sign_bits,
            )?;
            self.glyph_stream.write_u8(((abs_x - 1) & 0xff) as u8)?;
            self.glyph_stream.write_u8(((abs_y - 1) & 0xff) as u8)?;
        } else if abs_x < 4096 && abs_y < 4096 {
            // 情况5: X,Y较大 → 4字节
            self.flag_byte_stream
                .write_u8(on_curve_bit + 120 + xy_sign_bits)?;
            self.glyph_stream.write_u8((abs_x >> 4) as u8)?;
            self.glyph_stream
                .write_u8(((abs_x & 0xf) << 4 | (abs_y >> 8)) as u8)?;
            self.glyph_stream.write_u8((abs_y & 0xff) as u8)?;
        } else {
            // 情况6: X,Y很大 → 5字节
            self.flag_byte_stream
                .write_u8(on_curve_bit + 124 + xy_sign_bits)?;
            self.glyph_stream.write_u8((abs_x >> 8) as u8)?;
            self.glyph_stream.write_u8((abs_x & 0xff) as u8)?;
            self.glyph_stream.write_u8((abs_y >> 8) as u8)?;
            self.glyph_stream.write_u8((abs_y & 0xff) as u8)?;
        }
        Ok(())
    }

    /// 写入 BBox
    fn write_bbox(&mut self, glyph_id: u16, x_min: i16, y_min: i16, x_max: i16, y_max: i16) -> Result<(), FontError> {
        // 设置位图中的对应位
        let byte_idx = (glyph_id >> 3) as usize;
        let bit_idx = glyph_id & 7;
        if byte_idx < self.bbox_bitmap.len() {
            self.bbox_bitmap[byte_idx] |= 0x80 >> bit_idx;
        }

        // 写入 BBox 数据
        self.bbox_stream.write_u16(x_min as u16)?;
        self.bbox_stream.write_u16(y_min as u16)?;
        self.bbox_stream.write_u16(x_max as u16)?;
        self.bbox_stream.write_u16(y_max as u16)?;
        Ok(())
    }

    /// 写入指令
    fn write_instructions(&mut self, instructions: &[u8]) -> Result<(), FontError> {
        // 先写入长度（使用 255UShort 编码）
        // Self::write_255_ushort(&mut self.instruction_stream, instructions.len());
        self.glyph_stream.write_255_ushort(instructions.len() as u16)?;
        // 再写入指令数据
        self.instruction_stream.extend_from_slice(instructions);
        Ok(())
    }

/// 判断是否应该写入简单字形的 BBox
///
/// 参考 Google woff2 项目的 ShouldWriteSimpleGlyphBbox 实现：
/// 1. 空字形：只有当 bbox 非零时才写入
/// 2. 非空字形：遍历所有点计算实际 bbox，与预存值比较
///    - 如果一致：不写入（解码器可从坐标推导）
///    - 如果不一致：写入（保证数据正确性）
fn should_write_bbox(&self, glyph: &SimpleGlyph) -> bool {
    println!("should_write_bbox {}, {}, {}, {}", glyph.x_min, glyph.y_min, glyph.x_max, glyph.y_max);
    // 1. 空字形处理
    if glyph.num_contours <= 0 || glyph.end_pts_of_contours.is_empty() {
        return glyph.x_min != 0 || glyph.y_min != 0 || 
               glyph.x_max != 0 || glyph.y_max != 0;
    }
    
    // 2. 遍历所有点，计算实际 bbox
    let mut computed_x_min = i32::MAX;
    let mut computed_y_min = i32::MAX;
    let mut computed_x_max = i32::MIN;
    let mut computed_y_max = i32::MIN;
    
    for (&x, &y) in glyph.x_coordinates.iter().zip(glyph.y_coordinates.iter()) {
        let x = x as i32;
        let y = y as i32;
        computed_x_min = computed_x_min.min(x);
        computed_y_min = computed_y_min.min(y);
        computed_x_max = computed_x_max.max(x);
        computed_y_max = computed_y_max.max(y);
    }

    println!("should_write_bbox computed_x_min: {}, computed_y_min: {}, computed_x_max: {}, computed_y_max: {}", computed_x_min, computed_y_min, computed_x_max, computed_y_max);
    
    // 3. 比较预存 bbox 和计算 bbox
    glyph.x_min as i32 != computed_x_min ||
    glyph.y_min as i32 != computed_y_min ||
    glyph.x_max as i32 != computed_x_max ||
    glyph.y_max as i32 != computed_y_max
}

    /// 确保重叠位图已初始化
    fn ensure_overlap_bitmap(&mut self) {
        if self.overlap_bitmap.is_empty() {
            let size = (self.n_glyphs as usize).div_ceil(8);
            self.overlap_bitmap.resize(size, 0);
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let n_contour_stream_size =  self.n_contour_stream.len() as u32;
        let n_points_stream_size = self.n_points_stream.len() as u32;
        let flag_byte_stream_size = self.flag_byte_stream.len() as u32;
        let glyph_stream_size = self.glyph_stream.len() as u32;
        let composite_stream_size = self.composite_stream.len()  as u32;
        let bbox_bitmap_size = self.bbox_bitmap.len()  as u32;
        let bbox_stream_size = self.bbox_stream.len()  as u32;
        let instruction_stream_size = self.instruction_stream.len()  as u32;
        let overlap_bitmap_size = self.overlap_bitmap.len() as u32;

        tracing::debug!(
            n_contour_stream_size = n_contour_stream_size,
            n_points_stream_size = n_points_stream_size,
            flag_byte_stream_size = flag_byte_stream_size,
            glyph_stream_size = glyph_stream_size,
            composite_stream_size = composite_stream_size,
            bbox_bitmap_size = bbox_bitmap_size,
            bbox_stream_size = bbox_stream_size,
            instruction_stream_size = instruction_stream_size,
            "各流大小统计"
        );

        println!("转换后 glyph_stream: {:?}", self.glyph_stream.data);

        let stream_size = n_contour_stream_size + n_points_stream_size + flag_byte_stream_size + glyph_stream_size + composite_stream_size + bbox_bitmap_size + bbox_stream_size + instruction_stream_size + overlap_bitmap_size;

        let mut result = Vec::with_capacity(stream_size as usize + 36);
        let option_flages: u16 = if self.overlap_bitmap.is_empty() { 0 } else { 1 };
        result.extend_from_slice(&0u16.to_be_bytes()); // version
        result.extend_from_slice(&option_flages.to_be_bytes()); // optionFlags
        result.extend_from_slice(&self.n_glyphs.to_be_bytes());
        result.extend_from_slice(&(self.index_format as u16).to_be_bytes());
        // 写入stream 长度
        result.extend_from_slice(&n_contour_stream_size.to_be_bytes());
        result.extend_from_slice(&n_points_stream_size.to_be_bytes());
        result.extend_from_slice(&flag_byte_stream_size.to_be_bytes());
        result.extend_from_slice(&glyph_stream_size.to_be_bytes());
        result.extend_from_slice(&composite_stream_size.to_be_bytes());
        result.extend_from_slice(&(bbox_stream_size + bbox_bitmap_size).to_be_bytes());
        result.extend_from_slice(&instruction_stream_size.to_be_bytes());
        // 写入stream 数据
        result.extend_from_slice(&self.n_contour_stream.data);
        result.extend_from_slice(&self.n_points_stream.data);
        result.extend_from_slice(&self.flag_byte_stream.data);
        result.extend_from_slice(&self.glyph_stream.data);
        result.extend_from_slice(&self.composite_stream.data);
        result.extend_from_slice(&self.bbox_bitmap);
        result.extend_from_slice(&self.bbox_stream.data);
        result.extend_from_slice(&self.instruction_stream);
        result
    }


}

/// 转换 glyf 和 loca 表
///
/// 这是主要的入口函数，接收 glyf 记录和 loca 偏移量，返回转换后的数据
pub fn transform_glyf_and_loca(
    glyphs: &[GlyfRecord],
    index_format: u8,
) -> Result<(Vec<u8>, Vec<u8>), FontError> {
    let n_glyphs = glyphs.len() as u16;

    tracing::debug!(n_glyphs = n_glyphs, "开始 glyf/loca 转换");

    // 创建编码器并编码所有字形
    let mut encoder = GlyfEncoder::new(n_glyphs, index_format);

    tracing::debug!("开始编码字形");
    encoder.encode_glyphs(glyphs)?;
    tracing::debug!("字形编码完成");

    // 生成转换后的 glyf 数据
    let transformed_glyf = encoder.to_bytes();

    // 计算原始大小
    let original_size: usize = glyphs
        .iter()
        .map(|g| match &g.data {
            GlyphData::Empty => 0,
            GlyphData::Simple(s) => {
                10 + // header
            s.end_pts_of_contours.len() * 2 +
            s.instructions.len() +
            s.flags.len() +
            s.x_coordinates.len() * 2 +
            s.y_coordinates.len() * 2
            }
            GlyphData::Composite(c) => {
                10 + // header
            c.components.len() * 10 // 简化估计
            }
        })
        .sum();

    tracing::debug!(
        original_size = original_size,
        transformed_size = transformed_glyf.len(),
        "glyf 转换完成"
    );

    // 生成转换后的 loca 数据（在 WOFF2 中，loca 被省略，因为可以从其他信息推导）
    // 这里返回一个空的 loca，实际使用时需要根据具体规范调整
    let transformed_loca = Vec::new();

    Ok((transformed_glyf, transformed_loca))
}

#[cfg(test)]
mod tests {
    use rfont_types::Reader;

use crate::tables::woff2_transform::GlyfDecoder;

use super::*;

    #[test]
    fn test_triplet_encoding_zero_x() {
        // 测试 X=0, Y小值的情况
        let mut encoder = GlyfEncoder::new(1, 0);
        encoder.write_triplet(true, 0, 100).unwrap();

        assert_eq!(encoder.flag_byte_stream.len(), 1);
        assert_eq!(encoder.glyph_stream.len(), 1);
    }

    #[test]
    fn test_triplet_encoding_small_values() {
        // 测试 X,Y都很小的情况
        let mut encoder = GlyfEncoder::new(1, 0);
        encoder.write_triplet(true, 10, 20).unwrap();

        assert_eq!(encoder.flag_byte_stream.len(), 1);
        assert_eq!(encoder.glyph_stream.len(), 1);
    }

    #[test]
    fn test_encode_empty_glyph() {
        let glyphs = vec![GlyfRecord {
            glyph_id: 0,
            data: GlyphData::Empty,
        }];

        let (glyf_data, _loca_data) = transform_glyf_and_loca(&glyphs, 0).unwrap();

        // 空字形应该产生一些输出（n_contour = 0）
        assert!(!glyf_data.is_empty());
    }

    #[test]
    fn test_encode_simple_glyph() {
        let glyphs = vec![GlyfRecord {
            glyph_id: 0,
            data: GlyphData::Simple(SimpleGlyph {
                num_contours: 1, x_min: 55, y_min: -50, x_max: 185, y_max: 600,
                end_pts_of_contours: vec![11],
                instructions: vec![],
                flags: vec![54, 54, 53, 52, 39, 55, 6, 6, 21, 20, 23, 7], 
                x_coordinates: vec![65, 77, 77, 77, 55, 185, 175, 163, 163, 163, 175, 55], 
                y_coordinates: vec![21, 211, 306, 490, 590, 600, 533, 335, 231, 71, -40, -50]
            })
        }];
        // 转换字形
        let (glyf_data, loca_data) = transform_glyf_and_loca(&glyphs, 0).unwrap();
        println!("glyf_data: {:?}", glyf_data);
        assert!(loca_data.is_empty());
        assert!(!glyf_data.is_empty());
        // 解码字形
        let v = GlyfDecoder::decode(&glyf_data).unwrap();
        assert!(!v.0.is_empty());
        println!("decode glyf_data: {:?}", v);
        
        let mut reader = Reader::new(&v.0);
        let record = GlyfRecord::parse(&mut reader, 0).unwrap();
        println!("record: {:?}", record);
    }
}
