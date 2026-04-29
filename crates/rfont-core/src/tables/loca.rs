use rfont_types::{FontError, Reader};

#[derive(Debug, Clone)]
pub struct Loca {
    pub offsets: Vec<u32>,
}

impl Loca {
    pub fn read_from(reader: &mut Reader, index_to_loc_format: i16, num_glyphs: u16) -> Result<Self, FontError> {
        let mut offsets = Vec::with_capacity(num_glyphs as usize + 1);
        
        if index_to_loc_format == 0 {
            // Short format: u16, scaled by 2
            for _ in 0..=(num_glyphs as usize) {
                offsets.push(reader.read_u16()? as u32 * 2);
            }
        } else {
            // Long format: u32
            for _ in 0..=(num_glyphs as usize) {
                offsets.push(reader.read_u32()?);
            }
        }
        
        Ok(Self { offsets })
    }
}
