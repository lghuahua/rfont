//! SVG 文档生成

use std::collections::HashSet;

use rfont::Font;
use rfont_types::FontError;

use crate::outline::{GlyphOutline, glyph_outline};
use crate::path::{contours_to_path, num, outline_to_path};

/// SVG 输出选项
#[derive(Debug, Clone, PartialEq)]
pub struct SvgOptions {
    /// 字号（像素），决定文档中排版后的显示大小
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

/// 排版后的单个字形
#[derive(Debug, Clone)]
pub struct TextGlyph {
    /// 对应字符；按字形 ID 导出时为 None
    pub character: Option<char>,
    /// 字形 ID
    pub glyph_id: u16,
    /// 相对行首的水平偏移（字体单位）
    pub x: f64,
    /// 字形轮廓，原点未平移
    pub outline: GlyphOutline,
}

/// 单个字形的 path 数据（字体单位，Y 轴向上）
pub fn glyph_path(font: &Font, glyph_id: u16) -> Result<String, FontError> {
    Ok(outline_to_path(&glyph_outline(font, glyph_id)?))
}

/// 单行文本按 `hmtx` 步进宽度排版，逐字符保留轮廓
pub fn text_glyphs(font: &Font, text: &str) -> Result<Vec<TextGlyph>, FontError> {
    if text.is_empty() {
        return Err(FontError::NoGlyphsFound);
    }

    let mut pen = 0.0;
    let mut glyphs = Vec::new();
    for (character, glyph_id) in text.chars().zip(font.get_glyph_ids_for_text(text)) {
        let outline = glyph_outline(font, glyph_id)?;
        let advance = outline.advance_width as f64;
        glyphs.push(TextGlyph {
            character: Some(character),
            glyph_id,
            x: pen,
            outline,
        });
        pen += advance;
    }

    Ok(glyphs)
}

/// 单行文本的整体轮廓，字形按 `hmtx` 步进宽度累加
pub fn text_outline(font: &Font, text: &str) -> Result<GlyphOutline, FontError> {
    let glyphs = text_glyphs(font, text)?;

    let mut contours = Vec::new();
    let mut advance = 0.0;
    for glyph in &glyphs {
        let mut outline = glyph.outline.clone();
        outline.translate(glyph.x, 0.0);
        contours.extend(outline.contours);
        advance = glyph.x + glyph.outline.advance_width as f64;
    }

    Ok(GlyphOutline {
        contours,
        advance_width: advance.round().clamp(0.0, u16::MAX as f64) as u16,
    })
}

/// 生成单个字形的 SVG 文档
pub fn glyph_to_svg(font: &Font, glyph_id: u16, options: &SvgOptions) -> Result<String, FontError> {
    let glyphs = [TextGlyph {
        character: None,
        glyph_id,
        x: 0.0,
        outline: glyph_outline(font, glyph_id)?,
    }];

    render(font, &glyphs, options)
}

/// 生成单行文本的 SVG 文档
///
/// 每个不同字符定义为一个 `<symbol>`（`id="icon-字符"`），正文用 `<use>` 依次排开。
/// 这样整份文件仍是文本的完整画面，同时可以直接把 `<symbol>` 当作图标使用：
/// `<svg class="icon"><use href="#icon-字"/></svg>`。
pub fn text_to_svg(font: &Font, text: &str, options: &SvgOptions) -> Result<String, FontError> {
    let glyphs = text_glyphs(font, text)?;
    render(font, &glyphs, options)
}

/// 所有字形共用的坐标盒（字体单位），保证每个 `<symbol>` 的 viewBox 一致
#[derive(Debug, Clone, Copy)]
struct SpriteBox {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

fn render(font: &Font, glyphs: &[TextGlyph], options: &SvgOptions) -> Result<String, FontError> {
    // NaN 也走这里：字号必须是确定的有限正数
    if !options.font_size.is_finite() || options.font_size <= 0.0 {
        return Err(FontError::Generic("字号必须大于 0".to_string()));
    }

    let bounds = sprite_box(glyphs).ok_or(FontError::NoGlyphsFound)?;
    let units_per_em = font.head().units_per_em.max(1) as f64;
    let scale = options.font_size / units_per_em;
    let padding = options.padding.max(0.0);
    let fill = sanitize_fill(&options.fill);

    // viewBox 为 0 时某些渲染器会直接丢弃内容，兜底成 1 个单位
    let box_width = (bounds.max_x - bounds.min_x).max(1.0);
    let box_height = (bounds.max_y - bounds.min_y).max(1.0);
    let row_width = box_width * scale;
    let row_height = box_height * scale;

    // 宽度至少覆盖到最后一个字形的步进终点，行尾空白也算进画布
    let advance = glyphs
        .last()
        .map(|glyph| glyph.x + glyph.outline.advance_width as f64)
        .unwrap_or(0.0);
    let doc_max_x = bounds.max_x.max(advance);
    let width = (doc_max_x - bounds.min_x) * scale + padding * 2.0;
    let height = row_height + padding * 2.0;

    let mut symbols = String::new();
    let mut defined = HashSet::new();
    for glyph in glyphs {
        if glyph.outline.contours.is_empty() {
            // 空格等没有轮廓的字形只占用步进宽度
            continue;
        }
        let id = symbol_id(glyph);
        if !defined.insert(id.clone()) {
            continue;
        }
        let d = symbol_path(&glyph.outline, &bounds);
        symbols.push_str(&format!(
            "<symbol id=\"{id}\" viewBox=\"0 0 {w} {h}\"><path d=\"{d}\"/></symbol>",
            w = num(box_width),
            h = num(box_height),
        ));
    }

    let mut uses = String::new();
    for glyph in glyphs {
        if glyph.outline.contours.is_empty() {
            continue;
        }
        let x = padding + glyph.x * scale;
        uses.push_str(&format!(
            "<use href=\"#{id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\"/>",
            id = symbol_id(glyph),
            x = num(x),
            y = num(padding),
            w = num(row_width),
            h = num(row_height),
        ));
    }

    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><defs>{symbols}</defs><g fill=\"{fill}\">{uses}</g></svg>\n",
        width = num(width),
        height = num(height),
    ))
}

/// 每个字形轮廓与步进宽度共同决定的坐标盒
fn sprite_box(glyphs: &[TextGlyph]) -> Option<SpriteBox> {
    let mut bounds: Option<SpriteBox> = None;

    for glyph in glyphs {
        let Some((min_x, min_y, max_x, max_y)) = glyph.outline.bounds() else {
            continue;
        };
        bounds = Some(match bounds {
            Some(current) => SpriteBox {
                min_x: current.min_x.min(min_x),
                min_y: current.min_y.min(min_y),
                max_x: current.max_x.max(max_x),
                max_y: current.max_y.max(max_y),
            },
            None => SpriteBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        });
    }

    bounds
}

/// 把字形轮廓放进共享坐标盒，并翻转成 Y 轴向下（SVG 的坐标系）
fn symbol_path(outline: &GlyphOutline, sprite_box: &SpriteBox) -> String {
    let mut outline = outline.clone();
    for point in outline.contours.iter_mut().flatten() {
        point.x -= sprite_box.min_x;
        point.y = sprite_box.max_y - point.y;
    }

    contours_to_path(&outline.contours)
}

/// `<symbol>` 的 id：`icon-字符`，无法直接写进 id 的字符退回码位
fn symbol_id(glyph: &TextGlyph) -> String {
    match glyph.character {
        Some(character) if character.is_alphanumeric() => format!("icon-{character}"),
        Some(character) => format!("icon-u{:x}", character as u32),
        None => format!("icon-glyph{}", glyph.glyph_id),
    }
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
    fn glyph_svg_has_symbol_and_use() {
        let font = font();
        let glyph_id = font.text_to_glyph_ids("A")[0];
        let svg = glyph_to_svg(&font, glyph_id, &SvgOptions::default()).unwrap();

        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("<symbol id=\"icon-glyph"));
        assert!(svg.contains("<path d=\"M"));
        assert!(svg.contains("<use href=\"#icon-glyph"));
        assert!(svg.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn text_svg_defines_symbol_per_character() {
        let font = font();
        let svg = text_to_svg(&font, "ABA", &SvgOptions::default()).unwrap();

        // 不同字符各一个 symbol，出现几次就排几次 use
        assert_eq!(svg.matches("<symbol ").count(), 2);
        assert_eq!(svg.matches("<use ").count(), 3);
        assert!(svg.contains("id=\"icon-A\""));
        assert!(svg.contains("id=\"icon-B\""));
        assert!(svg.contains("href=\"#icon-A\""));
        assert_eq!(svg.matches("viewBox=\"0 0 ").count(), 3);
    }

    #[test]
    fn space_keeps_advance_but_emits_no_symbol() {
        let font = font();
        let svg = text_to_svg(&font, "A A", &SvgOptions::default()).unwrap();

        assert_eq!(svg.matches("<symbol ").count(), 1);
        assert_eq!(svg.matches("<use ").count(), 2);
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
    fn symbol_id_prefers_the_character() {
        let glyph = |character| TextGlyph {
            character: Some(character),
            glyph_id: 0,
            x: 0.0,
            outline: GlyphOutline::default(),
        };

        assert_eq!(symbol_id(&glyph('字')), "icon-字");
        assert_eq!(symbol_id(&glyph('a')), "icon-a");
        assert_eq!(symbol_id(&glyph('/')), "icon-u2f");
        assert_eq!(symbol_id(&glyph('"')), "icon-u22");
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
