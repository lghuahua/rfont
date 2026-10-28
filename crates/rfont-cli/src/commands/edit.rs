use anyhow::{Context, Result};
use colored::*;
use rfont::{Font, FontFormat, FontInfo, FontMetaEdit};
use std::path::{Path, PathBuf};
use tracing::{Level, debug, info, span};

/// 元信息编辑请求
pub struct EditRequest<'a> {
    /// TTC 字体集合中的字面索引
    pub face: usize,
    /// 输出字体文件路径
    pub output: Option<&'a Path>,
    /// 输出格式（ttf、otf、woff 或 woff2）
    pub format: Option<&'a str>,
    /// WOFF/WOFF2 压缩级别
    pub compression: Option<u8>,
    /// 待修改的元信息
    pub edit: &'a FontMetaEdit,
    /// 遇到授权限制时中止而不是只提示
    pub strict: bool,
}

/// 支持的输出格式
const SUPPORTED_FORMATS: &[&str] = &["ttf", "otf", "woff", "woff2"];

pub fn run(input: &Path, request: &EditRequest<'_>) -> Result<()> {
    let span = span!(Level::INFO, "edit_command",
                     input = ?input,
                     output = ?request.output,
                     face = request.face);
    let _enter = span.enter();

    request.edit.validate().context("元信息校验失败")?;
    if request.edit.is_empty() {
        anyhow::bail!("未指定任何要修改的字段");
    }

    // 加载字体
    println!("{}", "📖 加载字体...".bold().cyan());
    let font = Font::load_with_face(input, request.face)
        .with_context(|| format!("无法加载字体文件: {}", input.display()))?;
    let info = font.get_font_info().context("获取字体信息失败")?;
    debug!(glyph_count = info.glyph_count, "字体加载成功");

    let original_name = info
        .full_name
        .as_deref()
        .or(info.family.as_deref())
        .unwrap_or("<未知>");
    println!("  ✓ 成功加载字体：{}", original_name.bold());

    // 授权提示：改名不影响字形，但保留字体名等声明会限制改名后的分发
    super::report_restrictions(&info.restrictions.rename_warnings, request.strict)?;

    // 决定输出格式
    let format = match request.format {
        Some(value) => value.to_lowercase(),
        None => default_format(&info.format),
    };
    if !SUPPORTED_FORMATS.contains(&format.as_str()) {
        anyhow::bail!(
            "不支持的格式: {} (仅支持 {})",
            format,
            SUPPORTED_FORMATS.join(", ")
        );
    }
    let compression = super::resolve_compression(&format, request.compression);
    debug!(
        target_format = format,
        compression_level = compression,
        "输出格式已解析"
    );

    // 应用修改
    println!("\n{}", "📝 应用元信息修改...".bold().cyan());
    let edited = font
        .edit_metadata_to(request.edit, &format, compression)
        .context("元信息修改失败")?;

    let edited_info = Font::from_bytes(&edited)
        .context("修改后的字体无法解析")?
        .get_font_info()
        .context("获取修改后的字体信息失败")?;
    print_changes(&info, &edited_info);

    // 写入文件
    println!("\n{}", "💾 保存文件...".bold().cyan());
    let output_path = match request.output {
        Some(path) => path.to_path_buf(),
        None => default_output_path(input, request.face, info.face_count, &format),
    };
    std::fs::write(&output_path, &edited)
        .with_context(|| format!("无法写入输出文件: {}", output_path.display()))?;
    println!("  ✓ 文件已保存: {:?}", output_path);

    let original_size = font.font_data().as_bytes().len() as u64;
    let edited_size = edited.len() as u64;
    info!(
        original_size = original_size,
        edited_size = edited_size,
        output = %output_path.display(),
        "字体元信息修改完成"
    );

    println!("\n{}", "📊 统计信息:".bold().cyan());
    println!("  原始大小:   {}", format_size(original_size));
    println!("  修改后大小: {}", format_size(edited_size));
    println!("  字形数量:   {}", info.glyph_count);

    println!("\n{}", "✨ 修改完成！".bold().green());

    Ok(())
}

/// 对比修改前后的名称信息
fn print_changes(before: &FontInfo, after: &FontInfo) {
    let rows: [(&str, &Option<String>, &Option<String>); 9] = [
        ("家族名称", &before.family, &after.family),
        ("样式名称", &before.subfamily, &after.subfamily),
        ("完整名称", &before.full_name, &after.full_name),
        ("版本", &before.version, &after.version),
        (
            "PostScript 名称",
            &before.postscript_name,
            &after.postscript_name,
        ),
        ("制造商", &before.manufacturer, &after.manufacturer),
        ("设计者", &before.designer, &after.designer),
        ("版权", &before.copyright, &after.copyright),
        ("许可证", &before.license, &after.license),
    ];

    for (label, before, after) in rows {
        let before = before.as_deref().unwrap_or("<无>");
        let after = after.as_deref().unwrap_or("<无>");
        if before == after {
            continue;
        }
        println!(
            "  ✓ {}: {} {} {}",
            label.bold(),
            before.dimmed(),
            "→".bold(),
            after.green()
        );
    }
}

/// 未指定输出格式时沿用输入容器的格式
fn default_format(input: &FontFormat) -> String {
    match input {
        FontFormat::Woff => "woff".to_string(),
        FontFormat::Woff2 => "woff2".to_string(),
        FontFormat::Otf => "otf".to_string(),
        _ => "ttf".to_string(),
    }
}

fn default_output_path(input: &Path, face: usize, face_count: usize, format: &str) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("font");
    let name = if face_count > 1 {
        format!("{}.face{}_edited", stem, face)
    } else {
        format!("{}_edited", stem)
    };
    input.with_file_name(format!("{}.{}", name, format))
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
