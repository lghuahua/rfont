//! rfont 的 WebAssembly 绑定
//!
//! 将 [`rfont`] 的字体解析、信息读取、格式转换与子集化能力暴露给浏览器 / JS。
//! 所有函数以字节（JS 侧为 `Uint8Array`）为输入输出，不依赖文件系统，
//! 通过 [`rfont::Font::from_bytes_with_face`] 直接从内存解析字体。
//!
//! 出错时返回的 `Err(JsValue)` 会被 wasm-bindgen 转换成 JS 抛出的异常，
//! 前端用 `try/catch` 捕获即可。
//!
//! 每个导出函数都有一层纯 Rust 实现（`*_impl`）：它只返回 [`FontError`]，
//! 不触碰 JS 运行时，因此可以在原生平台上直接测试错误分支。

use rfont::{
    FaceSummary, Font, FontCollection, FontError, FontInfo, FontMetaEdit, OutputFormat,
    SubsetOptions,
};
use rfont_svg::SvgOptions;
use wasm_bindgen::prelude::*;

/// 将任意实现了 `Display` 的错误转换成可抛给 JS 的 `JsValue`。
fn to_js_error<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// 列出字体集合中的字面摘要。
fn list_faces_impl(data: &[u8]) -> Result<Vec<FaceSummary>, FontError> {
    if !FontCollection::is_collection(data) {
        return Ok(Vec::new());
    }
    FontCollection::from_bytes(data.to_vec())?.faces()
}

/// 提取指定字面为独立的 SFNT 字体字节。
///
/// 非集合文件且 `face` 为 0 时返回原始字节，保证调用方可以统一用该结果做预览。
fn face_bytes_impl(data: &[u8], face: usize) -> Result<Vec<u8>, FontError> {
    if FontCollection::is_collection(data) {
        return FontCollection::from_bytes(data.to_vec())?.face_bytes(face);
    }

    if face == 0 {
        Ok(data.to_vec())
    } else {
        Err(FontError::FaceIndexOutOfRange {
            index: face,
            count: 1,
        })
    }
}

/// 读取指定字面的字体信息。
fn font_info_impl(data: &[u8], face: usize) -> Result<FontInfo, FontError> {
    let font = Font::from_bytes_with_face(data, face)?;
    font.get_font_info()
}

/// 纯格式转换。
fn convert_impl(
    data: &[u8],
    format: &str,
    compression_level: u8,
    face: usize,
) -> Result<Vec<u8>, FontError> {
    let font = Font::from_bytes_with_face(data, face)?;
    font.convert_format(format, compression_level)
}

/// 按文本子集化字体。
fn subset_impl(
    data: &[u8],
    text: &str,
    format: &str,
    compression_level: u8,
    face: usize,
) -> Result<Vec<u8>, FontError> {
    let font = Font::from_bytes_with_face(data, face)?;

    let glyph_ids = font.get_glyph_ids_for_text(text);
    if glyph_ids.is_empty() {
        return Err(FontError::NoGlyphsFound);
    }

    let options = SubsetOptions {
        output_format: OutputFormat::parse(format).ok_or_else(|| {
            FontError::UnsupportedOutputFormat {
                format: format.to_string(),
            }
        })?,
        compression_level,
        ..Default::default()
    };

    font.subset_with_options(&glyph_ids, &options)
}

/// 修改字体元信息（名称、作者、版权等）。
fn edit_metadata_impl(
    data: &[u8],
    edit: &FontMetaEdit,
    format: &str,
    compression_level: u8,
    face: usize,
) -> Result<Vec<u8>, FontError> {
    let font = Font::from_bytes_with_face(data, face)?;
    font.edit_metadata_to(edit, format, compression_level)
}

/// 列出 TTC 字体集合中的字面摘要。
///
/// # 参数
/// - `data`: 字体文件字节
///
/// # 返回
/// - `Ok(String)`: 字面摘要数组的 JSON；非 TTC 文件返回空数组
/// - `Err(JsValue)`: 集合解析失败时的错误信息
#[wasm_bindgen]
pub fn list_faces(data: &[u8]) -> Result<String, JsValue> {
    let faces = list_faces_impl(data).map_err(to_js_error)?;
    serde_json::to_string(&faces).map_err(to_js_error)
}

/// 提取指定字面为独立的 SFNT 字体字节。
///
/// # 参数
/// - `data`: 字体文件字节
/// - `face`: 字面索引
///
/// # 返回
/// - `Ok(Vec<u8>)`: 字面对应的独立字体字节（JS 侧为 `Uint8Array`）
/// - `Err(JsValue)`: 索引越界或集合解析失败时的错误信息
#[wasm_bindgen]
pub fn face_bytes(data: &[u8], face: u32) -> Result<Vec<u8>, JsValue> {
    face_bytes_impl(data, face as usize).map_err(to_js_error)
}

/// 读取字体信息，返回 JSON 字符串。
///
/// # 参数
/// - `data`: 字体文件字节（TTF / TTC / WOFF / WOFF2）
/// - `face`: 字面索引，TTC 集合内按索引选择
///
/// # 返回
/// - `Ok(String)`: [`rfont::FontInfo`] 的 JSON 序列化结果
/// - `Err(JsValue)`: 解析失败时的错误信息
#[wasm_bindgen]
pub fn font_info(data: &[u8], face: u32) -> Result<String, JsValue> {
    let info = font_info_impl(data, face as usize).map_err(to_js_error)?;
    serde_json::to_string(&info).map_err(to_js_error)
}

/// 纯格式转换（保留字体中的全部字形，不做子集化）。
///
/// # 参数
/// - `data`: 原始字体字节
/// - `format`: 输出格式，"ttf" / "woff" / "woff2"
/// - `compression_level`: 压缩级别（WOFF: 0-9，WOFF2: 0-11，TTF 忽略）
/// - `face`: 字面索引，TTC 集合内按索引选择
///
/// # 返回
/// - `Ok(Vec<u8>)`: 转换后的字体字节（JS 侧为 `Uint8Array`）
/// - `Err(JsValue)`: 转换失败时的错误信息
#[wasm_bindgen]
pub fn convert(
    data: &[u8],
    format: &str,
    compression_level: u8,
    face: u32,
) -> Result<Vec<u8>, JsValue> {
    convert_impl(data, format, compression_level, face as usize).map_err(to_js_error)
}

/// 按文本子集化字体，并输出为指定格式。
///
/// 行为与桌面端一致：根据文本映射字形 ID，仅保留用到的字形后重建字体。
///
/// # 参数
/// - `data`: 原始字体字节
/// - `text`: 需要保留的文字内容
/// - `format`: 输出格式，"ttf" / "woff" / "woff2"
/// - `compression_level`: 压缩级别（WOFF: 0-9，WOFF2: 0-11，TTF 忽略）
/// - `face`: 字面索引，TTC 集合内按索引选择
///
/// # 返回
/// - `Ok(Vec<u8>)`: 子集化后的字体字节（JS 侧为 `Uint8Array`）
/// - `Err(JsValue)`: 文本为空或未匹配到任何字形等错误
#[wasm_bindgen]
pub fn subset(
    data: &[u8],
    text: &str,
    format: &str,
    compression_level: u8,
    face: u32,
) -> Result<Vec<u8>, JsValue> {
    subset_impl(data, text, format, compression_level, face as usize).map_err(to_js_error)
}

/// 修改字体元信息，返回新的字体字节。
///
/// 只改写 `name` 表并刷新 `head.modified`，字形与度量保持不变。
///
/// # 参数
/// - `data`: 原始字体字节
/// - `edit`: [`rfont::FontMetaEdit`] 的 JSON 序列化结果，未出现的字段保持原值
/// - `format`: 输出格式，"ttf" / "otf" / "woff" / "woff2"
/// - `compression_level`: 压缩级别（WOFF: 0-9，WOFF2: 0-11，TTF 忽略）
/// - `face`: 字面索引，TTC 集合内按索引选择
///
/// # 返回
/// - `Ok(Vec<u8>)`: 修改后的字体字节（JS 侧为 `Uint8Array`）
/// - `Err(JsValue)`: 参数非法或修改失败时的错误信息
#[wasm_bindgen]
pub fn edit_metadata(
    data: &[u8],
    edit: &str,
    format: &str,
    compression_level: u8,
    face: u32,
) -> Result<Vec<u8>, JsValue> {
    let edit: FontMetaEdit = serde_json::from_str(edit).map_err(to_js_error)?;
    edit_metadata_impl(data, &edit, format, compression_level, face as usize).map_err(to_js_error)
}

fn svg_options(font_size: f64, fill: &str) -> SvgOptions {
    SvgOptions {
        font_size,
        fill: fill.to_string(),
        padding: 4.0,
    }
}

/// 将单行文本导出为 SVG 文档。
fn svg_text_impl(
    data: &[u8],
    text: &str,
    font_size: f64,
    fill: &str,
    face: usize,
) -> Result<String, FontError> {
    let font = Font::from_bytes_with_face(data, face)?;
    rfont_svg::text_to_svg(&font, text, &svg_options(font_size, fill))
}

/// 将单个字形导出为 SVG 文档。
fn svg_glyph_impl(
    data: &[u8],
    glyph_id: u16,
    font_size: f64,
    fill: &str,
    face: usize,
) -> Result<String, FontError> {
    let font = Font::from_bytes_with_face(data, face)?;
    rfont_svg::glyph_to_svg(&font, glyph_id, &svg_options(font_size, fill))
}

/// 将单行文本导出为 SVG。
///
/// # 参数
/// - `data`: 原始字体字节
/// - `text`: 要导出的文本（单行）
/// - `font_size`: 字号（像素），决定输出尺寸
/// - `fill`: 填充色，接受 `#rgb`、`#rrggbb`、`rgb(...)` 等 CSS 颜色
/// - `face`: 字面索引，TTC 集合内按索引选择
///
/// # 返回
/// - `Ok(String)`: 完整的 SVG 文档
/// - `Err(JsValue)`: 文本为空、字体无对应字形或轮廓不支持时的错误信息
#[wasm_bindgen]
pub fn svg_text(
    data: &[u8],
    text: &str,
    font_size: f64,
    fill: &str,
    face: u32,
) -> Result<String, JsValue> {
    svg_text_impl(data, text, font_size, fill, face as usize).map_err(to_js_error)
}

/// 将单个字形导出为 SVG。
///
/// # 参数
/// - `data`: 原始字体字节
/// - `glyph_id`: 字形 ID
/// - `font_size`: 字号（像素）
/// - `fill`: 填充色
/// - `face`: 字面索引，TTC 集合内按索引选择
///
/// # 返回
/// - `Ok(String)`: 完整的 SVG 文档
/// - `Err(JsValue)`: 字形越界或轮廓不支持时的错误信息
#[wasm_bindgen]
pub fn svg_glyph(
    data: &[u8],
    glyph_id: u16,
    font_size: f64,
    fill: &str,
    face: u32,
) -> Result<String, JsValue> {
    svg_glyph_impl(data, glyph_id, font_size, fill, face as usize).map_err(to_js_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_font() -> Vec<u8> {
        std::fs::read("../crates/rfont/src/AlimamaDaoLiTi.ttf").unwrap()
    }

    #[test]
    fn font_info_returns_json() {
        let data = test_font();
        let json = font_info(&data, 0).unwrap();
        assert!(json.contains("glyph_count"));
    }

    #[test]
    fn font_info_includes_license_restrictions() {
        let data = test_font();
        let json = font_info(&data, 0).unwrap();

        assert!(json.contains("license_url"));
        assert!(json.contains("restrictions"));
        assert!(json.contains("embedding_label"));
    }

    #[test]
    fn convert_ttf_roundtrip() {
        let data = test_font();
        let out = convert(&data, "ttf", 0, 0).unwrap();
        assert!(!out.is_empty());
    }

    #[test]
    fn subset_shrinks_font() {
        let data = test_font();
        let out = subset(&data, "Hello", "ttf", 0, 0).unwrap();
        assert!(!out.is_empty());
        assert!(out.len() < data.len());
    }

    #[test]
    fn list_faces_empty_for_single_font() {
        assert!(list_faces_impl(&test_font()).unwrap().is_empty());
    }

    #[test]
    fn face_bytes_returns_original_for_single_font() {
        let data = test_font();
        assert_eq!(face_bytes_impl(&data, 0).unwrap(), data);
        assert!(matches!(
            face_bytes_impl(&data, 1),
            Err(FontError::FaceIndexOutOfRange { index: 1, count: 1 })
        ));
    }

    #[test]
    fn single_font_rejects_face_above_zero() {
        let data = test_font();
        assert!(matches!(
            font_info_impl(&data, 1),
            Err(FontError::FaceIndexOutOfRange { index: 1, count: 1 })
        ));
    }

    #[test]
    fn edit_metadata_updates_family_name() {
        let data = test_font();
        let edit = FontMetaEdit {
            family: Some("Wasm Edited".to_string()),
            ..Default::default()
        };

        let out = edit_metadata_impl(&data, &edit, "ttf", 0, 0).unwrap();
        let info = Font::from_bytes(&out).unwrap().get_font_info().unwrap();

        assert_eq!(info.family.as_deref(), Some("Wasm Edited"));
        assert_eq!(info.glyph_count, 7044);
    }

    #[test]
    fn edit_metadata_parses_partial_json() {
        let data = test_font();
        let edit: FontMetaEdit = serde_json::from_str(r#"{"designer":"Alice"}"#).unwrap();

        assert_eq!(edit.designer.as_deref(), Some("Alice"));
        assert!(edit.family.is_none());

        let out = edit_metadata_impl(&data, &edit, "ttf", 0, 0).unwrap();
        let info = Font::from_bytes(&out).unwrap().get_font_info().unwrap();
        assert_eq!(info.designer.as_deref(), Some("Alice"));
    }

    #[test]
    fn edit_metadata_rejects_invalid_postscript_name() {
        let data = test_font();
        let edit = FontMetaEdit {
            postscript_name: Some("Bad Name".to_string()),
            ..Default::default()
        };

        assert!(matches!(
            edit_metadata_impl(&data, &edit, "ttf", 0, 0),
            Err(FontError::InvalidPostScriptName { .. })
        ));
    }

    #[test]
    fn svg_text_returns_document() {
        let data = test_font();
        let svg = svg_text(&data, "AB", 64.0, "#000000", 0).unwrap();

        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("<path d=\"M"));
    }

    #[test]
    fn svg_glyph_returns_document() {
        let data = test_font();
        let svg = svg_glyph(&data, 36, 48.0, "#ff0000", 0).unwrap();

        assert!(svg.contains("fill=\"#ff0000\""));
    }

    #[test]
    fn svg_text_rejects_empty_text() {
        let data = test_font();
        assert!(matches!(
            svg_text_impl(&data, "", 64.0, "#000000", 0),
            Err(FontError::NoGlyphsFound)
        ));
    }
}
