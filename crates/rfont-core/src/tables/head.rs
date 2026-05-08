use font_macros::{ReadBytes, WriteBytes};
use rfont_types::primitives::LONGDATETIME;
use rfont_types::{FontError, ReadBytes, Reader, WriteBytes, Writer};

#[derive(Debug, Clone, ReadBytes, WriteBytes)]
pub struct Head {
    pub version: u32,
    pub font_revision: u32,
    pub check_sum_adjustment: u32,
    pub magic_number: u32,
    pub flags: u16,
    pub units_per_em: u16,
    pub created: LONGDATETIME,
    pub modified: LONGDATETIME,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub mac_style: u16,
    pub lowest_rec_ppem: u16,
    pub font_direction_hint: i16,
    pub index_to_loc_format: i16,
    pub glyph_data_format: i16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head_magic_number() {
        // Head 表的 magic_number 应该是 0x5F0F3CF5
        let data = vec![
            0x00, 0x01, 0x00, 0x00, // version
            0x00, 0x01, 0x00, 0x00, // font_revision
            0x00, 0x00, 0x00, 0x00, // check_sum_adjustment
            0x5F, 0x0F, 0x3C, 0xF5, // magic_number
            0x00, 0x00, // flags
            0x03, 0xE8, // units_per_em (1000)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // created
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // modified
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // x_min, y_min, x_max, y_max
            0x00, 0x00, // mac_style
            0x00, 0x08, // lowest_rec_ppem
            0x00, 0x00, // font_direction_hint
            0x00, 0x01, // index_to_loc_format (long format)
            0x00, 0x00, // glyph_data_format
        ];
        let mut reader = Reader::new(&data);
        let head = Head::read_from(&mut reader).unwrap();

        assert_eq!(head.magic_number, 0x5F0F3CF5);
        assert_eq!(head.units_per_em, 1000);
        assert_eq!(head.index_to_loc_format, 1);
    }

    #[test]
    fn test_head_units_per_em_validation() {
        let data = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5F, 0x0F,
            0x3C, 0xF5, 0x00, 0x00, 0x04, 0x00, // units_per_em (1024 - valid power of 2)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut reader = Reader::new(&data);
        let head = Head::read_from(&mut reader).unwrap();

        assert_eq!(head.units_per_em, 1024);
    }
}
