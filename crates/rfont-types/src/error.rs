use thiserror::Error;

/// 字体处理错误的结构化类型
#[derive(Debug, Error)]
pub enum FontError {
    // ==================== 文件格式错误 ====================
    #[error("Invalid magic number: expected {expected:#010X}, got {actual:#010X}")]
    InvalidMagicNumber { expected: u32, actual: u32 },

    #[error("Invalid file format: {reason} (length: {actual_length} bytes)")]
    InvalidFileFormat {
        reason: String,
        actual_length: usize,
    },

    // ==================== 表相关错误 ====================
    #[error("Table '{tag}' not found in font")]
    TableNotFound { tag: String },

    #[error("Invalid offset in table '{table}': offset {offset} exceeds maximum {max}")]
    InvalidOffset {
        table: String,
        offset: u32,
        max: u32,
    },

    #[error("Invalid table checksum: table '{tag}' expected {expected:#010X}, got {actual:#010X}")]
    InvalidChecksum {
        tag: String,
        expected: u32,
        actual: u32,
    },

    #[error("Table '{table}' is too short: need at least {min_size} bytes, got {actual_size}")]
    TableTooShort {
        table: String,
        min_size: usize,
        actual_size: usize,
    },

    // ==================== 数据解析错误 ====================
    #[error("Unsupported cmap format: {format}")]
    UnsupportedCmapFormat { format: u16 },

    #[error("Unexpected end of data at offset {offset}, needed {needed} bytes{context}", context = context.as_ref().map(|c| format!(" (in {})", c)).unwrap_or_default())]
    UnexpectedEndOfData { 
        offset: usize, 
        needed: usize,
        context: Option<String>,
    },

    #[error("Failed to parse {table} at offset {offset}: {reason}")]
    ParseError {
        table: String,
        offset: u64,
        reason: String,
    },

    // ==================== 压缩/解压缩错误 ====================
    #[error("WOFF decompression failed: {message}")]
    WoffDecompressionError { message: String },

    #[error("WOFF compression failed: {message}")]
    WoffCompressionError { message: String },

    #[error("WOFF2 Brotli decompression failed: {message}")]
    Woff2DecompressionError { message: String },

    // ==================== 其他错误 ====================
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

    /// 创建带上下文的 UnexpectedEndOfData 错误
    ///
    /// # 参数
    /// - `offset`: 当前偏移量
    /// - `needed`: 需要的字节数
    /// - `context`: 错误发生的上下文（如函数名 "Reader::read_u8"）
    pub fn unexpected_end(offset: usize, needed: usize, context: impl Into<String>) -> Self {
        FontError::UnexpectedEndOfData {
            offset,
            needed,
            context: Some(context.into()),
        }
    }

    /// 获取错误的恢复建议
    ///
    /// 对于常见错误，提供人类可读的修复建议
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            FontError::InvalidMagicNumber { .. } => {
                Some("文件可能不是有效的字体格式，请检查文件扩展名是否正确")
            }
            FontError::InvalidFileFormat { .. } => {
                Some("文件格式无效或不完整，请尝试重新下载或验证文件完整性")
            }
            FontError::TableNotFound { .. } => {
                Some("字体文件可能损坏或不完整，请尝试重新下载或验证文件完整性")
            }
            FontError::InvalidOffset { .. } => Some("字体文件的表偏移量无效，文件可能已损坏"),
            FontError::InvalidChecksum { .. } => {
                Some("字体校验和不匹配，文件可能在传输过程中损坏，请重新下载")
            }
            FontError::UnexpectedEndOfData { .. } => Some("文件被截断或不完整，请确保文件完整下载"),
            FontError::TableTooShort { .. } => Some("字体表长度不足，文件可能已损坏"),
            FontError::WoffDecompressionError { .. } => {
                Some("WOFF 解压缩失败，文件可能损坏或使用了不支持的压缩算法")
            }
            FontError::WoffCompressionError { .. } => {
                Some("WOFF 压缩失败，可能是内存不足或系统资源问题")
            }
            FontError::Woff2DecompressionError { .. } => {
                Some("WOFF2 Brotli 解压缩失败，文件可能损坏或使用了不支持的压缩参数")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_magic_number_error() {
        let err = FontError::InvalidMagicNumber {
            expected: 0x00010000,
            actual: 0x12345678,
        };

        let msg = format!("{}", err);
        assert!(msg.contains("0x00010000"));
        assert!(msg.contains("0x12345678"));

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("字体格式"));
    }

    #[test]
    fn test_table_not_found_error() {
        let err = FontError::TableNotFound {
            tag: "glyf".to_string(),
        };

        let msg = format!("{}", err);
        assert!(msg.contains("glyf"));

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
    }

    #[test]
    fn test_invalid_offset_error() {
        let err = FontError::InvalidOffset {
            table: "cmap".to_string(),
            offset: 1000,
            max: 500,
        };

        let msg = format!("{}", err);
        assert!(msg.contains("cmap"));
        assert!(msg.contains("1000"));
        assert!(msg.contains("500"));
    }

    #[test]
    fn test_unsupported_cmap_format_error() {
        let err = FontError::UnsupportedCmapFormat { format: 99 };

        let msg = format!("{}", err);
        assert!(msg.contains("99"));
    }

    #[test]
    fn test_invalid_checksum_error() {
        let err = FontError::InvalidChecksum {
            tag: "head".to_string(),
            expected: 0xB1B0AFBA,
            actual: 0x00000000,
        };

        let msg = format!("{}", err);
        assert!(msg.contains("head"));

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("损坏"));
    }

    #[test]
    fn test_unexpected_end_of_data_error() {
        let err = FontError::UnexpectedEndOfData {
            offset: 100,
            needed: 50,
            context: None,
        };

        let msg = format!("{}", err);
        assert!(msg.contains("100"));
        assert!(msg.contains("50"));

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("截断"));
    }

    #[test]
    fn test_unexpected_end_with_context() {
        let err = FontError::unexpected_end(100, 50, "Reader::read_u8");

        let msg = format!("{}", err);
        assert!(msg.contains("100"));
        assert!(msg.contains("50"));
        assert!(msg.contains("Reader::read_u8"));
        
        // 验证错误消息格式
        println!("Error message: {}", msg);
        assert_eq!(
            msg, 
            "Unexpected end of data at offset 100, needed 50 bytes (in Reader::read_u8)"
        );
    }

    #[test]
    fn test_unexpected_end_without_context() {
        let err = FontError::UnexpectedEndOfData {
            offset: 200,
            needed: 30,
            context: None,
        };

        let msg = format!("{}", err);
        println!("Error message without context: {}", msg);
        assert_eq!(
            msg, 
            "Unexpected end of data at offset 200, needed 30 bytes"
        );
        // 确保没有 "(in ...)" 后缀
        assert!(!msg.contains("(in"));
    }

    #[test]
    fn test_woff_decompression_error() {
        let err = FontError::WoffDecompressionError {
            message: "invalid data".to_string(),
        };

        let msg = format!("{}", err);
        assert!(msg.contains("invalid data"));

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
    }

    #[test]
    fn test_parse_error() {
        let err = FontError::ParseError {
            table: "glyf".to_string(),
            offset: 42,
            reason: "invalid flag".to_string(),
        };

        let msg = format!("{}", err);
        assert!(msg.contains("glyf"));
        assert!(msg.contains("42"));
        assert!(msg.contains("invalid flag"));
    }

    #[test]
    fn test_invalid_base_date_error() {
        let err = FontError::InvalidBaseDate;

        let msg = format!("{}", err);
        assert!(msg.contains("LONGDATETIME"));
    }

    #[test]
    fn test_generic_error() {
        let err = FontError::Generic("something went wrong".to_string());

        let msg = format!("{}", err);
        assert!(msg.contains("something went wrong"));
    }

    #[test]
    fn test_new_error() {
        let err = FontError::new("custom error");

        match err {
            FontError::Generic(msg) => assert_eq!(msg, "custom error"),
            _ => panic!("Expected Generic error"),
        }
    }

    #[test]
    fn test_suggestion_none_for_generic() {
        let err = FontError::Generic("test".to_string());
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn test_suggestion_none_for_parse_error() {
        let err = FontError::ParseError {
            table: "test".to_string(),
            offset: 0,
            reason: "test".to_string(),
        };
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn test_invalid_file_format_error() {
        let err = FontError::InvalidFileFormat {
            reason: "invalid length".to_string(),
            actual_length: 100,
        };

        let msg = format!("{}", err);
        assert!(msg.contains("invalid length"));
        assert!(msg.contains("100"));

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("不完整"));
    }

    #[test]
    fn test_table_too_short_error() {
        let err = FontError::TableTooShort {
            table: "head".to_string(),
            min_size: 54,
            actual_size: 30,
        };

        let msg = format!("{}", err);
        assert!(msg.contains("head"));
        assert!(msg.contains("54"));
        assert!(msg.contains("30"));

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("损坏"));
    }

    #[test]
    fn test_woff_compression_error() {
        let err = FontError::WoffCompressionError {
            message: "out of memory".to_string(),
        };

        let msg = format!("{}", err);
        assert!(msg.contains("out of memory"));

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("内存"));
    }

    #[test]
    fn test_woff2_decompression_error() {
        let err = FontError::Woff2DecompressionError {
            message: "brotli error".to_string(),
        };

        let msg = format!("{}", err);
        assert!(msg.contains("brotli error"));

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("损坏"));
    }

    #[test]
    fn test_io_error_from_std() {
        let std_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: FontError = FontError::from(std_err);

        match &err {
            FontError::Io(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
            }
            _ => panic!("Expected Io error"),
        }

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("文件不存在"));
    }

    #[test]
    fn test_io_error_permission_denied() {
        let std_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err: FontError = FontError::from(std_err);

        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("权限"));
    }
}
