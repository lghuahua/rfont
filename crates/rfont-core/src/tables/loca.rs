use rfont_types::{FontError, Reader};

#[derive(Debug, Clone)]
pub struct Loca {
    pub offsets: Vec<u32>,
}

impl Loca {
    pub fn read_from(
        reader: &mut Reader,
        index_to_loc_format: i16,
        num_glyphs: u16,
    ) -> Result<Self, FontError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::io::Reader;

    #[test]
    fn test_loca_short_format() {
        // Short format (index_to_loc_format = 0): u16 * 2
        let data = vec![
            0x00, 0x00, // offset[0] = 0
            0x00, 0x0A, // offset[1] = 10 * 2 = 20
            0x00, 0x14, // offset[2] = 20 * 2 = 40
        ];
        let mut reader = Reader::new(&data);
        let loca = Loca::read_from(&mut reader, 0, 2).unwrap();

        assert_eq!(loca.offsets.len(), 3); // num_glyphs + 1
        assert_eq!(loca.offsets[0], 0);
        assert_eq!(loca.offsets[1], 20);
        assert_eq!(loca.offsets[2], 40);
    }

    #[test]
    fn test_loca_long_format() {
        // Long format (index_to_loc_format = 1): u32
        let data = vec![
            0x00, 0x00, 0x00, 0x00, // offset[0] = 0
            0x00, 0x00, 0x00, 0x14, // offset[1] = 20
            0x00, 0x00, 0x00, 0x28, // offset[2] = 40
        ];
        let mut reader = Reader::new(&data);
        let loca = Loca::read_from(&mut reader, 1, 2).unwrap();

        assert_eq!(loca.offsets.len(), 3);
        assert_eq!(loca.offsets[0], 0);
        assert_eq!(loca.offsets[1], 20);
        assert_eq!(loca.offsets[2], 40);
    }

    #[test]
    fn test_loca_single_glyph() {
        // Single glyph (num_glyphs = 1)
        let data = vec![
            0x00, 0x00, // offset[0] = 0
            0x00, 0x0A, // offset[1] = 10 * 2 = 20
        ];
        let mut reader = Reader::new(&data);
        let loca = Loca::read_from(&mut reader, 0, 1).unwrap();

        assert_eq!(loca.offsets.len(), 2);
        assert_eq!(loca.offsets[0], 0);
        assert_eq!(loca.offsets[1], 20);
    }

    #[test]
    fn test_loca_zero_glyphs() {
        // Zero glyphs (edge case)
        let data = vec![0x00, 0x00]; // Just one offset
        let mut reader = Reader::new(&data);
        let loca = Loca::read_from(&mut reader, 0, 0).unwrap();

        assert_eq!(loca.offsets.len(), 1);
        assert_eq!(loca.offsets[0], 0);
    }
}
