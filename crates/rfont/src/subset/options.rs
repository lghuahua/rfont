use rfont_types::FontError;
use std::fmt;
use std::str::FromStr;

/// 子集输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// 未压缩的 SFNT（TTF/OTF）
    #[default]
    Ttf,
    /// WOFF 1.0（zlib 压缩）
    Woff,
    /// WOFF 2.0（Brotli 压缩）
    Woff2,
}

impl OutputFormat {
    /// 解析格式名称（不区分大小写），无法识别时返回 `None`
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ttf" | "otf" | "sfnt" => Some(Self::Ttf),
            "woff" => Some(Self::Woff),
            "woff2" => Some(Self::Woff2),
            _ => None,
        }
    }

    /// 返回格式名称
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ttf => "ttf",
            Self::Woff => "woff",
            Self::Woff2 => "woff2",
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for OutputFormat {
    type Err = FontError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or_else(|| {
            FontError::new(format!(
                "不支持的输出格式 '{value}'，可选值为 ttf、woff、woff2"
            ))
        })
    }
}

/// 子集化配置选项
///
/// 控制字体子集化的行为和输出格式。
/// 可以通过 `SubsetOptions::default()` 创建默认配置，或使用预设方法快速配置。
///
/// # 表保留策略
///
/// 子集化会重建 `head`、`maxp`、`cmap`、`hhea`、`hmtx`、`loca`、`glyf`，
/// 并按需更新 `name`、`OS/2`、`post`。
///
/// 布局类表（`GSUB`、`GPOS`、`GDEF`、`kern`、`morx` 等）一律丢弃：
/// 子集化后字形 ID 会重新编号，直接复制会导致引用错位，而这些表的
/// 子集化需要独立的字形重映射逻辑。
///
/// `cvt `、`fpgm`、`prep`、`gasp` 仅在 `keep_hinting` 为 `true` 时保留。
///
/// # 示例
/// ```no_run
/// use rfont::{OutputFormat, SubsetOptions};
///
/// // 使用默认配置
/// let options = SubsetOptions::default();
///
/// // 使用 Web 优化预设
/// let web_options = SubsetOptions::web_optimized();
///
/// // 自定义配置
/// let custom_options = SubsetOptions {
///     optimize_post_table: true,
///     strip_glyph_names: false,
///     compression_level: 6,
///     keep_hinting: true,
///     output_format: OutputFormat::Woff,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct SubsetOptions {
    /// 是否将 post 表裁剪为 3.0 版本（移除字形名称描述）
    pub optimize_post_table: bool,
    /// 是否移除字形名称
    ///
    /// 为 `false` 且 `optimize_post_table` 也为 `false` 时，post 2.0 表会按
    /// 子集字形重建，保留对应名称。
    pub strip_glyph_names: bool,
    /// 压缩级别（WOFF: 0-9, WOFF2: 0-11，仅在使用 WOFF/WOFF2 格式时有效）
    pub compression_level: u8,
    /// 是否保留 hinting 数据（glyf 指令及 `cvt `/`fpgm`/`prep`/`gasp` 表）
    pub keep_hinting: bool,
    /// 输出格式
    pub output_format: OutputFormat,
}

impl Default for SubsetOptions {
    fn default() -> Self {
        SubsetOptions {
            optimize_post_table: true,
            strip_glyph_names: true,
            compression_level: 6,
            keep_hinting: false,
            output_format: OutputFormat::Ttf,
        }
    }
}

impl SubsetOptions {
    /// 创建默认配置
    ///
    /// 等同于 `SubsetOptions::default()`。
    ///
    /// # 返回值
    /// 使用默认值的 `SubsetOptions` 实例
    pub fn new() -> Self {
        Self::default()
    }

    /// Web 优化预设（最小文件大小）
    ///
    /// 针对 Web 使用场景优化，追求最小的文件大小：
    /// - 输出格式：WOFF
    /// - 最大压缩级别（9）
    /// - 移除字形名称
    /// - 优化 post 表
    /// - 不保留 hinting 数据
    ///
    /// # 返回值
    /// Web 优化的 `SubsetOptions` 实例
    pub fn web_optimized() -> Self {
        SubsetOptions {
            optimize_post_table: true,
            strip_glyph_names: true,
            compression_level: 9,
            keep_hinting: false,
            output_format: OutputFormat::Woff,
        }
    }

    /// 打印优化预设（保留更多元数据）
    ///
    /// 针对打印和高质量显示场景优化，保留完整的字体信息：
    /// - 输出格式：TTF
    /// - 无压缩
    /// - 保留字形名称
    /// - 不优化 post 表
    /// - 保留 hinting 数据
    ///
    /// # 返回值
    /// 打印优化的 `SubsetOptions` 实例
    pub fn print_optimized() -> Self {
        SubsetOptions {
            optimize_post_table: false,
            strip_glyph_names: false,
            compression_level: 0,
            keep_hinting: true,
            output_format: OutputFormat::Ttf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parse() {
        assert_eq!(OutputFormat::parse("TTF"), Some(OutputFormat::Ttf));
        assert_eq!(OutputFormat::parse("woff"), Some(OutputFormat::Woff));
        assert_eq!(OutputFormat::parse(" Woff2 "), Some(OutputFormat::Woff2));
        assert_eq!(OutputFormat::parse("pfb"), None);
    }

    #[test]
    fn test_output_format_from_str_rejects_unknown() {
        assert_eq!(
            "woff2".parse::<OutputFormat>().unwrap(),
            OutputFormat::Woff2
        );
        assert!("exe".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_presets_have_expected_formats() {
        assert_eq!(
            SubsetOptions::web_optimized().output_format,
            OutputFormat::Woff
        );
        assert_eq!(
            SubsetOptions::print_optimized().output_format,
            OutputFormat::Ttf
        );
    }
}
