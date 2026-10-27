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

use rfont::{FaceSummary, Font, FontCollection, FontError, FontInfo, OutputFormat, SubsetOptions};
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
}
