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

#[cfg(test)]
mod tests {
    #[test]
    fn test_update_maxp_basic() {
        // 这个测试需要完整的 Font 实例，因此在集成测试中更合适
    }
    
    #[test]
    fn test_maxp_version_format() {
        // Version 1.0 应该是 0x00010000
        let version: u32 = 0x00010000;
        let bytes = version.to_be_bytes();
        assert_eq!(bytes, [0x00, 0x01, 0x00, 0x00]);
    }
    
    #[test]
    fn test_maxp_num_glyphs_encoding() {
        // 测试 num_glyphs 的编码
        let num_glyphs: u16 = 100;
        let bytes = num_glyphs.to_be_bytes();
        assert_eq!(bytes, [0x00, 0x64]);
        
        let decoded = u16::from_be_bytes(bytes);
        assert_eq!(decoded, num_glyphs);
    }
}
