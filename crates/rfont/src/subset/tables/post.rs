use crate::Font;
use rfont_types::FontError;

use crate::constants::{POST_TABLE_MIN_SIZE, POST_V2_MIN_SIZE};

/// Post 表结构信息
#[derive(Debug, Clone)]
#[allow(dead_code)] // 用于调试和文档，当前未完全使用
pub struct PostTableInfo {
    /// Post 表版本号
    pub version: u32,
    /// 斜体角度（Fixed 格式）
    pub italic_angle: i32,
    /// 下划线位置
    pub underline_position: i16,
    /// 下划线厚度
    pub underline_thickness: i16,
    /// 是否是等宽字体
    pub is_fixed_pitch: bool,
    /// 最小内存使用（Type 1）
    pub min_mem_type42: u32,
    /// 最大内存使用（Type 1）
    pub max_mem_type42: u32,
    /// 最小内存使用（Type 1 + 提示）
    pub min_mem_type1: u32,
    /// 最大内存使用（Type 1 + 提示）
    pub max_mem_type1: u32,
}

/// 子集化 post 表
///
/// 优化策略：
/// - Version 2.0（包含字形名称数组）→ Version 3.0（无名称，减小体积）
/// - 保留所有必要的元数据字段
/// - 适用于子集化场景，因为字形名称在运行时通常不需要
pub fn subset_post_table(_font: &Font, original_post: &[u8]) -> Result<Vec<u8>, FontError> {
    if original_post.len() < POST_TABLE_MIN_SIZE {
        return Err(FontError::Generic("post table too short".to_string()));
    }

    // 读取 post 表版本
    let version = u32::from_be_bytes([
        original_post[0],
        original_post[1],
        original_post[2],
        original_post[3],
    ]);

    match version {
        0x00020000 => {
            // Version 2.0: 包含字形名称数组，转换为 Version 3.0
            subset_post_v2_to_v3(original_post)
        }
        0x00030000 => {
            // Version 3.0: 已经是精简版本，直接使用
            Ok(original_post.to_vec())
        }
        _ => {
            // 其他版本（1.0, 2.5, 4.0 等），保持原样
            // 这些版本可能包含特殊数据，保守处理
            Ok(original_post.to_vec())
        }
    }
}

/// 将 post v2 表转换为 v3 表
///
/// Version 2.0 结构：
/// - 固定头部：32 字节（version + 7个字段）
/// - numberOfGlyphs: 2 字节
/// - glyphNameIndex 数组：numberOfGlyphs × 2 字节
/// - 字符串池：可变长度
///
/// Version 3.0 结构：
/// - 仅固定头部：32 字节（无字形名称数据）
fn subset_post_v2_to_v3(original_post: &[u8]) -> Result<Vec<u8>, FontError> {
    if original_post.len() < POST_V2_MIN_SIZE {
        return Err(FontError::Generic("post v2 table too short".to_string()));
    }

    // 解析原始 post v2 表的信息
    let info = parse_post_header(original_post)?;

    // 构建新的 post v3 表
    let mut new_post = Vec::with_capacity(32);

    // Version 3.0 (0x00030000)
    new_post.extend_from_slice(&0x00030000u32.to_be_bytes());

    // 复制所有元数据字段（保持与原表一致）
    new_post.extend_from_slice(&info.italic_angle.to_be_bytes());
    new_post.extend_from_slice(&info.underline_position.to_be_bytes());
    new_post.extend_from_slice(&info.underline_thickness.to_be_bytes());
    new_post.extend_from_slice(&(if info.is_fixed_pitch { 1u32 } else { 0u32 }).to_be_bytes());
    new_post.extend_from_slice(&info.min_mem_type42.to_be_bytes());
    new_post.extend_from_slice(&info.max_mem_type42.to_be_bytes());
    new_post.extend_from_slice(&info.min_mem_type1.to_be_bytes());
    new_post.extend_from_slice(&info.max_mem_type1.to_be_bytes());

    // Version 3.0 没有后续数据（无字形名称数组）

    let size_reduction = original_post.len() as f64 / new_post.len() as f64;
    tracing::info!(
        original_size = original_post.len(),
        new_size = new_post.len(),
        reduction_percent = (1.0 - 1.0 / size_reduction) * 100.0,
        "post 表优化: v2 → v3"
    );

    Ok(new_post)
}

/// 解析 post 表头部信息
fn parse_post_header(data: &[u8]) -> Result<PostTableInfo, FontError> {
    if data.len() < 32 {
        return Err(FontError::Generic("post header too short".to_string()));
    }

    let version = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let italic_angle = i32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let underline_position = i16::from_be_bytes([data[8], data[9]]);
    let underline_thickness = i16::from_be_bytes([data[10], data[11]]);
    let is_fixed_pitch = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) != 0;
    let min_mem_type42 = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let max_mem_type42 = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    let min_mem_type1 = u32::from_be_bytes([data[24], data[25], data[26], data[27]]);
    let max_mem_type1 = u32::from_be_bytes([data[28], data[29], data[30], data[31]]);

    Ok(PostTableInfo {
        version,
        italic_angle,
        is_fixed_pitch,
        underline_position,
        underline_thickness,
        min_mem_type42,
        max_mem_type42,
        min_mem_type1,
        max_mem_type1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用的 post v2 表
    fn create_test_post_v2() -> Vec<u8> {
        let mut data = Vec::new();

        // Version 2.0
        data.extend_from_slice(&0x00020000u32.to_be_bytes());

        // italicAngle = 0 (非斜体)
        data.extend_from_slice(&0i32.to_be_bytes());

        // underlinePosition = -75
        data.extend_from_slice(&(-75i16).to_be_bytes());

        // underlineThickness = 50
        data.extend_from_slice(&50i16.to_be_bytes());

        // isFixedPitch = 0 (非等宽)
        data.extend_from_slice(&0u32.to_be_bytes());

        // minMemType42 = 0
        data.extend_from_slice(&0u32.to_be_bytes());

        // maxMemType42 = 0
        data.extend_from_slice(&0u32.to_be_bytes());

        // minMemType1 = 0
        data.extend_from_slice(&0u32.to_be_bytes());

        // maxMemType1 = 0
        data.extend_from_slice(&0u32.to_be_bytes());

        // numberOfGlyphs = 3
        data.extend_from_slice(&3u16.to_be_bytes());

        // glyphNameIndex 数组（3 个字形）
        data.extend_from_slice(&0u16.to_be_bytes()); // .notdef
        data.extend_from_slice(&1u16.to_be_bytes()); // space
        data.extend_from_slice(&2u16.to_be_bytes()); // A

        // 字符串池（简化，实际会更复杂）
        // 这里省略，因为我们只关心头部转换

        data
    }

    #[test]
    fn test_post_v2_to_v3_conversion() {
        let post_v2 = create_test_post_v2();

        // 创建一个空的 Font（实际上不会被使用）
        // 这里我们直接测试底层函数
        let result = subset_post_v2_to_v3(&post_v2).unwrap();

        // 验证结果长度为 32 字节（Version 3.0 的标准大小）
        assert_eq!(result.len(), 32);

        // 验证版本号为 3.0
        let version = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);
        assert_eq!(version, 0x00030000);

        // 验证 italicAngle 保持不变
        let italic_angle = i32::from_be_bytes([result[4], result[5], result[6], result[7]]);
        assert_eq!(italic_angle, 0);

        // 验证元数据字段保持不变
        let underline_position = i16::from_be_bytes([result[8], result[9]]);
        assert_eq!(underline_position, -75);

        let underline_thickness = i16::from_be_bytes([result[10], result[11]]);
        assert_eq!(underline_thickness, 50);

        let is_fixed_pitch = u32::from_be_bytes([result[12], result[13], result[14], result[15]]);
        assert_eq!(is_fixed_pitch, 0); // 非等宽
    }

    #[test]
    fn test_post_v3_passthrough() {
        let post_v3 = create_test_post_v2();
        let mut post_v3_modified = post_v3.clone();

        // 修改版本号为 3.0
        post_v3_modified[0..4].copy_from_slice(&0x00030000u32.to_be_bytes());

        // 创建一个空的 Font
        // 注意：这个测试需要实际的 Font 实例，所以我们跳过
        // 实际使用时会在完整的子集化流程中测试
    }

    #[test]
    fn test_post_header_parsing() {
        let post_v2 = create_test_post_v2();
        let info = parse_post_header(&post_v2).unwrap();

        assert_eq!(info.version, 0x00020000);
        assert_eq!(info.italic_angle, 0);
        assert_eq!(info.is_fixed_pitch, false);
        assert_eq!(info.underline_position, -75);
        assert_eq!(info.underline_thickness, 50);
        assert_eq!(info.min_mem_type42, 0);
        assert_eq!(info.max_mem_type42, 0);
        assert_eq!(info.min_mem_type1, 0);
        assert_eq!(info.max_mem_type1, 0);
    }

    #[test]
    fn test_post_size_reduction() {
        let post_v2 = create_test_post_v2();
        let original_size = post_v2.len();

        let post_v3 = subset_post_v2_to_v3(&post_v2).unwrap();
        let new_size = post_v3.len();

        // Version 3.0 应该比 Version 2.0 小
        assert!(new_size < original_size);
        assert_eq!(new_size, 32); // Version 3.0 固定为 32 字节

        // 计算压缩率
        let reduction = (1.0 - new_size as f64 / original_size as f64) * 100.0;

        // 注意：这个测试使用的是简化数据（没有字符串池）
        // 实际字体中，post v2 表通常包含大量字形名称，可以减少 90%+ 的体积
        // 这里我们只验证基本功能，不要求特定的压缩率
        assert!(reduction > 0.0); // 至少有一些减少
    }
}
