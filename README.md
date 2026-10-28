# rfont - 字体子集化命令行工具

[![CI](https://github.com/lghuahua/rfont/actions/workflows/ci.yml/badge.svg)](https://github.com/lghuahua/rfont/actions/workflows/ci.yml)
[![Release](https://github.com/lghuahua/rfont/actions/workflows/release.yml/badge.svg)](https://github.com/lghuahua/rfont/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)
[![Web Demo](https://img.shields.io/badge/%E5%9C%A8%E7%BA%BF%E4%BD%93%E9%AA%8C-Web%20Demo-brightgreen)](https://lghuahua.github.io/rfont/)
[![Deploy Web](https://github.com/lghuahua/rfont/actions/workflows/deploy-web.yml/badge.svg)](https://github.com/lghuahua/rfont/actions/workflows/deploy-web.yml)

一个基于 Rust 开发的高性能字体子集化和转换命令行工具，支持 TTF、TTC、WOFF 和 WOFF2 格式。

## 在线体验

无需安装，直接在浏览器中体验字体信息查看、格式转换与子集化（基于本项目编译的 WebAssembly，字体文件仅在本地浏览器处理，不会上传到服务器）：

👉 **https://lghuahua.github.io/rfont/**

## 功能特性

- ✅ **多格式支持**：支持 TrueType (TTF)、TTC 字体集合、WOFF 和 WOFF2
- ✅ **字体集合**：列出 TTC 中的字面、按索引提取，或用 `--face` 指定字面处理；网页版与桌面版可在界面中直接切换字面
- ✅ **智能子集化**：根据指定文字生成精简的字体文件，大幅减小体积
- ✅ **格式转换**：在 TTF、WOFF、WOFF2 之间互相转换
- ✅ **批量处理**：支持批量转换多个字体文件
- ✅ **详细信息**：查看格式、名称记录（家族/样式/作者/版权等）、样式与度量、表结构
- ✅ **SVG 导出**：把文本或单个字形按字体轮廓导出为 SVG，三端均支持
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
  格式:         TrueType (TTF)
  字形数量:     7044
  Units per EM: 1000
  支持的字符:   65535
  边界框:       [-32, -260, 1924, 940]

名称信息:
  家族名称:     Alibaba PuHuiTi
  样式:         Regular
  完整名称:     Alibaba PuHuiTi Regular
  PostScript:   AlibabaPuHuiTi-Regular
  版本:         Version 1.000
  设计者:       Alibaba
  制造商:       Alibaba
  版权:         Copyright ...
  许可证:       ...

样式与度量:
  字重:         400
  字宽:         5
  嵌入权限:     0x0008
  Ascender:     880
  Descender:    -120
  Line Gap:     0
  HMetrics:     7044
```

使用 `-V` 额外输出全部 name 记录与表列表：

```bash
rfont info font.ttf --verbose-info
```

### 2. 处理 TTC 字体集合

```bash
# 列出集合中的字面
rfont ttc list collection.ttc

# JSON 输出
rfont ttc list collection.ttc --json

# 提取指定字面为独立字体（默认输出 collection.face1.ttf）
rfont ttc extract collection.ttc --face 1 -o face1.ttf

# 直接查看 / 子集化 / 转换集合中的某个字面
rfont info collection.ttc --face 1
rfont subset collection.ttc --face 1 --text "你好世界" -o subset.ttf
rfont convert collection.ttc --face 1 --format woff2 -o face1.woff2
```

`--face` 从 0 开始计数；对普通单字体文件保持默认 `0` 即可。

网页版与桌面版同样支持 TTC：加载集合后会在字体信息卡片中显示“字面”选择器，
切换后预览、子集化与格式转换都针对所选字面执行。

### 3. 创建字体子集

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

### 4. 格式转换

```bash
# TTF → WOFF
rfont convert font.ttf --format woff -o font.woff

# TTF → WOFF2
rfont convert font.ttf --format woff2 -o font.woff2

# WOFF → TTF
rfont convert font.woff --format ttf -o font.ttf
```

### 5. 批量转换

```bash
# 批量转换目录下所有 TTF 为 WOFF2
rfont batch convert "fonts/*.ttf" --format woff2

# TTC 默认处理第 0 个字面，可用 --face 指定
rfont batch convert "fonts/*.ttc" --face 1 --format woff2

# 同时转换为多种格式
rfont batch convert "*.ttf" --format woff --format woff2

# 指定输出目录
rfont batch convert "*.ttf" --format woff2 --output-dir web-fonts/
```

### 6. 修改字体元信息

```bash
# 修改家族名与作者（默认输出 font_edited.ttf）
rfont edit font.ttf --family "My Font" --designer "张三"

# 指定完整名称、PostScript 名称与版权
rfont edit font.ttf --full-name "My Font Bold" \
  --postscript-name "MyFont-Bold" \
  --copyright "(c) 2026 Example"

# 修改后直接输出 WOFF2
rfont edit font.ttf --family "Web Font" --format woff2 -o web.woff2

# 修改 TTC 集合中的某个字面
rfont edit collection.ttc --face 1 --family "Extracted Family"
```

只改写 `name` 表并刷新 `head.modified`，字形、度量与布局保持不变。
修改家族名或样式名时会自动推导完整名称与 PostScript 名称，无法推导时保留原值；
显式指定的 PostScript 名称会按 OpenType 规则校验（ASCII 可见字符、不超过 63 个字符）。

### 7. 授权限制提示

字体文件里的 `OS/2.fsType` 与 name 表的许可证记录（nameID 13/14）能反映作者的授权声明。
子集化或改名时会先检查这些声明并打印提示，但**不阻止操作**——声明只代表作者填写，实际授权以随字体分发的协议文件为准：

- `subset`：声明禁止子集化（bit 8）、禁止嵌入（bit 1）、仅位图嵌入（bit 9）或仅预览打印（bit 2）时提示
- `edit`：许可证声明了保留字体名（Reserved Font Name），或 `fsType` 禁止嵌入 / 仅预览打印时提示

```bash
# 提示输出到 stderr；CI 中可用 --strict 把提示升级为错误退出
rfont subset font.ttf --text "Hello" --strict
rfont edit font.ttf --family "My Font" --strict
```

`info` 命令会展示嵌入权限、识别出的许可证类型与许可证网址；`--json` 输出中对应 `fs_type`、`license_url` 与 `restrictions` 字段（含子集化 / 改名的提示文案）。Web 与桌面端在加载字体后同样会显示授权提示。

### 8. 导出 SVG

```bash
# 按字体轮廓导出文本（默认 64px、黑色、4px 留白）
rfont svg font.ttf --text "Hello" -o hello.svg

# 导出单个字形
rfont svg font.ttf --glyph 36 -o glyph36.svg

# 指定字号、颜色与留白
rfont svg font.ttf --text "字体" --size 128 --color "#1f6feb" --padding 8 -o font.svg
```

SVG 的 `viewBox` 使用像素单位，`--size` 只决定默认显示大小，路径按 `unitsPerEm` 缩放，放大不会失真。
文本按 `hmtx` 的前进宽度排成一行，不做字距调整（kerning），也不处理阿拉伯文、印度系文字的重排——
需要复杂排版时请先用排版引擎生成字形序列，再用 `--glyph` 逐个导出。

只支持 TrueType 轮廓（`glyf`），CFF 字体导出时会返回错误。网页版与桌面端在预览区右上角提供
“导出 SVG”按钮，按当前预览字号导出输入框中的文本。

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

| 格式 | 信息查看 | 格式转换 | 子集化 | 导出 SVG |
| --- | --- | --- | --- | --- |
| TTF（TrueType 轮廓，`glyf`/`loca`） | ✅ | ✅ | ✅ | ✅ |
| WOFF（TrueType 轮廓） | ✅ | ✅ | ✅ | ✅ |
| WOFF2（TrueType 轮廓） | ✅ | ✅ | ✅ | ✅ |
| TTC/OTC（TrueType 轮廓） | ✅（`--face`） | ✅（`--face`） | ✅（`--face`） | ✅（`--face`） |
| OTF（CFF 轮廓） | ❌ | ❌ | ❌ | ❌ |
| TTC/OTC（CFF 轮廓） | ❌ | ❌ | ❌ | ❌ |

### 已知限制

- 只处理 TrueType 轮廓（`glyf`/`loca`）。CFF 轮廓的 OTF 及其 TTC 集合暂不支持，会在缺少 `loca` 表时返回错误。
- TTC 只支持按字面单独处理，不会直接输出新的集合文件。
- WOFF2 使用 Brotli 压缩；文本类子集建议优先输出 WOFF2，体积更小。


## 开发

### 项目结构

```
rfont/
├── crates/
│   ├── rfont-types/      # 基础类型定义和 IO Trait
│   ├── rfont-core/       # 表解析和序列化逻辑
│   ├── rfont/            # 高层 API 和子集化逻辑
│   ├── rfont-cli/        # 命令行工具
│   └── rfont-macros/     # 自定义过程宏
├── rfont-desktop/        # 桌面应用程序（Tauri）
└── rfont-web/            # WASM 绑定 + Web 前端（www），部署到 GitHub Pages
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

# 构建 Web（WASM + 前端生产包，需要 wasm-pack 与 pnpm）
just build-web

# 运行测试
cargo test --workspace

# 代码检查
cargo clippy --workspace
cargo fmt --all
```

### Web 在线体验（rfont-web）

`rfont-web` 将核心库编译为 WebAssembly，配合 `rfont-web/www` 下的 Vue 前端提供浏览器端在线体验，部署在 GitHub Pages。字体文件仅在本地浏览器处理，不会上传服务器。

```bash
# 本地开发（构建 WASM 并启动 dev server）
just web-dev

# 生产构建与本地预览
just build-web
just web-preview
```

推送到 `main` 且改动 `crates/**` 或 `rfont-web/**` 时，`deploy-web.yml` 会自动构建并发布到 GitHub Pages。

> 首次使用需在仓库 **Settings → Pages → Build and deployment → Source** 选择 **GitHub Actions**。

### 依赖

- `flate2`: WOFF 解压缩
- `brotli`: WOFF2 解压缩
- `tracing`: 结构化日志系统

## 许可证

MIT


---
