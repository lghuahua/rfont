//! rfont 的 WebAssembly 绑定
//!
//! 将 [`rfont`] 的字体解析、信息读取、格式转换与子集化能力暴露给浏览器 / JS。
//! 所有函数以字节（JS 侧为 `Uint8Array`）为输入输出，不依赖文件系统，
//! 通过 [`rfont::Font::from_bytes`] 直接从内存解析字体。
//!
//! 出错时返回的 `Err(JsValue)` 会被 wasm-bindgen 转换成 JS 抛出的异常，
//! 前端用 `try/catch` 捕获即可。

use rfont::{Font, OutputFormat, SubsetOptions};
use wasm_bindgen::prelude::*;

/// 将任意实现了 `Display` 的错误转换成可抛给 JS 的 `JsValue`。
fn to_js_error<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// 读取字体信息，返回 JSON 字符串。
///
/// # 参数
/// - `data`: 字体文件字节（TTF / WOFF / WOFF2）
///
/// # 返回
/// - `Ok(String)`: [`rfont::FontInfo`] 的 JSON 序列化结果
/// - `Err(JsValue)`: 解析失败时的错误信息
#[wasm_bindgen]
pub fn font_info(data: &[u8]) -> Result<String, JsValue> {
    let font = Font::from_bytes(data).map_err(to_js_error)?;
    let info = font.get_font_info().map_err(to_js_error)?;
    serde_json::to_string(&info).map_err(to_js_error)
}

/// 纯格式转换（保留字体中的全部字形，不做子集化）。
///
/// # 参数
/// - `data`: 原始字体字节
/// - `format`: 输出格式，"ttf" / "woff" / "woff2"
/// - `compression_level`: 压缩级别（WOFF: 0-9，WOFF2: 0-11，TTF 忽略）
///
/// # 返回
/// - `Ok(Vec<u8>)`: 转换后的字体字节（JS 侧为 `Uint8Array`）
/// - `Err(JsValue)`: 转换失败时的错误信息
#[wasm_bindgen]
pub fn convert(data: &[u8], format: &str, compression_level: u8) -> Result<Vec<u8>, JsValue> {
    let font = Font::from_bytes(data).map_err(to_js_error)?;
    font.convert_format(format, compression_level)
        .map_err(to_js_error)
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
) -> Result<Vec<u8>, JsValue> {
    let font = Font::from_bytes(data).map_err(to_js_error)?;

    let glyph_ids = font.get_glyph_ids_for_text(text);
    if glyph_ids.is_empty() {
        return Err(JsValue::from_str("No glyphs found for the specified text"));
    }

    let options = SubsetOptions {
        output_format: OutputFormat::parse(format)
            .ok_or_else(|| JsValue::from_str(&format!("Unsupported output format: {}", format)))?,
        compression_level,
        ..Default::default()
    };

    font.subset_with_options(&glyph_ids, &options)
        .map_err(to_js_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_info_returns_json() {
        let data = std::fs::read("../crates/rfont/src/AlimamaDaoLiTi.ttf").unwrap();
        let json = font_info(&data).unwrap();
        assert!(json.contains("glyph_count"));
    }

    #[test]
    fn convert_ttf_roundtrip() {
        let data = std::fs::read("../crates/rfont/src/AlimamaDaoLiTi.ttf").unwrap();
        let out = convert(&data, "ttf", 0).unwrap();
        assert!(!out.is_empty());
    }

    #[test]
    fn subset_shrinks_font() {
        let data = std::fs::read("../crates/rfont/src/AlimamaDaoLiTi.ttf").unwrap();
        let out = subset(&data, "Hello", "ttf", 0).unwrap();
        assert!(!out.is_empty());
        assert!(out.len() < data.len());
    }
}
