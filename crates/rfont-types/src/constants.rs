/// OpenType 通用常量
///
/// 这些常量在 OpenType/TrueType 规范中定义，被多个 crate 共享使用。

// SFNT 版本和校验和
pub const SFNT_CHECKSUM_MAGIC: u32 = 0xB1B0AFBA;
pub const SFNT_VERSION_TTF: u32 = 0x00010000;
pub const SFNT_VERSION_OTF: u32 = 0x4F54544F; // 'OTTO'
pub const LONGDATETIME_EPOCH_YEAR: i32 = 1904;

// 表结构常量
pub const TABLE_DIR_ENTRY_SIZE: usize = 16;
pub const CMAP_HEADER_SIZE: usize = 4; // version + numTables
pub const ENCODING_RECORD_SIZE: usize = 8; // platformID + encodingID + offset
pub const HEAD_TABLE_SIZE: usize = 54;
pub const POST_TABLE_MIN_SIZE: usize = 32;
pub const POST_V2_MIN_SIZE: usize = 36;

// WOFF/WOFF2 常量
pub const WOFF_SIGNATURE: u32 = 0x774F4646; // 'wOFF'
pub const WOFF2_SIGNATURE: u32 = 0x774F4632; // 'wOF2'
