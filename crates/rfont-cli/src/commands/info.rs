use anyhow::{Context, Result};
use colored::*;
use rfont::Font;
use std::path::Path;
use tracing::{Level, debug, info, span};

pub fn run(font_path: &Path, json: bool, verbose: bool) -> Result<()> {
    let span =
        span!(Level::INFO, "info_command", path = ?font_path, json = json, verbose = verbose);
    let _enter = span.enter();

    debug!("开始加载字体文件");

    // 加载字体
    let font = Font::load(font_path.to_str().unwrap())
        .context(format!("无法加载字体文件: {:?}", font_path))?;

    debug!(glyph_count = font.maxp().num_glyphs, "字体加载成功");

    if json {
        // JSON 输出模式
        debug!("使用 JSON 输出模式");
        output_json(&font)?;
    } else {
        // 人类可读输出模式
        debug!("使用人类可读输出模式");
        output_human(&font, verbose)?;
    }

    info!("字体信息查询完成");

    Ok(())
}

fn output_human(font: &Font, verbose: bool) -> Result<()> {
    println!("{}", "📝 字体信息".bold().cyan());
    println!("{}", "─".repeat(60).dimmed());

    // 基本信息
    let info = font.get_font_info().context("获取字体信息失败")?;

    println!("\n{}", "基本信息:".bold());
    println!("  格式:         {}", info.format);
    println!("  字形数量:     {}", info.glyph_count);
    println!("  Units per EM: {}", info.units_per_em);
    println!(
        "  边界框:       [{}, {}, {}, {}]",
        info.bbox.0, info.bbox.1, info.bbox.2, info.bbox.3
    );

    if let Some(version) = &info.version {
        println!("  版本:         {}", version);
    }

    // Name 表信息
    println!("\n{}", "名称信息:".bold());
    if let Some(name_table) = font.get_name_table() {
        name_table.print_info();
    } else {
        println!("  未找到 name 表");
    }

    // 水平度量信息
    println!("\n{}", "水平度量:".bold());
    println!("  Ascender:     {}", info.ascender);
    println!("  Descender:    {}", info.descender);
    println!("  Line Gap:     {}", info.line_gap);

    // 表列表
    if verbose {
        println!("\n{}", "字体表:".bold());
        let tables = font.get_table_list();

        for table in &tables {
            let size_str = format_size(table.length);
            println!(
                "  {:<8} offset={:<8} size={:<10} checksum=0x{:08X}",
                table.tag.bold().yellow(),
                table.offset,
                size_str,
                table.checksum
            );
        }

        println!("\n  总计: {} 个表", tables.len());
    }
    // println!("  {:?} 字符", font.cmap.subtables);
    // // 支持的字符统计
    // let supported_chars = font.get_supported_characters();
    // println!("\n{}", "字符支持:".bold());
    // println!("  支持的 Unicode 字符数: {}", supported_chars.len());

    // if verbose && !supported_chars.is_empty() {
    //     // 显示前 20 个字符作为示例
    //     let sample: String = supported_chars
    //         .iter()
    //         .take(20)
    //         .filter_map(|&c| char::from_u32(c))
    //         .collect();
    //     println!("  示例字符: {}", sample);
    // }

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
