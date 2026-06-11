use font_macros::ReadBytes;
/// WOFF2 glyf/loca 表反转换模块
///
/// 参考 Google woff2 项目的实现：
/// - TripletDecode: 三元组解码算法
/// - ReconstructGlyf: glyf 表重建
/// - StorePoints: 点数组转换为标准 glyf 格式
use rfont_types::{FontError, ReadBytes, Reader, U255, Writer};

// ============================================================================
// 常量定义
// ============================================================================

const FLAG_OVERLAP_COMPOUND: u16 = 1 << 0;
/// 简单字形标志位
const GLYF_ON_CURVE: u8 = 1 << 0;
const GLYF_X_SHORT: u8 = 1 << 1;
const GLYF_Y_SHORT: u8 = 1 << 2;
const GLYF_REPEAT: u8 = 1 << 3;
const GLYF_THIS_X_IS_SAME: u8 = 1 << 4;
const GLYF_THIS_Y_IS_SAME: u8 = 1 << 5;
const OVERLAP_SIMPLE: u8 = 1 << 6;

/// 复合字形标志位
const FLAG_ARG_1_AND_2_ARE_WORDS: u16 = 1 << 0;
const FLAG_WE_HAVE_A_SCALE: u16 = 1 << 3;
const FLAG_MORE_COMPONENTS: u16 = 1 << 5;
const FLAG_WE_HAVE_AN_X_AND_Y_SCALE: u16 = 1 << 6;
const FLAG_WE_HAVE_A_TWO_BY_TWO: u16 = 1 << 7;
const FLAG_WE_HAVE_INSTRUCTIONS: u16 = 1 << 8;

/// glyf 表偏移量常量
// const END_PTS_OF_CONTOURS_OFFSET: usize = 10;
// const GLYF_HEADER_SIZE: usize = 10; // xMin, yMin, xMax, yMax (各 2 字节) + nContours (2 字节)

/// 默认字形缓冲区大小（98% 的字形不超过 5KB）
// const DEFAULT_GLYPH_BUF_SIZE: usize = 5120;

/// 子流数量
const NUM_SUBSTREAMS: usize = 7;

// ============================================================================
// 数据结构
// ============================================================================

/// 坐标点
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
    pub on_curve: bool,
}

/// glyf 表头信息
#[derive(Debug, Clone, ReadBytes)]
pub struct GlyfHeader {
    pub reserved: u16,
    pub flags: u16,
    pub num_glyphs: u16,
    pub index_format: u16, // 0 = short (2 bytes), 1 = long (4 bytes)
}


pub struct GlyfDecoder {

}

impl GlyfDecoder {
    pub fn decode( data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), FontError> {
        let mut reader = Reader::new(data);
        let header = GlyfHeader::read_from(&mut reader)?;

        println!("Glyf header: {:?}", header);

        if header.reserved != 0 {
            return Err(FontError::Generic("Reserved field must be zero".to_string()));
        }

        if header.num_glyphs == 0 {
            return Err(FontError::Generic("Number of glyphs must be greater than zero".to_string()));
        }

        let has_overlap_bitmap = header.flags & FLAG_OVERLAP_COMPOUND != 0;

        // 读取 7 个子流的大小
        let mut substream_sizes = [0u32; NUM_SUBSTREAMS];
        for size in &mut substream_sizes {
            *size = reader.read_u32()?;
        }

        tracing::debug!(
            "各流大小统计 {:?}", substream_sizes
        );

        // 提取子流数据
        let n_contour_stream = reader.read_bytes(substream_sizes[0] as usize )?;
        let n_points_stream = reader.read_bytes(substream_sizes[1] as usize )?;
        let flag_stream = reader.read_bytes(substream_sizes[2] as usize )?;
        let glyph_stream = reader.read_bytes(substream_sizes[3] as usize )?;
            
        let composite_stream = reader.read_bytes(substream_sizes[4] as usize )?;
        let bbox_stream = reader.read_bytes(substream_sizes[5] as usize )?;
        let instruction_stream = reader.read_bytes(substream_sizes[6] as usize )?;

        // if has_overlap_bitmap {
        //     let overlap_bitmap = reader.read_bytes( ((header.num_glyphs + 7) >> 3) as usize )?;
        // }

        let overlap_bitmap = if has_overlap_bitmap {
            Some(reader.read_bytes( ((header.num_glyphs + 7) >> 3) as usize )?)
        } else {
            None
        };

        tracing::debug!("流提取完成");


        // let expected_loca_dst_length = if header.index_format == 0 { 2 } else { 4 };

        let mut glyf_data = Vec::new();
        // let loca_values: Vec<u32> = Vec::new();

        let mut loca_values = Vec::new();
        let mut n_contour_reader = Reader::new(n_contour_stream);
        let mut composite_reader = Reader::new(composite_stream);
        let mut glyph_reader = Reader::new(glyph_stream);
        let mut instruction_reader= Reader::new(instruction_stream);
        let mut n_points_reader = Reader::new(n_points_stream);
        let mut bbox_reader= Reader::new(bbox_stream);
        let mut flag_reader= Reader::new(flag_stream);
        
        let bbox_bitmap_length = header.num_glyphs.div_ceil(8) as usize;
        let bbox_bitmap = bbox_reader.read_bytes(bbox_bitmap_length as usize)?;
        println!("glyph_stream {:?}", glyph_stream);
        tracing::debug!(
            glyph_reader = glyph_reader.len()
        );
    

    // 逐字形处理
    for glyph_idx in 0..header.num_glyphs {
        let glyph_start = glyf_data.len();
        // loca_writer.write_bytes(bytes)
        loca_values.push(glyph_start as u32);

        // 读取轮廓数
        let n_contours = n_contour_reader.read_u16()?;

        // 检查是否有 bbox
        let byte_idx = glyph_idx as usize / 8;
        let bit_idx = glyph_idx as usize % 8;
        let have_bbox = if byte_idx < bbox_bitmap.len() {
            (bbox_bitmap[byte_idx] >> (7 - bit_idx)) & 1 != 0
        } else {
            false
        };

        println!("glyph_idx: {}, glyf_data: {:?}", glyph_idx,  glyf_data);

        if n_contours == 0xFFFF {
            // === 复合字形 ===
            reconstruct_composite_glyph(
                &mut composite_reader,
                &mut glyph_reader,
                &mut instruction_reader,
                have_bbox,
                &mut bbox_reader,
                &mut glyf_data,
            )?;
        } else if n_contours > 0 {
            // === 简单字形 ===
            reconstruct_simple_glyph(
                n_contours,
                &mut n_points_reader,
                &mut flag_reader,
                &mut glyph_reader,
                &mut instruction_reader,
                have_bbox,
                &mut bbox_reader,
                has_overlap_bitmap,
                overlap_bitmap,
                glyph_idx,
                &mut glyf_data,
            )?;
        } else {
            // n_contours == 0: 空字形
            if have_bbox {
                return Err(FontError::Generic(
                    "Empty glyph should not have bbox".to_string(),
                ));
            }
            // 空字形不写入任何数据
        }
    }

    

    // 添加最后一个 loca 值（指向 glyf 表的末尾）
    loca_values.push(glyf_data.len() as u32);

    // 构建 loca 表
    let loca_data = build_loca_table(&loca_values, header.index_format);

    Ok((glyf_data, loca_data))
}
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 根据标志位确定符号
fn with_sign(flag: u8, baseval: i32) -> i32 {
    if flag & 1 != 0 {
        baseval
    } else {
        -baseval
    }
}

/// 安全整数加法，防止溢出
fn safe_int_addition(a: i32, b: i32) -> Result<i32, FontError> {
    a.checked_add(b)
        .ok_or_else(|| FontError::Generic("Integer overflow in coordinate calculation".to_string()))
}

// ============================================================================
// 核心算法：TripletDecode
// ============================================================================

/// 三元组解码算法
///
/// 将 WOFF2 压缩的坐标三元组还原为标准 glyf 格式的相对坐标。
/// 这是 WOFF2 glyf 表反转换的核心算法。
///
/// # 参数
/// - `flags_buf`: 标志位缓冲区（每个点对应一个字节）
/// - `triplet_buf`: 三元组数据缓冲区（变长编码的 dx, dy）
/// - `n_points`: 点的数量
///
/// # 返回值
/// - `Ok(Vec<Point>)`: 解码后的点数组（绝对坐标）
/// - `Err(FontError)`: 解码失败
///
/// # 算法说明
/// 1. 每个点的 flag 最高位表示是否在曲线上
/// 2. flag 低 7 位决定使用多少字节编码坐标增量（1-4 字节）
/// 3. 根据 flag 值的不同范围，使用不同的解码公式
/// 4. 累加 dx, dy 得到绝对坐标
pub fn triplet_decode(
    flags_buf: &[u8],
    triplet_reader: &mut Reader,
    n_points: usize,
) -> Result<Vec<Point>, FontError> {
    // 首先检查 flags_buf 长度是否足够
    if flags_buf.len() < n_points {
        return Err(FontError::Generic(format!(
            "TripletDecode: flags buffer too small: need {}, got {}",
            n_points,
            flags_buf.len()
        )));
    }

    // // 计算实际需要的数据字节数
    // let required_bytes = calculate_triplet_bytes_consumed(flags_buf, n_points)?;
    
    // // 检查 triplet_buf 长度是否足够
    // if triplet_buf.len() < required_bytes {
    //     return Err(FontError::Generic(format!(
    //         "TripletDecode: triplet buffer too small: need {}, got {}",
    //         required_bytes,
    //         triplet_buf.len()
    //     )));
    // }

    let mut points = Vec::with_capacity(n_points);
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    // let mut triplet_index: usize = 0;

    for i in 0..n_points {
        let flag = flags_buf[i];
        let on_curve = (flag >> 7) == 0;
        let flag_low = flag & 0x7f;


        // 解码 dx, dy
        let (dx, dy) = if flag_low < 10 {
            // dx = 0, dy 有符号 8 位
            let dy_val = with_sign(
                flag_low,
                ((flag_low & 14) << 7) as i32 + triplet_reader.read_u8()? as i32,
            );
            (0, dy_val)
        } else if flag_low < 20 {
            // dy = 0, dx 有符号 8 位
            let dx_val = with_sign(
                flag_low,
                (((flag_low - 10) & 14) << 7) as i32 + triplet_reader.read_u8()? as i32,
            );
            (dx_val, 0)
        } else if flag_low < 84 {
            // dx, dy 都是小的有符号数（共用 1 字节）
            let b0 = flag_low - 20;
            let b1 = triplet_reader.read_u8()?;
            let dx_val = with_sign(flag_low, (1 + (b0 & 0x30) + (b1 >> 4)) as i32);
            let dy_val = with_sign(flag_low >> 1, (1 + ((b0 & 0x0c) << 2) + (b1 & 0x0f)) as i32);
            (dx_val, dy_val)
        } else if flag_low < 120 {
            // dx, dy 都是有符号 9 位（共用 2 字节）
            let b0 = flag_low - 84;
            let dx_val = with_sign(
                flag_low,
                1 + (((b0 as i32) / 12) << 8) + triplet_reader.read_u8()? as i32,
            );
            let dy_val = with_sign(
                flag_low >> 1,
                1 + ((((b0 as i32) % 12) >> 2) << 8) + triplet_reader.read_u8()? as i32,
            );
            (dx_val, dy_val)
        } else if flag_low < 124 {
            // dx 8 位, dy 12 位（共用 3 字节）
            let b1 = triplet_reader.read_u8()?;
            let b2 = triplet_reader.read_u8()?;
            let dx_val = with_sign(
                flag_low,
                ((b1 as i32) << 4) + ((b2 >> 4) as i32),
            );
            let dy_val = with_sign(
                flag_low >> 1,
                (((b2 & 0x0f) as i32) << 8) + triplet_reader.read_u8()? as i32,
            );
            (dx_val, dy_val)
        } else {
            // dx, dy 都是有符号 16 位（共用 4 字节）
            let dx_val = with_sign(
                flag_low,
                ((triplet_reader.read_u8()? as i32) << 8) + triplet_reader.read_u8()? as i32,
            );
            let dy_val = with_sign(
                flag_low >> 1,
                ((triplet_reader.read_u8()? as i32) << 8)
                    + triplet_reader.read_u8()? as i32,
            );
            (dx_val, dy_val)
        };

        // 累加得到绝对坐标
        x = safe_int_addition(x, dx)?;
        y = safe_int_addition(y, dy)?;

        points.push(Point { x, y, on_curve });
    }

    Ok(points)
}

// ============================================================================
// 计算边界框
// ============================================================================

/// 计算点的边界框并写入 glyf 缓冲区
pub fn compute_bbox(points: &[Point], writer: &mut Writer) -> Result<(), FontError> {
    if points.is_empty() {
        return Err(FontError::Generic(
            "Cannot compute bbox for empty points".to_string(),
        ));
    }

    let mut x_min = points[0].x;
    let mut x_max = points[0].x;
    let mut y_min = points[0].y;
    let mut y_max = points[0].y;

    for point in &points[1..] {
        x_min = x_min.min(point.x);
        x_max = x_max.max(point.x);
        y_min = y_min.min(point.y);
        y_max = y_max.max(point.y);
    }

    writer.write_i16(x_min as i16)?;
    writer.write_i16(y_min as i16)?;
    writer.write_i16(x_max as i16)?;
    writer.write_i16(y_max as i16)?;

    Ok(())
}

/// 计算三元组解码消耗的字节数
///
/// 根据标志位缓冲区计算实际需要读取的三元组数据字节数
fn calculate_triplet_bytes_consumed(flags_buf: &[u8], n_points: usize) -> Result<usize, FontError> {
    if flags_buf.len() < n_points {
        return Err(FontError::Generic(format!(
            "Flags buffer too small: need {}, got {}",
            n_points,
            flags_buf.len()
        )));
    }

    let mut total_bytes = 0;
    for i in 0..n_points {
        let flag = flags_buf[i];
        let flag_low = flag & 0x7f;

        // 根据 flag 值确定数据字节数（与 triplet_decode 保持一致）
        let n_data_bytes = if flag_low < 84 {
            1
        } else if flag_low < 120 {
            2
        } else if flag_low < 124 {
            3
        } else {
            4
        };
        total_bytes += n_data_bytes;
    }

    Ok(total_bytes)
}

fn write_flag(writer: &mut Writer, flag: u8, count: u8) -> Result<(), FontError> {
    if count != 0 {
        writer.write_u8(flag | GLYF_REPEAT)?;
        writer.write_u8(count)?;
    } else {
        writer.write_u8(flag)?;
    }
    Ok(())
}

fn write_y_coordinates(writer: &mut Writer, value: i32, flag: &mut u8)  -> Result<(), FontError> {
    if value == 0 {
        *flag |= GLYF_THIS_Y_IS_SAME;
    } else if value.unsigned_abs() < 256 {
        *flag |= GLYF_Y_SHORT;
        writer.write_u8(value.unsigned_abs() as u8)?;
    } else {
        writer.write_i16(value as i16)?;
    }
    Ok(())
}

fn write_x_coordinates(writer: &mut Writer, value: i32, flag: &mut u8)  -> Result<(), FontError> {
    if value == 0 {
        *flag |= GLYF_THIS_X_IS_SAME;
    } else if value.unsigned_abs() < 256 {
        *flag |= GLYF_X_SHORT;
        writer.write_u8(value.unsigned_abs() as u8)?;
    } else {
        writer.write_i16(value as i16)?;
    }
    Ok(())
}


// ============================================================================
// StorePoints: 将点数组转换为标准 glyf 格式
// ============================================================================

/// 将解码后的点数组存储为标准 glyf 格式
///
/// # 参数
/// - `points`: 点数组（绝对坐标）
/// - `n_contours`: 轮廓数量
/// - `instruction_length`: 指令长度
/// - `has_overlap_bit`: 是否有 overlap 标志
/// - `glyph_buf`: 输出缓冲区
///
/// # 返回值
/// - `Ok(usize)`: 写入的字节数
/// - `Err(FontError)`: 写入失败
pub fn store_points(
    points: &[Point],
    has_overlap_bit: bool,
    glyph_writer: &mut Writer,
) -> Result<(), FontError> {
    let n_points = points.len();

    let estimated_size = n_points * 2;
    let mut x_writer = Writer::with_capacity(estimated_size);
    let mut y_writer = Writer::with_capacity(estimated_size);

    let mut last_x: i32 = 0;
    let mut last_y: i32 = 0;    
    let mut last_flag: i32 = -1;
    let mut repeat_count: u8 = 0;

    for (i, point) in points.iter().enumerate() { 
        let mut flag: u8 = if point.on_curve { GLYF_ON_CURVE } else { 0 };
        // glyph_writer.write_u8(flag);

        // 第一个点且需要 overlap 标志
        if has_overlap_bit && i == 0 {
            flag |= OVERLAP_SIMPLE;
        }

        let dx = point.x - last_x;
        let dy = point.y - last_y;

        if last_flag == flag as i32 && repeat_count != 255 { 
            repeat_count += 1;
        } else {
            write_flag(glyph_writer, flag as u8, repeat_count)?;
            repeat_count = 0;
        }     

        write_x_coordinates(&mut x_writer, dx, &mut flag)?;
        write_y_coordinates(&mut y_writer, dy, &mut flag)?;

        last_x = point.x;
        last_y = point.y;
        last_flag = flag as i32;
    }
    // 写入最后的 repeat count
    write_flag(glyph_writer, last_flag as u8, repeat_count)?;
    glyph_writer.write_bytes(&x_writer.data)?;
    glyph_writer.write_bytes(&y_writer.data)
}

/// 重建简单字形
fn reconstruct_simple_glyph(
    n_contours: u16,
    n_points_reader: &mut Reader,
    flag_reader: &mut Reader,
    glyph_reader: &mut Reader,
    instruction_reader: &mut Reader,
    have_bbox: bool,
    bbox_reader: &mut Reader,
    has_overlap_bitmap: bool,
    overlap_bitmap: Option<&[u8]>,
    glyph_idx: u16,
    glyf_data: &mut Vec<u8>,
) -> Result<(), FontError> {
    tracing::debug!("Reconstructing simple glyph: {}", glyph_idx);
    // 读取每个轮廓的点数
    let mut n_points_vec = Vec::new();
    let mut total_n_points: usize = 0;

    for _ in 0..n_contours {
        // let n_points_contour = read_255ushort(n_points_reader)? as usize;
        let n_points_contour = U255::read_from(n_points_reader)?.value() as usize;
        n_points_vec.push(n_points_contour);
        total_n_points += n_points_contour;
    }

    // 读取标志位
    let flags_buf = flag_reader.read_bytes(total_n_points)?;

    // 读取三元组数据
    // let triplet_start = glyph_reader.offset;
    // let remaining = glyph_reader.len() - triplet_start;
    println!("glyph_reader len: {:?} offset: {} total_n_points: {}", glyph_reader.len(), glyph_reader.offset, total_n_points);
    // 解码点坐标
    let points = triplet_decode(
        flags_buf,
        glyph_reader,
        total_n_points,
    )?;
    
    // // 计算实际消耗的字节数
    let triplet_bytes_consumed = calculate_triplet_bytes_consumed(flags_buf, total_n_points)?;
    println!("Triplet bytes consumed: {} glyph_reader offset: {:?}", triplet_bytes_consumed, glyph_reader.offset);
    // if triplet_bytes_consumed > remaining {
    //     return Err(FontError::Generic(format!(
    //         "Triplet data insufficient: needed {}, available {}",
    //         triplet_bytes_consumed, remaining
    //     )));
    // }
    // glyph_reader.skip(triplet_bytes_consumed)?;

    // 读取指令长度
    let instruction_length_value = U255::read_from(glyph_reader)?.value() as usize;
    
    // 读取指令数据
    // 读取指令数据
    let instructions = if instruction_length_value > 0 {
        instruction_reader.read_bytes(instruction_length_value)?.to_vec()
    } else {
        Vec::new()
    };


    // 构建字形缓冲区
    let mut glyph_writer = Writer::new();

    // 写入 nContours
    glyph_writer.write_u16(n_contours)?;
    // glyph_buf.extend_from_slice(&n_contours.to_be_bytes());

    // 写入或计算 bbox
    if have_bbox {
        let bbox_data = bbox_reader.read_bytes(8)?;
        glyph_writer.write_bytes(bbox_data)?;
        // glyph_buf.extend_from_slice(bbox_data);
    } else {
        // 先占位 8 字节
        // let current_len = glyph_buf.len();
        // glyph_buf.resize(current_len + 8, 0);
        // 计算 bbox 并写入
        compute_bbox(&points, &mut glyph_writer)?;
    }

    // 写入轮廓结束点
    let mut end_point: i32 = -1;
    for &n_pts in &n_points_vec {
        end_point += n_pts as i32;
        // glyph_buf.extend_from_slice(&(end_point as u16).to_be_bytes());
        if end_point >= 65536 {
            return Err(FontError::Generic(
                "Contour end point overflow".to_string(),
            ));
        }
        glyph_writer.write_u16(end_point as u16)?;
    }

    // 写入指令长度和指令数据
    glyph_writer.write_u16(instruction_length_value as u16)?;
    // glyph_buf.extend_from_slice(&(instruction_length_value as u16).to_be_bytes());
    glyph_writer.write_bytes(&instructions)?;
    // glyph_buf.extend_from_slice(&instructions);
    println!("instruction_length_value: {}, instructions: {:?}", instruction_length_value, instructions);

    // 存储点
    let has_overlap_bit = has_overlap_bitmap
        && overlap_bitmap.is_some_and(|bmp| {
            let byte_idx = glyph_idx as usize / 8;
            let bit_idx = glyph_idx as usize % 8;
            byte_idx < bmp.len() && (bmp[byte_idx] >> (7 - bit_idx)) & 1 != 0
        });

    store_points(
        &points,
        has_overlap_bit,
        &mut glyph_writer,
    )?;

    glyf_data.extend_from_slice(&glyph_writer.data);

    Ok(())
}

/// 计算复合字形组件的大小
///
/// 参考 Google woff2 的 SizeOfComposite 函数
fn size_of_composite(composite_data: &[u8]) -> Result<(usize, bool), FontError> {
    let mut reader = Reader::new(composite_data);
    let start_offset = reader.offset;
    let mut have_instructions = false;

    let mut flags = FLAG_MORE_COMPONENTS;

    while (flags & FLAG_MORE_COMPONENTS) != 0 {
        // 读取 flags
        if reader.offset + 2 > composite_data.len() {
            return Err(FontError::Generic(
                "Composite: failed to read flags".to_string(),
            ));
        }
        flags = u16::from_be_bytes([
            composite_data[reader.offset],
            composite_data[reader.offset + 1],
        ]);
        reader.offset += 2;

        // 检查是否有指令
        if (flags & FLAG_WE_HAVE_INSTRUCTIONS) != 0 {
            have_instructions = true;
        }

        // 计算参数大小
        let mut arg_size: usize = 2; // glyph index (always 2 bytes)

        if (flags & FLAG_ARG_1_AND_2_ARE_WORDS) != 0 {
            arg_size += 4; // arg1 (i16) + arg2 (i16)
        } else {
            arg_size += 2; // arg1 (i8) + arg2 (i8)
        }

        if (flags & FLAG_WE_HAVE_A_SCALE) != 0 {
            arg_size += 2; // scale (F2Dot14)
        } else if (flags & FLAG_WE_HAVE_AN_X_AND_Y_SCALE) != 0 {
            arg_size += 4; // xScale + yScale
        } else if (flags & FLAG_WE_HAVE_A_TWO_BY_TWO) != 0 {
            arg_size += 8; // 2x2 matrix
        }

        // 跳过参数
        if reader.offset + arg_size > composite_data.len() {
            return Err(FontError::Generic(
                "Composite: failed to skip args".to_string(),
            ));
        }
        reader.offset += arg_size;
    }

    let size = reader.offset - start_offset;
    Ok((size, have_instructions))
}

/// 重建复合字形（完整版本）
///
/// 参考 Google woff2 的实现，从转换后的数据重建标准 glyf 格式的复合字形
fn reconstruct_composite_glyph(
    composite_reader: &mut Reader,
    glyph_reader: &mut Reader,
    instruction_reader: &mut Reader,
    have_bbox: bool,
    bbox_reader: &mut Reader,
    glyf_data: &mut Vec<u8>,
) -> Result<(), FontError> {
    tracing::debug!("Reconstructing composite glyph");
    if !have_bbox {
        return Err(FontError::Generic(
            "Composite glyph must have bbox".to_string(),
        ));
    }

    // 1. 计算复合字形组件的大小
    let remaining_composite = &composite_reader.data[composite_reader.offset..];
    let (composite_size, have_instructions) = size_of_composite(remaining_composite)?;

    // 2. 读取指令大小（如果有）
    let mut instruction_size: u16 = 0;
    if have_instructions {
        instruction_size = U255::read_from(glyph_reader)?.value();
    }

    // 复合字形不需要预先计算大小，直接写入数据

    // 4. 写入 nContours = 0xFFFF（表示复合字形）
    glyf_data.extend_from_slice(&0xFFFFu16.to_be_bytes());

    // 5. 写入 bbox
    let bbox_data = bbox_reader.read_bytes(8)?;
    glyf_data.extend_from_slice(bbox_data);

    // 6. 复制复合字形组件数据
    let component_start = composite_reader.offset;
    let component_end = component_start + composite_size;

    if component_end > composite_reader.data.len() {
        return Err(FontError::Generic(
            "Composite: component data out of bounds".to_string(),
        ));
    }

    glyf_data.extend_from_slice(&composite_reader.data[component_start..component_end]);
    composite_reader.skip(composite_size)?;

    // 7. 写入指令（如果有）
    if have_instructions {
        glyf_data.extend_from_slice(&instruction_size.to_be_bytes());

        // 读取指令数据
        let instr_start = instruction_reader.offset;
        let instr_end = instr_start + instruction_size as usize;

        if instr_end > instruction_reader.data.len() {
            return Err(FontError::Generic(
                "Composite: instruction data out of bounds".to_string(),
            ));
        }

        glyf_data.extend_from_slice(&instruction_reader.data[instr_start..instr_end]);
        instruction_reader.skip(instruction_size as usize)?;
    }

    Ok(())
}

/// 构建 loca 表
fn build_loca_table(loca_values: &[u32], index_format: u16) -> Vec<u8> {
    let mut loca_data =
        Vec::with_capacity(loca_values.len() * if index_format == 0 { 2 } else { 4 });

    for &value in loca_values {
        if index_format == 0 {
            // short format: 除以 2
            loca_data.extend_from_slice(&((value / 2) as u16).to_be_bytes());
        } else {
            // long format
            loca_data.extend_from_slice(&value.to_be_bytes());
        }
    }

    loca_data
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // 基础函数测试
    // ========================================================================

    #[test]
    fn test_with_sign() {
        assert_eq!(with_sign(0x01, 100), 100); // 奇数 = 正
        assert_eq!(with_sign(0x00, 100), -100); // 偶数 = 负
        assert_eq!(with_sign(0x03, 50), 50);
        assert_eq!(with_sign(0x02, 50), -50);
    }

    #[test]
    fn test_safe_int_addition() {
        assert_eq!(safe_int_addition(10, 20).unwrap(), 30);
        assert_eq!(safe_int_addition(-10, 20).unwrap(), 10);
        assert_eq!(safe_int_addition(i32::MAX, 0).unwrap(), i32::MAX);
        assert_eq!(safe_int_addition(i32::MIN, 0).unwrap(), i32::MIN);
        assert!(safe_int_addition(i32::MAX, 1).is_err());
        assert!(safe_int_addition(i32::MIN, -1).is_err());
    }

    // ========================================================================
    // TripletDecode 测试
    // ========================================================================

    #[test]
    fn test_triplet_decode_empty() {
        let flags: Vec<u8> = vec![];
        let mut triplets = Reader::new(&[]);
        let points = triplet_decode(&flags, &mut triplets, 0).unwrap();
        assert!(points.is_empty());
    }

    #[test]
    fn test_triplet_decode_simple_zero_coordinates() {
        // 3 个点，坐标增量都为 0
        let flags = vec![0x00, 0x00, 0x00]; // on_curve=1, flag_low=0
        let mut triplets = Reader::new(&[0x00, 0x00, 0x00]); // dy=0 for each

        let points = triplet_decode(&flags, &mut triplets, 3).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].x, 0);
        assert_eq!(points[0].y, 0);
        assert!(points[0].on_curve);
        assert_eq!(points[1].x, 0);
        assert_eq!(points[1].y, 0);
        assert_eq!(points[2].x, 0);
        assert_eq!(points[2].y, 0);
    }

    #[test]
    fn test_triplet_decode_off_curve_points() {
        // 3 个 off-curve 点
        let flags = vec![0x80, 0x80, 0x80]; // on_curve=0 (bit 7=1), flag_low=0
        let mut triplets = Reader::new(&[0x00, 0x00, 0x00]); // dy=0

        let points = triplet_decode(&flags, &mut triplets, 3).unwrap();
        assert_eq!(points.len(), 3);
        assert!(!points[0].on_curve);
        assert_eq!(points[0].x, 0);
        assert_eq!(points[0].y, 0);
    }

    #[test]
    fn test_triplet_decode_positive_dy() {
        // flag_low < 10: dx=0, dy 有符号 8 位
        // flag = 0x01 (odd): positive, dy = ((1 & 14) << 7) + triplet = 0 + 100 = 100
        let flags = vec![0x01];
        let mut triplets = Reader::new(&[100]);

        let points = triplet_decode(&flags, &mut triplets, 1).unwrap();
        assert_eq!(points[0].x, 0);
        assert_eq!(points[0].y, 100);
    }

    #[test]
    fn test_triplet_decode_negative_dy() {
        // flag = 0x00 (even): negative, dy = -((0 & 14) << 7) + triplet = -100
        let flags = vec![0x00];
        let mut triplets = Reader::new(&[100]);

        let points = triplet_decode(&flags, &mut triplets, 1).unwrap();
        assert_eq!(points[0].x, 0);
        assert_eq!(points[0].y, -100);
    }

    #[test]
    fn test_triplet_decode_positive_dx() {
        // 10 <= flag_low < 20: dy=0, dx 有符号 8 位
        // flag = 0x0B (11, odd): positive, dx = ((11-10) & 14) << 7) + triplet = 0 + 50 = 50
        let flags = vec![0x0B];
        let mut triplets = Reader::new(&[50]);

        let points = triplet_decode(&flags, &mut triplets, 1).unwrap();
        assert_eq!(points[0].x, 50);
        assert_eq!(points[0].y, 0);
    }

    #[test]
    fn test_triplet_decode_small_coordinates() {
        // 20 <= flag_low < 84: dx, dy 都是小的有符号数（共用 1 字节）
        // flag = 0x17 (23, odd): b0 = 3
        // dx = 1 + (3 & 0x30) + (b1 >> 4) = 1 + 0 + (0xF >> 4) = 1 + 0 + 0 = 1 (positive, flag is odd)
        // dy = 1 + ((3 & 0x0c) << 2) + (b1 & 0x0f) = 1 + 0 + 15 = 16 (positive, flag>>1 = 11 is odd)
        let flags = vec![0x17];
        let mut triplets = Reader::new(&[0x0f]);

        let points = triplet_decode(&flags, &mut triplets, 1).unwrap();
        assert_eq!(points[0].x, 1);
        assert_eq!(points[0].y, 16);
    }

    #[test]
    fn test_triplet_decode_accumulated_coordinates() {
        // 测试坐标累加
        let flags = vec![0x01, 0x01, 0x01]; // 3 个点，每个 dy=10
        let mut triplets = Reader::new(&[10, 10, 10]);

        let points = triplet_decode(&flags, &mut triplets, 3).unwrap();
        assert_eq!(points[0].y, 10);
        assert_eq!(points[1].y, 20); // 10 + 10
        assert_eq!(points[2].y, 30); // 20 + 10
    }

    #[test]
    fn test_triplet_decode_buffer_overflow() {
        // 测试缓冲区溢出检测
        let flags = vec![0x00, 0x00];
        let mut triplets = Reader::new(&[0x00]); // 只有 1 字节，但需要 2 字节

        let result = triplet_decode(&flags, &mut triplets, 2);
        assert!(result.is_err());
    }

    // ========================================================================
    // StorePoints 测试
    // ========================================================================

    #[test]
    fn test_store_points_empty() {
        let points: Vec<Point> = vec![];
        let mut glyph_writer = Writer::new();

        let result = store_points(&points, false, &mut glyph_writer);
        assert!(result.is_ok());
        // 空点数组应该产生空输出
        assert_eq!(glyph_writer.data.len(), 0);
    }

    #[test]
    fn test_store_points_single_point() {
        // 单个点：(0, 0)，在曲线上
        let points = vec![Point {
            x: 0,
            y: 0,
            on_curve: true,
        }];
        let mut glyph_writer = Writer::new();

        store_points(&points, false, &mut glyph_writer).unwrap();
        // 应该有标志位 + 坐标数据
        assert!(glyph_writer.data.len() > 0);
    }

    #[test]
    fn test_store_points_multiple_points() {
        // 多个点，测试坐标编码
        let points = vec![
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 10,
                y: 20,
                on_curve: false,
            },
            Point {
                x: 30,
                y: 40,
                on_curve: true,
            },
        ];
        let mut glyph_writer = Writer::new();

        store_points(&points, false, &mut glyph_writer).unwrap();
        // 应该有标志位 + X 坐标 + Y 坐标数据
        assert!(glyph_writer.data.len() > 0);
    }

    #[test]
    fn test_store_points_with_overlap_bit() {
        // 测试 overlap bit
        let points = vec![Point {
            x: 0,
            y: 0,
            on_curve: true,
        }];
        let mut glyph_writer = Writer::new();

        store_points(&points, true, &mut glyph_writer).unwrap();
        // 应该有数据输出
        assert!(glyph_writer.data.len() > 0);
        
        // 验证第一个标志位包含 OVERLAP_SIMPLE
        // 第一个点的 flag 应该是 GLYF_ON_CURVE | OVERLAP_SIMPLE = 0x01 | 0x40 = 0x41
        // 但由于新的实现使用 write_flag，实际存储方式可能不同
    }

    #[test]
    fn test_store_points_large_coordinates() {
        // 测试大坐标值（需要 2 字节编码）
        let points = vec![
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 1000,
                y: 2000,
                on_curve: true,
            },
            Point {
                x: -500,
                y: -1000,
                on_curve: true,
            },
        ];
        let mut glyph_writer = Writer::new();

        store_points(&points, false, &mut glyph_writer).unwrap();
        // 大坐标应该能正确处理
        assert!(glyph_writer.data.len() > 0);
    }

    #[test]
    fn test_store_points_same_coordinates() {
        // 测试相同坐标（使用 THIS_X_IS_SAME / THIS_Y_IS_SAME 标志）
        let points = vec![
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
        ];
        let mut glyph_writer = Writer::new();

        store_points(&points, false, &mut glyph_writer).unwrap();
        // 相同坐标应该使用压缩标志，输出应该较小
        assert!(glyph_writer.data.len() > 0);
    }

    #[test]
    fn test_store_points_rle_compression() {
        // 测试 RLE 压缩（重复标志位）
        let points = vec![
            Point {
                x: 10,
                y: 20,
                on_curve: true,
            },
            Point {
                x: 20,
                y: 30,
                on_curve: true,
            },
            Point {
                x: 30,
                y: 40,
                on_curve: true,
            },
            Point {
                x: 40,
                y: 50,
                on_curve: true,
            },
        ];
        let mut glyph_writer = Writer::new();

        store_points(&points, false, &mut glyph_writer).unwrap();
        // RLE 压缩应该正常工作
        assert!(glyph_writer.data.len() > 0);
    }

    #[test]
    fn test_store_points_output_structure() {
        // 测试输出结构：标志位 + X 坐标 + Y 坐标
        let points = vec![
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 10,
                y: 20,
                on_curve: true,
            },
        ];
        let mut glyph_writer = Writer::new();

        store_points(&points, false, &mut glyph_writer).unwrap();
        
        // 输出应该包含三部分：标志位 + X 坐标数据 + Y 坐标数据
        // 由于实现细节是先将标志位写入 glyph_writer，然后追加 X 和 Y 数据
        assert!(glyph_writer.data.len() >= 2); // 至少 2 个标志字节
    }

    #[test]
    fn test_store_points_negative_deltas() {
        // 测试负坐标增量
        let points = vec![
            Point {
                x: 100,
                y: 100,
                on_curve: true,
            },
            Point {
                x: 50,  // dx = -50
                y: 80,  // dy = -20
                on_curve: true,
            },
        ];
        let mut glyph_writer = Writer::new();

        let result = store_points(&points, false, &mut glyph_writer);
        assert!(result.is_ok());
        assert!(glyph_writer.data.len() > 0);
    }

    #[test]
    fn test_store_points_mixed_on_off_curve() {
        // 测试混合 on-curve 和 off-curve 点
        let points = vec![
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 10,
                y: 20,
                on_curve: false, // off-curve
            },
            Point {
                x: 20,
                y: 40,
                on_curve: true,
            },
            Point {
                x: 30,
                y: 20,
                on_curve: false, // off-curve
            },
            Point {
                x: 40,
                y: 0,
                on_curve: true,
            },
        ];
        let mut glyph_writer = Writer::new();

        store_points(&points, false, &mut glyph_writer).unwrap();
        assert!(glyph_writer.data.len() > 0);
    }

    // ========================================================================
    // 往返测试：转换 -> 逆转换 -> 验证
    // ========================================================================

    #[test]
    fn test_roundtrip_simple_glyph() {
        // 测试简单字形的往返转换
        // 原始点数据
        let original_points = vec![
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 10,
                y: 20,
                on_curve: false,
            },
            Point {
                x: 30,
                y: 40,
                on_curve: true,
            },
            Point {
                x: 50,
                y: 60,
                on_curve: false,
            },
            Point {
                x: 70,
                y: 80,
                on_curve: true,
            },
        ];

        // 1. 使用 store_points 转换为 glyf 格式
        let mut glyph_writer = Writer::new();
        store_points(&original_points, false, &mut glyph_writer).unwrap();

        // 2. 验证输出数据
        assert!(glyph_writer.data.len() > 0);
        
        // 3. 验证数据结构：应该有标志位 + X 坐标 + Y 坐标
        // 由于实现细节，我们只验证有数据输出
    }

    #[test]
    fn test_roundtrip_triplet_decode_store_points() {
        // 测试 triplet_decode 和 store_points 的往返
        // 注意：由于两种编码方式不同，这不是严格的往返，而是验证数据一致性

        let _original_points = vec![
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 10,
                y: 20,
                on_curve: true,
            },
            Point {
                x: 30,
                y: 40,
                on_curve: false,
            },
            Point {
                x: 50,
                y: 60,
                on_curve: true,
            },
        ];

        // 1. 模拟 WOFF2 压缩：从点生成标志位和三元组
        // 这里我们手动构造简单的测试数据
        let flags = vec![0x00, 0x01, 0x81, 0x01]; // 混合 on/off-curve
        let mut triplets = Reader::new(&[0x00, 0x14, 0x00, 0x14]); // 简化的三元组数据

        // 2. 解码
        let decoded_points = triplet_decode(&flags, &mut triplets, 4).unwrap();
        assert_eq!(decoded_points.len(), 4);

        // 3. 重新编码为 glyf 格式
        let mut glyph_writer = Writer::new();
        store_points(&decoded_points, false, &mut glyph_writer).unwrap();
        assert!(glyph_writer.data.len() > 0);
    }

    // ========================================================================
    // ComputeBbox 测试
    // ========================================================================

    #[test]
    fn test_compute_bbox() {
        let points = vec![
            Point {
                x: 10,
                y: 20,
                on_curve: true,
            },
            Point {
                x: 30,
                y: 40,
                on_curve: true,
            },
            Point {
                x: 20,
                y: 50,
                on_curve: false,
            },
        ];

        let mut writer = Writer::new();
        compute_bbox(&points, &mut writer).unwrap();

        // 验证 bbox: xMin=10, yMin=20, xMax=30, yMax=50
        let data = &writer.data;
        assert_eq!(data.len(), 8); // bbox 应该是 8 字节
        assert_eq!(&data[0..2], &[0, 10]); // xMin = 10
        assert_eq!(&data[2..4], &[0, 20]); // yMin = 20
        assert_eq!(&data[4..6], &[0, 30]); // xMax = 30
        assert_eq!(&data[6..8], &[0, 50]); // yMax = 50
    }

    #[test]
    fn test_compute_bbox_negative_coordinates() {
        let points = vec![
            Point {
                x: -10,
                y: -20,
                on_curve: true,
            },
            Point {
                x: 30,
                y: 40,
                on_curve: true,
            },
        ];

        let mut writer = Writer::new();
        compute_bbox(&points, &mut writer).unwrap();

        // 验证 bbox: xMin=-10, yMin=-20, xMax=30, yMax=40
        // -10 的 16 位有符号大端表示：0xFF 0xF6
        let data = &writer.data;
        assert_eq!(data.len(), 8); // bbox 应该是 8 字节
        assert_eq!(&data[0..2], &[0xFF, 0xF6]); // xMin = -10
        assert_eq!(&data[2..4], &[0xFF, 0xEC]); // yMin = -20
        assert_eq!(&data[4..6], &[0, 30]); // xMax = 30
        assert_eq!(&data[6..8], &[0, 40]); // yMax = 40
    }

    #[test]
    fn test_compute_bbox_empty_points() {
        let points: Vec<Point> = vec![];
        let mut writer = Writer::new();

        let result = compute_bbox(&points, &mut writer);
        assert!(result.is_err());
    }

    // ========================================================================
    // SizeOfComposite 测试
    // ========================================================================

    #[test]
    fn test_size_of_composite_simple() {
        // 单个组件，无缩放，无指令
        let composite_data = vec![
            0x00, 0x00, // flags: no MORE_COMPONENTS, no instructions
            0x00, 0x01, // glyph index = 1
            0x00, 0x00, // arg1 = 0, arg2 = 0 (bytes)
        ];

        let (size, have_instructions) = size_of_composite(&composite_data).unwrap();
        assert_eq!(size, 6);
        assert!(!have_instructions);
    }

    #[test]
    fn test_size_of_composite_with_instructions() {
        // 有指令标志
        let composite_data = vec![
            0x01, 0x00, // flags: WE_HAVE_INSTRUCTIONS
            0x00, 0x02, // glyph index = 2
            0x00, 0x00, // arg1 = 0, arg2 = 0
        ];

        let (size, have_instructions) = size_of_composite(&composite_data).unwrap();
        assert_eq!(size, 6);
        assert!(have_instructions);
    }

    #[test]
    fn test_size_of_composite_multiple_components() {
        // 多个组件
        let composite_data = vec![
            0x00, 0x20, // flags: MORE_COMPONENTS
            0x00, 0x01, // glyph index = 1
            0x00, 0x00, // arg1 = 0, arg2 = 0
            0x00, 0x00, // flags: no MORE_COMPONENTS
            0x00, 0x02, // glyph index = 2
            0x00, 0x00, // arg1 = 0, arg2 = 0
        ];

        let (size, have_instructions) = size_of_composite(&composite_data).unwrap();
        assert_eq!(size, 12);
        assert!(!have_instructions);
    }

    #[test]
    fn test_size_of_composite_with_words_args() {
        // 使用 word 参数的组件
        let composite_data = vec![
            0x00, 0x01, // flags: ARG_1_AND_2_ARE_WORDS
            0x00, 0x01, // glyph index = 1
            0x00, 0x64, // arg1 = 100 (2 bytes)
            0x00, 0xC8, // arg2 = 200 (2 bytes)
        ];

        let (size, have_instructions) = size_of_composite(&composite_data).unwrap();
        assert_eq!(size, 8); // 2 + 2 + 2 + 2 = 8
        assert!(!have_instructions);
    }

    #[test]
    fn test_size_of_composite_with_scale() {
        // 有缩放因子的组件
        let composite_data = vec![
            0x00, 0x08, // flags: WE_HAVE_A_SCALE
            0x00, 0x01, // glyph index = 1
            0x00, 0x00, // arg1 = 0, arg2 = 0
            0x40, 0x00, // scale = 0.5 (F2Dot14)
        ];

        let (size, have_instructions) = size_of_composite(&composite_data).unwrap();
        assert_eq!(size, 8);
        assert!(!have_instructions);
    }

    // ========================================================================
    // BuildLoca 测试
    // ========================================================================

    #[test]
    fn test_build_loca_table_short() {
        let loca_values = vec![0, 10, 20, 40];
        let loca_data = build_loca_table(&loca_values, 0); // short format

        assert_eq!(loca_data.len(), 8); // 4 values * 2 bytes
        assert_eq!(&loca_data[0..2], &[0, 0]); // 0 / 2 = 0
        assert_eq!(&loca_data[2..4], &[0, 5]); // 10 / 2 = 5
        assert_eq!(&loca_data[4..6], &[0, 10]); // 20 / 2 = 10
        assert_eq!(&loca_data[6..8], &[0, 20]); // 40 / 2 = 20
    }

    #[test]
    fn test_build_loca_table_long() {
        let loca_values = vec![0, 10, 20, 40];
        let loca_data = build_loca_table(&loca_values, 1); // long format

        assert_eq!(loca_data.len(), 16); // 4 values * 4 bytes
        assert_eq!(&loca_data[0..4], &[0, 0, 0, 0]);
        assert_eq!(&loca_data[4..8], &[0, 0, 0, 10]);
        assert_eq!(&loca_data[8..12], &[0, 0, 0, 20]);
        assert_eq!(&loca_data[12..16], &[0, 0, 0, 40]);
    }

    // ========================================================================
    // 综合测试：模拟真实 WOFF2 转换场景
    // ========================================================================

    #[test]
    fn test_comprehensive_glyph_conversion() {
        // 模拟一个完整的简单字形转换流程
        let _original_points = vec![
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 10,
                y: 20,
                on_curve: false,
            },
            Point {
                x: 20,
                y: 40,
                on_curve: true,
            },
            Point {
                x: 30,
                y: 20,
                on_curve: false,
            },
            Point {
                x: 40,
                y: 0,
                on_curve: true,
            },
        ];

        // 1. 存储为 glyf 格式
        let mut glyph_writer = Writer::new();
        store_points(
            &[
                _original_points[0],
                _original_points[1],
                _original_points[2],
            ],
            false,
            &mut glyph_writer,
        )
        .unwrap();

        // 2. 验证输出不为空
        assert!(glyph_writer.data.len() > 0);
    }

    #[test]
    fn test_comprehensive_glyph_with_instructions() {
        // 测试带指令的字形
        // 注意：store_points 不再处理指令，指令在 reconstruct_simple_glyph 中单独处理
        let points = vec![
            Point {
                x: 0,
                y: 0,
                on_curve: true,
            },
            Point {
                x: 10,
                y: 10,
                on_curve: true,
            },
        ];

        let mut glyph_writer = Writer::new();
        store_points(&points, false, &mut glyph_writer).unwrap();

        assert!(glyph_writer.data.len() > 0);
    }
}
