pub mod batch;
pub mod convert;
pub mod info;
pub mod subset;

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
