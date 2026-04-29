use crate::Font;
use rfont_types::FontError;

/// 更新 maxp 表
pub fn update_maxp(font: &Font, num_glyphs: u16) -> Result<Vec<u8>, FontError> {
    let mut data = Vec::new();
    
    // version 1.0
    data.extend_from_slice(&0x00010000u32.to_be_bytes());
    data.extend_from_slice(&num_glyphs.to_be_bytes());
    
    // v1.0 字段（如果存在则写入，否则使用默认值 0）
    data.extend_from_slice(&font.maxp.max_points.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_contours.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_composite_points.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_composite_contours.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_zones.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_twilight_points.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_storage.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_function_defs.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_instruction_defs.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_stack_elements.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_size_of_instructions.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_component_elements.unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&font.maxp.max_component_depth.unwrap_or(0).to_be_bytes());
    
    Ok(data)
}
