/// WOFF2 glyf/loca 表反转换模块
///
/// 参考 Google woff2 项目的实现：
/// - TripletDecode: 三元组解码算法
/// - ReconstructGlyf: glyf 表重建
/// - StorePoints: 点数组转换为标准 glyf 格式
use rfont_types::{FontError, Reader};

// ============================================================================
// 常量定义
// ============================================================================

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
const END_PTS_OF_CONTOURS_OFFSET: usize = 10;
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
#[derive(Debug, Clone)]
pub struct GlyfHeader {
    pub version: u16,
    pub flags: u16,
    pub num_glyphs: u16,
    pub index_format: u16, // 0 = short (2 bytes), 1 = long (4 bytes)
    pub has_overlap_bitmap: bool,
}

/// 子流数据
#[derive(Debug, Clone)]
pub struct SubStreams {
    pub n_contour_stream: Vec<u8>,
    pub n_points_stream: Vec<u8>,
    pub flag_stream: Vec<u8>,
    pub glyph_stream: Vec<u8>,
    pub composite_stream: Vec<u8>,
    pub bbox_stream: Vec<u8>,
    pub instruction_stream: Vec<u8>,
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

/// 读取 255 编码的无符号短整数
fn read_255ushort(reader: &mut Reader) -> Result<u16, FontError> {
    let byte1 = reader.read_u8()?;
    if byte1 < 255 {
        Ok(byte1 as u16)
    } else {
        let byte2 = reader.read_u8()?;
        let byte3 = reader.read_u8()?;
        Ok(255 + ((byte2 as u16) << 8) + byte3 as u16)
    }
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
    triplet_buf: &[u8],
    n_points: usize,
) -> Result<Vec<Point>, FontError> {
    if n_points > triplet_buf.len() {
        return Err(FontError::Generic(format!(
            "TripletDecode: n_points ({}) exceeds buffer size ({})",
            n_points,
            triplet_buf.len()
        )));
    }

    let mut points = Vec::with_capacity(n_points);
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    let mut triplet_index: usize = 0;

    for i in 0..n_points {
        let flag = flags_buf[i];
        let on_curve = (flag >> 7) == 0;
        let flag_low = flag & 0x7f;

        // 根据 flag 值确定数据字节数
        let n_data_bytes = if flag_low < 84 {
            1
        } else if flag_low < 120 {
            2
        } else if flag_low < 124 {
            3
        } else {
            4
        };

        // 边界检查
        if triplet_index + n_data_bytes > triplet_buf.len() {
            return Err(FontError::Generic(format!(
                "TripletDecode: buffer overflow at point {}",
                i
            )));
        }

        // 解码 dx, dy
        let (dx, dy) = if flag_low < 10 {
            // dx = 0, dy 有符号 8 位
            let dy_val = with_sign(
                flag_low,
                ((flag_low & 14) << 7) as i32 + triplet_buf[triplet_index] as i32,
            );
            (0, dy_val)
        } else if flag_low < 20 {
            // dy = 0, dx 有符号 8 位
            let dx_val = with_sign(
                flag_low,
                (((flag_low - 10) & 14) << 7) as i32 + triplet_buf[triplet_index] as i32,
            );
            (dx_val, 0)
        } else if flag_low < 84 {
            // dx, dy 都是小的有符号数（共用 1 字节）
            let b0 = flag_low - 20;
            let b1 = triplet_buf[triplet_index];
            let dx_val = with_sign(flag_low, (1 + (b0 & 0x30) + (b1 >> 4)) as i32);
            let dy_val = with_sign(flag_low >> 1, (1 + ((b0 & 0x0c) << 2) + (b1 & 0x0f)) as i32);
            (dx_val, dy_val)
        } else if flag_low < 120 {
            // dx, dy 都是有符号 9 位（共用 2 字节）
            let b0 = flag_low - 84;
            let dx_val = with_sign(
                flag_low,
                1 + (((b0 as i32) / 12) << 8) + triplet_buf[triplet_index] as i32,
            );
            let dy_val = with_sign(
                flag_low >> 1,
                1 + ((((b0 as i32) % 12) >> 2) << 8) + triplet_buf[triplet_index + 1] as i32,
            );
            (dx_val, dy_val)
        } else if flag_low < 124 {
            // dx 8 位, dy 12 位（共用 3 字节）
            let b2 = triplet_buf[triplet_index + 1];
            let dx_val = with_sign(
                flag_low,
                ((triplet_buf[triplet_index] as i32) << 4) + ((b2 >> 4) as i32),
            );
            let dy_val = with_sign(
                flag_low >> 1,
                (((b2 & 0x0f) as i32) << 8) + triplet_buf[triplet_index + 2] as i32,
            );
            (dx_val, dy_val)
        } else {
            // dx, dy 都是有符号 16 位（共用 4 字节）
            let dx_val = with_sign(
                flag_low,
                ((triplet_buf[triplet_index] as i32) << 8) + triplet_buf[triplet_index + 1] as i32,
            );
            let dy_val = with_sign(
                flag_low >> 1,
                ((triplet_buf[triplet_index + 2] as i32) << 8)
                    + triplet_buf[triplet_index + 3] as i32,
            );
            (dx_val, dy_val)
        };

        triplet_index += n_data_bytes;

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
pub fn compute_bbox(points: &[Point], dst: &mut [u8], offset: usize) -> Result<(), FontError> {
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

    // 写入 xMin, yMin, xMax, yMax（各 2 字节，big-endian）
    let mut pos = offset;

    // xMin
    dst[pos] = ((x_min >> 8) & 0xFF) as u8;
    dst[pos + 1] = (x_min & 0xFF) as u8;
    pos += 2;

    // yMin
    dst[pos] = ((y_min >> 8) & 0xFF) as u8;
    dst[pos + 1] = (y_min & 0xFF) as u8;
    pos += 2;

    // xMax
    dst[pos] = ((x_max >> 8) & 0xFF) as u8;
    dst[pos + 1] = (x_max & 0xFF) as u8;
    pos += 2;

    // yMax
    dst[pos] = ((y_max >> 8) & 0xFF) as u8;
    dst[pos + 1] = (y_max & 0xFF) as u8;

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
    n_contours: u16,
    instruction_length: u16,
    has_overlap_bit: bool,
    glyph_buf: &mut Vec<u8>,
) -> Result<usize, FontError> {
    let n_points = points.len();

    // 计算标志位的起始偏移
    // glyf 结构: nContours(2) + endPts[nContours*2] + instructions(2) + flags[...] + x[] + y[]
    let flag_offset =
        END_PTS_OF_CONTOURS_OFFSET + (n_contours as usize) * 2 + 2 + instruction_length as usize;

    // 确保缓冲区足够大
    // 最坏情况：每个点都需要 2 字节标志 + 2 字节 x + 2 字节 y
    let estimated_size = flag_offset + n_points * 5 + instruction_length as usize;
    if glyph_buf.len() < estimated_size {
        glyph_buf.resize(estimated_size, 0);
    }

    let mut last_flag: i32 = -1;
    let mut repeat_count: u8 = 0;
    let mut last_x: i32 = 0;
    let mut last_y: i32 = 0;
    let mut x_bytes: usize = 0;
    let mut y_bytes: usize = 0;
    let mut current_flag_offset = flag_offset;

    // 第一轮：生成标志位并计算 x/y 字节数
    for (i, point) in points.iter().enumerate() {
        let mut flag: u8 = if point.on_curve { GLYF_ON_CURVE } else { 0 };

        // 第一个点且需要 overlap 标志
        if has_overlap_bit && i == 0 {
            flag |= OVERLAP_SIMPLE;
        }

        let dx = point.x - last_x;
        let dy = point.y - last_y;

        // 判断 x 的编码方式
        if dx == 0 {
            flag |= GLYF_THIS_X_IS_SAME;
        } else if dx > -256 && dx < 256 {
            flag |= GLYF_X_SHORT | if dx > 0 { GLYF_THIS_X_IS_SAME } else { 0 };
            x_bytes += 1;
        } else {
            x_bytes += 2;
        }

        // 判断 y 的编码方式
        if dy == 0 {
            flag |= GLYF_THIS_Y_IS_SAME;
        } else if dy > -256 && dy < 256 {
            flag |= GLYF_Y_SHORT | if dy > 0 { GLYF_THIS_Y_IS_SAME } else { 0 };
            y_bytes += 1;
        } else {
            y_bytes += 2;
        }

        // RLE 压缩标志位
        if flag as i32 == last_flag && repeat_count != 255 {
            // 设置前一个字节的 REPEAT 位
            glyph_buf[current_flag_offset - 1] |= GLYF_REPEAT;
            repeat_count += 1;
        } else {
            if repeat_count != 0 {
                if current_flag_offset >= glyph_buf.len() {
                    return Err(FontError::Generic(
                        "StorePoints: flag buffer overflow".to_string(),
                    ));
                }
                glyph_buf[current_flag_offset] = repeat_count;
                current_flag_offset += 1;
            }
            if current_flag_offset >= glyph_buf.len() {
                return Err(FontError::Generic(
                    "StorePoints: flag buffer overflow".to_string(),
                ));
            }
            glyph_buf[current_flag_offset] = flag;
            current_flag_offset += 1;
            repeat_count = 0;
        }

        last_x = point.x;
        last_y = point.y;
        last_flag = flag as i32;
    }

    // 写入最后的 repeat count
    if repeat_count != 0 {
        if current_flag_offset >= glyph_buf.len() {
            return Err(FontError::Generic(
                "StorePoints: flag buffer overflow".to_string(),
            ));
        }
        glyph_buf[current_flag_offset] = repeat_count;
        current_flag_offset += 1;
    }

    // 第二轮：写入 x/y 坐标数据
    let _xy_bytes = x_bytes + y_bytes; // 保留用于调试
    let x_offset = current_flag_offset;
    let y_offset = current_flag_offset + x_bytes;

    // 确保缓冲区足够大
    let total_needed = y_offset + y_bytes;
    if glyph_buf.len() < total_needed {
        glyph_buf.resize(total_needed, 0);
    }

    let mut current_x_offset = x_offset;
    let mut current_y_offset = y_offset;
    last_x = 0;
    last_y = 0;

    for point in points {
        let dx = point.x - last_x;
        if dx == 0 {
            // 不需要写入
        } else if dx > -256 && dx < 256 {
            glyph_buf[current_x_offset] = dx.unsigned_abs() as u8;
            current_x_offset += 1;
        } else {
            // 2 字节
            glyph_buf[current_x_offset] = ((dx >> 8) & 0xFF) as u8;
            glyph_buf[current_x_offset + 1] = (dx & 0xFF) as u8;
            current_x_offset += 2;
        }
        last_x += dx;

        let dy = point.y - last_y;
        if dy == 0 {
            // 不需要写入
        } else if dy > -256 && dy < 256 {
            glyph_buf[current_y_offset] = dy.unsigned_abs() as u8;
            current_y_offset += 1;
        } else {
            // 2 字节
            glyph_buf[current_y_offset] = ((dy >> 8) & 0xFF) as u8;
            glyph_buf[current_y_offset + 1] = (dy & 0xFF) as u8;
            current_y_offset += 2;
        }
        last_y += dy;
    }

    Ok(current_y_offset)
}

// ============================================================================
// ReconstructGlyf: 完整重建 glyf 表
// ============================================================================

/// 重建 glyf 和 loca 表
///
/// 这是 WOFF2 glyf 表反转换的主入口函数。
///
/// # 参数
/// - `transformed_data`: 转换后的 glyf 表数据
/// - `orig_length`: glyf 表的原始长度（未转换）
/// - `index_format`: loca 表的索引格式（0=short, 1=long）
/// - `num_glyphs`: 字形数量
///
/// # 返回值
/// - `Ok((glyf_data, loca_data))`: 重建后的 glyf 和 loca 表数据
/// - `Err(FontError)`: 重建失败
pub fn reconstruct_glyf_loca(
    transformed_data: &[u8],
    orig_length: u32,
    index_format: u16,
    num_glyphs: u16,
) -> Result<(Vec<u8>, Vec<u8>), FontError> {
    let mut reader = Reader::new(transformed_data);

    // 读取版本和标志
    let _version = reader.read_u16()?;
    let flags = reader.read_u16()?;
    let has_overlap_bitmap = (flags & 0x01) != 0;

    // 验证 num_glyphs 和 index_format
    let parsed_num_glyphs = reader.read_u16()?;
    let parsed_index_format = reader.read_u16()?;

    if parsed_num_glyphs != num_glyphs {
        return Err(FontError::Generic(format!(
            "Glyph count mismatch: expected {}, got {}",
            num_glyphs, parsed_num_glyphs
        )));
    }

    if parsed_index_format != index_format {
        return Err(FontError::Generic(format!(
            "Index format mismatch: expected {}, got {}",
            index_format, parsed_index_format
        )));
    }

    // 读取 7 个子流的大小
    let mut substream_sizes = [0u32; NUM_SUBSTREAMS];
    for size in &mut substream_sizes {
        *size = reader.read_u32()?;
    }

    // 计算子流的起始偏移
    let data_start = reader.offset;
    let mut substream_offsets = [0usize; NUM_SUBSTREAMS];
    let mut current_offset = data_start;

    for (i, &size) in substream_sizes.iter().enumerate() {
        substream_offsets[i] = current_offset;
        current_offset += size as usize;
    }

    // 提取子流数据
    let n_contour_stream =
        &transformed_data[substream_offsets[0]..substream_offsets[0] + substream_sizes[0] as usize];
    let n_points_stream =
        &transformed_data[substream_offsets[1]..substream_offsets[1] + substream_sizes[1] as usize];
    let flag_stream =
        &transformed_data[substream_offsets[2]..substream_offsets[2] + substream_sizes[2] as usize];
    let glyph_stream =
        &transformed_data[substream_offsets[3]..substream_offsets[3] + substream_sizes[3] as usize];
    let composite_stream =
        &transformed_data[substream_offsets[4]..substream_offsets[4] + substream_sizes[4] as usize];
    let bbox_stream =
        &transformed_data[substream_offsets[5]..substream_offsets[5] + substream_sizes[5] as usize];
    let instruction_stream =
        &transformed_data[substream_offsets[6]..substream_offsets[6] + substream_sizes[6] as usize];

    // overlap bitmap（如果存在）
    let overlap_bitmap = if has_overlap_bitmap {
        let bitmap_length = (num_glyphs as usize).div_ceil(8);
        let bitmap_start = current_offset;
        Some(&transformed_data[bitmap_start..bitmap_start + bitmap_length])
    } else {
        None
    };

    // bbox bitmap（总是存在）
    let _bbox_bitmap_start = substream_offsets[5];
    let bbox_bitmap_length = (num_glyphs as usize).div_ceil(32) * 4;
    let bbox_bitmap = &bbox_stream[0..bbox_bitmap_length.min(bbox_stream.len())];

    // 初始化输出缓冲区
    let mut glyf_data = Vec::with_capacity(orig_length as usize);
    let mut loca_values = Vec::with_capacity(num_glyphs as usize + 1);

    // 创建子流读取器
    let mut n_contour_reader = Reader::new(n_contour_stream);
    let mut n_points_reader = Reader::new(n_points_stream);
    let mut flag_reader = Reader::new(flag_stream);
    let mut glyph_reader = Reader::new(glyph_stream);
    let mut composite_reader = Reader::new(composite_stream);
    let mut bbox_reader = Reader::new(bbox_stream);
    let mut instruction_reader = Reader::new(instruction_stream);

    // 跳过 bbox bitmap
    bbox_reader.skip(bbox_bitmap_length)?;

    // 逐字形处理
    for glyph_idx in 0..num_glyphs {
        let glyph_start = glyf_data.len();
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

    // 最后一个 loca 值
    loca_values.push(glyf_data.len() as u32);

    // 构建 loca 表
    let loca_data = build_loca_table(&loca_values, index_format);

    Ok((glyf_data, loca_data))
}

/// 重建简单字形
fn reconstruct_simple_glyph(
    n_contours: u16,
    n_points_reader: &mut Reader,
    flag_reader: &mut Reader,
    glyph_reader: &mut Reader,
    _instruction_reader: &mut Reader,
    have_bbox: bool,
    bbox_reader: &mut Reader,
    has_overlap_bitmap: bool,
    overlap_bitmap: Option<&[u8]>,
    glyph_idx: u16,
    glyf_data: &mut Vec<u8>,
) -> Result<(), FontError> {
    // 读取每个轮廓的点数
    let mut n_points_vec = Vec::new();
    let mut total_n_points: usize = 0;

    for _ in 0..n_contours {
        let n_points_contour = read_255ushort(n_points_reader)? as usize;
        n_points_vec.push(n_points_contour);
        total_n_points += n_points_contour;
    }

    // 读取标志位
    let flags_start = flag_reader.offset;
    let flags_buf = &flag_reader.data[flags_start..flags_start + total_n_points];
    flag_reader.skip(total_n_points)?;

    // 读取三元组数据
    let triplet_start = glyph_reader.offset;
    let remaining = glyph_reader.data.len() - triplet_start;

    // 解码点坐标
    let points = triplet_decode(
        flags_buf,
        &glyph_reader.data[triplet_start..],
        total_n_points,
    )?;

    // 计算消耗的字节数（简化：假设全部消耗）
    // TODO: 正确计算 triplet_decode 消耗的字节数
    let triplet_bytes_consumed = remaining.min(points.len() * 4); // 最坏情况 4 字节/点
    glyph_reader.skip(triplet_bytes_consumed)?;

    // 读取指令长度
    let instruction_length = read_255ushort(glyph_reader)?;

    // 读取指令数据
    let instructions = if instruction_length > 0 {
        let instr_start = glyph_reader.offset;
        let instr_data =
            glyph_reader.data[instr_start..instr_start + instruction_length as usize].to_vec();
        glyph_reader.skip(instruction_length as usize)?;
        instr_data
    } else {
        Vec::new()
    };

    // 构建字形缓冲区
    let mut glyph_buf = Vec::new();

    // 写入 nContours
    glyph_buf.extend_from_slice(&n_contours.to_be_bytes());

    // 写入或计算 bbox
    if have_bbox {
        let bbox_data = bbox_reader.read_bytes(8)?;
        glyph_buf.extend_from_slice(bbox_data);
    } else {
        // 先占位 8 字节
        let current_len = glyph_buf.len();
        glyph_buf.resize(current_len + 8, 0);
        // 计算 bbox 并写入
        compute_bbox(&points, &mut glyph_buf, current_len)?;
    }

    // 写入轮廓结束点
    let mut end_point: i32 = -1;
    for &n_pts in &n_points_vec {
        end_point += n_pts as i32;
        glyph_buf.extend_from_slice(&(end_point as u16).to_be_bytes());
    }

    // 写入指令长度和指令数据
    glyph_buf.extend_from_slice(&instruction_length.to_be_bytes());
    glyph_buf.extend_from_slice(&instructions);

    // 存储点
    let has_overlap_bit = has_overlap_bitmap
        && overlap_bitmap.is_some_and(|bmp| {
            let byte_idx = glyph_idx as usize / 8;
            let bit_idx = glyph_idx as usize % 8;
            byte_idx < bmp.len() && (bmp[byte_idx] >> (7 - bit_idx)) & 1 != 0
        });

    let final_size = store_points(
        &points,
        n_contours,
        instruction_length,
        has_overlap_bit,
        &mut glyph_buf,
    )?;
    glyph_buf.truncate(final_size);

    // 追加到 glyf_data
    glyf_data.extend_from_slice(&glyph_buf);

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
        instruction_size = read_255ushort(glyph_reader)?;
    }

    // 3. 计算总大小并分配缓冲区
    // glyf 结构: nContours(2) + bbox(8) + components(composite_size) + instructionsLen(2) + instructions
    let total_size = 2
        + 8
        + composite_size
        + if have_instructions {
            2 + instruction_size as usize
        } else {
            0
        };
    let start_pos = glyf_data.len();
    glyf_data.resize(start_pos + total_size, 0);

    let mut pos = start_pos;

    // 4. 写入 nContours = 0xFFFF（表示复合字形）
    glyf_data[pos] = 0xFF;
    glyf_data[pos + 1] = 0xFF;
    pos += 2;

    // 5. 写入 bbox
    let bbox_data = bbox_reader.read_bytes(8)?;
    glyf_data[pos..pos + 8].copy_from_slice(bbox_data);
    pos += 8;

    // 6. 复制复合字形组件数据
    let component_start = composite_reader.offset;
    let component_end = component_start + composite_size;

    if component_end > composite_reader.data.len() {
        return Err(FontError::Generic(
            "Composite: component data out of bounds".to_string(),
        ));
    }

    glyf_data[pos..pos + composite_size]
        .copy_from_slice(&composite_reader.data[component_start..component_end]);
    composite_reader.skip(composite_size)?;
    pos += composite_size;

    // 7. 写入指令（如果有）
    if have_instructions {
        glyf_data[pos] = ((instruction_size >> 8) & 0xFF) as u8;
        glyf_data[pos + 1] = (instruction_size & 0xFF) as u8;
        pos += 2;

        // 读取指令数据
        let instr_start = instruction_reader.offset;
        let instr_end = instr_start + instruction_size as usize;

        if instr_end > instruction_reader.data.len() {
            return Err(FontError::Generic(
                "Composite: instruction data out of bounds".to_string(),
            ));
        }

        glyf_data[pos..pos + instruction_size as usize]
            .copy_from_slice(&instruction_reader.data[instr_start..instr_end]);
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

    #[test]
    fn test_triplet_decode_simple() {
        // 简单的测试用例：3 个点，都不在曲线上（off-curve），坐标增量为 0
        // flag = 0x80: on_curve=0 (bit 7=1 means off-curve), flag_low=0 (< 10)
        // 当 flag_low < 10 且为偶数时：dx=0, dy 有符号 8 位
        let flags = vec![0x80, 0x80, 0x80]; // 所有点都不在曲线上
        let triplets = vec![0x00, 0x00, 0x00]; // dy=0 for each point

        let points = triplet_decode(&flags, &triplets, 3).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].x, 0);
        assert_eq!(points[0].y, 0);
        assert!(!points[0].on_curve); // bit 7 = 1 means off-curve
        assert_eq!(points[1].x, 0);
        assert_eq!(points[1].y, 0);
        assert_eq!(points[2].x, 0);
        assert_eq!(points[2].y, 0);
    }

    #[test]
    fn test_size_of_composite_simple() {
        // 测试简单的复合字形：单个组件，无缩放，有指令标志
        // flags(2) + glyph_index(2) + arg1(1) + arg2(1) = 6 bytes
        let composite_data = vec![
            0x00, 0x00, // flags: no MORE_COMPONENTS, no instructions
            0x00, 0x01, // glyph index = 1
            0x00, 0x00, // arg1 = 0, arg2 = 0 (bytes, not words)
        ];

        let (size, have_instructions) = size_of_composite(&composite_data).unwrap();
        assert_eq!(size, 6);
        assert!(!have_instructions);
    }

    #[test]
    fn test_size_of_composite_with_instructions() {
        // 复合字形：有指令标志
        let composite_data = vec![
            0x01, 0x00, // flags: WE_HAVE_INSTRUCTIONS set
            0x00, 0x02, // glyph index = 2
            0x00, 0x00, // arg1 = 0, arg2 = 0
        ];

        let (size, have_instructions) = size_of_composite(&composite_data).unwrap();
        assert_eq!(size, 6);
        assert!(have_instructions);
    }

    #[test]
    fn test_size_of_composite_multiple_components() {
        // 复合字形：多个组件
        let composite_data = vec![
            0x00, 0x20, // flags: MORE_COMPONENTS set
            0x00, 0x01, // glyph index = 1
            0x00, 0x00, // arg1 = 0, arg2 = 0
            0x00, 0x00, // flags: no MORE_COMPONENTS
            0x00, 0x02, // glyph index = 2
            0x00, 0x00, // arg1 = 0, arg2 = 0
        ];

        let (size, have_instructions) = size_of_composite(&composite_data).unwrap();
        assert_eq!(size, 12); // 2 components * 6 bytes each
        assert!(!have_instructions);
    }

    #[test]
    fn test_with_sign() {
        assert_eq!(with_sign(0x01, 100), 100); // 奇数 = 正
        assert_eq!(with_sign(0x00, 100), -100); // 偶数 = 负
    }

    #[test]
    fn test_safe_int_addition() {
        assert_eq!(safe_int_addition(10, 20).unwrap(), 30);
        assert!(safe_int_addition(i32::MAX, 1).is_err());
    }
}
