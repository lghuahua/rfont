use anyhow::{Context, Result, bail};
use colored::*;
use rfont::Font;
use rfont_svg::{SvgOptions, glyph_to_svg, text_to_svg};
use std::path::Path;
use tracing::{Level, debug, info, span};

/// `svg` 子命令的输入参数
pub struct SvgRequest<'a> {
    /// 要导出的文本（与 `glyph` 二选一）
    pub text: Option<&'a str>,
    /// 要导出的字形 ID（与 `text` 二选一）
    pub glyph: Option<u16>,
    /// 字号（像素）
    pub size: f64,
    /// 填充色
    pub color: &'a str,
    /// 四周留白（像素）
    pub padding: f64,
}

pub fn run(
    input: &Path,
    output: Option<&Path>,
    face: usize,
    request: SvgRequest<'_>,
) -> Result<()> {
    let span = span!(Level::INFO, "svg_command",
                     input = ?input,
                     output = ?output,
                     face = face,
                     glyph = ?request.glyph);
    let _enter = span.enter();

    let font = Font::load_with_face(input, face)
        .with_context(|| format!("无法加载字体文件: {}", input.display()))?;
    let options = SvgOptions {
        font_size: request.size,
        fill: request.color.to_string(),
        padding: request.padding,
    };

    let (svg, label) = match (request.text, request.glyph) {
        (Some(text), None) => {
            if text.is_empty() {
                bail!("--text 不能为空");
            }
            (
                text_to_svg(&font, text, &options)?,
                format!("文本 \"{text}\""),
            )
        }
        (None, Some(glyph)) => (
            glyph_to_svg(&font, glyph, &options)?,
            format!("字形 #{glyph}"),
        ),
        _ => bail!("需要且只能指定 --text 或 --glyph 之一"),
    };

    let output_path = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| input.with_extension("svg"));

    std::fs::write(&output_path, svg)
        .with_context(|| format!("无法写入输出文件: {}", output_path.display()))?;

    debug!(
        size = request.size,
        color = request.color,
        padding = request.padding,
        "SVG 参数"
    );
    info!(output = ?output_path, "SVG 导出完成");

    println!("  ✓ 已导出 {label} 的 SVG");
    println!(
        "  ✓ 文件已保存: {}",
        output_path.display().to_string().bold()
    );
    Ok(())
}
