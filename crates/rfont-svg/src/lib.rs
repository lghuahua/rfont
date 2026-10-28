//! 将 TrueType 字形轮廓导出为 SVG
//!
//! 输入是已经加载的 [`rfont::Font`]，输出是 SVG `path` 数据或完整文档。
//! 支持简单字形与复合字形（含缩放、2x2 变换与锚点匹配），文本按 `hmtx`
//! 步进宽度单行排版。
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
pub use svg::{SvgOptions, glyph_path, glyph_to_svg, text_outline, text_path, text_to_svg};
