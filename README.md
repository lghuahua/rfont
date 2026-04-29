# rfont - Rust 字体处理库

一个基于 Rust 开发的字体解析、处理和子集化工具库，支持 TTF 和 WOFF 格式。

## 功能特性

- ✅ **多格式支持**：支持 TrueType (TTF) 和 Web Open Font Format (WOFF)
- ✅ **核心表解析**：完整解析 cmap, glyf, head, hhea, hmtx, loca, maxp 等关键表
- ✅ **字体子集化**：根据指定文字生成精简的字体文件
- ✅ **校验和计算**：符合 OpenType 规范的校验和与全局调整
- ✅ **坐标压缩**：使用相对增量编码优化字形数据体积

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

**注意**：WOFF2 格式目前尚未支持（需要 brotli 解压缩）。

## 运行示例

```bash
# 运行子集化测试
cargo run --example test_alimama -p rfont

# 这将：
# 1. 加载 src/AlimamaDaoLiTi.ttf 或 .woff
# 2. 提取指定文字
# 3. 生成子集字体文件
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
- ✅ **20 个单元测试**，全部通过
- ✅ 覆盖 cmap、head、maxp 等核心表的解析
- ✅ 验证校验和计算、字形去重等业务逻辑

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
