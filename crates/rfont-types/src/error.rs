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

    #[error("Failed to parse {table} at offset {offset}: {reason}")]
    ParseError {
        table: String,
        offset: u64,
        reason: String,
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

    /// 获取错误的恢复建议
    /// 
    /// 对于常见错误，提供人类可读的修复建议
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            FontError::InvalidMagicNumber { .. } => {
                Some("文件可能不是有效的字体格式，请检查文件扩展名是否正确")
            }
            FontError::TableNotFound { .. } => {
                Some("字体文件可能损坏或不完整，请尝试重新下载或验证文件完整性")
            }
            FontError::InvalidOffset { .. } => {
                Some("字体文件的表偏移量无效，文件可能已损坏")
            }
            FontError::InvalidChecksum { .. } => {
                Some("字体校验和不匹配，文件可能在传输过程中损坏，请重新下载")
            }
            FontError::UnexpectedEndOfData { .. } => {
                Some("文件被截断或不完整，请确保文件完整下载")
            }
            FontError::WoffDecompressionError { .. } => {
                Some("WOFF 解压缩失败，文件可能损坏或使用了不支持的压缩算法")
            }
            FontError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Some("文件不存在，请检查路径是否正确")
            }
            FontError::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Some("没有权限访问该文件，请检查文件权限")
            }
            _ => None,
        }
    }
}
