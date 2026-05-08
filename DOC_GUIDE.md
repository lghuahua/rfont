# 文档生成指南

## 问题说明

在 workspace 项目中运行 `cargo doc --open` 时可能会遇到以下问题：

1. **包含依赖库的文档** - 默认会生成所有依赖库的文档
2. **文档覆盖问题** - rfont 核心库和 rfont-cli 的文档会相互覆盖

## 解决方案

### 1. 只生成 rfont 核心库文档（推荐）

```bash
# 只生成 rfont 核心库的文档，不包含依赖
cargo doc -p rfont --no-deps --open

# 或者生成 rfont 及其直接依赖的文档
cargo doc -p rfont --open
```

### 2. 只生成 rfont-cli 命令行工具文档

```bash
# 只生成 CLI 工具的文档
cargo doc -p rfont-cli --no-deps --open
```

### 3. 生成整个 workspace 的文档

```bash
# 生成所有成员的文档（包含依赖）
cargo doc --workspace --open

# 生成所有成员的文档（不包含依赖）
cargo doc --workspace --no-deps --open
```

### 4. 生成特定 crate 的文档

```bash
# rfont-types
cargo doc -p rfont-types --no-deps --open

# rfont-core
cargo doc -p rfont-core --no-deps --open

# font_macros
cargo doc -p font_macros --no-deps --open
```

## 常用参数说明

| 参数 | 说明 |
|------|------|
| `-p <package>` | 指定要生成文档的包 |
| `--no-deps` | 不生成依赖库的文档 |
| `--open` | 生成后自动在浏览器中打开 |
| `--workspace` | 为 workspace 中所有成员生成文档 |
| `--document-private-items` | 包含私有项的文档 |

## 文档位置

生成的文档位于 `target/doc/` 目录下：

- `target/doc/rfont/` - rfont 核心库文档
- `target/doc/rfont_cli/` - rfont-cli 命令行工具文档（如果存在）
- `target/doc/rfont_types/` - rfont-types 文档
- `target/doc/rfont_core/` - rfont-core 文档

## 最佳实践

### 开发时使用

```bash
# 快速查看核心库 API
cargo doc -p rfont --no-deps --open

# 查看所有公共 API（包括依赖）
cargo doc -p rfont --open
```

### 发布前检查

```bash
# 生成完整文档并检查
cargo doc --workspace --no-deps

# 检查文档警告
cargo doc --workspace --no-deps 2>&1 | Select-String "warning"
```

### 持续集成

```bash
# CI 中生成文档并检查是否有 broken links
cargo doc --workspace --no-deps
```

## 注意事项

1. **包名 vs 二进制名**：rfont-cli 的包名是 `rfont-cli`，但二进制名是 `rfont`，这会导致文档目录冲突
2. **使用 `--no-deps`**：可以避免生成大量依赖库的文档，加快生成速度
3. **清理旧文档**：如果文档有问题，可以先清理再重新生成
   ```bash
   cargo clean -p rfont
   cargo doc -p rfont --no-deps --open
   ```

## 示例工作流

```bash
# 1. 清理旧的文档
cargo clean -p rfont

# 2. 生成新的文档
cargo doc -p rfont --no-deps

# 3. 在浏览器中查看
start target/doc/rfont/index.html  # Windows
open target/doc/rfont/index.html   # macOS
xdg-open target/doc/rfont/index.html  # Linux
```

## 常见问题

### Q: 为什么 `cargo doc --open` 打开的是错误的文档？

A: 因为 workspace 中有多个包，最后生成的包会覆盖之前的。使用 `-p` 参数指定具体的包。

### Q: 如何只查看我编写的代码的文档？

A: 使用 `--no-deps` 参数，这样不会包含第三方库的文档。

### Q: 文档中包含太多内容，如何精简？

A: 
1. 使用 `--no-deps` 排除依赖
2. 使用 `-p` 指定特定的包
3. 在代码中使用 `#[doc(hidden)]` 隐藏不需要公开的实现细节
