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
pub use tables::name::{NameId, NameRecord, NameTable, NameVersion, PlatformId};

pub fn pad4(data: &mut Vec<u8>) {
    while !data.len().is_multiple_of(4) {
        data.push(0);
    }
}

/// 将 u32 值向上对齐到最近的 4 的倍数
///
/// 如果向上舍入会导致溢出，则返回原值
///
/// # Examples
///
/// ```
/// use rfont_core::round4;
/// assert_eq!(round4(0), 0);
/// assert_eq!(round4(1), 4);
/// assert_eq!(round4(4), 4);
/// assert_eq!(round4(5), 8);
/// assert_eq!(round4(u32::MAX), u32::MAX); // 溢出，返回原值
/// ```
pub fn round4(value: u32) -> u32 {
    if value > u32::MAX - 3 {
        value
    } else {
        (value + 3) & !3
    }
}
