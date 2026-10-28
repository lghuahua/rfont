//! SVG 文档生成

use rfont::Font;
use rfont_types::FontError;

use crate::outline::{GlyphOutline, glyph_outline};
use crate::path::{contours_to_path, num, outline_to_path};

/// SVG 输出选项
#[derive(Debug, Clone, PartialEq)]
pub struct SvgOptions {
    /// 字号（像素），决定输出尺寸
    pub font_size: f64,
    /// 填充色，接受 `#rgb`、`#rrggbb`、`rgb(...)` 等 CSS 颜色
    pub fill: String,
    /// 四周留白（像素）
    pub padding: f64,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            font_size: 64.0,
            fill: "#000000".to_string(),
            padding: 4.0,
        }
    }
}

/// 单个字形的 path 数据（字体单位，Y 轴向上）
pub fn glyph_path(font: &Font, glyph_id: u16) -> Result<String, FontError> {
    Ok(outline_to_path(&glyph_outline(font, glyph_id)?))
}

/// 单行文本的 path 数据，按字体的步进宽度横向排列
pub fn text_path(font: &Font, text: &str) -> Result<String, FontError> {
    Ok(contours_to_path(&text_outline(font, text)?.contours))
}

/// 单行文本的整体轮廓，字形按 `hmtx` 步进宽度累加
pub fn text_outline(font: &Font, text: &str) -> Result<GlyphOutline, FontError> {
    let glyph_ids = font.get_glyph_ids_for_text(text);
    if glyph_ids.is_empty() {
        return Err(FontError::NoGlyphsFound);
    }

    let mut contours = Vec::new();
    let mut pen = 0.0;
    for glyph_id in glyph_ids {
        let mut outline = glyph_outline(font, glyph_id)?;
        let advance = outline.advance_width as f64;
        outline.translate(pen, 0.0);
        contours.extend(outline.contours);
        pen += advance;
    }

    Ok(GlyphOutline {
        contours,
        advance_width: pen.round().clamp(0.0, u16::MAX as f64) as u16,
    })
}

/// 生成单个字形的 SVG 文档
pub fn glyph_to_svg(font: &Font, glyph_id: u16, options: &SvgOptions) -> Result<String, FontError> {
    let outline = glyph_outline(font, glyph_id)?;
    render(font, &outline, options)
}

/// 生成单行文本的 SVG 文档
pub fn text_to_svg(font: &Font, text: &str, options: &SvgOptions) -> Result<String, FontError> {
    let outline = text_outline(font, text)?;
    render(font, &outline, options)
}

fn render(font: &Font, outline: &GlyphOutline, options: &SvgOptions) -> Result<String, FontError> {
    // NaN 也走这里：字号必须是确定的有限正数
    if !options.font_size.is_finite() || options.font_size <= 0.0 {
        return Err(FontError::Generic("字号必须大于 0".to_string()));
    }

    let units_per_em = font.head().units_per_em.max(1) as f64;
    let scale = options.font_size / units_per_em;
    let padding = options.padding.max(0.0);
    let fill = sanitize_fill(&options.fill);

    let advance = outline.advance_width as f64;
    let path = contours_to_path(&outline.contours);
    let (min_x, min_y, max_x, max_y) = outline
        .bounds()
        .map(|(min_x, min_y, max_x, max_y)| (min_x, min_y, max_x.max(advance), max_y))
        .unwrap_or((0.0, 0.0, advance, 0.0));

    let width = (max_x - min_x) * scale + padding * 2.0;
    let height = (max_y - min_y) * scale + padding * 2.0;
    // 先缩放再平移：把字体单位映射到像素，并翻转 Y 轴
    let translate_x = padding - min_x * scale;
    let translate_y = padding + max_y * scale;

    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><path d=\"{path}\" fill=\"{fill}\" transform=\"translate({tx} {ty}) scale({scale} -{scale})\"/></svg>\n",
        w = num(width),
        h = num(height),
        tx = num(translate_x),
        ty = num(translate_y),
        scale = num(scale),
    ))
}

/// 只接受 CSS 颜色里可能出现的字符，避免把引号或标签注进属性
fn sanitize_fill(fill: &str) -> String {
    let looks_like_color = fill.chars().any(|ch| ch.is_ascii_alphanumeric())
        && fill.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(ch, '#' | '(' | ')' | ',' | '.' | '%' | '-' | ' ')
        });
    if looks_like_color {
        fill.to_string()
    } else {
        "#000000".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn font() -> Font {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rfont/src/AlimamaDaoLiTi.ttf");
        Font::load(path).unwrap()
    }

    #[test]
    fn glyph_svg_has_document_and_path() {
        let font = font();
        let glyph_id = font.text_to_glyph_ids("A")[0];
        let svg = glyph_to_svg(&font, glyph_id, &SvgOptions::default()).unwrap();

        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("<path d=\"M"));
        assert!(svg.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn text_svg_is_wider_than_single_glyph() {
        let font = font();
        let one = text_to_svg(&font, "A", &SvgOptions::default()).unwrap();
        let three = text_to_svg(&font, "AAA", &SvgOptions::default()).unwrap();

        let width = |svg: &str| -> f64 {
            let start = svg.find("width=\"").unwrap() + 7;
            let end = svg[start..].find('"').unwrap() + start;
            svg[start..end].parse().unwrap()
        };
        assert!(width(&three) > width(&one));
    }

    #[test]
    fn text_outline_advances_by_glyph_width() {
        let font = font();
        let outline = text_outline(&font, "AB").unwrap();

        let expected: u16 = font
            .text_to_glyph_ids("AB")
            .into_iter()
            .map(|id| font.hmtx().get_metric(id).unwrap().advance_width)
            .sum();
        assert_eq!(outline.advance_width, expected);
    }

    #[test]
    fn empty_text_is_rejected() {
        let font = font();
        assert!(matches!(
            text_to_svg(&font, "", &SvgOptions::default()),
            Err(FontError::NoGlyphsFound)
        ));
    }

    #[test]
    fn zero_font_size_is_rejected() {
        let font = font();
        let options = SvgOptions {
            font_size: 0.0,
            ..Default::default()
        };
        assert!(glyph_to_svg(&font, 1, &options).is_err());
    }

    #[test]
    fn fill_with_quotes_falls_back_to_black() {
        assert_eq!(sanitize_fill("#fff"), "#fff");
        assert_eq!(sanitize_fill("rgb(0, 0, 0)"), "rgb(0, 0, 0)");
        assert_eq!(sanitize_fill("\"/><script>"), "#000000");
    }
}
