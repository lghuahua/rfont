# 结构化错误处理实现文档

## 📋 概述

rfont 项目现已采用结构化的错误处理机制，使用 `thiserror` crate 提供类型安全、信息丰富的错误类型。

---

## 🎯 核心改进

### 1. FontError 枚举定义

```rust
#[derive(Debug, Error)]
pub enum FontError {
    #[error("Invalid magic number: expected {expected:#010X}, got {actual:#010X}")]
    InvalidMagicNumber { expected: u32, actual: u32 },

    #[error("Table '{tag}' not found in font")]
    TableNotFound { tag: String },

    #[error("Invalid offset in table '{table}': offset {offset} exceeds maximum {max}")]
    InvalidOffset { table: String, offset: u32, max: u32 },

    #[error("Unsupported cmap format: {format}")]
    UnsupportedCmapFormat { format: u16 },

    #[error("Invalid table checksum: table '{tag}' expected {expected:#010X}, got {actual:#010X}")]
    InvalidChecksum { tag: String, expected: u32, actual: u32 },

    #[error("Unexpected end of data at offset {offset}, needed {needed} bytes")]
    UnexpectedEndOfData { offset: usize, needed: usize },

    #[error("Invalid glyph index: {glyph_id} exceeds maximum {max_glyphs}")]
    InvalidGlyphIndex { glyph_id: u16, max_glyphs: u16 },

    #[error("Invalid units per em: {value} (must be between 16 and 16384)")]
    InvalidUnitsPerEm { value: u16 },

    #[error("WOFF decompression failed: {message}")]
    WoffDecompressionError { message: String },

    #[error("Invalid base date for LONGDATETIME calculation")]
    InvalidBaseDate,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Generic error: {0}")]
    Generic(String),
}
```

---

## 💡 使用示例

### 模式匹配错误

```rust
use rfont::Font;
use rfont_types::FontError;

fn load_font_safely(path: &str) -> Result<Font, FontError> {
    match Font::load(path) {
        Ok(font) => Ok(font),
        Err(FontError::Io(e)) => {
            eprintln!("文件读取失败: {}", e);
            eprintln!("提示: 请检查文件路径是否正确");
            Err(FontError::Io(e))
        }
        Err(FontError::InvalidMagicNumber { expected, actual }) => {
            eprintln!("无效的字体格式");
            eprintln!("  期望魔数: {:#010X}", expected);
            eprintln!("  实际魔数: {:#010X}", actual);
            Err(FontError::InvalidMagicNumber { expected, actual })
        }
        Err(e) => {
            eprintln!("其他错误: {}", e);
            Err(e)
        }
    }
}
```

### CLI 中的友好错误提示

```rust
fn main() {
    let font_path = "font.ttf";
    
    match Font::load(font_path) {
        Ok(font) => {
            println!("✅ 字体加载成功！");
            println!("   字形数量: {}", font.maxp.num_glyphs);
        }
        Err(FontError::Io(_)) => {
            eprintln!("❌ 错误: 无法找到文件 '{}'", font_path);
            eprintln!("💡 提示: 请确认文件路径正确且文件存在");
            std::process::exit(1);
        }
        Err(FontError::TableNotFound { tag }) => {
            eprintln!("❌ 错误: 字体中缺少 '{}' 表", tag);
            eprintln!("💡 提示: 该字体可能已损坏或不是有效的 OpenType 字体");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("❌ 错误: {}", e);
            std::process::exit(1);
        }
    }
}
```

### 桌面应用中的错误展示

```typescript
// Tauri Rust 后端
#[tauri::command]
async fn load_font(path: String) -> Result<FontInfo, String> {
    match Font::load(&path) {
        Ok(font) => Ok(FontInfo::from(&font)),
        Err(FontError::Io(e)) => Err(format!("文件读取失败: {}", e)),
        Err(FontError::InvalidMagicNumber { .. }) => {
            Err("文件格式不支持，仅支持 TTF 和 WOFF 格式".to_string())
        }
        Err(e) => Err(format!("字体加载失败: {}", e)),
    }
}

// React 前端
const loadFont = async (path: string) => {
    try {
        const info = await invoke('load_font', { path });
        setFontInfo(info);
    } catch (error) {
        showErrorNotification(error); // 显示友好的错误提示
    }
};
```

---

## 📊 优势对比

### 之前（字符串错误）

```rust
// ❌ 缺点：难以区分错误类型，无法模式匹配
Err(FontError("Table head not found".to_string()))

match result {
    Err(FontError(msg)) => {
        if msg.contains("not found") {
            // 脆弱的字符串匹配
        }
    }
}
```

### 现在（结构化错误）

```rust
// ✅ 优点：类型安全，易于匹配，信息丰富
Err(FontError::TableNotFound { tag: "head".to_string() })

match result {
    Err(FontError::TableNotFound { tag }) => {
        // 类型安全的模式匹配
        println!("缺少表: {}", tag);
    }
    _ => {}
}
```

---

## 🔧 技术细节

### 依赖添加

```toml
# crates/rfont-types/Cargo.toml
[dependencies]
thiserror = "1.0"
```

### 自动 Trait 实现

`thiserror` 自动为 `FontError` 实现：
- ✅ `std::fmt::Display` - 人类可读的错误消息
- ✅ `std::error::Error` - 标准错误 trait
- ✅ `std::fmt::Debug` - 调试输出
- ✅ `From<std::io::Error>` - IO 错误自动转换

### 向后兼容

保留了通用错误构造方法：

```rust
// 仍然可以使用通用错误（向后兼容）
FontError::new("自定义错误消息")
FontError::Generic("通用错误".to_string())
```

---

## 📈 影响范围

### 修改的文件

| 文件 | 修改内容 | 行数变化 |
|------|---------|---------|
| `crates/rfont-types/Cargo.toml` | 添加 thiserror 依赖 | +1 |
| `crates/rfont-types/src/io.rs` | 定义 FontError 枚举 | +75 |
| `crates/rfont-types/src/primitives.rs` | 更新 LONGDATETIME 错误 | ~2 |
| `crates/rfont-core/src/tables/cmap.rs` | 更新 cmap 格式错误 | ~1 |
| `crates/rfont-core/src/tables/woff.rs` | 更新 WOFF 签名错误 | ~3 |
| `crates/rfont/src/lib.rs` | 批量更新错误构造 | ~15 |

**总计**: 6 个文件，~97 行代码变更

### 测试覆盖

- ✅ 所有 20 个单元测试通过
- ✅ 编译无警告
- ✅ 向后兼容现有代码

---

## 🚀 下一步优化

基于结构化错误处理，可以继续实现：

1. **错误上下文链** - 使用 `source()` 方法追踪错误源头
2. **错误恢复策略** - 根据错误类型提供自动修复建议
3. **日志集成** - 将错误直接记录到 tracing 系统
4. **国际化支持** - 为不同语言提供本地化错误消息

---

*完成时间: 2026-04-29*  
*作者: rfont 开发团队*
