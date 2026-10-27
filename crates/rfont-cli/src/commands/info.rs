use anyhow::{Context, Result};
use colored::*;
use rfont::Font;
use std::path::Path;
use tracing::{Level, debug, info, span};

pub fn run(font_path: &Path, face: usize, json: bool, verbose: bool) -> Result<()> {
    let span = span!(Level::INFO, "info_command", path = ?font_path, face = face, json = json, verbose = verbose);
    let _enter = span.enter();

    debug!("开始加载字体文件");

    let font = Font::load_with_face(font_path, face)
        .with_context(|| format!("无法加载字体文件: {}", font_path.display()))?;

    debug!(glyph_count = font.maxp().num_glyphs, "字体加载成功");

    if json {
        debug!("使用 JSON 输出模式");
        output_json(&font)?;
    } else {
        debug!("使用人类可读输出模式");
        output_human(&font, verbose)?;
    }

    info!("字体信息查询完成");

    Ok(())
}

fn output_human(font: &Font, verbose: bool) -> Result<()> {
    let info = font.get_font_info().context("获取字体信息失败")?;

    println!("{}", "📝 字体信息".bold().cyan());
    println!("{}", "─".repeat(60).dimmed());

    println!("\n{}", "基本信息:".bold());
    println!("  格式:          {}", info.format);
    if info.face_count > 1 {
        println!(
            "  字面:          {}/{}",
            info.face_index + 1,
            info.face_count
        );
    }
    println!("  字形数量:      {}", info.glyph_count);
    println!("  Units per EM:  {}", info.units_per_em);
    println!("  支持的字符:    {}", info.supported_char_count);
    println!(
        "  边界框:        [{}, {}, {}, {}]",
        info.bbox.0, info.bbox.1, info.bbox.2, info.bbox.3
    );

    println!("\n{}", "名称信息:".bold());
    let name_fields = [
        ("家族名称:     ", &info.family),
        ("排版家族:     ", &info.typographic_family),
        ("样式:         ", &info.subfamily),
        ("完整名称:     ", &info.full_name),
        ("PostScript:   ", &info.postscript_name),
        ("版本:         ", &info.version),
        ("设计者:       ", &info.designer),
        ("制造商:       ", &info.manufacturer),
        ("版权:         ", &info.copyright),
        ("许可证:       ", &info.license),
    ];

    let mut printed = false;
    for (label, value) in name_fields {
        if let Some(value) = value {
            println!("  {}{}", label, value);
            printed = true;
        }
    }
    if !printed {
        println!("  未找到可用的 name 表记录");
    }

    println!("\n{}", "样式与度量:".bold());
    println!("  字重:          {}", info.weight_class);
    println!("  字宽:          {}", info.width_class);
    println!("  嵌入权限:      0x{:04X}", info.fs_type);
    println!("  Ascender:      {}", info.ascender);
    println!("  Descender:     {}", info.descender);
    println!("  Line Gap:      {}", info.line_gap);
    println!("  HMetrics:      {}", info.number_of_h_metrics);

    if verbose {
        if !info.names.is_empty() {
            println!("\n{}", format!("名称记录（{}）:", info.names.len()).bold());
            for name in &info.names {
                println!(
                    "  nameID={:<3} platform={}:{} lang=0x{:04X}  {}",
                    name.name_id, name.platform_id, name.encoding_id, name.language_id, name.value
                );
            }
        }

        println!("\n{}", "字体表:".bold());
        for table in &info.tables {
            println!(
                "  {:<8} offset={:<8} size={:<10} checksum=0x{:08X}",
                table.tag.bold().yellow(),
                table.offset,
                format_size(table.length),
                table.checksum
            );
        }

        println!("\n  总计: {} 个表", info.tables.len());
    }

    Ok(())
}

fn output_json(font: &Font) -> Result<()> {
    let info = font.get_font_info().context("获取字体信息失败")?;
    println!("{}", serde_json::to_string_pretty(&info)?);

    Ok(())
}

fn format_size(bytes: u32) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
