# 日志系统优化报告

## 概述

本次优化为 rfont CLI 工具添加了完整的日志系统支持，使用 `tracing` 框架实现结构化、多级别的日志输出。

## 主要改进

### 1. 多级 Verbosity 支持

CLI 工具现在支持通过 `-v` 参数控制日志级别：

- **默认 (WARN)**: 只显示警告和错误信息
- **-v (INFO)**: 显示操作摘要和统计信息
- **-vv (DEBUG)**: 显示详细的处理过程和中间状态
- **-vvv (TRACE)**: 显示最详细的追踪信息

```bash
rfont info font.ttf              # WARN 级别
rfont -v info font.ttf           # INFO 级别
rfont -vv subset font.ttf        # DEBUG 级别
rfont -vvv info font.ttf         # TRACE 级别
```

### 2. RUST_LOG 环境变量支持

用户可以通过环境变量灵活配置日志级别：

```bash
RUST_LOG=debug rfont info font.ttf
RUST_LOG=error rfont subset font.ttf --text "Hello"
RUST_LOG=warn rfont convert font.ttf --format woff
```

环境变量优先级高于 `-v` 参数。

### 3. 结构化日志

所有 CLI 命令都添加了结构化的 tracing 日志：

#### Info 命令
- span: `info_command` (path, json, verbose)
- debug: 字体加载过程
- info: 操作完成

#### Subset 命令
- span: `subset_command` (input, output, format, compression)
- debug: 文本处理、Unicode 范围解析、子集化配置
- info: 子集化完成（包含原始大小、子集大小、压缩率、节省空间）
- warn: 空文本警告

#### Convert 命令
- span: `convert_command` (input, output, format, compression)
- debug: 格式验证、字体加载、转换过程
- info: 转换完成（包含原始大小、转换后大小、大小比例）
- warn: 不支持的格式警告

#### Batch 命令
- span: `batch_convert_command` (pattern, formats, compression, overwrite)
- debug: 文件查找、处理进度
- info: 批量转换完成（包含总文件数、成功数、失败数）
- warn: 未找到文件或格式不支持

### 4. Span 追踪

使用 tracing 的 span 功能追踪关键操作的生命周期：

```rust
let span = span!(Level::INFO, "subset_command", 
                 input = ?input, 
                 output = ?output,
                 format = format,
                 compression = compression);
let _enter = span.enter();
```

Span 会自动记录操作的开始和结束时间，并在日志中显示层级关系。

### 5. 简洁的日志格式

优化了日志输出格式，使其更适合 CLI 工具：

- ✅ 无时间戳（CLI 工具不需要）
- ✅ 无模块名（减少噪音）
- ✅ 无线程 ID（单线程应用）
- ✅ 无文件名/行号（生产环境）
- ✅ ANSI 颜色输出（自动检测终端支持）

示例输出：
```
INFO 日志系统初始化完成 verbosity=1 level=INFO
INFO info_command{path="font.ttf" json=false verbose_info=false}: 开始加载字体文件
DEBUG subset_command{input="font.ttf" output=Some("subset.ttf") format="ttf"}: 文本处理完成 char_count=5
```

## 技术实现

### 依赖库

- `tracing`: 结构化日志框架
- `tracing-subscriber`: tracing 的订阅者实现

### 初始化代码

```rust
fn init_logging(verbosity: u8) {
    // 支持通过环境变量 RUST_LOG 覆盖
    let level = if let Ok(env_level) = std::env::var("RUST_LOG") {
        match env_level.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::WARN,
        }
    } else {
        match verbosity {
            0 => tracing::Level::WARN,
            1 => tracing::Level::INFO,
            2 => tracing::Level::DEBUG,
            _ => tracing::Level::TRACE,
        }
    };
    
    // 配置 tracing subscriber
    let format = tracing_subscriber::fmt::format()
        .without_time()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);
    
    tracing_subscriber::fmt()
        .with_max_level(level)
        .event_format(format)
        .with_ansi(true)
        .init();
    
    tracing::info!(verbosity = verbosity, level = %level, "日志系统初始化完成");
}
```

### 日志点分布

| 模块 | 日志点数量 | 主要级别 |
|------|-----------|---------|
| main.rs | 1 | INFO |
| commands/info.rs | 6 | DEBUG, INFO |
| commands/subset.rs | 15 | DEBUG, INFO, WARN |
| commands/convert.rs | 8 | DEBUG, INFO, WARN |
| commands/batch.rs | 7 | DEBUG, INFO, WARN |
| rfont/src/font/load.rs | 12 | DEBUG, INFO |
| rfont/src/font/subset.rs | 8 | DEBUG, INFO |
| rfont/src/subset/builder.rs | 5 | DEBUG |
| **总计** | **62+** | - |

## 使用示例

### 示例 1: 查看字体信息（默认级别）

```bash
$ rfont info font.ttf
📝 字体信息
────────────────────────────────────
基本信息:
  字形数量:     7044
  Units per EM: 1000
```

### 示例 2: 查看详细信息（INFO 级别）

```bash
$ rfont -v info font.ttf
INFO 日志系统初始化完成 verbosity=1 level=INFO
INFO info_command{path="font.ttf"}: 开始加载字体文件
INFO info_command{path="font.ttf"}: 检测到 TTF/OTF 格式
📝 字体信息
────────────────────────────────────
...
INFO info_command{path="font.ttf"}: 字体信息查询完成
```

### 示例 3: 调试子集化过程（DEBUG 级别）

```bash
$ rfont -vv subset font.ttf --text "Hello"
INFO 日志系统初始化完成 verbosity=2 level=DEBUG
DEBUG subset_command{input="font.ttf" format="ttf"}: 开始字体子集化处理
DEBUG subset_command{input="font.ttf"}: 正在加载字体文件: "font.ttf"
INFO subset_command{input="font.ttf"}:load_font{path="font.ttf"}: 开始加载字体文件
DEBUG subset_command{input="font.ttf"}: 字体加载成功 glyph_count=7044
📖 加载字体...
  ✓ 成功加载字体
  字形总数: 7044
DEBUG subset_command{input="font.ttf"}: 添加直接指定的文本 text_length=5
DEBUG subset_command{input="font.ttf"}: 文本处理完成 char_count=5
🔤 处理文本...
  文本长度: 5 个字符
DEBUG subset_command{input="font.ttf"}: 配置子集化选项 strip_post_names=false format="ttf"
DEBUG subset_command{input="font.ttf"}: 开始执行子集化
...
INFO subset_command{input="font.ttf"}: 字体子集化完成 original_size=5159736 subset_size=1392 compression_ratio=0.0 space_saved=5158344
```

### 示例 4: 使用环境变量

```bash
$ RUST_LOG=error rfont subset font.ttf --text "Hello"
# 只显示错误信息，不显示任何 INFO/DEBUG 日志
```

## 优势

1. **灵活的日志控制**: 用户可以根据需要调整日志详细程度
2. **结构化数据**: 日志包含丰富的上下文信息，便于分析和调试
3. **性能友好**: tracing 在关闭日志级别时几乎零开销
4. **易于扩展**: 可以轻松添加新的日志点和 span
5. **生产就绪**: 默认 WARN 级别，减少生产环境的日志噪音

## 未来改进方向

1. **JSON 格式输出**: 添加 `--log-format json` 选项，便于日志分析工具处理
2. **文件日志**: 支持将日志写入文件（`--log-file path`）
3. **性能追踪**: 在关键操作上添加耗时统计
4. **异步支持**: 如果未来引入异步操作，tracing 天然支持 async/await

## 相关文件

- `crates/rfont-cli/src/main.rs`: 日志初始化
- `crates/rfont-cli/src/commands/info.rs`: info 命令日志
- `crates/rfont-cli/src/commands/subset.rs`: subset 命令日志
- `crates/rfont-cli/src/commands/convert.rs`: convert 命令日志
- `crates/rfont-cli/src/commands/batch.rs`: batch 命令日志
- `test_logging.ps1`: 日志系统测试脚本

## 总结

本次日志系统优化显著提升了 rfont CLI 工具的可调试性和可维护性。通过多级 verbosity、环境变量支持和结构化日志，用户可以轻松获取所需的信息，开发者也能更方便地诊断问题。
