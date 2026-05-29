pub mod checksum;
pub mod tables;

pub use checksum::calc_sfnt_checksum;

pub use tables::cmap::Cmap;
pub use tables::glyf::{GlyfRecord, GlyphData};
pub use tables::head::Head;
pub use tables::hhea::Hhea;
pub use tables::hmtx::{Hmtx, HmtxRecord};
pub use tables::loca::Loca;
pub use tables::maxp::Maxp;

pub fn pad4(data: &mut Vec<u8>) {
    while !data.len().is_multiple_of(4) {
        data.push(0);
    }
}