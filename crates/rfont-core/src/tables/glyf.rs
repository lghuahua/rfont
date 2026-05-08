use rfont_types::{FontError, Reader, WriteBytes, Writer};

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

#[derive(Debug, Clone)]
pub struct CompositeComponent {
    pub flags: u16,
    pub glyph_index: u16,
    pub argument1: i16, // x offset or point
    pub argument2: i16, // y offset or point
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

            // 读取 Flags
            let mut flags = Vec::with_capacity(total_points);
            let mut remaining = total_points;
            while remaining > 0 {
                let flag = reader.read_u8()?;
                flags.push(flag);
                remaining -= 1;

                // 如果设置了 Repeat 位 (3)，则后面跟着一个重复次数
                if flag & 0x08 != 0 {
                    let repeat_count = reader.read_u8()? as usize;
                    for _ in 0..repeat_count {
                        flags.push(flag);
                    }
                    remaining -= repeat_count;
                }
            }

            // 读取坐标 (X 和 Y 是分开存储的，且可能是相对值)
            // 这里为了简化，先实现基础读取，后续可能需要处理相对坐标转换
            let mut x_coordinates = Vec::with_capacity(total_points);
            let mut y_coordinates = Vec::with_capacity(total_points);

            // 简化的坐标读取逻辑（实际规范更复杂，涉及相对坐标和标志位解析）
            // 暂时占位，确保结构完整
            for _ in 0..total_points {
                x_coordinates.push(0);
                y_coordinates.push(0);
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

                let arg_is_1_and_2_words = (flags & 0x0001) != 0;
                let (arg1, arg2) = if arg_is_1_and_2_words {
                    (reader.read_i16()?, reader.read_i16()?)
                } else {
                    (reader.read_u8()? as i16, reader.read_u8()? as i16)
                };

                components.push(CompositeComponent {
                    flags,
                    glyph_index,
                    argument1: arg1,
                    argument2: arg2,
                });

                // 如果 MORE_COMPONENTS 位 (5) 为 0，则结束
                if (flags & 0x0020) == 0 {
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

                // 4. 处理坐标：转换为相对增量并生成 Flags
                let total_points = simple.flags.len();
                if total_points == 0 {
                    return Ok(());
                }

                let mut x_coords_delta = Vec::with_capacity(total_points);
                let mut y_coords_delta = Vec::with_capacity(total_points);
                let mut processed_flags = Vec::with_capacity(total_points);

                let mut last_x: i16 = 0;
                let mut last_y: i16 = 0;

                for i in 0..total_points {
                    let curr_x = simple.x_coordinates[i];
                    let curr_y = simple.y_coordinates[i];

                    let dx = curr_x - last_x;
                    let dy = curr_y - last_y;

                    x_coords_delta.push(dx);
                    y_coords_delta.push(dy);

                    let mut flag = simple.flags[i] & 0xCF; // 保留原始标志的核心位，清除重复位

                    // 处理 X 坐标标志
                    if dx == 0 {
                        flag |= 0x02; // THIS_X_IS_SAME
                    } else if dx > 0 && dx <= 255 {
                        flag |= 0x12; // X_IS_BYTE | THIS_X_IS_SAME (positive)
                    } else if (-255..0).contains(&dx) {
                        flag |= 0x10; // X_IS_BYTE (negative, bit 1 is 0)
                                      // 注意：在写入时，负数需要取绝对值存入单字节
                    }

                    // 处理 Y 坐标标志
                    if dy == 0 {
                        flag |= 0x04; // THIS_Y_IS_SAME
                    } else if dy > 0 && dy <= 255 {
                        flag |= 0x24; // Y_IS_BYTE | THIS_Y_IS_SAME (positive)
                    } else if (-255..0).contains(&dy) {
                        flag |= 0x20; // Y_IS_BYTE (negative, bit 2 is 0)
                    }

                    // 处理重复标志 (Run-length encoding for flags)
                    // 这是一个简化处理，实际规范更复杂。这里我们暂时不合并重复标志，
                    // 而是确保每个点都有对应的标志位。
                    processed_flags.push(flag);

                    last_x = curr_x;
                    last_y = curr_y;
                }

                // 5. 写入标志位
                for flag in &processed_flags {
                    writer.write_u8(*flag)?;
                }

                // 6. 写入 X 坐标增量
                for (i, &dx) in x_coords_delta.iter().enumerate() {
                    let flag = processed_flags[i];
                    if (flag & 0x10) != 0 {
                        // X_IS_BYTE
                        // 如果是单字节存储，根据 THIS_X_IS_SAME 位判断正负
                        if (flag & 0x02) != 0 {
                            writer.write_u8(dx as u8)?; // Positive or Zero
                        } else {
                            writer.write_u8((-dx) as u8)?; // Negative
                        }
                    } else if (flag & 0x02) == 0 {
                        // Not same, must be short
                        writer.write_i16(dx)?;
                    }
                }

                // 7. 写入 Y 坐标增量
                for (i, &dy) in y_coords_delta.iter().enumerate() {
                    let flag = processed_flags[i];
                    if (flag & 0x20) != 0 {
                        // Y_IS_BYTE
                        if (flag & 0x04) != 0 {
                            writer.write_u8(dy as u8)?;
                        } else {
                            writer.write_u8((-dy) as u8)?;
                        }
                    } else if (flag & 0x04) == 0 {
                        writer.write_i16(dy)?;
                    }
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
                    let mut flags = comp.flags;
                    if i < last {
                        flags |= 0x0020;
                    } // MORE_COMPONENTS

                    writer.write_u16(flags)?;
                    writer.write_u16(comp.glyph_index)?;

                    let arg_is_words = (flags & 0x0001) != 0;
                    if arg_is_words {
                        writer.write_i16(comp.argument1)?;
                        writer.write_i16(comp.argument2)?;
                    } else {
                        writer.write_u8(comp.argument1 as u8)?;
                        writer.write_u8(comp.argument2 as u8)?;
                    }
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
        // 测试简单的单轮廓字形
        // 结构：num_contours=1, bbox, end_pts, instruction_len, instructions
        let data = vec![
            // num_contours = 1 (简单字形)
            0x00, 0x01, // x_min = -50 (0xFFCE)
            0xFF, 0xCE, // y_min = -100 (0xFF9C)
            0xFF, 0x9C, // x_max = 200 (0x00C8)
            0x00, 0xC8, // y_max = 300 (0x012C)
            0x01, 0x2C, // end_pts_of_contours: [3] (4个点，索引从0开始)
            0x00, 0x03, // instruction_length = 0
            0x00, 0x00, // flags (4个点的标志)
            0x01, 0x01, 0x01, 0x01, // x_coordinates (4个值，这里简化为0)
            0x00, 0x00, 0x00, 0x00, // y_coordinates (4个值)
            0x00, 0x00, 0x00, 0x00,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 1).unwrap();

        assert_eq!(record.glyph_id, 1);
        match &record.data {
            GlyphData::Simple(simple) => {
                assert_eq!(simple.num_contours, 1);
                assert_eq!(simple.x_min, -50);
                assert_eq!(simple.y_min, -100);
                assert_eq!(simple.x_max, 200);
                assert_eq!(simple.y_max, 300);
                assert_eq!(simple.end_pts_of_contours, vec![3]);
                assert!(simple.instructions.is_empty());
                assert_eq!(simple.flags.len(), 4);
            }
            _ => panic!("Expected Simple glyph"),
        }
    }

    #[test]
    fn test_glyf_simple_glyph_with_instructions() {
        // 测试带指令的字形
        let data = vec![
            // num_contours = 1
            0x00, 0x01, // bbox
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64, // end_pts_of_contours: [2]
            0x00, 0x02, // instruction_length = 3
            0x00, 0x03, // instructions: [0x10, 0x20, 0x30]
            0x10, 0x20, 0x30, // flags (3个点)
            0x01, 0x01, 0x01, // coordinates (简化)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 2).unwrap();

        match &record.data {
            GlyphData::Simple(simple) => {
                assert_eq!(simple.instructions, vec![0x10, 0x20, 0x30]);
                assert_eq!(simple.end_pts_of_contours, vec![2]);
            }
            _ => panic!("Expected Simple glyph"),
        }
    }

    #[test]
    fn test_glyf_simple_glyph_multiple_contours() {
        // 测试多轮廓字形（如字母 "B" 有两个孔）
        let data = vec![
            // num_contours = 2
            0x00, 0x02, // bbox
            0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x64,
            // end_pts_of_contours: [3, 7] (第一个轮廓4点，第二个轮廓4点)
            0x00, 0x03, 0x00, 0x07, // instruction_length = 0
            0x00, 0x00, // flags (8个点)
            0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, // coordinates (简化)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 3).unwrap();

        match &record.data {
            GlyphData::Simple(simple) => {
                assert_eq!(simple.num_contours, 2);
                assert_eq!(simple.end_pts_of_contours, vec![3, 7]);
                assert_eq!(simple.flags.len(), 8);
            }
            _ => panic!("Expected Simple glyph"),
        }
    }

    #[test]
    fn test_glyf_simple_glyph_flag_repeat() {
        // 测试标志位的重复编码
        // 如果有 repeat 标志 (bit 3 = 1)，后面跟着重复次数
        let data = vec![
            // num_contours = 1
            0x00, 0x01, // bbox
            0x00, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x0A, // end_pts_of_contours: [4]
            0x00, 0x04, // instruction_length = 0
            0x00, 0x00, // flags: 第一个标志有 repeat 位，重复 3 次
            0x09, 0x03, // 0x09 = 0x01 | 0x08 (repeat), 0x03 = repeat count
            0x01, // 最后一个标志
            // coordinates (5个点)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 4).unwrap();

        match &record.data {
            GlyphData::Simple(simple) => {
                // 应该有 5 个标志：1个原始 + 3个重复 + 1个最后
                assert_eq!(simple.flags.len(), 5);
                assert_eq!(simple.flags[0], 0x09);
                assert_eq!(simple.flags[1], 0x09);
                assert_eq!(simple.flags[2], 0x09);
                assert_eq!(simple.flags[3], 0x09);
                assert_eq!(simple.flags[4], 0x01);
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
                assert_eq!(composite.components[0].glyph_index, 5);
                assert_eq!(composite.components[0].argument1, 10);
                assert_eq!(composite.components[0].argument2, 20);
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
                assert_eq!(composite.components[0].glyph_index, 3);
                assert_eq!(composite.components[1].glyph_index, 7);
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
                assert_eq!(composite.components[0].glyph_index, 2);
                assert_eq!(composite.components[0].argument1, 5);
                assert_eq!(composite.components[0].argument2, 10);
            }
            _ => panic!("Expected Composite glyph"),
        }
    }

    #[test]
    fn test_glyf_composite_glyph_negative_offsets() {
        // 测试负偏移的复合字形
        let data = vec![
            // num_contours = -1
            0xFF, 0xFF, // bbox
            0xFF, 0xF6, 0xFF, 0xF6, 0x00, 0x64, 0x00, 0x64,
            // Component: flags=0x0001, glyph_index=1
            0x00, 0x01, 0x00, 0x01, // argument1 = -10 (0xFFF6), argument2 = -20 (0xFFEC)
            0xFF, 0xF6, 0xFF, 0xEC,
        ];

        let mut reader = Reader::new(&data);
        let record = GlyfRecord::parse(&mut reader, 13).unwrap();

        match &record.data {
            GlyphData::Composite(composite) => {
                assert_eq!(composite.components[0].argument1, -10);
                assert_eq!(composite.components[0].argument2, -20);
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
                argument1: 10,
                argument2: 20,
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
                    argument1: 5,
                    argument2: 10,
                },
                CompositeComponent {
                    flags: 0x0001,
                    glyph_index: 7,
                    argument1: 15,
                    argument2: 20,
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
