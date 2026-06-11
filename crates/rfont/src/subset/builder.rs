use crate::Font;
use rfont_types::FontError;
use std::collections::HashSet;
use tracing::debug;

use super::options::SubsetOptions;

/// 字体子集化 Builder
///
/// 提供链式调用 API，用于配置和执行字体子集化操作。
/// 支持基于文本、字形 ID 或 Unicode 范围创建子集。
///
/// # 示例
/// ```no_run
/// use rfont::Font;
///
/// let font = Font::load("font.ttf").unwrap();
///
/// // 基本用法：基于文本
/// let subset = font.subset_builder()
///     .text("Hello World")
///     .build()
///     .unwrap();
///
/// // 高级用法：自定义配置
/// let subset = font.subset_builder()
///     .unicode_range(0x0041, 0x005A) // A-Z
///     .optimize_post(true)
///     .output_format("woff")
///     .compression_level(9)
///     .build()
///     .unwrap();
/// ```
pub struct FontSubsetBuilder<'a> {
    font: &'a Font,
    text: Option<String>,
    glyph_ids: Option<Vec<u16>>,
    options: SubsetOptions,
}

impl<'a> FontSubsetBuilder<'a> {
    /// 创建新的 Builder
    ///
    /// # 参数
    /// - `font`: 要子集化的字体引用
    ///
    /// # 返回值
    /// 新的 `FontSubsetBuilder` 实例，使用默认配置
    pub fn new(font: &'a Font) -> Self {
        FontSubsetBuilder {
            font,
            text: None,
            glyph_ids: None,
            options: SubsetOptions::default(),
        }
    }

    /// 设置要包含的文本
    ///
    /// 从文本中提取所有字符对应的字形 ID。
    ///
    /// # 参数
    /// - `text`: 要包含的文本字符串
    ///
    /// # 返回值
    /// 更新后的 Builder（支持链式调用）
    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    /// 设置要包含的字形 ID 列表
    ///
    /// 直接指定要包含在子集中的字形 ID。
    ///
    /// # 参数
    /// - `ids`: 字形 ID 向量
    ///
    /// # 返回值
    /// 更新后的 Builder（支持链式调用）
    pub fn glyph_ids(mut self, ids: Vec<u16>) -> Self {
        self.glyph_ids = Some(ids);
        self
    }

    /// 设置是否优化 post 表
    ///
    /// 优化 post 表可以减小文件大小，但会移除字形名称信息。
    ///
    /// # 参数
    /// - `optimize`: 是否优化
    ///
    /// # 返回值
    /// 更新后的 Builder（支持链式调用）
    pub fn optimize_post(mut self, optimize: bool) -> Self {
        self.options.optimize_post_table = optimize;
        self
    }

    /// 设置是否移除字形名称
    ///
    /// 移除字形名称可以显著减小文件大小，但会丢失字形的可读名称。
    ///
    /// # 参数
    /// - `strip`: 是否移除
    ///
    /// # 返回值
    /// 更新后的 Builder（支持链式调用）
    pub fn strip_glyph_names(mut self, strip: bool) -> Self {
        self.options.strip_glyph_names = strip;
        self
    }

    /// 设置 WOFF 压缩级别（0-9）
    ///
    /// 仅在输出格式为 WOFF 时有效。更高的压缩级别会产生更小的文件，但需要更长的处理时间。
    ///
    /// # 参数
    /// - `level`: 压缩级别（0-9），超过 9 会被截断为 9
    ///
    /// # 返回值
    /// 更新后的 Builder（支持链式调用）
    pub fn compression_level(mut self, level: u8) -> Self {
        self.options.compression_level = level.min(9);
        self
    }

    /// 设置是否保留 hinting 数据
    ///
    /// Hinting 数据用于改善小字号下的显示效果，但会增加文件大小。
    ///
    /// # 参数
    /// - `keep`: 是否保留
    ///
    /// # 返回值
    /// 更新后的 Builder（支持链式调用）
    pub fn keep_hinting(mut self, keep: bool) -> Self {
        self.options.keep_hinting = keep;
        self
    }

    /// 设置输出格式（"ttf"、"woff" 或 "woff2"）
    ///
    /// # 参数
    /// - `format`: 输出格式字符串（不区分大小写）
    ///
    /// # 返回值
    /// 更新后的 Builder（支持链式调用）
    pub fn output_format(mut self, format: &str) -> Self {
        self.options.output_format = format.to_lowercase();
        self
    }

    /// 使用预设配置
    ///
    /// 提供常用的配置预设，简化子集化配置过程。
    ///
    /// # 参数
    /// - `preset`: 预设名称
    ///   - `"web"`: Web 优化（WOFF 格式，最大压缩，移除字形名称）
    ///   - `"print"`: 打印优化（TTF 格式，保留所有元数据）
    ///
    /// # 返回值
    /// 更新后的 Builder（支持链式调用）
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// let subset = font.subset_builder()
    ///     .text("Hello")
    ///     .preset("web")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn preset(mut self, preset: &str) -> Self {
        match preset {
            "web" => self.options = SubsetOptions::web_optimized(),
            "print" => self.options = SubsetOptions::print_optimized(),
            _ => {} // 未知预设，保持默认
        }
        self
    }

    /// 构建并执行子集化
    ///
    /// 根据配置收集所有需要包含的字形 ID，执行子集化操作，返回最终的字体数据。
    ///
    /// # 返回值
    /// - `Ok(Vec<u8>)`: 子集化后的字体数据
    /// - `Err(FontError)`: 如果未指定任何字形或子集化失败
    ///
    /// # 错误
    /// - `Generic`: 当没有指定任何字形时（文本、字形 ID、Unicode 范围都为空）
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// match font.subset_builder().text("Hello").build() {
    ///     Ok(data) => println!("子集化成功: {} bytes", data.len()),
    ///     Err(e) => eprintln!("子集化失败: {}", e),
    /// }
    /// ```
    pub fn build(self) -> Result<Vec<u8>, FontError> {
        // 收集需要包含的字形 ID
        let mut needed_glyphs = HashSet::new();

        // 从文本中提取字形 ID
        if let Some(ref text) = self.text {
            let glyph_ids = self.font.text_to_glyph_ids(text);
            needed_glyphs.extend(glyph_ids);
        }

        // 直接指定的字形 ID
        if let Some(ref ids) = self.glyph_ids {
            needed_glyphs.extend(ids.iter().cloned());
        }

        // 确保至少有一个字形
        if needed_glyphs.is_empty() {
            return Err(FontError::Generic(
                "No glyphs specified for subset".to_string(),
            ));
        }

        // 转换为排序的向量
        let mut glyph_ids: Vec<u16> = needed_glyphs.into_iter().collect();
        glyph_ids.sort();

        debug!(glyph_count = glyph_ids.len(), "开始子集化处理");

        // 执行子集化
        self.font.subset_with_options(&glyph_ids, &self.options)
    }
}
