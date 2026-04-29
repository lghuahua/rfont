# rfont CLI - 命令行字体子集化和转换工具

`rfont` 是一个功能强大的命令行工具，用于字体子集化、格式转换和信息查询。

## 📦 安装

```bash
# 从源码构建
cd f:\rfont
cargo build --package rfont-cli --release

# 可执行文件位于 target/release/rfont.exe (Windows) 或 target/release/rfont (Linux/macOS)
```

## 🚀 快速开始

### 查看字体信息

```bash
# 基本字体信息
rfont info font.ttf

# 详细信息（包括所有表）
rfont info font.ttf -v

# JSON 格式输出
rfont info font.ttf --json
```

### 创建字体子集

```bash
# 从文本创建子集
rfont subset font.ttf --text "你好世界" -o subset.ttf

# 从文件读取字符
rfont subset font.ttf --text-file characters.txt -o subset.ttf

# 使用 Unicode 范围
rfont subset font.ttf --range "U+4E00-U+9FFF" -o subset.ttf

# 优化 post 表并转换为 WOFF
rfont subset font.ttf --text "阿里巴巴" --strip-post-names --format woff -o subset.woff

# 指定压缩级别
rfont subset font.ttf --text "测试" --format woff --compression 9 -o subset.woff
```

### 转换字体格式

```bash
# TTF → WOFF
rfont convert font.ttf --format woff -o font.woff

# WOFF → TTF
rfont convert font.woff --format ttf -o font.ttf

# 指定压缩级别
rfont convert font.ttf --format woff --compression 9 -o font.woff
```

## 📖 命令详解

### `info` - 显示字体信息

显示字体的基本信息，包括字形数量、Units per EM、边界框等。

**选项：**
- `<FONT>`: 字体文件路径（必需）
- `-v, --verbose`: 显示详细信息（包括所有表）
- `--json`: 以 JSON 格式输出

**示例：**
```bash
rfont info AlimamaDaoLiTi.ttf
rfont info AlimamaDaoLiTi.ttf -v
rfont info AlimamaDaoLiTi.ttf --json | jq '.glyph_count'
```

### `subset` - 创建字体子集

从完整字体中提取指定的字符，生成更小的子集字体。

**参数：**
- `<INPUT>`: 输入字体文件路径（必需）

**选项：**
- `-o, --output <OUTPUT>`: 输出文件路径（可选，默认为 `{name}_subset.ttf`）
- `-t, --text <TEXT>`: 要包含的文本
- `--text-file <FILE>`: 从文件读取字符列表
- `--range <RANGE>`: Unicode 范围（例如：U+4E00-U+9FFF），可多次使用
- `--strip-post-names`: 优化 post 表（移除字形名称，减小文件大小）
- `--format <FORMAT>`: 输出格式（ttf 或 woff，默认：ttf）
- `--compression <LEVEL>`: WOFF 压缩级别（0-9，默认：6）

**示例：**
```bash
# 从文本创建子集
rfont subset font.ttf --text "Hello World" -o hello.ttf

# 合并多个来源
rfont subset font.ttf \
  --text "常用字" \
  --text-file extra_chars.txt \
  --range "U+4E00-U+9FFF" \
  -o chinese_subset.ttf

# Web 优化（最小文件大小）
rfont subset font.ttf \
  --text "阿里巴巴妈妈" \
  --strip-post-names \
  --format woff \
  --compression 9 \
  -o web_optimized.woff
```

### `convert` - 转换字体格式

在 TTF 和 WOFF 格式之间转换，保留所有字形。

**参数：**
- `<INPUT>`: 输入字体文件路径（必需）

**选项：**
- `-o, --output <OUTPUT>`: 输出文件路径（可选，默认为 `{name}_converted.{ext}`）
- `-f, --format <FORMAT>`: 目标格式（ttf 或 woff，必需）
- `--compression <LEVEL>`: WOFF 压缩级别（0-9，默认：6）

**示例：**
```bash
# TTF 转 WOFF
rfont convert font.ttf --format woff -o font.woff

# WOFF 转 TTF
rfont convert font.woff --format ttf -o font.ttf

# 高压缩比
rfont convert font.ttf --format woff --compression 9 -o font.woff
```

## 💡 使用技巧

### 1. 批量处理

可以使用脚本批量处理多个字体：

```bash
# Bash 示例
for font in *.ttf; do
  rfont subset "$font" --text "阿里巴巴" -o "subset_${font}"
done
```

```powershell
# PowerShell 示例
Get-ChildItem *.ttf | ForEach-Object {
  rfont subset $_.FullName --text "阿里巴巴" -o "subset_$($_.Name)"
}
```

### 2. 从文件读取字符

创建一个文本文件 `chars.txt`，包含所有需要的字符：

```
阿里巴巴妈妈
你好世界
测试文字
```

然后使用：
```bash
rfont subset font.ttf --text-file chars.txt -o subset.ttf
```

### 3. Unicode 范围

常用的 Unicode 范围：

- **CJK 统一汉字**: `U+4E00-U+9FFF`
- **拉丁字母**: `U+0041-U+005A` (大写), `U+0061-U+007A` (小写)
- **数字**: `U+0030-U+0039`
- **标点符号**: `U+3000-U+303F` (CJK 标点)

可以组合多个范围：
```bash
rfont subset font.ttf \
  --range "U+4E00-U+9FFF" \
  --range "U+0041-U+005A" \
  --range "U+0061-U+007A" \
  -o mixed_subset.ttf
```

### 4. 性能优化

对于 Web 使用，推荐以下配置：

```bash
rfont subset font.ttf \
  --text "你的文本" \
  --strip-post-names \
  --format woff \
  --compression 9 \
  -o web_font.woff
```

这可以显著减小文件大小（通常减少 90% 以上）。

## 📊 输出示例

### Info 命令输出

```
📝 字体信息
────────────────────────────────────────────────────────────

基本信息:
  字形数量:     7044
  Units per EM: 1000
  边界框:       [-32, -260, 1924, 940]

水平度量:
  Ascender:     880
  Descender:    -120
  Line Gap:     64

字体表:
  glyf     offset=268      size=4.7 MB     checksum=0x3797C030
  cmap     offset=5028108  size=28.3 KB    checksum=0x338925C3
  ...

  总计: 16 个表

字符支持:
  支持的 Unicode 字符数: 7019
```

### Subset 命令输出

```
📖 加载字体...
  ✓ 成功加载字体
  字形总数: 7044

🔤 处理文本...
  文本长度: 4 个字符
⠁ [00:00:00] [========================================] 100% 完成！

💾 保存文件...
  ✓ 文件已保存: "test_subset.ttf"

📊 统计信息:
  原始大小:   4.9 MB
  子集大小:   3.3 KB
  压缩率:     0.1%
  节省空间:   4.9 MB

✨ 完成！
```

## ⚙️ 高级选项

### 详细模式

添加 `-v` 或 `--verbose` 标志启用调试输出：

```bash
rfont -v info font.ttf
rfont -v subset font.ttf --text "测试" -o subset.ttf
```

### 帮助信息

```bash
# 主帮助
rfont --help

# 子命令帮助
rfont info --help
rfont subset --help
rfont convert --help
```

## 🐛 常见问题

### Q: 为什么子集化后文件反而变大了？

A: 如果选择的字符太多，子集可能不会比原文件小很多。尝试只选择真正需要的字符。

### Q: WOFF 和 WOFF2 有什么区别？

A: WOFF2 使用 Brotli 压缩，通常比 WOFF 小 30%。当前版本仅支持 WOFF，WOFF2 支持正在开发中。

### Q: 如何检查子集字体是否包含某个字符？

A: 使用 `info` 命令查看支持的字符列表：
```bash
rfont info subset.ttf -v
```

### Q: 可以批量处理多个字体吗？

A: 是的，使用 shell 脚本或批处理文件循环处理。参见上面的"批量处理"部分。

## 📝 许可证

本项目采用 MIT 许可证。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 🔗 相关链接

- [项目主页](https://github.com/yourusername/rfont)
- [核心库文档](../README.md)
- [性能优化报告](../PERFORMANCE_OPTIMIZATION.md)
