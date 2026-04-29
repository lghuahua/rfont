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

        /// 目标格式（ttf 或 woff）
        #[arg(short, long)]
        format: String,

        /// WOFF 压缩级别（0-9，仅用于 WOFF 输出）
        #[arg(long, default_value = "6")]
        compression: u8,
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
    }

    Ok(())
}
