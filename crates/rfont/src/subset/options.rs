/// 子集化配置选项
#[derive(Debug, Clone)]
pub struct SubsetOptions {
    /// 是否优化 post 表（移除字形名称以减小文件大小）
    pub optimize_post_table: bool,
    /// 是否移除字形名称
    pub strip_glyph_names: bool,
    /// WOFF 压缩级别（0-9，仅在使用 WOFF 格式时有效）
    pub compression_level: u8,
    /// 是否保留 hinting 数据
    pub keep_hinting: bool,
    /// 输出格式（"ttf"、"woff" 或 "woff2"）
    pub output_format: String,
}

impl Default for SubsetOptions {
    fn default() -> Self {
        SubsetOptions {
            optimize_post_table: true,
            strip_glyph_names: true,
            compression_level: 6,
            keep_hinting: false,
            output_format: "ttf".to_string(),
        }
    }
}

impl SubsetOptions {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Web 优化预设（最小文件大小）
    pub fn web_optimized() -> Self {
        SubsetOptions {
            optimize_post_table: true,
            strip_glyph_names: true,
            compression_level: 9,
            keep_hinting: false,
            output_format: "woff".to_string(),
        }
    }
    
    /// 打印优化预设（保留更多元数据）
    pub fn print_optimized() -> Self {
        SubsetOptions {
            optimize_post_table: false,
            strip_glyph_names: false,
            compression_level: 0,
            keep_hinting: true,
            output_format: "ttf".to_string(),
        }
    }
}
