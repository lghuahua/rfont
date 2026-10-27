use anyhow::{Context, Result, anyhow};
use colored::*;
use rfont::{FontCollection, FontFormat};
use std::path::{Path, PathBuf};
use tracing::{Level, debug, info, span};

/// 列出 TTC 集合中的字面
pub fn list(font_path: &Path, json: bool) -> Result<()> {
    let span = span!(Level::INFO, "ttc_list_command", path = ?font_path, json = json);
    let _enter = span.enter();

    let collection = FontCollection::load(font_path)
        .with_context(|| format!("无法加载 TTC 字体集合: {}", font_path.display()))?;
    let faces = collection.faces().context("读取字面信息失败")?;

    debug!(face_count = faces.len(), "字面信息读取完成");

    if json {
        println!("{}", serde_json::to_string_pretty(&faces)?);
    } else {
        println!(
            "{}",
            format!("📚 TTC 字体集合（{} 个字面）", collection.len())
                .bold()
                .cyan()
        );
        println!("{}", "─".repeat(60).dimmed());

        for face in &faces {
            let family = face
                .family
                .as_deref()
                .or(face.full_name.as_deref())
                .unwrap_or("<未知>");
            let style = face.subfamily.as_deref().unwrap_or("");
            println!(
                "  [{:>2}] {} {}  ({} 字形, {})",
                face.index, family, style, face.glyph_count, face.format
            );
        }
    }

    info!(face_count = faces.len(), "TTC 字面列表输出完成");

    Ok(())
}

/// 提取指定字面为独立字体文件
pub fn extract(font_path: &Path, face: usize, output: Option<&Path>) -> Result<()> {
    let span = span!(Level::INFO, "ttc_extract_command", path = ?font_path, face = face);
    let _enter = span.enter();

    let collection = FontCollection::load(font_path)
        .with_context(|| format!("无法加载 TTC 字体集合: {}", font_path.display()))?;
    let faces = collection.faces().context("读取字面信息失败")?;
    let summary = faces
        .get(face)
        .ok_or_else(|| anyhow!("字面索引 {} 超出范围（共 {} 个字面）", face, faces.len()))?;

    let bytes = collection.face_bytes(face).context("提取字面失败")?;
    let output_path = match output {
        Some(path) => path.to_path_buf(),
        None => default_output_path(font_path, face, summary.format),
    };

    std::fs::write(&output_path, &bytes)
        .with_context(|| format!("无法写入输出文件: {}", output_path.display()))?;

    info!(
        face = face,
        bytes = bytes.len(),
        output = %output_path.display(),
        "字面提取完成"
    );
    println!(
        "  ✓ 已提取字面 {}：{}（{}）",
        face,
        output_path.display(),
        format_size(bytes.len() as u64)
    );

    Ok(())
}

fn default_output_path(font_path: &Path, face: usize, format: FontFormat) -> PathBuf {
    let stem = font_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("font");
    let ext = match format {
        FontFormat::Otf => "otf",
        _ => "ttf",
    };
    font_path.with_file_name(format!("{}.face{}.{}", stem, face, ext))
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
