//! 将 TrueType 字形轮廓导出为 SVG
//!
//! 输入是已经加载的 [`rfont::Font`]，输出是 SVG `path` 数据或完整文档。
//! 支持简单字形与复合字形（含缩放、2x2 变换与锚点匹配），文本按 `hmtx`
//! 步进宽度单行排版：每个不同字符定义成一个 `<symbol id="icon-字符">`，
//! 正文用 `<use>` 排开。整份文件仍是文本的画面，同时可以直接当图标使用：
//! `<svg class="icon"><use href="#icon-字"/></svg>`。
//!
//! # 限制
//! - 仅支持 TrueType `glyf` 轮廓，CFF/CFF2 字体返回错误
//! - 不做 kerning 与复杂文字排版（连字、双向文本）
//!
//! # 示例
//! ```no_run
//! use rfont::Font;
//! use rfont_svg::{SvgOptions, text_to_svg};
//!
//! let font = Font::load("font.ttf").unwrap();
//! let svg = text_to_svg(&font, "Hello", &SvgOptions::default()).unwrap();
//! std::fs::write("hello.svg", svg).unwrap();
//! ```

mod outline;
mod path;
mod svg;

pub use outline::{GlyphOutline, Point, glyph_outline};
pub use path::{contours_to_path, outline_to_path};
pub use svg::{
    SvgOptions, TextGlyph, glyph_path, glyph_to_svg, text_glyphs, text_outline, text_to_svg,
};
