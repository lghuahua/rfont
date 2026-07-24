# rfont - 字体子集化命令行工具

[![CI](https://github.com/lghuahua/rfont/actions/workflows/ci.yml/badge.svg)](https://github.com/lghuahua/rfont/actions/workflows/ci.yml)
[![Release](https://github.com/lghuahua/rfont/actions/workflows/release.yml/badge.svg)](https://github.com/lghuahua/rfont/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)

一个基于 Rust 开发的高性能字体子集化和转换命令行工具，支持 TTF、WOFF 和 WOFF2 格式。

## 功能特性

- ✅ **多格式支持**：支持 TrueType (TTF)、WOFF 和 WOFF2
- ✅ **智能子集化**：根据指定文字生成精简的字体文件，大幅减小体积
- ✅ **格式转换**：在 TTF、WOFF、WOFF2 之间互相转换
- ✅ **批量处理**：支持批量转换多个字体文件
- ✅ **详细信息**：查看字体的完整信息和表结构
- ✅ **高性能**：基于 Rust 编译，速度快，内存占用低

## 快速开始

### 安装

#### 从源码编译

```bash
git clone https://github.com/lghuahua/rfont.git
cd rfont
cargo install --path crates/rfont-cli
```

#### 下载预编译版本

从 [Releases](https://github.com/lghuahua/rfont/releases) 页面下载对应平台的预编译二进制文件。

## 使用指南

### 1. 查看字体信息

```bash
# 查看字体基本信息
rfont info font.ttf

# 启用详细日志
rfont -vv info font.ttf
```

输出示例：
```
📝 字体信息
────────────────────────────────────────────

基本信息:
  字形数量:     7044
  Units per EM: 1000
  边界框:       [-32, -260, 1924, 940]

水平度量:
  Ascender:     880
  Descender:    -120
  Line Gap:     0

字符支持:
  支持的 Unicode 字符数: 65535

表信息:
  head, hhea, maxp, cmap, glyf, loca, hmtx, post, name, OS/2
```

### 2. 创建字体子集

```bash
# 基本用法
rfont subset font.ttf --text "Hello World" -o subset.ttf

# 从文件读取文本
rfont subset font.ttf --text-file chars.txt -o subset.ttf

# 启用详细日志查看处理过程
rfont -vv subset font.ttf --text "你好世界" -o subset.ttf
```

**效果**：
- 原始字体：4.9 MB（7044 个字形）
- 子集字体：2.6 KB（5 个字形）
- 压缩率：**99.9%** 🎉

### 3. 格式转换

```bash
# TTF → WOFF
rfont convert font.ttf --format woff -o font.woff

# TTF → WOFF2
rfont convert font.ttf --format woff2 -o font.woff2

# WOFF → TTF
rfont convert font.woff --format ttf -o font.ttf
```

### 4. 批量转换

```bash
# 批量转换目录下所有 TTF 为 WOFF2
rfont batch convert "fonts/*.ttf" --format woff2

# 同时转换为多种格式
rfont batch convert "*.ttf" --format woff --format woff2

# 指定输出目录
rfont batch convert "*.ttf" --format woff2 --output-dir web-fonts/
```

## 日志控制

CLI 工具支持多级日志输出，方便调试和监控：

```bash
# 默认级别（只显示警告和错误）
rfont info font.ttf

# INFO 级别（显示操作摘要）
rfont -v subset font.ttf --text "Hello"

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

## 技术细节

### 子集化原理

1. **字形选择**：根据输入文本查找对应的字形 ID
2. **ID 重新映射**：将选中的字形重新编号（从 0 开始连续）
3. **表重建**：
   - `glyf/loca`：只保留选中的字形数据
   - `hmtx`：只保留选中字形的度量信息
   - `cmap`：Unicode → 新字形 ID 的映射
   - `maxp`：更新字形数量
   - `head`：更新校验和和时间戳
4. **校验和计算**：确保符合 OpenType 规范

### 支持的格式

#### TTF (TrueType Font)
#### WOFF (Web Open Font Format)
#### WOFF2 (Web Open Font Format 2)


## 开发

### 项目结构

```
rfont/
├── crates/
│   ├── rfont-types/      # 基础类型定义和 IO Trait
│   ├── rfont-core/       # 表解析和序列化逻辑
│   ├── rfont/            # 高层 API 和子集化逻辑
│   └── rfont-cli/        # 命令行工具
└── font_macros/          # 自定义过程宏
```

### 使用 Just 管理项目

本项目使用 [just](https://github.com/casey/just) 作为任务管理器，简化开发流程。

**安装 just**：
```bash
cargo install just
```

**常用命令**：
```bash
# 查看所有可用命令
just --list

# 快速开始（格式化 + 检查 + 测试）
just dev

# 提交前检查
just pre-commit

# 生成覆盖率报告
just coverage-html

# 查看帮助
just help
```


### 手动构建

```bash
# 构建 CLI 工具
cargo build --release -p rfont-cli

# 运行测试
cargo test --workspace

# 代码检查
cargo clippy --workspace
cargo fmt --all
```

### 依赖

- `flate2`: WOFF 解压缩
- `brotli`: WOFF2 解压缩
- `tracing`: 结构化日志系统

## 许可证

MIT

## CI/CD

本项目使用 GitHub Actions 实现自动化 CI/CD：

- **CI**: 多平台测试、代码质量检查、覆盖率报告
- **CD**: 构建多平台 CLI、和桌面应用。创建 GitHub Release

---
