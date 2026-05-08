/// 字体格式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFormat {
    /// TrueType 字体 (TTF)
    Ttf,
    /// OpenType with CFF (OTF)
    Otf,
    /// Web Open Font Format (WOFF)
    Woff,
    /// Web Open Font Format 2 (WOFF2)
    Woff2,
}

impl std::fmt::Display for FontFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontFormat::Ttf => write!(f, "TrueType (TTF)"),
            FontFormat::Otf => write!(f, "OpenType (OTF/CFF)"),
            FontFormat::Woff => write!(f, "WOFF"),
            FontFormat::Woff2 => write!(f, "WOFF2"),
        }
    }
}

/// 压缩类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    /// Zlib 压缩 (WOFF)
    Zlib,
    /// Brotli 压缩 (WOFF2)
    Brotli,
}

impl std::fmt::Display for CompressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionType::Zlib => write!(f, "Zlib"),
            CompressionType::Brotli => write!(f, "Brotli"),
        }
    }
}

/// 字体格式详细信息
#[derive(Debug, Clone)]
pub struct FontFormatInfo {
    /// 字体格式
    pub format: FontFormat,
    /// SFNT 版本（flavor）
    pub version: String,
    /// 是否为可变字体（Variable Font）
    pub is_variable: bool,
    /// 压缩类型（仅 WOFF/WOFF2）
    pub compression: Option<CompressionType>,
    /// 必需表列表
    pub required_tables: Vec<String>,
    /// 可选表列表
    pub optional_tables: Vec<String>,
    /// 所有表的总数
    pub total_tables: u16,
}

impl FontFormatInfo {
    /// 创建新的 FontFormatInfo
    pub fn new(
        format: FontFormat,
        version: String,
        is_variable: bool,
        compression: Option<CompressionType>,
        required_tables: Vec<String>,
        optional_tables: Vec<String>,
        total_tables: u16,
    ) -> Self {
        Self {
            format,
            version,
            is_variable,
            compression,
            required_tables,
            optional_tables,
            total_tables,
        }
    }
    
    /// 检查是否包含所有必需的表
    pub fn has_required_tables(&self, available_tables: &[String]) -> bool {
        self.required_tables.iter().all(|req| available_tables.contains(req))
    }
    
    /// 获取缺失的必需表列表
    pub fn missing_required_tables(&self, available_tables: &[String]) -> Vec<String> {
        self.required_tables
            .iter()
            .filter(|req| !available_tables.contains(req))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_format_display() {
        assert_eq!(format!("{}", FontFormat::Ttf), "TrueType (TTF)");
        assert_eq!(format!("{}", FontFormat::Otf), "OpenType (OTF/CFF)");
        assert_eq!(format!("{}", FontFormat::Woff), "WOFF");
        assert_eq!(format!("{}", FontFormat::Woff2), "WOFF2");
    }

    #[test]
    fn test_compression_type_display() {
        assert_eq!(format!("{}", CompressionType::Zlib), "Zlib");
        assert_eq!(format!("{}", CompressionType::Brotli), "Brotli");
    }

    #[test]
    fn test_font_format_info_new() {
        let info = FontFormatInfo::new(
            FontFormat::Ttf,
            "0x00010000".to_string(),
            false,
            None,
            vec!["cmap".to_string(), "head".to_string()],
            vec!["glyf".to_string()],
            3,
        );

        assert_eq!(info.format, FontFormat::Ttf);
        assert_eq!(info.version, "0x00010000");
        assert!(!info.is_variable);
        assert!(info.compression.is_none());
        assert_eq!(info.required_tables.len(), 2);
        assert_eq!(info.optional_tables.len(), 1);
        assert_eq!(info.total_tables, 3);
    }

    #[test]
    fn test_has_required_tables_all_present() {
        let info = FontFormatInfo::new(
            FontFormat::Ttf,
            "0x00010000".to_string(),
            false,
            None,
            vec!["cmap".to_string(), "head".to_string()],
            vec![],
            2,
        );

        let available = vec!["cmap".to_string(), "head".to_string()];
        assert!(info.has_required_tables(&available));
    }

    #[test]
    fn test_has_required_tables_missing() {
        let info = FontFormatInfo::new(
            FontFormat::Ttf,
            "0x00010000".to_string(),
            false,
            None,
            vec!["cmap".to_string(), "head".to_string(), "maxp".to_string()],
            vec![],
            3,
        );

        let available = vec!["cmap".to_string(), "head".to_string()];
        assert!(!info.has_required_tables(&available));
    }

    #[test]
    fn test_missing_required_tables() {
        let info = FontFormatInfo::new(
            FontFormat::Ttf,
            "0x00010000".to_string(),
            false,
            None,
            vec!["cmap".to_string(), "head".to_string(), "maxp".to_string()],
            vec![],
            3,
        );

        let available = vec!["cmap".to_string(), "head".to_string()];
        let missing = info.missing_required_tables(&available);
        
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "maxp");
    }

    #[test]
    fn test_missing_required_tables_none() {
        let info = FontFormatInfo::new(
            FontFormat::Ttf,
            "0x00010000".to_string(),
            false,
            None,
            vec!["cmap".to_string(), "head".to_string()],
            vec![],
            2,
        );

        let available = vec!["cmap".to_string(), "head".to_string(), "extra".to_string()];
        let missing = info.missing_required_tables(&available);
        
        assert_eq!(missing.len(), 0);
    }

    #[test]
    fn test_font_format_equality() {
        assert_eq!(FontFormat::Ttf, FontFormat::Ttf);
        assert_ne!(FontFormat::Ttf, FontFormat::Otf);
        assert_eq!(FontFormat::Woff, FontFormat::Woff);
        assert_ne!(FontFormat::Woff, FontFormat::Woff2);
    }

    #[test]
    fn test_compression_type_equality() {
        assert_eq!(CompressionType::Zlib, CompressionType::Zlib);
        assert_ne!(CompressionType::Zlib, CompressionType::Brotli);
    }

    #[test]
    fn test_font_format_info_with_compression() {
        let info = FontFormatInfo::new(
            FontFormat::Woff,
            "wOFF".to_string(),
            false,
            Some(CompressionType::Zlib),
            vec![],
            vec![],
            0,
        );

        assert_eq!(info.format, FontFormat::Woff);
        assert_eq!(info.compression, Some(CompressionType::Zlib));
    }

    #[test]
    fn test_font_format_info_variable_font() {
        let info = FontFormatInfo::new(
            FontFormat::Ttf,
            "0x00010000".to_string(),
            true, // 可变字体
            None,
            vec![],
            vec!["fvar".to_string()],
            1,
        );

        assert!(info.is_variable);
        assert_eq!(info.optional_tables[0], "fvar");
    }
}
