use crate::Font;
use rfont_types::{FontError, Tag};

/// 提取 glyf 和 loca 数据
pub fn extract_glyf_and_loca(font: &Font, subset_glyphs: &[u16]) -> Result<(Vec<u8>, Vec<u8>), FontError> {
    let num_glyphs = subset_glyphs.len();
    let index_to_loc_format = font.head.index_to_loc_format;
    
    // 构建新的 loca 偏移表
    let mut new_loca_offsets = Vec::with_capacity(num_glyphs + 1);
    let mut new_glyf_data = Vec::new();
    let mut current_offset = 0u32;
    
    for &glyph_id in subset_glyphs {
        new_loca_offsets.push(current_offset);
        
        // 从原始 glyf 表中提取字形数据
        if (glyph_id as usize) < font.loca.offsets.len() - 1 {
            let start = font.loca.offsets[glyph_id as usize];
            let end = font.loca.offsets[glyph_id as usize + 1];
            
            if start < end {
                // 获取原始 glyf 表的字节数据
                let glyf_bytes = font.font_data.get_table_bytes(Tag(*b"glyf"))
                    .ok_or(FontError::TableNotFound { tag: "glyf".to_string() })?;
                
                if (end as usize) <= glyf_bytes.len() {
                    new_glyf_data.extend_from_slice(&glyf_bytes[start as usize..end as usize]);
                    current_offset += end - start;

                    // 对齐到 4 字节边界
                    let padding = (4 - (current_offset % 4)) % 4;
                    for _ in 0..padding {
                        new_glyf_data.push(0);
                        current_offset += 1;
                    }
                }
            }
        }
    }
    
    // 添加最后一个偏移量（指向 glyf 表末尾）
    new_loca_offsets.push(current_offset);
    
    // 编码 loca 表
    let loca_data = if index_to_loc_format == 0 {
        // short format (offset / 2)
        let mut data = Vec::with_capacity(new_loca_offsets.len() * 2);
        for &offset in &new_loca_offsets {
            data.extend_from_slice(&((offset / 2) as u16).to_be_bytes());
        }
        data
    } else {
        // long format
        let mut data = Vec::with_capacity(new_loca_offsets.len() * 4);
        for &offset in &new_loca_offsets {
            data.extend_from_slice(&offset.to_be_bytes());
        }
        data
    };
    
    Ok((loca_data, new_glyf_data))
}
