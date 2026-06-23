use anyhow::{Context, Result};
use colored::*;
use glob::glob;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rfont::Font;
use std::path::{Path, PathBuf};
use tracing::{Level, debug, info, span, warn};

/// 批量转换命令的参数
pub struct BatchConvertArgs {
    pub pattern: String,             // 文件匹配模式（如 *.ttf）
    pub formats: Vec<String>,        // 目标格式列表（woff/woff2/ttf）
    pub output_dir: Option<PathBuf>, // 输出目录
    pub compression: Option<u8>,     // 压缩级别（不指定则按格式自动选择最优值）
    pub overwrite: bool,             // 是否覆盖已存在的文件
    #[cfg(feature = "parallel")]
    pub jobs: Option<usize>, // 并行任务数
}

/// 单个文件的转换结果
struct ConvertResult {
    input_path: PathBuf,
    output_path: PathBuf,
    success: bool,
    error: Option<String>,
    original_size: u64,
    converted_size: u64,
}

pub fn run(args: &BatchConvertArgs) -> Result<()> {
    let span = span!(Level::INFO, "batch_convert_command",
                     pattern = args.pattern,
                     formats = ?args.formats,
                     compression = ?args.compression,
                     overwrite = args.overwrite);
    let _enter = span.enter();

    debug!("开始批量格式转换");

    println!("{}", "🔄 批量格式转换".bold().cyan());
    println!();

    // 1. 查找匹配的字体文件
    debug!(pattern = args.pattern, "查找匹配的字体文件");
    let files = find_font_files(&args.pattern)?;

    if files.is_empty() {
        warn!(pattern = args.pattern, "未找到匹配的文件");
        return Err(anyhow::anyhow!("未找到匹配的文件: {}", args.pattern));
    }

    // 验证格式列表
    for format in &args.formats {
        let fmt = format.to_lowercase();
        if fmt != "ttf" && fmt != "woff" && fmt != "woff2" {
            warn!(format = format, "不支持的格式");
            return Err(anyhow::anyhow!(
                "不支持的格式: {} (仅支持 ttf, woff, woff2)",
                format
            ));
        }
    }

    debug!(
        file_count = files.len(),
        format_count = args.formats.len(),
        "文件查找完成"
    );
    println!("  找到 {} 个字体文件", files.len());
    println!(
        "  目标格式: {}",
        args.formats
            .iter()
            .map(|f| f.to_uppercase())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  压缩级别: {}", args.compression.map(|c| c.to_string()).unwrap_or_else(|| "自动".to_string()));
    println!();

    // 2. 创建输出目录（如果指定）
    if let Some(ref dir) = args.output_dir {
        std::fs::create_dir_all(dir).context(format!("无法创建输出目录: {:?}", dir))?;
        println!("  输出目录: {:?}", dir);
        println!();
    }

    // 3. 创建进度条
    let multi_progress = MultiProgress::new();
    let total_tasks = files.len() * args.formats.len();
    let overall_progress = multi_progress.add(ProgressBar::new(total_tasks as u64));
    overall_progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=> "),
    );
    overall_progress.set_message("处理中...");

    // 4. 处理文件（并行或串行）
    #[cfg(feature = "parallel")]
    let all_results = if let Some(jobs) = args.jobs {
        // 使用指定的线程数
        rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global()
            .ok();
        process_files_parallel(
            &files,
            &args.formats,
            &args.output_dir,
            args.compression,
            args.overwrite,
            &multi_progress,
            &overall_progress,
        )?
    } else {
        // 使用默认线程数（CPU 核心数）
        process_files_parallel(
            &files,
            &args.formats,
            &args.output_dir,
            args.compression,
            args.overwrite,
            &multi_progress,
            &overall_progress,
        )?
    };

    #[cfg(not(feature = "parallel"))]
    let all_results = process_files_sequential(
        &files,
        &args.formats,
        &args.output_dir,
        args.compression,
        args.overwrite,
        &multi_progress,
        &overall_progress,
    );

    overall_progress.finish_with_message("批量转换完成！");

    // 5. 显示统计信息
    print_summary(&all_results);

    let success_count = all_results.iter().filter(|r| r.success).count();
    let failed_count = all_results.len() - success_count;

    info!(
        total_files = all_results.len(),
        success_count = success_count,
        failed_count = failed_count,
        "批量转换完成"
    );

    Ok(())
}

/// 查找匹配的字体文件
fn find_font_files(pattern: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    // 检查是否是通配符模式
    if pattern.contains('*') || pattern.contains('?') {
        for entry in glob(pattern).context(format!("无效的文件模式: {}", pattern))? {
            match entry {
                Ok(path) => {
                    if path.is_file() {
                        files.push(path);
                    }
                }
                Err(e) => eprintln!("警告: 无法读取文件: {}", e),
            }
        }
    } else {
        // 单个文件
        let path = PathBuf::from(pattern);
        if path.exists() && path.is_file() {
            files.push(path);
        } else {
            return Err(anyhow::anyhow!("文件不存在: {}", pattern));
        }
    }

    // 按文件名排序
    files.sort();

    Ok(files)
}

/// 转换单个文件
fn convert_single_file(
    input_path: &Path,
    format: &str,
    output_dir: &Option<PathBuf>,
    compression: Option<u8>,
    overwrite: bool,
    progress: &ProgressBar,
) -> ConvertResult {
    progress.inc(10);

    // 确定输出路径
    let output_path = determine_output_path(input_path, format, output_dir);

    // 检查文件是否已存在
    if output_path.exists() && !overwrite {
        return ConvertResult {
            input_path: input_path.to_path_buf(),
            output_path,
            success: false,
            error: Some("文件已存在，使用 --overwrite 覆盖".to_string()),
            original_size: 0,
            converted_size: 0,
        };
    }

    progress.inc(10);
    progress.set_message("加载中...");

    // 加载字体
    let font = match Font::load(input_path.to_str().unwrap()) {
        Ok(f) => f,
        Err(e) => {
            return ConvertResult {
                input_path: input_path.to_path_buf(),
                output_path,
                success: false,
                error: Some(format!("加载失败: {}", e)),
                original_size: 0,
                converted_size: 0,
            };
        }
    };

    progress.inc(20);
    progress.set_message("转换中...");

    // 执行转换
    let converted_data = match perform_conversion(&font, format, compression) {
        Ok(data) => data,
        Err(e) => {
            return ConvertResult {
                input_path: input_path.to_path_buf(),
                output_path,
                success: false,
                error: Some(format!("转换失败: {}", e)),
                original_size: std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0),
                converted_size: 0,
            };
        }
    };

    progress.inc(40);
    progress.set_message("保存中...");

    // 写入文件
    if let Err(e) = std::fs::write(&output_path, &converted_data) {
        return ConvertResult {
            input_path: input_path.to_path_buf(),
            output_path,
            success: false,
            error: Some(format!("保存失败: {}", e)),
            original_size: std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0),
            converted_size: 0,
        };
    }

    progress.inc(20);

    let original_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
    let converted_size = converted_data.len() as u64;

    ConvertResult {
        input_path: input_path.to_path_buf(),
        output_path,
        success: true,
        error: None,
        original_size,
        converted_size,
    }
}

/// 执行格式转换
fn perform_conversion(font: &Font, format: &str, compression: Option<u8>) -> Result<Vec<u8>> {
    let font_info = font.get_font_info().context("获取字体信息失败")?;
    let all_glyph_ids: Vec<u16> = (0..font_info.glyph_count).collect();

    // 根据格式解析压缩级别
    let compression = super::resolve_compression(format, compression);

    let data = font
        .subset_builder()
        .glyph_ids(all_glyph_ids)
        .output_format(format)
        .compression_level(compression)
        .build()
        .context(format!("转换为 {} 失败", format.to_uppercase()))?;

    Ok(data)
}

/// 确定输出文件路径
fn determine_output_path(input_path: &Path, format: &str, output_dir: &Option<PathBuf>) -> PathBuf {
    let stem = input_path.file_stem().unwrap().to_str().unwrap();
    let ext = match format.to_lowercase().as_str() {
        "woff" => "woff",
        "woff2" => "woff2",
        _ => "ttf",
    };

    let filename = format!("{}.{}", stem, ext);

    match output_dir {
        Some(dir) => dir.join(&filename),
        None => input_path.with_file_name(&filename),
    }
}

/// 打印统计摘要
fn print_summary(results: &[ConvertResult]) {
    println!();
    println!("{}", "📊 转换统计:".bold().cyan());
    println!();

    let total = results.len();
    let success_count = results.iter().filter(|r| r.success).count();
    let failed_count = total - success_count;

    // 总体统计
    println!("  总文件数:   {}", total);
    println!("  成功:       {} {}", success_count, "✓".green());
    println!(
        "  失败:       {} {}",
        failed_count,
        if failed_count > 0 {
            "✗".red()
        } else {
            "".normal()
        }
    );
    println!();

    if success_count > 0 {
        let total_original: u64 = results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.original_size)
            .sum();
        let total_converted: u64 = results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.converted_size)
            .sum();
        let total_saved = total_original.saturating_sub(total_converted);
        let ratio = if total_original > 0 {
            (total_converted as f64 / total_original as f64) * 100.0
        } else {
            0.0
        };

        println!("  原始总大小:     {}", format_size(total_original));
        println!("  转换后总大小:   {}", format_size(total_converted));
        println!("  总节省空间:     {}", format_size(total_saved));
        println!("  平均压缩率:     {:.1}%", ratio);
        println!();
    }

    // 详细列表
    if failed_count > 0 {
        println!("  {}", "失败的文件:".bold().red());
        for result in results.iter().filter(|r| !r.success) {
            let filename = result.input_path.file_name().unwrap().to_str().unwrap();
            println!("    ✗ {} - {}", filename, result.error.as_ref().unwrap());
        }
        println!();
    }

    if success_count > 0 {
        println!("  {}", "成功的文件:".bold().green());
        for result in results.iter().filter(|r| r.success) {
            let filename = result.input_path.file_name().unwrap().to_str().unwrap();
            let saved = result.original_size.saturating_sub(result.converted_size);
            let ratio = if result.original_size > 0 {
                (result.converted_size as f64 / result.original_size as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "    ✓ {} → {} ({:.1}%, 节省 {})",
                filename,
                result.output_path.file_name().unwrap().to_str().unwrap(),
                ratio,
                format_size(saved)
            );
        }
    }

    println!();
    if failed_count == 0 {
        println!("{}", "✨ 所有文件转换成功！".bold().green());
    } else {
        println!(
            "{}",
            format!("⚠️  部分文件转换失败 ({}/{})", failed_count, total)
                .bold()
                .yellow()
        );
    }
}

/// 格式化文件大小
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// 串行处理文件（未启用 parallel feature 时使用）
fn process_files_sequential(
    files: &[PathBuf],
    formats: &[String],
    output_dir: &Option<PathBuf>,
    compression: Option<u8>,
    overwrite: bool,
    multi_progress: &MultiProgress,
    overall_progress: &ProgressBar,
) -> Vec<ConvertResult> {
    let mut all_results = Vec::new();

    for (index, input_path) in files.iter().enumerate() {
        let filename = input_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        overall_progress.set_message(format!("处理 {}/{}: {}", index + 1, files.len(), filename));

        for format in formats {
            let file_progress =
                multi_progress.insert_after(overall_progress, ProgressBar::new(100));
            file_progress.set_style(
                ProgressStyle::default_bar()
                    .template("  {spinner:.yellow} [{bar:20.green/blue}] {pos:>3}% {msg}")
                    .unwrap()
                    .progress_chars("=> "),
            );
            file_progress.set_message(format!("{} → {}", filename, format.to_uppercase()));

            let result = convert_single_file(
                input_path,
                format,
                output_dir,
                compression,
                overwrite,
                &file_progress,
            );

            file_progress.finish_and_clear();
            overall_progress.inc(1);

            all_results.push(result);
        }
    }

    all_results
}

/// 并行处理文件（启用 parallel feature 时使用）
#[cfg(feature = "parallel")]
fn process_files_parallel(
    files: &[PathBuf],
    formats: &[String],
    output_dir: &Option<PathBuf>,
    compression: Option<u8>,
    overwrite: bool,
    multi_progress: &MultiProgress,
    overall_progress: &ProgressBar,
) -> Result<Vec<ConvertResult>> {
    use rayon::prelude::*;
    use std::sync::Mutex;

    // 创建线程安全的进度计数器
    let completed = Mutex::new(0u64);
    let total = (files.len() * formats.len()) as u64;

    // 并行处理所有文件和格式的组合
    let results: Vec<ConvertResult> = files
        .par_iter()
        .flat_map(|input_path| {
            formats
                .par_iter()
                .map(|format| {
                    // 注意：并行模式下不使用单个文件的进度条，因为会混乱
                    // 只更新总体进度
                    let result = convert_single_file_simple(
                        input_path,
                        format,
                        output_dir,
                        compression,
                        overwrite,
                    );

                    // 更新总体进度
                    {
                        let mut count = completed.lock().unwrap();
                        *count += 1;
                        overall_progress.set_position(*count);
                        overall_progress.set_message(format!("处理 {}/{}", *count, total));
                    }

                    result
                })
                .collect::<Vec<_>>()
        })
        .collect();

    Ok(results)
}

/// 简化的转换函数（用于并行模式，不带进度条）
#[cfg(feature = "parallel")]
fn convert_single_file_simple(
    input_path: &Path,
    format: &str,
    output_dir: &Option<PathBuf>,
    compression: Option<u8>,
    overwrite: bool,
) -> ConvertResult {
    let original_size = std::fs::metadata(input_path).unwrap().len();

    // 加载字体
    let font = match Font::load(input_path.to_str().unwrap()) {
        Ok(f) => f,
        Err(e) => {
            return ConvertResult {
                input_path: input_path.to_path_buf(),
                output_path: PathBuf::new(),
                success: false,
                error: Some(format!("加载字体失败: {}", e)),
                original_size,
                converted_size: 0,
            };
        }
    };

    // 确定输出路径
    let output_path = determine_output_path(input_path, format, output_dir);

    // 检查是否已存在
    if !overwrite && output_path.exists() {
        return ConvertResult {
            input_path: input_path.to_path_buf(),
            output_path,
            success: false,
            error: Some("文件已存在，使用 --overwrite 覆盖".to_string()),
            original_size,
            converted_size: 0,
        };
    }

    // 执行转换
    match perform_conversion(&font, format, compression) {
        Ok(data) => {
            let converted_size = data.len() as u64;
            if let Err(e) = std::fs::write(&output_path, &data) {
                return ConvertResult {
                    input_path: input_path.to_path_buf(),
                    output_path,
                    success: false,
                    error: Some(format!("写入文件失败: {}", e)),
                    original_size,
                    converted_size: 0,
                };
            }
            ConvertResult {
                input_path: input_path.to_path_buf(),
                output_path,
                success: true,
                error: None,
                original_size,
                converted_size,
            }
        }
        Err(e) => ConvertResult {
            input_path: input_path.to_path_buf(),
            output_path,
            success: false,
            error: Some(format!("{}", e)),
            original_size,
            converted_size: 0,
        },
    }
}
