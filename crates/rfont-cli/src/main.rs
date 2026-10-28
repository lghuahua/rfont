use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::debug;

mod commands;

/// 初始化日志系统
fn init_logging(verbosity: u8) {
    // 支持通过环境变量 RUST_LOG 覆盖
    let level = if let Ok(env_level) = std::env::var("RUST_LOG") {
        // 使用环境变量指定的级别
        match env_level.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => {
                eprintln!("警告: 无效的 RUST_LOG 级别 '{}', 使用默认值", env_level);
                tracing::Level::WARN
            }
        }
    } else {
        // 根据 -v 参数数量决定日志级别
        match verbosity {
            0 => tracing::Level::WARN,  // 默认：只显示警告和错误
            1 => tracing::Level::INFO,  // -v: 显示信息
            2 => tracing::Level::DEBUG, // -vv: 显示调试信息
            _ => tracing::Level::TRACE, // -vvv+: 显示追踪信息
        }
    };

    // 配置 tracing subscriber
    let format = tracing_subscriber::fmt::format()
        .without_time() // 不显示时间戳（CLI 工具不需要）
        .with_target(false) // 不显示目标模块名
        .with_thread_ids(false) // 不显示线程 ID
        .with_file(false) // 不显示文件名
        .with_line_number(false); // 不显示行号

    tracing_subscriber::fmt()
        .with_max_level(level)
        .event_format(format)
        .with_ansi(true) // 启用 ANSI 颜色输出
        .init();

    // 记录启动信息（只在 INFO 及以上级别显示）
    tracing::info!(verbosity = verbosity, level = %level, "日志系统初始化完成");
}

/// rfont - 命令行字体子集化和转换工具
#[derive(Parser)]
#[command(name = "rfont")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// 启用详细输出（可多次指定：-v=INFO, -vv=DEBUG, -vvv=TRACE）
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 显示字体信息
    Info {
        /// 字体文件路径
        font: PathBuf,

        /// TTC 字体集合中的字面索引（单字体请保持 0）
        #[arg(long, default_value_t = 0)]
        face: usize,

        /// 以 JSON 格式输出
        #[arg(long)]
        json: bool,

        /// 显示详细信息（包括所有表）
        #[arg(short = 'V', long = "verbose-info")]
        verbose_info: bool,
    },

    /// 列出或提取 TTC 字体集合中的字面
    Ttc {
        #[command(subcommand)]
        command: TtcCommands,
    },

    /// 创建字体子集
    Subset {
        /// 输入字体文件路径
        input: PathBuf,

        /// TTC 字体集合中的字面索引
        #[arg(long, default_value_t = 0)]
        face: usize,

        /// 输出字体文件路径
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 要包含的文本
        #[arg(short, long)]
        text: Option<String>,

        /// 从文件读取字符列表
        #[arg(long = "text-file")]
        text_file: Option<PathBuf>,

        /// Unicode 范围（例如：U+4E00-U+9FFF）
        #[arg(long = "range")]
        ranges: Vec<String>,

        /// 保留 hinting 数据（glyf 指令与 cvt/fpgm/prep/gasp 表）
        #[arg(long = "keep-hinting")]
        keep_hinting: bool,

        /// 保留 post 表中的字形名称
        #[arg(long = "keep-post-names")]
        keep_post_names: bool,

        /// 输出格式（ttf、woff 或 woff2）
        #[arg(long, default_value = "ttf")]
        format: String,

        /// WOFF/WOFF2 压缩级别（WOFF: 0-9, WOFF2: 0-11，不指定则按格式自动选择最优值）
        #[arg(long)]
        compression: Option<u8>,

        /// 遇到字体授权限制时中止而不是只提示
        #[arg(long)]
        strict: bool,
    },

    /// 转换字体格式
    Convert {
        /// 输入字体文件路径
        input: PathBuf,

        /// TTC 字体集合中的字面索引
        #[arg(long, default_value_t = 0)]
        face: usize,

        /// 输出字体文件路径
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 目标格式（ttf、woff 或 woff2）
        #[arg(short, long)]
        format: String,

        /// WOFF/WOFF2 压缩级别（WOFF: 0-9, WOFF2: 0-11，不指定则按格式自动选择最优值）
        #[arg(long)]
        compression: Option<u8>,
    },

    /// 修改字体元信息（名称、作者、版权等）
    Edit {
        /// 输入字体文件路径
        input: PathBuf,

        /// TTC 字体集合中的字面索引
        #[arg(long, default_value_t = 0)]
        face: usize,

        /// 输出字体文件路径（默认为 <文件名>_edited.<格式>）
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 输出格式（ttf、otf、woff 或 woff2，不指定则沿用输入格式）
        #[arg(short, long)]
        format: Option<String>,

        /// WOFF/WOFF2 压缩级别（WOFF: 0-9, WOFF2: 0-11，不指定则按格式自动选择最优值）
        #[arg(long)]
        compression: Option<u8>,

        /// 家族名称（nameID 1）
        #[arg(long)]
        family: Option<String>,

        /// 样式名称（nameID 2）
        #[arg(long)]
        subfamily: Option<String>,

        /// 完整名称（nameID 4，不指定时由家族名与样式名组合）
        #[arg(long = "full-name")]
        full_name: Option<String>,

        /// 版本字符串（nameID 5）
        #[arg(long)]
        version: Option<String>,

        /// PostScript 名称（nameID 6，只能包含 ASCII 可见字符且不超过 63 个字符）
        #[arg(long = "postscript-name")]
        postscript_name: Option<String>,

        /// 制造商（nameID 8）
        #[arg(long)]
        manufacturer: Option<String>,

        /// 设计者 / 作者（nameID 9）
        #[arg(long)]
        designer: Option<String>,

        /// 版权声明（nameID 0）
        #[arg(long)]
        copyright: Option<String>,

        /// 许可证描述（nameID 13）
        #[arg(long)]
        license: Option<String>,

        /// 遇到字体授权限制时中止而不是只提示
        #[arg(long)]
        strict: bool,
    },

    /// 将文本或字形导出为 SVG
    Svg {
        /// 输入字体文件路径
        input: PathBuf,

        /// TTC 字体集合中的字面索引
        #[arg(long, default_value_t = 0)]
        face: usize,

        /// 要导出的文本（单行）
        #[arg(long, conflicts_with = "glyph")]
        text: Option<String>,

        /// 要导出的字形 ID
        #[arg(long)]
        glyph: Option<u16>,

        /// 输出 SVG 文件路径（默认与输入同名的 .svg）
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 字号（像素）
        #[arg(long, default_value_t = 64.0)]
        size: f64,

        /// 填充色（CSS 颜色）
        #[arg(long, default_value = "#000000")]
        color: String,

        /// 四周留白（像素）
        #[arg(long, default_value_t = 4.0)]
        padding: f64,
    },

    /// 批量处理字体文件
    Batch {
        /// 子命令：convert（批量转换）
        #[command(subcommand)]
        command: BatchCommands,
    },
}

#[derive(Subcommand)]
enum TtcCommands {
    /// 列出集合中的所有字面
    List {
        /// TTC 字体文件路径
        font: PathBuf,

        /// 以 JSON 格式输出
        #[arg(long)]
        json: bool,
    },

    /// 提取指定字面为独立字体
    Extract {
        /// TTC 字体文件路径
        font: PathBuf,

        /// 字面索引（从 0 开始）
        #[arg(long, default_value_t = 0)]
        face: usize,

        /// 输出文件路径（默认为 <文件名>.face<N>.ttf/otf）
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum BatchCommands {
    /// 批量转换字体格式
    Convert {
        /// 文件匹配模式（支持通配符，如 *.ttf）
        pattern: String,

        /// TTC 字体集合中的字面索引
        #[arg(long, default_value_t = 0)]
        face: usize,

        /// 目标格式（可指定多个，如 -f woff -f woff2）
        #[arg(short, long = "format", required = true)]
        formats: Vec<String>,

        /// 输出目录（默认为输入文件所在目录）
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// WOFF/WOFF2 压缩级别（WOFF: 0-9, WOFF2: 0-11，不指定则按格式自动选择最优值）
        #[arg(long)]
        compression: Option<u8>,

        /// 覆盖已存在的文件
        #[arg(long)]
        overwrite: bool,

        /// 并行任务数（默认使用 CPU 核心数，需要启用 parallel feature）
        #[cfg(feature = "parallel")]
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志系统（支持多级 verbosity）
    init_logging(cli.verbose);

    match cli.command {
        Commands::Info {
            font,
            face,
            json,
            verbose_info,
        } => {
            commands::info::run(&font, face, json, verbose_info)?;
        }
        Commands::Ttc { command } => match command {
            TtcCommands::List { font, json } => commands::ttc::list(&font, json)?,
            TtcCommands::Extract { font, face, output } => {
                commands::ttc::extract(&font, face, output.as_deref())?
            }
        },
        Commands::Subset {
            input,
            face,
            output,
            text,
            text_file,
            ranges,
            keep_hinting,
            keep_post_names,
            format,
            compression,
            strict,
        } => {
            // 合并 text 和 text_file 为一个文本源
            let text_source = match (text, text_file) {
                (Some(t), None) => Some(t),
                (t, Some(file_path)) => {
                    let text = t.unwrap_or("".to_string());
                    // 从文件读取内容
                    debug!(path = ?file_path, "从文件读取文本");
                    let content = std::fs::read_to_string(&file_path)
                        .context(format!("无法读取文本文件: {:?}", file_path))?;
                    debug!(content_length = content.len(), "文件读取成功");
                    Some(format!("{}{}", text, content))
                }
                (None, None) => None,
            };

            commands::subset::run(
                &input,
                output.as_deref(),
                commands::subset::SubsetRequest {
                    text: text_source.as_deref(),
                    ranges: &ranges,
                    face,
                    keep_hinting,
                    keep_post_names,
                    format: &format,
                    compression,
                    strict,
                },
            )?;
        }
        Commands::Convert {
            input,
            face,
            output,
            format,
            compression,
        } => {
            commands::convert::run(&input, output.as_deref(), &format, compression, face)?;
        }
        Commands::Edit {
            input,
            face,
            output,
            format,
            compression,
            family,
            subfamily,
            full_name,
            version,
            postscript_name,
            manufacturer,
            designer,
            copyright,
            license,
            strict,
        } => {
            let edit = rfont::FontMetaEdit {
                family,
                subfamily,
                full_name,
                version,
                postscript_name,
                manufacturer,
                designer,
                copyright,
                license,
            };
            commands::edit::run(
                &input,
                &commands::edit::EditRequest {
                    face,
                    output: output.as_deref(),
                    format: format.as_deref(),
                    compression,
                    edit: &edit,
                    strict,
                },
            )?;
        }
        Commands::Svg {
            input,
            face,
            text,
            glyph,
            output,
            size,
            color,
            padding,
        } => {
            commands::svg::run(
                &input,
                output.as_deref(),
                face,
                commands::svg::SvgRequest {
                    text: text.as_deref(),
                    glyph,
                    size,
                    color: &color,
                    padding,
                },
            )?;
        }
        Commands::Batch { command } => match command {
            BatchCommands::Convert {
                pattern,
                face,
                formats,
                output_dir,
                compression,
                overwrite,
                #[cfg(feature = "parallel")]
                jobs,
            } => {
                let args = commands::batch::BatchConvertArgs {
                    pattern,
                    formats,
                    output_dir,
                    compression,
                    overwrite,
                    face,
                    #[cfg(feature = "parallel")]
                    jobs,
                };
                commands::batch::run(&args)?;
            }
        },
    }

    Ok(())
}
