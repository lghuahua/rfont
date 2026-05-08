# rfont - Rust 字体处理库

[![CI](https://github.com/lghuahua/rfont/actions/workflows/ci.yml/badge.svg)](https://github.com/lghuahua/rfont/actions/workflows/ci.yml)
[![Release](https://github.com/lghuahua/rfont/actions/workflows/release.yml/badge.svg)](https://github.com/lghuahua/rfont/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)

一个基于 Rust 开发的字体解析、处理和子集化工具库，支持 TTF 和 WOFF 格式。

## 功能特性

- ✅ **多格式支持**：支持 TrueType (TTF)、OpenType (OTF/CFF)、WOFF 和 WOFF2
- ✅ **核心表解析**：完整解析 cmap, glyf, head, hhea, hmtx, loca, maxp 等关键表
- ✅ **字体子集化**：根据指定文字生成精简的字体文件
- ✅ **校验和计算**：符合 OpenType 规范的校验和与全局调整
- ✅ **坐标压缩**：使用相对增量编码优化字形数据体积
- ✅ **格式检测增强**：详细的格式信息、可变字体识别、必需表验证

## 快速开始

### 加载字体

```rust
use rfont::Font;

// 自动检测格式（支持 .ttf 和 .woff）
let font = Font::load("path/to/font.ttf")?;
// 或
let font = Font::load("path/to/font.woff")?;
```

### 提取字形并生成子集字体

```rust
let text = "月到风来";

// 获取文字对应的 GlyphID 列表
let glyph_ids = font.get_glyph_ids_for_text(text);

// 生成子集字体
let subset_data = font.subset_and_serialize(&glyph_ids)?;

// 保存为新文件
std::fs::write("subset.ttf", &subset_data)?;
```

### 格式检测（无需加载完整字体）

```rust
use rfont::Font;
use rfont_types::{FontFormat, CompressionType};

// 读取字体文件数据
let data = std::fs::read("font.ttf")?;

// 检测格式
let format_info = Font::detect_format(&data)?;

println!("格式: {}", format_info.format);           // TrueType (TTF)
println!("版本: {}", format_info.version);           // 0x00010000
println!("可变字体: {}", format_info.is_variable);   // false

if let Some(compression) = format_info.compression {
    println!("压缩: {}", compression);               // Zlib 或 Brotli
}

// 验证必需表
let all_tables: Vec<String> = format_info.required_tables
    .iter()
    .chain(format_info.optional_tables.iter())
    .cloned()
    .collect();

if format_info.has_required_tables(&all_tables) {
    println!("✅ 所有必需表都存在");
} else {
    let missing = format_info.missing_required_tables(&all_tables);
    println!("⚠️ 缺失表: {:?}", missing);
}
```

## 项目结构

```
rfont/
├── crates/
│   ├── rfont-types/      # 基础类型定义和 IO Trait
│   ├── rfont-core/       # 表解析和序列化逻辑
│   └── rfont/            # 高层 API 和应用逻辑
└── font_macros/          # 自定义过程宏
```

## 支持的格式

### TTF (TrueType Font)
- 直接解析 SFNT 结构
- 完整支持简单字形和复合字形

### WOFF (Web Open Font Format)
- 自动检测 "wOFF" 签名
- 使用 flate2 解压缩 zlib 数据
- 转换为标准 SFNT 结构后处理

### WOFF2 (Web Open Font Format 2)
- 自动检测 "wOF2" 签名
- 使用 brotli 解压缩（更高压缩率）
- 支持可变字体（Variable Fonts）
- 表目录使用变长编码优化

## 运行示例

```bash
# 运行子集化测试
cargo run --example test_alimama -p rfont

# 这将：
# 1. 加载 src/AlimamaDaoLiTi.ttf 或 .woff
# 2. 提取指定文字
# 3. 生成子集字体文件

# 运行格式检测示例
cargo run --example format_detection_demo -p rfont

# 这将：
# 1. 检测 TTF/WOFF/WOFF2 三种格式
# 2. 显示详细的格式信息
# 3. 验证必需表是否存在
```

## 测试

项目包含完整的单元测试套件，覆盖基础类型、表解析和高层 API。

```bash
# 运行所有测试
cargo test --workspace

# 运行特定 crate 的测试
cargo test -p rfont-types
cargo test -p rfont-core
cargo test -p rfont

# 查看详细输出
cargo test -- --nocapture
```

详细测试文档请参考 [TESTING.md](./TESTING.md)。

当前测试覆盖：
- ✅ **177 个单元测试**，全部通过
- ✅ 覆盖 cmap、glyf、head、hmtx、maxp、loca、woff、woff2 等核心表的解析
- ✅ 验证校验和计算、字形去重、格式检测等业务逻辑
- ✅ 核心解析逻辑覆盖率 > 80%（glyf: 95%, hmtx: 100%, maxp: 98%, loca: 100%）
- ✅ rfont-types 模块覆盖率 > 69%（io: 100%, format: 100%, error: 93%, primitives: 80%）
- ✅ rfont-core 模块覆盖率 > 64%（woff: 93%, woff2: 87%, loca: 100%, cmap: 57%）
- ✅ **rfont 主库子集化测试完善**（cmap: 96%, hmtx: 100%, maxp: 100%, head: 93%, glyf_loca: 81%, post: 90%）
- ✅ **Reader/Writer 边界条件测试完善**（空数据、精确边界、offset 跟踪、混合读取、数组读取）
- ✅ **整体代码覆盖率 64.85%**，接近 70% 目标

📊 **代码覆盖率报告**: [COVERAGE_REPORT.md](./COVERAGE_REPORT.md) | [HTML 报告](./coverage/tarpaulin-report.html)

## 命令行工具

### 安装 CLI 工具

```bash
cargo install --path crates/rfont-cli
```

### 基本用法

```bash
# 查看字体信息
rfont info font.ttf

# 创建字体子集
rfont subset font.ttf --text "Hello World" -o subset.ttf

# 格式转换
rfont convert font.ttf --format woff -o font.woff

# 批量转换
rfont batch convert "*.ttf" --format woff --format woff2
```

### 日志控制

CLI 工具支持多级日志输出，方便调试和监控：

```bash
# 默认级别（只显示警告和错误）
rfont info font.ttf

# INFO 级别（显示操作摘要）
rfont -v info font.ttf

# DEBUG 级别（显示详细信息）
rfont -vv subset font.ttf --text "Hello"

# TRACE 级别（显示追踪信息）
rfont -vvv info font.ttf

# 使用环境变量覆盖
RUST_LOG=debug rfont info font.ttf
RUST_LOG=error rfont subset font.ttf --text "Hello"
```

**日志级别说明**：
- `WARN` (默认)：只显示警告和错误信息
- `INFO` (-v)：显示操作摘要和统计信息
- `DEBUG` (-vv)：显示详细的处理过程和中间状态
- `TRACE` (-vvv)：显示最详细的追踪信息

**结构化日志特性**：
- ✅ Span 追踪关键操作的生命周期
- ✅ 带上下文的字段（size, count, format 等）
- ✅ ANSI 颜色输出（自动检测终端支持）
- ✅ 简洁格式（无时间戳、无模块名）

---

## 技术细节

### 校验和计算
- 每个表独立计算 32 位校验和
- `head.checkSumAdjustment` 使用公式：`0xB1B0AFBA - sum(all_tables)`

### 坐标序列化
- 使用相对增量编码（Delta Encoding）
- 标志位压缩（Flags Compression）
- 小数值使用单字节存储

## 依赖

- `flate2`: WOFF 解压缩
- `chrono`: 时间戳处理
- `encoding_rs`: 字符串编码转换

## License

MIT

## Changelog

查看完整的版本变更历史：[CHANGELOG.md](CHANGELOG.md)

CHANGELOG 采用自动化生成，基于 [Conventional Commits](https://www.conventionalcommits.org/) 规范。

**自动生成**:
```bash
# Windows PowerShell
.\scripts\generate-changelog.ps1

# Linux/Mac
./scripts/generate-changelog.sh
```

详见 [scripts/README.md](scripts/README.md) 和 [CHANGELOG_GUIDE.md](CHANGELOG_GUIDE.md)。

## CI/CD

本项目使用 GitHub Actions 实现自动化 CI/CD：

- **CI**: 多平台测试、代码质量检查、覆盖率报告
- **CD**: 自动生成 CHANGELOG、构建多平台 CLI、创建 GitHub Release

详细配置请参考 [CICD_GUIDE.md](CICD_GUIDE.md)。

---

*最后更新: 2026-05-08*
