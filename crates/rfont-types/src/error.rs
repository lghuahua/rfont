use thiserror::Error;

/// 字体处理错误的结构化类型
#[derive(Debug, Error)]
pub enum FontError {
    #[error("Invalid magic number: expected {expected:#010X}, got {actual:#010X}")]
    InvalidMagicNumber { 
        expected: u32, 
        actual: u32 
    },

    #[error("Table '{tag}' not found in font")]
    TableNotFound { 
        tag: String 
    },

    #[error("Invalid offset in table '{table}': offset {offset} exceeds maximum {max}")]
    InvalidOffset { 
        table: String, 
        offset: u32, 
        max: u32 
    },

    #[error("Unsupported cmap format: {format}")]
    UnsupportedCmapFormat { 
        format: u16 
    },

    #[error("Invalid table checksum: table '{tag}' expected {expected:#010X}, got {actual:#010X}")]
    InvalidChecksum { 
        tag: String,
        expected: u32,
        actual: u32 
    },

    #[error("Unexpected end of data at offset {offset}, needed {needed} bytes")]
    UnexpectedEndOfData { 
        offset: usize, 
        needed: usize 
    },

    #[error("Invalid glyph index: {glyph_id} exceeds maximum {max_glyphs}")]
    InvalidGlyphIndex { 
        glyph_id: u16, 
        max_glyphs: u16 
    },

    #[error("Invalid units per em: {value} (must be between 16 and 16384)")]
    InvalidUnitsPerEm { 
        value: u16 
    },

    #[error("WOFF decompression failed: {message}")]
    WoffDecompressionError { 
        message: String 
    },

    #[error("Invalid base date for LONGDATETIME calculation")]
    InvalidBaseDate,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Generic error: {0}")]
    Generic(String),
}

impl FontError {
    /// 创建通用错误（向后兼容）
    pub fn new(message: impl Into<String>) -> Self {
        FontError::Generic(message.into())
    }
}
