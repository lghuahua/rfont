use crate::Font;
use rfont_types::FontError;

/// 重建 hmtx 表
pub fn rebuild_hmtx(font: &Font, subset_glyphs: &[u16]) -> Result<Vec<u8>, FontError> {
    let mut data = Vec::new();
    
    for &glyph_id in subset_glyphs {
        if (glyph_id as usize) < font.hmtx.metrics.len() {
            let metric = &font.hmtx.metrics[glyph_id as usize];
            data.extend_from_slice(&metric.advance_width.to_be_bytes());
            data.extend_from_slice(&metric.lsb.to_be_bytes());
        }
    }
    
    Ok(data)
}
