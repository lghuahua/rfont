use crate::Font;
use rfont_types::FontError;

use crate::constants::{POST_TABLE_MIN_SIZE, POST_V2_MIN_SIZE};

/// 子集化 post 表
pub fn subset_post_table(_font: &Font, original_post: &[u8]) -> Result<Vec<u8>, FontError> {
    if original_post.len() < POST_TABLE_MIN_SIZE {
        return Err(FontError::Generic("post table too short".to_string()));
    }
    
    // 读取 post 表版本
    let version = u32::from_be_bytes([original_post[0], original_post[1], original_post[2], original_post[3]]);
    
    if version == 0x00020000 {
        // Version 2.0: 包含字形名称数组，需要子集化
        subset_post_v2(original_post)
    } else {
        // 其他版本（如 3.0）不包含字形名称，可以直接使用
        Ok(original_post.to_vec())
    }
}

/// 子集化 post v2 表
fn subset_post_v2(original_post: &[u8]) -> Result<Vec<u8>, FontError> {
    if original_post.len() < POST_V2_MIN_SIZE {
        return Err(FontError::Generic("post v2 table too short".to_string()));
    }
    
    // 简化策略：对于只有几个字形的情况，直接使用 post version 3.0（无名称）
    // 这样可以大幅减小文件大小
    let mut new_post = Vec::new();
    new_post.extend_from_slice(&0x00030000u32.to_be_bytes()); // version 3.0
    new_post.extend_from_slice(&original_post[4..36]); // 复制其他字段
    // version 3.0 没有后续数据
    
    println!("  post 表优化: v2 ({}) -> v3 ({})", original_post.len(), new_post.len());
    Ok(new_post)
}
