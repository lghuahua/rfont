use crate::Font;
use rfont_types::FontError;
use std::collections::HashSet;
use tracing::debug;

use super::options::SubsetOptions;

/// 字体子集化 Builder
pub struct FontSubsetBuilder<'a> {
    font: &'a Font,
    text: Option<String>,
    glyph_ids: Option<Vec<u16>>,
    unicode_ranges: Option<Vec<(u32, u32)>>,
    options: SubsetOptions,
}

impl<'a> FontSubsetBuilder<'a> {
    /// 创建新的 Builder
    pub fn new(font: &'a Font) -> Self {
        FontSubsetBuilder {
            font,
            text: None,
            glyph_ids: None,
            unicode_ranges: None,
            options: SubsetOptions::default(),
        }
    }

    /// 设置要包含的文本
    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    /// 设置要包含的字形 ID 列表
    pub fn glyph_ids(mut self, ids: Vec<u16>) -> Self {
        self.glyph_ids = Some(ids);
        self
    }

    /// 添加 Unicode 范围（start, end）
    pub fn unicode_range(mut self, start: u32, end: u32) -> Self {
        if self.unicode_ranges.is_none() {
            self.unicode_ranges = Some(Vec::new());
        }
        self.unicode_ranges.as_mut().unwrap().push((start, end));
        self
    }

    /// 设置是否优化 post 表
    pub fn optimize_post(mut self, optimize: bool) -> Self {
        self.options.optimize_post_table = optimize;
        self
    }

    /// 设置是否移除字形名称
    pub fn strip_glyph_names(mut self, strip: bool) -> Self {
        self.options.strip_glyph_names = strip;
        self
    }

    /// 设置 WOFF 压缩级别（0-9）
    pub fn compression_level(mut self, level: u8) -> Self {
        self.options.compression_level = level.min(9);
        self
    }

    /// 设置是否保留 hinting 数据
    pub fn keep_hinting(mut self, keep: bool) -> Self {
        self.options.keep_hinting = keep;
        self
    }

    /// 设置输出格式（"ttf" 或 "woff"）
    pub fn output_format(mut self, format: &str) -> Self {
        self.options.output_format = format.to_lowercase();
        self
    }

    /// 使用预设配置
    pub fn preset(mut self, preset: &str) -> Self {
        match preset {
            "web" => self.options = SubsetOptions::web_optimized(),
            "print" => self.options = SubsetOptions::print_optimized(),
            _ => {} // 未知预设，保持默认
        }
        self
    }

    /// 构建并执行子集化
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

        // 从 Unicode 范围中提取字形 ID
        if let Some(ref ranges) = self.unicode_ranges {
            for &(start, end) in ranges {
                for unicode in start..=end {
                    if let Some(&glyph_id) = self.font.cmap.unicode_map.get(&unicode) {
                        needed_glyphs.insert(glyph_id);
                    }
                }
            }
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
