/// 子集化配置选项
///
/// 控制字体子集化的行为和输出格式。
/// 可以通过 `SubsetOptions::default()` 创建默认配置，或使用预设方法快速配置。
///
/// # 示例
/// ```no_run
/// use rfont::SubsetOptions;
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
///     output_format: "woff".to_string(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct SubsetOptions {
    /// 是否优化 post 表（移除字形名称以减小文件大小）
    pub optimize_post_table: bool,
    /// 是否移除字形名称
    pub strip_glyph_names: bool,
    /// 压缩级别（WOFF: 0-9, WOFF2: 0-11，仅在使用 WOFF/WOFF2 格式时有效）
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
            output_format: "woff".to_string(),
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
            output_format: "ttf".to_string(),
        }
    }
}
