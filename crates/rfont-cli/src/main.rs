use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

        /// 以 JSON 格式输出
        #[arg(long)]
        json: bool,

        /// 显示详细信息（包括所有表）
        #[arg(short = 'V', long = "verbose-info")]
        verbose_info: bool,
    },

    /// 创建字体子集
    Subset {
        /// 输入字体文件路径
        input: PathBuf,

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

        /// 优化 post 表（移除字形名称）
        #[arg(long = "strip-post-names")]
        strip_post_names: bool,

        /// 输出格式（ttf 或 woff）
        #[arg(long, default_value = "ttf")]
        format: String,

        /// WOFF 压缩级别（0-9）
        #[arg(long, default_value = "6")]
        compression: u8,
    },

    /// 转换字体格式
    Convert {
        /// 输入字体文件路径
        input: PathBuf,

        /// 输出字体文件路径
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 目标格式（ttf、woff 或 woff2）
        #[arg(short, long)]
        format: String,

        /// WOFF/WOFF2 压缩级别（0-9，仅用于 WOFF/WOFF2 输出）
        #[arg(long, default_value = "6")]
        compression: u8,
    },

    /// 批量处理字体文件
    Batch {
        /// 子命令：convert（批量转换）
        #[command(subcommand)]
        command: BatchCommands,
    },
}

#[derive(Subcommand)]
enum BatchCommands {
    /// 批量转换字体格式
    Convert {
        /// 文件匹配模式（支持通配符，如 *.ttf）
        pattern: String,

        /// 目标格式（可指定多个，如 -f woff -f woff2）
        #[arg(short, long = "format", required = true)]
        formats: Vec<String>,

        /// 输出目录（默认为输入文件所在目录）
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// WOFF/WOFF2 压缩级别（0-9）
        #[arg(long, default_value = "6")]
        compression: u8,

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
            json,
            verbose_info,
        } => {
            commands::info::run(&font, json, verbose_info)?;
        }
        Commands::Subset {
            input,
            output,
            text,
            text_file,
            ranges,
            strip_post_names,
            format,
            compression,
        } => {
            commands::subset::run(
                &input,
                output.as_deref(),
                text.as_deref(),
                text_file.as_deref(),
                &ranges,
                strip_post_names,
                &format,
                compression,
            )?;
        }
        Commands::Convert {
            input,
            output,
            format,
            compression,
        } => {
            commands::convert::run(&input, output.as_deref(), &format, compression)?;
        }
        Commands::Batch { command } => match command {
            BatchCommands::Convert {
                pattern,
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
                    #[cfg(feature = "parallel")]
                    jobs,
                };
                commands::batch::run(&args)?;
            }
        },
    }

    Ok(())
}
