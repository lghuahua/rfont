use anyhow::{Context, Result};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use rfont::Font;
use std::path::Path;
use tracing::{debug, info, span, warn, Level};

pub fn run(input: &Path, output: Option<&Path>, format: &str, compression: u8) -> Result<()> {
    let span = span!(Level::INFO, "convert_command", 
                     input = ?input, 
                     output = ?output,
                     format = format,
                     compression = compression);
    let _enter = span.enter();

    debug!("开始字体格式转换");

    // 验证目标格式
    let format = format.to_lowercase();
    if format != "ttf" && format != "woff" && format != "woff2" {
        warn!(format = format, "不支持的格式");
        return Err(anyhow::anyhow!(
            "不支持的格式: {} (仅支持 ttf, woff 和 woff2)",
            format
        ));
    }

    debug!(target_format = format, "目标格式验证通过");

    // 加载字体
    println!("{}", "📖 加载字体...".bold().cyan());
    debug!("正在加载字体文件");
    let font =
        Font::load(input.to_str().unwrap()).context(format!("无法加载字体文件: {:?}", input))?;

    debug!(
        glyph_count = font.get_font_info().glyph_count,
        "字体加载成功"
    );
    println!("  ✓ 成功加载字体");

    // 创建进度条
    let progress = ProgressBar::new(100);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>3}% {msg}")
            .unwrap()
            .progress_chars("=> "),
    );
    progress.set_message("转换中...");

    // 执行转换
    debug!(
        target_format = format,
        compression_level = compression,
        "开始格式转换"
    );
    let converted_data = if format == "woff" {
        // TTF → WOFF：使用所有字形 ID
        let all_glyph_ids: Vec<u16> = (0..font.get_font_info().glyph_count as u16).collect();
        font.subset_builder()
            .glyph_ids(all_glyph_ids)
            .output_format("woff")
            .compression_level(compression)
            .build()
            .context("转换为 WOFF 失败")?
    } else if format == "woff2" {
        // TTF → WOFF2：使用所有字形 ID
        let all_glyph_ids: Vec<u16> = (0..font.get_font_info().glyph_count as u16).collect();
        font.subset_builder()
            .glyph_ids(all_glyph_ids)
            .output_format("woff2")
            .compression_level(compression)
            .build()
            .context("转换为 WOFF2 失败")?
    } else {
        // WOFF/WOFF2 → TTF（或其他情况）
        let all_glyph_ids: Vec<u16> = (0..font.get_font_info().glyph_count as u16).collect();
        font.subset_builder()
            .glyph_ids(all_glyph_ids)
            .output_format("ttf")
            .build()
            .context("转换为 TTF 失败")?
    };

    debug!(converted_size = converted_data.len(), "转换完成");
    progress.inc(80);

    // 确定输出路径
    let output_path = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let stem = input.file_stem().unwrap().to_str().unwrap();
            let ext = if format == "woff" {
                "woff"
            } else if format == "woff2" {
                "woff2"
            } else {
                "ttf"
            };
            input.with_file_name(format!("{}_converted.{}", stem, ext))
        }
    };

    // 写入文件
    println!("\n{}", "💾 保存文件...".bold().cyan());
    debug!(output_path = ?output_path, "正在写入输出文件");
    std::fs::write(&output_path, &converted_data)
        .context(format!("无法写入输出文件: {:?}", output_path))?;

    debug!("文件写入成功");
    progress.inc(20);
    progress.finish_with_message("完成！");

    // 计算文件大小变化
    let original_size = std::fs::metadata(input)?.len();
    let converted_size = converted_data.len() as u64;
    let ratio = (converted_size as f64 / original_size as f64) * 100.0;

    info!(
        original_size = original_size,
        converted_size = converted_size,
        size_ratio = ratio,
        output_path = ?output_path,
        "字体格式转换完成"
    );

    println!("  ✓ 文件已保存: {:?}", output_path);
    println!("\n{}", "📊 统计信息:".bold().cyan());
    println!("  原始大小:   {}", format_size(original_size));
    println!("  转换后大小: {}", format_size(converted_size));
    println!("  大小比例:   {:.1}%", ratio);

    if (format == "woff" || format == "woff2") && converted_size < original_size {
        let saved = original_size - converted_size;
        println!("  节省空间:   {}", format_size(saved));
    }

    println!("\n{}", "✨ 转换完成！".bold().green());

    Ok(())
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
