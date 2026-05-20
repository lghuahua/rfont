/// WOFF2 Glyf 表转换模块
///
/// 参照 woff2 项目实现 glyf/loca 表的转换，用于提高 Brotli 压缩率。
///
/// 参考: https://github.com/google/woff2/blob/master/src/transform.cc
use crate::tables::glyf::{CompositeGlyph, GlyfRecord, GlyphData, SimpleGlyph};
use rfont_types::FontError;

// ==================== 常量定义 ====================

// ==================== 数据结构 ====================

/// Glyf 编码器
///
/// 将 glyf 表转换为多个独立的流，以提高 Brotli 压缩率。
pub struct GlyfEncoder {
    n_glyphs: u16,

    // 7个主要流
    n_contour_stream: Vec<u8>,   // 轮廓数量流
    n_points_stream: Vec<u8>,    // 点数流
    flag_byte_stream: Vec<u8>,   // 标志位流
    glyph_stream: Vec<u8>,       // 字形数据流（坐标三元组）
    composite_stream: Vec<u8>,   // 复合字形流
    bbox_bitmap: Vec<u8>,        // BBox 位图
    bbox_stream: Vec<u8>,        // BBox 数据流
    instruction_stream: Vec<u8>, // 指令流

    // 重叠位图（可选）
    overlap_bitmap: Vec<u8>,
}

impl GlyfEncoder {
    /// 创建新的编码器
    pub fn new(n_glyphs: u16) -> Self {
        let bbox_bitmap_size = (n_glyphs as usize).div_ceil(8);

        Self {
            n_glyphs,
            n_contour_stream: Vec::new(),
            n_points_stream: Vec::new(),
            flag_byte_stream: Vec::new(),
            glyph_stream: Vec::new(),
            composite_stream: Vec::new(),
            bbox_bitmap: vec![0u8; bbox_bitmap_size],
            bbox_stream: Vec::new(),
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
                    Self::write_ushort(&mut self.n_contour_stream, 0);
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
        Self::write_ushort(&mut self.n_contour_stream, num_contours);

        // 条件写入 BBox
        if self.should_write_bbox(glyph) {
            self.write_bbox(glyph_id, glyph.x_min, glyph.y_min, glyph.x_max, glyph.y_max);
        }

        // 写入每个轮廓的点数
        let end_pts = glyph.end_pts_of_contours.iter();
        let mut prev_end = 0u16;
        for &end_pt in end_pts.clone() {
            let num_points = if prev_end == 0 {
                end_pt + 1
            } else {
                end_pt - prev_end
            };
            Self::write_255_ushort(&mut self.n_points_stream, num_points as usize);
            prev_end = end_pt;
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

            self.write_triplet(on_curve, dx, dy);

            last_x = x as i32;
            last_y = y as i32;
        }

        // 写入指令
        if !glyph.instructions.is_empty() {
            self.write_instructions(&glyph.instructions);
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
        Self::write_ushort(&mut self.n_contour_stream, 0xFFFF);

        // 写入 BBox
        self.write_bbox(glyph_id, glyph.x_min, glyph.y_min, glyph.x_max, glyph.y_max);

        // 写入复合组件数据（需要重新构建原始字节）
        self.write_composite_data(glyph)?;

        Ok(())
    }

    /// 写入三元组编码（核心优化算法）
    fn write_triplet(&mut self, on_curve: bool, dx: i32, dy: i32) {
        let abs_x = dx.abs();
        let abs_y = dy.abs();
        let on_curve_bit: u8 = if on_curve { 0 } else { 128 };
        let x_sign_bit: u8 = if dx < 0 { 0 } else { 1 };
        let y_sign_bit: u8 = if dy < 0 { 0 } else { 1 };
        let xy_sign_bits: u8 = x_sign_bit + 2 * y_sign_bit;

        if dx == 0 && abs_y < 1280 {
            // 情况1: X=0, Y小值 → 2字节
            self.flag_byte_stream
                .push(on_curve_bit + ((abs_y & 0xf00) >> 7) as u8 + y_sign_bit);
            self.glyph_stream.push((abs_y & 0xff) as u8);
        } else if dy == 0 && abs_x < 1280 {
            // 情况2: Y=0, X小值 → 2字节
            self.flag_byte_stream
                .push(on_curve_bit + 10 + ((abs_x & 0xf00) >> 7) as u8 + x_sign_bit);
            self.glyph_stream.push((abs_x & 0xff) as u8);
        } else if abs_x < 65 && abs_y < 65 {
            // 情况3: X,Y都很小 → 2字节
            self.flag_byte_stream.push(
                on_curve_bit
                    + 20
                    + ((abs_x - 1) & 0x30) as u8
                    + (((abs_y - 1) & 0x30) >> 2) as u8
                    + xy_sign_bits,
            );
            self.glyph_stream
                .push((((abs_x - 1) & 0xf) << 4 | ((abs_y - 1) & 0xf)) as u8);
        } else if abs_x < 769 && abs_y < 769 {
            // 情况4: X,Y中等 → 3字节
            self.flag_byte_stream.push(
                on_curve_bit
                    + 84
                    + (12 * (((abs_x - 1) & 0x300) >> 8)) as u8
                    + (((abs_y - 1) & 0x300) >> 6) as u8
                    + xy_sign_bits,
            );
            self.glyph_stream.push(((abs_x - 1) & 0xff) as u8);
            self.glyph_stream.push(((abs_y - 1) & 0xff) as u8);
        } else if abs_x < 4096 && abs_y < 4096 {
            // 情况5: X,Y较大 → 4字节
            self.flag_byte_stream
                .push(on_curve_bit + 120 + xy_sign_bits);
            self.glyph_stream.push((abs_x >> 4) as u8);
            self.glyph_stream
                .push(((abs_x & 0xf) << 4 | (abs_y >> 8)) as u8);
            self.glyph_stream.push((abs_y & 0xff) as u8);
        } else {
            // 情况6: X,Y很大 → 5字节
            self.flag_byte_stream
                .push(on_curve_bit + 124 + xy_sign_bits);
            self.glyph_stream.push((abs_x >> 8) as u8);
            self.glyph_stream.push((abs_x & 0xff) as u8);
            self.glyph_stream.push((abs_y >> 8) as u8);
            self.glyph_stream.push((abs_y & 0xff) as u8);
        }
    }

    /// 写入 BBox
    fn write_bbox(&mut self, glyph_id: u16, x_min: i16, y_min: i16, x_max: i16, y_max: i16) {
        // 设置位图中的对应位
        let byte_idx = (glyph_id >> 3) as usize;
        let bit_idx = glyph_id & 7;
        if byte_idx < self.bbox_bitmap.len() {
            self.bbox_bitmap[byte_idx] |= 0x80 >> bit_idx;
        }

        // 写入 BBox 数据
        Self::write_ushort(&mut self.bbox_stream, x_min as u16);
        Self::write_ushort(&mut self.bbox_stream, y_min as u16);
        Self::write_ushort(&mut self.bbox_stream, x_max as u16);
        Self::write_ushort(&mut self.bbox_stream, y_max as u16);
    }

    /// 写入指令
    fn write_instructions(&mut self, instructions: &[u8]) {
        // 先写入长度（使用 255UShort 编码）
        Self::write_255_ushort(&mut self.instruction_stream, instructions.len());
        // 再写入指令数据
        self.instruction_stream.extend_from_slice(instructions);
    }

    /// 写入复合字形数据
    fn write_composite_data(&mut self, glyph: &CompositeGlyph) -> Result<(), FontError> {
        // 将组件重新编码为原始字节格式
        for component in &glyph.components {
            // 写入 flags
            Self::write_ushort(&mut self.composite_stream, component.flags);
            // 写入 glyph_index
            Self::write_ushort(&mut self.composite_stream, component.glyph_index);
            // 写入参数
            if component.flags & 0x0001 != 0 {
                // ARG_1_AND_2_ARE_WORDS: 双字节
                Self::write_short(&mut self.composite_stream, component.argument1);
                Self::write_short(&mut self.composite_stream, component.argument2);
            } else {
                // 单字节
                self.composite_stream.push(component.argument1 as u8);
                self.composite_stream.push(component.argument2 as u8);
            }
        }
        Ok(())
    }

    /// 判断是否应该写入 BBox
    fn should_write_bbox(&self, glyph: &SimpleGlyph) -> bool {
        // 简化策略：如果 bbox 非零则写入
        glyph.x_min != 0 || glyph.y_min != 0 || glyph.x_max != 0 || glyph.y_max != 0
    }

    /// 确保重叠位图已初始化
    fn ensure_overlap_bitmap(&mut self) {
        if self.overlap_bitmap.is_empty() {
            let size = (self.n_glyphs as usize).div_ceil(8);
            self.overlap_bitmap.resize(size, 0);
        }
    }

    /// 辅助函数：写入 UShort (2字节)
    #[inline]
    fn write_ushort(stream: &mut Vec<u8>, value: u16) {
        stream.push((value >> 8) as u8);
        stream.push((value & 0xFF) as u8);
    }

    /// 辅助函数：写入 Short (2字节有符号)
    #[inline]
    fn write_short(stream: &mut Vec<u8>, value: i16) {
        let unsigned = value as u16;
        stream.push((unsigned >> 8) as u8);
        stream.push((unsigned & 0xFF) as u8);
    }

    /// 辅助函数：写入 255UShort 编码
    /// 如果值 < 255，直接写入1字节；否则写入 0xFF + 2字节
    #[inline]
    fn write_255_ushort(stream: &mut Vec<u8>, value: usize) {
        if value < 255 {
            stream.push(value as u8);
        } else {
            stream.push(0xFF);
            stream.push((value >> 8) as u8);
            stream.push((value & 0xFF) as u8);
        }
    }

    /// 获取编码后的结果
    pub fn get_encoded_data(self) -> EncodedGlyfData {
        EncodedGlyfData {
            n_contour_stream: self.n_contour_stream,
            n_points_stream: self.n_points_stream,
            flag_byte_stream: self.flag_byte_stream,
            glyph_stream: self.glyph_stream,
            composite_stream: self.composite_stream,
            bbox_bitmap: self.bbox_bitmap,
            bbox_stream: self.bbox_stream,
            instruction_stream: self.instruction_stream,
            overlap_bitmap: self.overlap_bitmap,
        }
    }
}

/// 编码后的 glyf 数据
pub struct EncodedGlyfData {
    pub n_contour_stream: Vec<u8>,
    pub n_points_stream: Vec<u8>,
    pub flag_byte_stream: Vec<u8>,
    pub glyph_stream: Vec<u8>,
    pub composite_stream: Vec<u8>,
    pub bbox_bitmap: Vec<u8>,
    pub bbox_stream: Vec<u8>,
    pub instruction_stream: Vec<u8>,
    pub overlap_bitmap: Vec<u8>,
}

impl EncodedGlyfData {
    /// 将所有流合并为一个字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // 按照 woff2 规范的顺序合并流
        result.extend_from_slice(&self.n_contour_stream);
        result.extend_from_slice(&self.n_points_stream);
        result.extend_from_slice(&self.flag_byte_stream);
        result.extend_from_slice(&self.glyph_stream);
        result.extend_from_slice(&self.composite_stream);
        result.extend_from_slice(&self.bbox_bitmap);
        result.extend_from_slice(&self.bbox_stream);
        result.extend_from_slice(&self.instruction_stream);

        // 如果有重叠位图，也加入
        if !self.overlap_bitmap.is_empty() {
            result.extend_from_slice(&self.overlap_bitmap);
        }

        result
    }

    /// 获取总大小
    pub fn total_size(&self) -> usize {
        self.n_contour_stream.len()
            + self.n_points_stream.len()
            + self.flag_byte_stream.len()
            + self.glyph_stream.len()
            + self.composite_stream.len()
            + self.bbox_bitmap.len()
            + self.bbox_stream.len()
            + self.instruction_stream.len()
            + self.overlap_bitmap.len()
    }
}

/// 转换 glyf 和 loca 表
///
/// 这是主要的入口函数，接收 glyf 记录和 loca 偏移量，返回转换后的数据
pub fn transform_glyf_and_loca(
    glyphs: &[GlyfRecord],
    _loca_offsets: &[u32],
) -> Result<(Vec<u8>, Vec<u8>), FontError> {
    let n_glyphs = glyphs.len() as u16;

    // 创建编码器并编码所有字形
    let mut encoder = GlyfEncoder::new(n_glyphs);
    encoder.encode_glyphs(glyphs)?;

    // 获取编码后的数据
    let encoded = encoder.get_encoded_data();

    // 生成转换后的 glyf 数据
    let transformed_glyf = encoded.to_bytes();

    // 生成转换后的 loca 数据（在 WOFF2 中，loca 被省略，因为可以从其他信息推导）
    // 这里返回一个空的 loca，实际使用时需要根据具体规范调整
    let transformed_loca = Vec::new();

    Ok((transformed_glyf, transformed_loca))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triplet_encoding_zero_x() {
        // 测试 X=0, Y小值的情况
        let mut encoder = GlyfEncoder::new(1);
        encoder.write_triplet(true, 0, 100);

        assert_eq!(encoder.flag_byte_stream.len(), 1);
        assert_eq!(encoder.glyph_stream.len(), 1);
    }

    #[test]
    fn test_triplet_encoding_small_values() {
        // 测试 X,Y都很小的情况
        let mut encoder = GlyfEncoder::new(1);
        encoder.write_triplet(true, 10, 20);

        assert_eq!(encoder.flag_byte_stream.len(), 1);
        assert_eq!(encoder.glyph_stream.len(), 1);
    }

    #[test]
    fn test_255_ushort_encoding() {
        let mut stream = Vec::new();

        // 小值：< 255
        GlyfEncoder::write_255_ushort(&mut stream, 100);
        assert_eq!(stream.len(), 1);
        assert_eq!(stream[0], 100);

        // 大值：>= 255
        stream.clear();
        GlyfEncoder::write_255_ushort(&mut stream, 300);
        assert_eq!(stream.len(), 3);
        assert_eq!(stream[0], 0xFF);
    }

    #[test]
    fn test_encode_empty_glyph() {
        let glyphs = vec![GlyfRecord {
            glyph_id: 0,
            data: GlyphData::Empty,
        }];

        let (glyf_data, _loca_data) = transform_glyf_and_loca(&glyphs, &[0, 0]).unwrap();

        // 空字形应该产生一些输出（n_contour = 0）
        assert!(!glyf_data.is_empty());
    }
}
