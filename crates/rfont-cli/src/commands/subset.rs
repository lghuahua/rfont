use colored::*;
use anyhow::{Result, Context};
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};
use rfont::Font;

pub fn run(
    input: &Path,
    output: Option<&Path>,
    text: Option<&str>,
    text_file: Option<&Path>,
    ranges: &[String],
    strip_post_names: bool,
    format: &str,
    compression: u8,
) -> Result<()> {
    // 加载字体
    println!("{}", "📖 加载字体...".bold().cyan());
    let font = Font::load(input.to_str().unwrap())
        .context(format!("无法加载字体文件: {:?}", input))?;
    
    println!("  ✓ 成功加载字体");
    println!("  字形总数: {}", font.get_font_info().glyph_count);

    // 收集要包含的字符
    let mut all_text = String::new();
    
    if let Some(t) = text {
        all_text.push_str(t);
    }
    
    if let Some(file_path) = text_file {
        let content = std::fs::read_to_string(file_path)
            .context(format!("无法读取文本文件: {:?}", file_path))?;
        all_text.push_str(&content);
    }
    
    // 解析 Unicode 范围
    for range_str in ranges {
        let chars = parse_unicode_range(range_str)?;
        for ch in chars {
            all_text.push(ch);
        }
    }
    
    if all_text.is_empty() {
        return Err(anyhow::anyhow!("未指定任何文本或字符范围"));
    }
    
    println!("\n{}", "🔤 处理文本...".bold().cyan());
    println!("  文本长度: {} 个字符", all_text.chars().count());
    
    // 创建进度条
    let progress = ProgressBar::new(100);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>3}% {msg}")
            .unwrap()
            .progress_chars("=> ")
    );
    progress.set_message("子集化中...");

    // 执行子集化
    let mut builder = font.subset_builder().text(&all_text);
    
    if strip_post_names {
        builder = builder.strip_glyph_names(true);
    }
    
    if format == "woff" {
        builder = builder.output_format("woff").compression_level(compression);
    }
    
    progress.inc(50);
    
    let subset_data = builder.build()
        .context("子集化失败")?;
    
    progress.inc(50);
    progress.finish_with_message("完成！");

    // 确定输出路径
    let output_path = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let stem = input.file_stem().unwrap().to_str().unwrap();
            let ext = if format == "woff" { "woff" } else { "ttf" };
            input.with_file_name(format!("{}_subset.{}", stem, ext))
        }
    };

    // 写入文件
    println!("\n{}", "💾 保存文件...".bold().cyan());
    std::fs::write(&output_path, &subset_data)
        .context(format!("无法写入输出文件: {:?}", output_path))?;
    
    // 计算压缩率
    let original_size = std::fs::metadata(input)?.len();
    let subset_size = subset_data.len() as u64;
    let ratio = (subset_size as f64 / original_size as f64) * 100.0;
    let saved = original_size - subset_size;
    
    println!("  ✓ 文件已保存: {:?}", output_path);
    println!("\n{}", "📊 统计信息:".bold().cyan());
    println!("  原始大小:   {}", format_size(original_size));
    println!("  子集大小:   {}", format_size(subset_size));
    println!("  压缩率:     {:.1}%", ratio);
    println!("  节省空间:   {}", format_size(saved));
    
    println!("\n{}", "✨ 完成！".bold().green());

    Ok(())
}

fn parse_unicode_range(range_str: &str) -> Result<Vec<char>> {
    // 解析格式：U+4E00-U+9FFF 或 U+4E00..U+9FFF
    let range_str = range_str.trim();
    
    let parts: Vec<&str> = if range_str.contains("-") {
        range_str.split("-").collect()
    } else if range_str.contains("..") {
        range_str.split("..").collect()
    } else {
        return Err(anyhow::anyhow!("无效的 Unicode 范围格式: {}", range_str));
    };
    
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Unicode 范围必须包含起始和结束值"));
    }
    
    let start = parse_unicode_codepoint(parts[0])?;
    let end = parse_unicode_codepoint(parts[1])?;
    
    if start > end {
        return Err(anyhow::anyhow!("起始码点不能大于结束码点"));
    }
    
    let chars: Vec<char> = (start..=end)
        .filter_map(|cp| char::from_u32(cp))
        .collect();
    
    Ok(chars)
}

fn parse_unicode_codepoint(s: &str) -> Result<u32> {
    let s = s.trim();
    let s = if s.starts_with("U+") || s.starts_with("u+") {
        &s[2..]
    } else if s.starts_with("0x") || s.starts_with("0X") {
        &s[2..]
    } else {
        s
    };
    
    u32::from_str_radix(s, 16)
        .context(format!("无效的 Unicode 码点: {}", s))
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
