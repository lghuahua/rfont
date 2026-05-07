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
