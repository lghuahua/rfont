use rfont_types::{FontError, Reader, WriteBytes, Writer};

// ==================== Glyf 表标志位常量 ====================
// 参考 OpenType 规范和 woff2 项目 glyph.cc

// 简单字形标志位 (Simple Glyph Flags)
pub const FLAG_ON_CURVE: u8 = 0x01; // bit 0: 点在曲线上
pub const FLAG_X_SHORT: u8 = 0x02; // bit 1: X 坐标使用单字节
pub const FLAG_Y_SHORT: u8 = 0x04; // bit 2: Y 坐标使用单字节
pub const FLAG_REPEAT: u8 = 0x08; // bit 3: 标志位重复
pub const FLAG_X_IS_SAME_OR_POSITIVE: u8 = 0x10; // bit 4: X 增量为正或相同
pub const FLAG_Y_IS_SAME_OR_POSITIVE: u8 = 0x20; // bit 5: Y 增量为正或相同
pub const FLAG_OVERLAP_SIMPLE: u8 = 0x40; // bit 6: 简单字形重叠

// 复合字形标志位 (Composite Glyph Flags)
pub const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001; // bit 0: 参数是双字节
pub const ARGS_ARE_XY_VALUES: u16 = 0x0002; // bit 1: 参数是 XY 值（而非点）
pub const ROUND_XY_TO_GRID: u16 = 0x0004; // bit 2: 舍入到网格
pub const WE_HAVE_A_SCALE: u16 = 0x0008; // bit 3: 有缩放因子
pub const MORE_COMPONENTS: u16 = 0x0020; // bit 5: 还有更多组件
pub const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040; // bit 6: 有 X 和 Y 缩放
pub const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080; // bit 7: 有 2x2 变换矩阵
pub const WE_HAVE_INSTRUCTIONS: u16 = 0x0100; // bit 8: 有指令
pub const USE_MY_METRICS: u16 = 0x0200; // bit 9: 使用我的度量
pub const OVERLAP_COMPOUND: u16 = 0x0400; // bit 10: 复合字形重叠
pub const SCALED_COMPONENT_OFFSET: u16 = 0x0800; // bit 11: 缩放的组件偏移
pub const UNSCALED_COMPONENT_OFFSET: u16 = 0x1000; // bit 12: 未缩放的组件偏移

#[derive(Debug, Clone)]
pub struct GlyfRecord {
    pub glyph_id: u16,
    pub data: GlyphData,
}

#[derive(Debug, Clone)]
pub enum GlyphData {
    Empty,
    Simple(SimpleGlyph),
    Composite(CompositeGlyph),
}

#[derive(Debug, Clone)]
pub struct SimpleGlyph {
    pub num_contours: i16,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub end_pts_of_contours: Vec<u16>,
    pub instructions: Vec<u8>,
    pub flags: Vec<u8>,
    pub x_coordinates: Vec<i16>,
    pub y_coordinates: Vec<i16>,
}

#[derive(Debug, Clone)]
pub struct CompositeGlyph {
    pub num_contours: i16, // 通常为 -1
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub components: Vec<CompositeComponent>,
}

impl WriteBytes for CompositeGlyph {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_i16(self.num_contours)?;
        writer.write_i16(self.x_min)?;
        writer.write_i16(self.y_min)?;
        writer.write_i16(self.x_max)?;
        writer.write_i16(self.y_max)?;

        if self.components.is_empty() {
            return Ok(());
        }

        for comp in self.components.iter() {
            comp.write_to(writer)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CompositeComponent {
    pub flags: u16,
    pub glyph_index: u16,
    pub data: Vec<u8>
}

impl WriteBytes for CompositeComponent {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_u16(self.flags)?;
        writer.write_u16(self.glyph_index)?;
        writer.write_bytes(&self.data)?;
        Ok(())
    }
}

impl GlyfRecord {
    pub fn parse(reader: &mut Reader, glyph_id: u16) -> Result<Self, FontError> {
        // 检查是否为空字形
        if reader.data.is_empty() {
            return Ok(Self {
                glyph_id,
                data: GlyphData::Empty,
            });
        }

        let num_contours = reader.read_i16()?;
        let x_min = reader.read_i16()?;
        let y_min = reader.read_i16()?;
        let x_max = reader.read_i16()?;
        let y_max = reader.read_i16()?;

        let data = if num_contours >= 0 {
            // 简单字形
            let end_pts_of_contours = reader.read_array::<u16>(num_contours as usize)?;
            let instruction_length = reader.read_u16()? as usize;
            let instructions = reader.read_array::<u8>(instruction_length)?;

            // 计算总点数
            let total_points = end_pts_of_contours.last().copied().unwrap_or(0) as usize + 1;

            // 读取 Flags（支持 run-length 编码）
            let mut flags = Vec::with_capacity(total_points);
            let mut remaining = total_points;
            while remaining > 0 {
                let flag = reader.read_u8()?;
                flags.push(flag);
                remaining -= 1;

                // 如果设置了 Repeat 位 (bit 3)，则后面跟着一个重复次数
                if flag & FLAG_REPEAT != 0 {
                    let repeat_count = reader.read_u8()? as usize;
                    for _ in 0..repeat_count {
                        flags.push(flag);
                    }
                    remaining -= repeat_count;
                }
            }

            // 读取 X 坐标（相对增量编码）
            let mut x_coordinates = Vec::with_capacity(total_points);
            let mut prev_x: i16 = 0;

            for i in 0..total_points {
                let flag = flags[i];

                // bit 1: X_IS_SHORT (单字节)
                if flag & FLAG_X_SHORT != 0 {
                    // 单字节坐标值
                    let x_byte = reader.read_u8()?;

                    // bit 4: X_IS_SAME_OR_POSITIVE
                    // 如果为 1，表示正值；如果为 0，表示负值
                    if flag & FLAG_X_IS_SAME_OR_POSITIVE != 0 {
                        // 正值
                        prev_x = prev_x.wrapping_add(x_byte as i16);
                    } else {
                        // 负值
                        prev_x = prev_x.wrapping_sub(x_byte as i16);
                    }
                } else {
                    // bit 4: X_IS_SAME_OR_ZERO
                    if flag & FLAG_X_IS_SAME_OR_POSITIVE != 0 {
                        // X 坐标与前一个相同（增量为 0）
                        // prev_x 保持不变
                    } else {
                        // 双字节有符号坐标值
                        let x_delta = reader.read_i16()?;
                        prev_x = prev_x.wrapping_add(x_delta);
                    }
                }

                x_coordinates.push(prev_x);
            }

            // 读取 Y 坐标（相对增量编码）
            let mut y_coordinates = Vec::with_capacity(total_points);
            let mut prev_y: i16 = 0;

            for i in 0..total_points {
                let flag = flags[i];

                // bit 2: Y_IS_SHORT (单字节)
                if flag & FLAG_Y_SHORT != 0 {
                    // 单字节坐标值
                    let y_byte = reader.read_u8()?;

                    // bit 5: Y_IS_SAME_OR_POSITIVE
                    // 如果为 1，表示正值；如果为 0，表示负值
                    if flag & FLAG_Y_IS_SAME_OR_POSITIVE != 0 {
                        // 正值
                        prev_y = prev_y.wrapping_add(y_byte as i16);
                    } else {
                        // 负值
                        prev_y = prev_y.wrapping_sub(y_byte as i16);
                    }
                } else {
                    // bit 5: Y_IS_SAME_OR_ZERO
                    if flag & FLAG_Y_IS_SAME_OR_POSITIVE != 0 {
                        // Y 坐标与前一个相同（增量为 0）
                        // prev_y 保持不变
                    } else {
                        // 双字节有符号坐标值
                        let y_delta = reader.read_i16()?;
                        prev_y = prev_y.wrapping_add(y_delta);
                    }
                }

                y_coordinates.push(prev_y);
            }

            GlyphData::Simple(SimpleGlyph {
                num_contours,
                x_min,
                y_min,
                x_max,
                y_max,
                end_pts_of_contours,
                instructions,
                flags,
                x_coordinates,
                y_coordinates,
            })
        } else {
            // 复合字形
            let mut components = Vec::new();
            loop {
                let flags = reader.read_u16()?;
                let glyph_index = reader.read_u16()?;
  
                let mut arg_size = 0;
                if flags & ARG_1_AND_2_ARE_WORDS != 0 {
                    arg_size += 4;
                } else {
                    arg_size += 2;
                }

                if flags & WE_HAVE_A_SCALE != 0 {
                    arg_size += 2;
                } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                    arg_size += 4;
                } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                    arg_size += 8;
                }

                components.push(CompositeComponent {
                    flags,
                    glyph_index,
                    data: reader.read_bytes(arg_size)?.to_vec(),
                });

                // 如果 MORE_COMPONENTS 位 (5) 为 0，则结束
                if (flags & MORE_COMPONENTS) == 0 {
                    break;
                }
            }
            GlyphData::Composite(CompositeGlyph {
                num_contours,
                x_min,
                y_min,
                x_max,
                y_max,
                components,
            })
        };

        Ok(Self { glyph_id, data })
    }
}

impl WriteBytes for GlyfRecord {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        match &self.data {
            GlyphData::Empty => Ok(()),
            GlyphData::Simple(simple) => {
                // 1. 写入头部信息
                writer.write_i16(simple.num_contours)?;
                writer.write_i16(simple.x_min)?;
                writer.write_i16(simple.y_min)?;
                writer.write_i16(simple.x_max)?;
                writer.write_i16(simple.y_max)?;

                // 2. 写入轮廓结束点索引
                for pt in &simple.end_pts_of_contours {
                    writer.write_u16(*pt)?;
                }

                // 3. 写入指令长度及指令数据
                let instruction_len = simple.instructions.len();
                writer.write_u16(instruction_len as u16)?;
                for inst in &simple.instructions {
                    writer.write_u8(*inst)?;
                }

                // 4. 处理坐标：转换为相对增量并重新生成 Flags
                let total_points = simple.x_coordinates.len();
                if total_points == 0 {
                    return Ok(());
                }

                let mut processed_flags = Vec::with_capacity(total_points);
                let mut last_x: i16 = 0;
                let mut last_y: i16 = 0;

                for i in 0..total_points {
                    let curr_x = simple.x_coordinates[i];
                    let curr_y = simple.y_coordinates[i];

                    let dx = curr_x - last_x;
                    let dy = curr_y - last_y;

                    // 从原始 flags 中保留 on-curve 位 (bit 0) 和 overlap 位 (bit 6)
                    let original_flag = if i < simple.flags.len() {
                        simple.flags[i]
                    } else {
                        0
                    };

                    let mut flag = original_flag & (FLAG_ON_CURVE | FLAG_OVERLAP_SIMPLE);

                    // 处理 X 坐标标志
                    if dx == 0 {
                        // X 增量为 0：设置 bit 1 (X_IS_SHORT) 和 bit 4 (X_IS_SAME)
                        flag |= FLAG_X_SHORT | FLAG_X_IS_SAME_OR_POSITIVE;
                    } else if dx > -256 && dx < 256 {
                        // X 增量在单字节范围内：设置 bit 1 (X_IS_SHORT)
                        flag |= FLAG_X_SHORT;
                        // bit 4 (X_IS_SAME_OR_POSITIVE): 1=正, 0=负
                        if dx > 0 {
                            flag |= FLAG_X_IS_SAME_OR_POSITIVE;
                        }
                        // 如果 dx < 0，bit 4 保持为 0
                    }
                    // 否则使用双字节，bit 1 和 bit 4 都为 0

                    // 处理 Y 坐标标志
                    if dy == 0 {
                        // Y 增量为 0：设置 bit 2 (Y_IS_SHORT) 和 bit 5 (Y_IS_SAME)
                        flag |= FLAG_Y_SHORT | FLAG_Y_IS_SAME_OR_POSITIVE;
                    } else if dy > -256 && dy < 256 {
                        // Y 增量在单字节范围内：设置 bit 2 (Y_IS_SHORT)
                        flag |= FLAG_Y_SHORT;
                        // bit 5 (Y_IS_SAME_OR_POSITIVE): 1=正, 0=负
                        if dy > 0 {
                            flag |= FLAG_Y_IS_SAME_OR_POSITIVE;
                        }
                        // 如果 dy < 0，bit 5 保持为 0
                    }
                    // 否则使用双字节，bit 2 和 bit 5 都为 0

                    processed_flags.push(flag);
                    last_x = curr_x;
                    last_y = curr_y;
                }

                // 5. 写入标志位（不实现 run-length 编码以保持简单）
                for flag in &processed_flags {
                    writer.write_u8(*flag)?;
                }

                // 6. 写入 X 坐标增量
                last_x = 0;
                for (i, &curr_x) in simple.x_coordinates.iter().enumerate() {
                    let dx = curr_x - last_x;
                    let flag = processed_flags[i];

                    if (flag & FLAG_X_SHORT) != 0 {
                        // X_IS_SHORT: 单字节
                        let abs_dx = dx.unsigned_abs() as u8;
                        writer.write_u8(abs_dx)?;
                    } else if (flag & FLAG_X_IS_SAME_OR_POSITIVE) == 0 {
                        // 非相同且非单字节：双字节有符号
                        writer.write_i16(dx)?;
                    }
                    // 如果 bit 4 为 1 且 bit 1 为 0，表示增量为 0，不写入数据

                    last_x = curr_x;
                }

                // 7. 写入 Y 坐标增量
                last_y = 0;
                for (i, &curr_y) in simple.y_coordinates.iter().enumerate() {
                    let dy = curr_y - last_y;
                    let flag = processed_flags[i];

                    if (flag & FLAG_Y_SHORT) != 0 {
                        // Y_IS_SHORT: 单字节
                        let abs_dy = dy.unsigned_abs() as u8;
                        writer.write_u8(abs_dy)?;
                    } else if (flag & FLAG_Y_IS_SAME_OR_POSITIVE) == 0 {
                        // 非相同且非单字节：双字节有符号
                        writer.write_i16(dy)?;
                    }
                    // 如果 bit 5 为 1 且 bit 2 为 0，表示增量为 0，不写入数据

                    last_y = curr_y;
                }

                Ok(())
            }
            GlyphData::Composite(composite) => {
                writer.write_i16(composite.num_contours)?;
                writer.write_i16(composite.x_min)?;
                writer.write_i16(composite.y_min)?;
                writer.write_i16(composite.x_max)?;
                writer.write_i16(composite.y_max)?;

                if composite.components.is_empty() {
                    return Ok(());
                }

                let last = composite.components.len() - 1;
                for (i, comp) in composite.components.iter().enumerate() {
                    // 对于非最后一个组件，设置 MORE_COMPONENTS 标志
                    if i < last {
                        let modified_flags = comp.flags | MORE_COMPONENTS;
                        writer.write_u16(modified_flags)?;
                    } else {
                        writer.write_u16(comp.flags)?;
                    }
                    writer.write_u16(comp.glyph_index)?;
                    writer.write_bytes(&comp.data)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::Reader;

    // ==================== 简单字形测试 ====================

    #[test]
    fn test_glyf_empty_glyph() {
        // 测试空字形（没有数据）
        let data = vec![];
        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 0).unwrap();

        assert_eq!(record.glyph_id, 0);
        match record.data {
            GlyphData::Empty => {}
            _ => panic!("Expected Empty glyph"),
        }
    }

    #[test]
    fn test_glyf_simple_glyph_basic() {
        // 测试简单的单轮廓字形（正方形）
        // 4个点：(0,0), (100,0), (100,100), (0,100)
        let data = vec![
            // num_contours = 1 (简单字形)
            0x00, 0x01, // bbox: x_min=0, y_min=0, x_max=100, y_max=100
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64,
            // end_pts_of_contours: [3] (4个点，索引从0开始)
            0x00, 0x03, // instruction_length = 0
            0x00, 0x00,
            // flags (4个点):
            // bit 0=on-curve, bit 1=x-short, bit 2=y-short, bit 4=x-positive/same, bit 5=y-positive/same
            // 点0 (0,0): on-curve(1), x-delta=0(bit1+bit4=0x12), y-delta=0(bit2+bit5=0x24) => 0x01|0x12|0x24 = 0x37
            0x37,
            // 点1 (100,0): on-curve(1), x-delta=100(short+positive=0x12), y-delta=0(0x24) => 0x01|0x12|0x24 = 0x37
            0x37,
            // 点2 (100,100): on-curve(1), x-delta=0(0x12), y-delta=100(short+positive=0x24) => 0x01|0x12|0x24 = 0x37
            0x37,
            // 点3 (0,100): on-curve(1), x-delta=-100(short+negative: bit1=1,bit4=0 => 0x02), y-delta=0(0x24) => 0x01|0x02|0x24 = 0x27
            0x27, // x coordinates: 0, 100, 0, 100 (absolute values for short encoding)
            0x00, 0x64, 0x00, 0x64, // y coordinates: 0, 0, 100, 0
            0x00, 0x00, 0x64, 0x00,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 1).unwrap();

        assert_eq!(record.glyph_id, 1);
        match &record.data {
            GlyphData::Simple(simple) => {
                assert_eq!(simple.num_contours, 1);
                assert_eq!(simple.x_min, 0);
                assert_eq!(simple.y_min, 0);
                assert_eq!(simple.x_max, 100);
                assert_eq!(simple.y_max, 100);
                assert_eq!(simple.end_pts_of_contours, vec![3]);
                assert!(simple.instructions.is_empty());
                assert_eq!(simple.flags.len(), 4);
                // 验证坐标解码正确
                assert_eq!(simple.x_coordinates, vec![0, 100, 100, 0]);
                assert_eq!(simple.y_coordinates, vec![0, 0, 100, 100]);
            }
            _ => panic!("Expected Simple glyph"),
        }
    }

    #[test]
    fn test_glyf_simple_glyph_with_instructions() {
        // 测试带指令的字形（三角形）
        // 3个点：(0,0), (50,100), (100,0)
        let data = vec![
            // num_contours = 1
            0x00, 0x01, // bbox
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, // end_pts_of_contours: [2]
            0x00, 0x02, // instruction_length = 3
            0x00, 0x03, // instructions: [0x10, 0x20, 0x30]
            0x10, 0x20, 0x30,
            // flags (3个点): on-curve + coordinates
            // 点0 (0,0): x=0,y=0 => 0x01|0x12|0x24 = 0x37
            0x37,
            // 点1 (50,100): dx=50(short+pos=0x12), dy=100(short+pos=0x24) => 0x01|0x12|0x24 = 0x37
            0x37,
            // 点2 (100,0): dx=50(short+pos=0x12), dy=-100(short+neg: bit2=1,bit5=0 => 0x04) => 0x01|0x12|0x04 = 0x17
            0x17, // x coordinates: 0, 50, 50
            0x00, 0x32, 0x32, // y coordinates: 0, 100, 100 (abs value)
            0x00, 0x64, 0x64,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 2).unwrap();

        match &record.data {
            GlyphData::Simple(simple) => {
                assert_eq!(simple.instructions, vec![0x10, 0x20, 0x30]);
                assert_eq!(simple.end_pts_of_contours, vec![2]);
                assert_eq!(simple.x_coordinates, vec![0, 50, 100]);
                assert_eq!(simple.y_coordinates, vec![0, 100, 0]);
            }
            _ => panic!("Expected Simple glyph"),
        }
    }

    #[test]
    fn test_glyf_simple_glyph_multiple_contours() {
        // 测试多轮廓字形（如字母 "B" 有两个孔）
        // 8个点，2个轮廓
        let data = vec![
            // num_contours = 2
            0x00, 0x02, // bbox
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64,
            // end_pts_of_contours: [3, 7] (第一个轮廓4点，第二个轮廓4点)
            0x00, 0x03, 0x00, 0x07, // instruction_length = 0
            0x00, 0x00, // flags (8个点): all on-curve with zero deltas
            0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, // x coordinates: all zeros
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // y coordinates: all zeros
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 3).unwrap();

        match &record.data {
            GlyphData::Simple(simple) => {
                assert_eq!(simple.num_contours, 2);
                assert_eq!(simple.end_pts_of_contours, vec![3, 7]);
                assert_eq!(simple.flags.len(), 8);
                assert_eq!(simple.x_coordinates.len(), 8);
                assert_eq!(simple.y_coordinates.len(), 8);
            }
            _ => panic!("Expected Simple glyph"),
        }
    }

    #[test]
    fn test_glyf_simple_glyph_flag_repeat() {
        // 测试标志位的重复编码
        // 如果有 repeat 标志 (bit 3 = 1)，后面跟着重复次数
        // 5个点，所有点都有相同的标志
        let data = vec![
            // num_contours = 1
            0x00, 0x01, // bbox
            0x00, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x0A, // end_pts_of_contours: [4]
            0x00, 0x04, // instruction_length = 0
            0x00, 0x00,
            // flags: 第一个标志有 repeat 位，重复 3 次
            // 0x33 | 0x08 = 0x3B (on-curve + x-short+same + y-short+same + repeat)
            0x3B, 0x03, // repeat count = 3
            // 最后一个标志
            0x33, // coordinates (5个点): all zeros
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 4).unwrap();

        match &record.data {
            GlyphData::Simple(simple) => {
                // 应该有 5 个标志：1个原始 + 3个重复 + 1个最后
                assert_eq!(simple.flags.len(), 5);
                assert_eq!(simple.flags[0], 0x3B);
                assert_eq!(simple.flags[1], 0x3B);
                assert_eq!(simple.flags[2], 0x3B);
                assert_eq!(simple.flags[3], 0x3B);
                assert_eq!(simple.flags[4], 0x33);
                // 所有坐标都应该是 0
                assert_eq!(simple.x_coordinates, vec![0, 0, 0, 0, 0]);
                assert_eq!(simple.y_coordinates, vec![0, 0, 0, 0, 0]);
            }
            _ => panic!("Expected Simple glyph"),
        }
    }

    // ==================== 复合字形测试 ====================

    #[test]
    fn test_glyf_composite_glyph_basic() {
        // 测试基本的复合字形（num_contours = -1）
        let data = vec![
            // num_contours = -1 (0xFFFF) 表示复合字形
            0xFF, 0xFF, // bbox
            0xFF, 0xCE, 0xFF, 0x9C, 0x00, 0xC8, 0x01, 0x2C,
            // Component 1: flags=0x0001 (ARG_1_AND_2_ARE_WORDS), glyph_index=5
            0x00, 0x01, 0x00, 0x05, // argument1 (i16) = 10, argument2 (i16) = 20
            0x00, 0x0A, 0x00, 0x14,
            // No MORE_COMPONENTS bit, so this is the last component
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 10).unwrap();

        match &record.data {
            GlyphData::Composite(composite) => {
                assert_eq!(composite.num_contours, -1);
                assert_eq!(composite.components.len(), 1);
            }
            _ => panic!("Expected Composite glyph"),
        }
    }

    #[test]
    fn test_glyf_composite_glyph_multiple_components() {
        // 测试多组件复合字形
        let data = vec![
            // num_contours = -1
            0xFF, 0xFF, // bbox
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64,
            // Component 1: flags=0x0021 (MORE_COMPONENTS | ARG_1_AND_2_ARE_WORDS), glyph_index=3
            0x00, 0x21, 0x00, 0x03, // offset x=5, y=10
            0x00, 0x05, 0x00, 0x0A,
            // Component 2: flags=0x0001 (ARG_1_AND_2_ARE_WORDS, no MORE), glyph_index=7
            0x00, 0x01, 0x00, 0x07, // offset x=15, y=20
            0x00, 0x0F, 0x00, 0x14,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 11).unwrap();

        match &record.data {
            GlyphData::Composite(composite) => {
                assert_eq!(composite.components.len(), 2);
            }
            _ => panic!("Expected Composite glyph"),
        }
    }

    #[test]
    fn test_glyf_composite_glyph_byte_args() {
        // 测试使用单字节参数的复合字形
        let data = vec![
            // num_contours = -1
            0xFF, 0xFF, // bbox
            0x00, 0x00, 0x00, 0x00, 0x00, 0x32, 0x00, 0x32,
            // Component: flags=0x0000 (args are bytes), glyph_index=2
            0x00, 0x00, 0x00, 0x02, // argument1 (u8) = 5, argument2 (u8) = 10
            0x05, 0x0A,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 12).unwrap();

        match &record.data {
            GlyphData::Composite(composite) => {
                assert_eq!(composite.components.len(), 1);
            }
            _ => panic!("Expected Composite glyph"),
        }
    }


    // ==================== 写入测试 ====================

    #[test]
    fn test_glyf_write_empty() {
        // 测试空字形的写入
        use rfont_types::Writer;

        let record = GlyfRecord {
            glyph_id: 0,
            data: GlyphData::Empty,
        };

        let mut writer = Writer::new();
        record.write_to(&mut writer).unwrap();

        assert!(writer.data.is_empty());
    }

    #[test]
    fn test_glyf_write_simple_glyph() {
        // 测试简单字形的写入
        use rfont_types::Writer;

        let simple = SimpleGlyph {
            num_contours: 1,
            x_min: -50,
            y_min: -100,
            x_max: 200,
            y_max: 300,
            end_pts_of_contours: vec![3],
            instructions: vec![],
            flags: vec![0x01, 0x01, 0x01, 0x01],
            x_coordinates: vec![0, 100, 100, 0],
            y_coordinates: vec![0, 0, 200, 200],
        };

        let record = GlyfRecord {
            glyph_id: 1,
            data: GlyphData::Simple(simple),
        };

        let mut writer = Writer::new();
        record.write_to(&mut writer).unwrap();

        // 验证写入的数据不为空
        assert!(!writer.data.is_empty());

        // 验证头部信息
        assert_eq!(writer.data[0..2], [0x00, 0x01]); // num_contours = 1
        assert_eq!(writer.data[2..4], [0xFF, 0xCE]); // x_min = -50
    }

    #[test]
    fn test_glyf_write_composite_glyph() {
        // 测试复合字形的写入
        use rfont_types::Writer;

        let composite = CompositeGlyph {
            num_contours: -1,
            x_min: 0,
            y_min: 0,
            x_max: 100,
            y_max: 100,
            components: vec![CompositeComponent {
                flags: 0x0001,
                glyph_index: 5,
                data: vec![0x00, 0x05, 0x00, 0x0A] // argument1=5, argument2=10
            }],
        };

        let record = GlyfRecord {
            glyph_id: 10,
            data: GlyphData::Composite(composite),
        };

        let mut writer = Writer::new();
        record.write_to(&mut writer).unwrap();

        // 验证写入的数据
        assert!(!writer.data.is_empty());
        assert_eq!(writer.data[0..2], [0xFF, 0xFF]); // num_contours = -1
    }

    #[test]
    fn test_glyf_roundtrip_simple() {
        // 测试简单字形的往返（写入后再读取）
        use rfont_types::Writer;

        let original = SimpleGlyph {
            num_contours: 1,
            x_min: 0,
            y_min: 0,
            x_max: 100,
            y_max: 100,
            end_pts_of_contours: vec![3],
            instructions: vec![0x10, 0x20],
            flags: vec![0x01, 0x01, 0x01, 0x01],
            x_coordinates: vec![0, 50, 50, 0],
            y_coordinates: vec![0, 0, 50, 50],
        };

        let record = GlyfRecord {
            glyph_id: 1,
            data: GlyphData::Simple(original.clone()),
        };

        // 写入
        let mut writer = Writer::new();
        record.write_to(&mut writer).unwrap();

        // 读取
        let mut reader = Reader::new(&writer.data);
        let parsed = GlyfRecord::parse(&mut reader, 1).unwrap();

        match &parsed.data {
            GlyphData::Simple(simple) => {
                assert_eq!(simple.num_contours, 1);
                assert_eq!(simple.x_min, 0);
                assert_eq!(simple.y_max, 100);
                assert_eq!(simple.end_pts_of_contours, vec![3]);
                assert_eq!(simple.instructions, vec![0x10, 0x20]);
            }
            _ => panic!("Expected Simple glyph after roundtrip"),
        }
    }

    #[test]
    fn test_glyf_roundtrip_composite() {
        // 测试复合字形的往返
        use rfont_types::Writer;

        let original = CompositeGlyph {
            num_contours: -1,
            x_min: -10,
            y_min: -10,
            x_max: 110,
            y_max: 110,
            components: vec![
                CompositeComponent {
                    flags: 0x0001,
                    glyph_index: 3,
                    data: vec![0x00, 0x05, 0x00, 0x0A], // argument1=5, argument2=10
                },
                CompositeComponent {
                    flags: 0x0001,
                    glyph_index: 7,
                    data: vec![0x00, 0x0F, 0x00, 0x14], // argument1=15, argument2=20
                },
            ],
        };

        let record = GlyfRecord {
            glyph_id: 20,
            data: GlyphData::Composite(original.clone()),
        };

        // 写入
        let mut writer = Writer::new();
        record.write_to(&mut writer).unwrap();

        // 读取
        let mut reader = Reader::new(&writer.data);
        let parsed = GlyfRecord::parse(&mut reader, 20).unwrap();

        match &parsed.data {
            GlyphData::Composite(composite) => {
                assert_eq!(composite.num_contours, -1);
                assert_eq!(composite.components.len(), 2);
                assert_eq!(composite.components[0].glyph_index, 3);
                assert_eq!(composite.components[1].glyph_index, 7);
            }
            _ => panic!("Expected Composite glyph after roundtrip"),
        }
    }
}
