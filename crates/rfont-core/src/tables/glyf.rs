use rfont_types::{FontError, Reader, Writer, WriteBytes};

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
            return Ok(Self { glyph_id, data: GlyphData::Empty });
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

                components.push(CompositeComponent { flags, glyph_index, argument1: arg1, argument2: arg2 });

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
                if total_points == 0 { return Ok(()); }

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
                    } else if dx < 0 && dx >= -255 {
                        flag |= 0x10; // X_IS_BYTE (negative, bit 1 is 0)
                        // 注意：在写入时，负数需要取绝对值存入单字节
                    }
                    
                    // 处理 Y 坐标标志
                    if dy == 0 {
                        flag |= 0x04; // THIS_Y_IS_SAME
                    } else if dy > 0 && dy <= 255 {
                        flag |= 0x24; // Y_IS_BYTE | THIS_Y_IS_SAME (positive)
                    } else if dy < 0 && dy >= -255 {
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
                    if (flag & 0x10) != 0 { // X_IS_BYTE
                        // 如果是单字节存储，根据 THIS_X_IS_SAME 位判断正负
                        if (flag & 0x02) != 0 {
                            writer.write_u8(dx as u8)?; // Positive or Zero
                        } else {
                            writer.write_u8((-dx) as u8)?; // Negative
                        }
                    } else {
                        if (flag & 0x02) == 0 { // Not same, must be short
                            writer.write_i16(dx)?;
                        }
                        // If THIS_X_IS_SAME and not X_IS_BYTE, dx should be 0, write nothing
                    }
                }

                // 7. 写入 Y 坐标增量
                for (i, &dy) in y_coords_delta.iter().enumerate() {
                    let flag = processed_flags[i];
                    if (flag & 0x20) != 0 { // Y_IS_BYTE
                        if (flag & 0x04) != 0 {
                            writer.write_u8(dy as u8)?;
                        } else {
                            writer.write_u8((-dy) as u8)?;
                        }
                    } else {
                        if (flag & 0x04) == 0 {
                            writer.write_i16(dy)?;
                        }
                    }
                }

                Ok(())
            },
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
                    if i < last { flags |= 0x0020; } // MORE_COMPONENTS
                    
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
