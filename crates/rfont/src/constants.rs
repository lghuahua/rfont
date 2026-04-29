// OpenType 常量
pub const SFNT_CHECKSUM_MAGIC: u32 = 0xB1B0AFBA;
pub const SFNT_VERSION_TTF: u32 = 0x00010000;
pub const LONGDATETIME_EPOCH_YEAR: i32 = 1904;

// 表结构常量
pub const TABLE_DIR_ENTRY_SIZE: usize = 16;
pub const CMAP_HEADER_SIZE: usize = 4; // version + numTables
pub const ENCODING_RECORD_SIZE: usize = 8; // platformID + encodingID + offset
pub const HEAD_TABLE_SIZE: usize = 54;
pub const POST_TABLE_MIN_SIZE: usize = 32;
pub const POST_V2_MIN_SIZE: usize = 36;
