pub mod batch;
pub mod convert;
pub mod edit;
pub mod info;
pub mod subset;
pub mod ttc;

use anyhow::Result;
use colored::*;

/// 根据输出格式解析压缩级别
///
/// - WOFF: 默认 9，有效范围 0-9
/// - WOFF2: 默认 11，有效范围 0-11
/// - 其他格式: 返回 0
///
/// 用户显式指定值时，会按格式上限进行截断，避免传入无效值。
pub fn resolve_compression(format: &str, compression: Option<u8>) -> u8 {
    match format {
        "woff" => compression.unwrap_or(9).min(9),
        "woff2" => compression.unwrap_or(11).min(11),
        _ => compression.unwrap_or(0),
    }
}

/// 输出授权提示
///
/// 提示写到 stderr，不影响标准输出。`strict` 为真时提示升级为错误并中止，
/// 供 CI 流水线把字体授权检查变成强制门禁。
pub fn report_restrictions(warnings: &[String], strict: bool) -> Result<()> {
    if warnings.is_empty() {
        return Ok(());
    }

    eprintln!("{}", "⚠️  授权提示:".bold().yellow());
    for warning in warnings {
        eprintln!("  • {warning}");
    }

    if strict {
        anyhow::bail!("检测到字体授权限制，已按 --strict 中止");
    }

    Ok(())
}
