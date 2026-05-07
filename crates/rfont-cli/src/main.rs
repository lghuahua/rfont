use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;

mod commands;

/// rfont - 命令行字体子集化和转换工具
#[derive(Parser)]
#[command(name = "rfont")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// 启用详细输出
    #[arg(short, long, global = true)]
    verbose: bool,

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
        #[arg(short, long)]
        verbose: bool,
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

    // 初始化日志
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    match cli.command {
        Commands::Info { font, json, verbose } => {
            commands::info::run(&font, json, verbose)?;
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
        Commands::Batch { command } => {
            match command {
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
            }
        }
    }

    Ok(())
}
