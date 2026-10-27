//! 当前时间获取。
//!
//! wasm32-unknown-unknown 的 std 未实现时钟，`SystemTime::now()` 会 panic，
//! 浏览器端改由 JS 提供当前时间。

use rfont_types::primitives::LONGDATETIME;

/// 当前时间（Mac 纪元秒）。
pub(crate) fn now() -> LONGDATETIME {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        LONGDATETIME::from_unix_seconds((js_sys::Date::now() / 1000.0) as i64)
    }
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        LONGDATETIME::from_system_time(std::time::SystemTime::now())
    }
}
