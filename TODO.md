# rfont 项目待办事项

> **最后更新**: 2026-04-29 (Post 表优化完成)  
> **当前状态**: CLI 工具开发完成，Cargo.toml 优化完成，Post 表优化完成，准备进入桌面应用阶段

---

## 📊 项目概览

| 指标 | 数值 |
|------|------|
| 单元测试总数 | 49 (rfont: 10, rfont-core: 34, rfont-types: 5) |
| 测试通过率 | 100% ✅ |
| 编译状态 | 无警告 ✅ |
| 支持格式 | TTF, WOFF, WOFF2 |
| CLI 命令 | info, subset, convert, batch |
| 核心功能 | 懒加载、LRU 缓存、流式处理、Builder API、批量转换 |

---

## ✅ 已完成工作（历史记录）

### 核心库开发
- ✅ **模块化重构** - 拆分为 rfont-types, rfont-core, rfont, font_macros
- ✅ **结构化错误处理** - 使用 thiserror 定义 FontError
- ✅ **字体元数据 API** - get_font_info(), get_table_list(), text_to_glyph_ids() 等
- ✅ **Builder 模式 API** - FontSubsetBuilder 链式调用接口
- ✅ **性能优化** - 懒加载机制、LRU 缓存（256 条目）、流式迭代器
- ✅ **Post 表子集化优化** - 自动将 post v2 转换为 v3，移除字形名称数组，减小体积

### 测试覆盖
- ✅ **glyf 表测试** - 14 个测试用例（简单字形、复合字形、边界情况）
- ✅ **hmtx 表测试** - 7 个测试用例（水平度量、advanceWidth、lsb）
- ✅ **cmap 表测试** - Unicode 映射、格式 0/4 解析
- ✅ **WOFF2 测试** - 4 个测试用例（Header、Base128 编码、标签验证）
- ✅ **Post 表测试** - 4 个测试用例（v2→v3 转换、头部解析、大小减少验证）
- ✅ **基础类型测试** - Tag、Fixed、FWord、LongDateTime

### CLI 工具
- ✅ **info 命令** - 字体信息查询（基本元数据、表列表、JSON 输出）
- ✅ **subset 命令** - 字体子集化（文本输入、文件读取、Unicode 范围）
- ✅ **convert 命令** - 格式转换（TTF ↔ WOFF ↔ WOFF2）
- ✅ **batch 命令** - 批量处理（批量格式转换、通配符支持、进度显示）
- ✅ **用户体验** - 彩色输出、进度条、友好错误提示、压缩率统计

### 格式支持
- ✅ **TTF 读写** - 完整的 SFNT 结构解析和序列化
- ✅ **WOFF 支持** - zlib 压缩/解压缩
- ✅ **WOFF2 支持** - Brotli 压缩/解压缩（压缩率 43%）
- ✅ **Base128 编码** - WOFF2 变长整数编码实现

### Post 表优化
- ✅ **版本检测** - 自动识别 post v1.0/2.0/2.5/3.0/4.0
- ✅ **v2 → v3 转换** - 移除字形名称数组，保留所有元数据字段
- ✅ **字段完整性** - italicAngle, underlinePosition/Thickness, isFixedPitch, min/maxMemType*
- ✅ **单元测试** - 4 个测试用例验证转换正确性和往返一致性
- ✅ **体积减少** - 实际字体中可减少 90%+ 的 post 表体积（取决于字形数量）

#### 技术实现详情
```rust
// Post 表结构 (32 字节头部)
struct PostTableInfo {
    version: u32,              // 版本号
    italic_angle: i32,         // 斜体角度 (Fixed)
    underline_position: i16,   // 下划线位置
    underline_thickness: i16,  // 下划线厚度
    is_fixed_pitch: bool,      // 是否等宽
    min_mem_type42: u32,       // Type 42 最小内存
    max_mem_type42: u32,       // Type 42 最大内存
    min_mem_type1: u32,        // Type 1 最小内存
    max_mem_type1: u32,        // Type 1 最大内存
}

// 转换逻辑
if version == 0x00020000 {
    // Version 2.0: 包含字形名称数组 → 转换为 Version 3.0
    subset_post_v2_to_v3(original_post)
} else {
    // 其他版本保持原样
    Ok(original_post.to_vec())
}
```

#### 测试结果
- ✅ `test_post_header_parsing` - 头部字段解析正确
- ✅ `test_post_v2_to_v3_conversion` - 转换后长度为 32 字节
- ✅ `test_post_size_reduction` - 体积减少验证（测试数据 20%，实际字体 90%+）
- ✅ `test_post_v3_passthrough` - v3 表直接透传

#### 收益分析
- **小字体子集** (10 个字形): post 表从 ~200 bytes → 32 bytes (84% 减少)
- **中等字体子集** (100 个字形): post 表从 ~2 KB → 32 bytes (98% 减少)
- **大字体子集** (1000 个字形): post 表从 ~20 KB → 32 bytes (99.8% 减少)

**文件**: `crates/rfont/src/subset/tables/post.rs` (245 行)  
**完成时间**: 2026-04-29

### 构建优化
- ✅ **Workspace 依赖管理** - 统一版本控制，消除 11 处重复声明
- ✅ **Resolver = "2"** - Rust 2021 推荐配置
- ✅ **Release Profile** - LTO + strip + panic=abort（二进制 2.13 MB）
- ✅ **Dev Profile** - 依赖包 opt-level=2，加速编译
- ✅ **Package Metadata** - 完整的 description、license、keywords、categories

### 文档与示例
- ✅ **示例代码** - font_info_demo.rs, builder_demo.rs, error_handling_demo.rs
- ✅ **优化报告** - CARGO_TOML_OPTIMIZATION.md（411 行详细分析）
- ✅ **WOFF2 文档** - WOFF2_IMPLEMENTATION.md（技术细节、API 示例）

---

## 🔴 高优先级任务（立即执行）

### 1. CLI batch 命令实现 ✅ COMPLETED
**优先级**: 🔴 高  
**预计工作量**: 8-12 小时  
**影响范围**: `crates/rfont-cli/src/commands/batch.rs`（新建）

#### 已完成功能
- [x] **批量格式转换**
  - [x] 支持通配符匹配多个字体文件（`*.ttf`）
  - [x] 支持 TTF → WOFF/WOFF2 和 WOFF/WOFF2 → TTF
  - [x] **多格式同时转换**（一次命令转换为多种格式，如 `-f woff -f woff2`）
  - [x] 保持原始文件名，仅修改扩展名
  - [x] 支持自定义输出目录 (`--output-dir`)
  - [x] 跳过已存在的文件（默认），支持 `--overwrite` 覆盖
  
- [x] **进度显示**
  - [x] 总体进度条（MultiProgress）
  - [x] 单个文件进度条
  - [x] 实时显示处理状态

- [x] **错误处理与报告**
  - [x] 单个文件失败不影响其他文件
  - [x] 生成详细的处理报告（成功/失败统计）
  - [x] 显示压缩率和节省空间
  - [x] 列出所有失败的文件及原因

#### 测试结果
- ✅ 成功转换 3 个 TTF → WOFF2（压缩率 56.5%）
- ✅ 成功转换 3 个 TTF → WOFF（压缩率 63.9%）
- ✅ **多格式同时转换**：2 个 TTF → WOFF + WOFF2（4 个输出文件）
- ✅ `--overwrite` 保护功能正常
- ✅ 错误文件自动跳过，不影响其他文件

#### 技术实现
```bash
# 单格式批量转换
rfont batch convert "*.ttf" --format woff2 --output-dir ./output --compression 9

# 多格式同时转换（新功能）
rfont batch convert "*.ttf" -f woff -f woff2 --output-dir ./output

# 批量转换为 WOFF
rfont batch convert "test_*.ttf" --format woff --output-dir ./woff_output

# 覆盖已存在的文件
rfont batch convert "*.ttf" --format woff2 --overwrite
```

**依赖新增**: `glob = "0.3"`  
**完成时间**: 2026-04-29

---

### 2. glyf/hmtx 测试补充
**优先级**: 🔴 高  
**预计工作量**: 4-6 小时  
**影响范围**: `crates/rfont-core/src/tables/glyf.rs`, `hmtx.rs`

#### 待补充测试
虽然已有基础测试，但以下场景仍需覆盖：

- [ ] **glyf 表边界情况**
  - [ ] 超大字形（> 1000 个点）
  - [ ] 嵌套复合字形（复合中包含复合）
  - [ ] 负坐标偏移的正确性
  - [ ] 标志位重复计数的边界值
  
- [ ] **hmtx 表完整验证**
  - [ ] numberOfHMetrics < numGlyphs 的情况
  - [ ] 所有字形共享相同 advanceWidth
  - [ ] lsb 为负值的渲染正确性
  - [ ] 零宽度字形（如空格、控制字符）

**注意**: 当前测试已通过，这些是增强性测试，用于提高覆盖率到 90%+

---

## 🟡 中优先级任务（1-2 周内）

### 4. 异步支持
**优先级**: 🟡 中  
**预计工作量**: 12-16 小时  
**影响范围**: `crates/rfont/src/lib.rs`, `crates/rfont-core/src/reader.rs`

#### 功能设计
- [ ] **异步 API**
  - [ ] `Font::load_async()` - 异步加载字体文件
  - [ ] `Font::subset_async()` - 异步子集化处理
  - [ ] 使用 `tokio` 或 `async-std` 运行时
  
- [ ] **进度反馈机制**
  - [ ] 定义 `ProgressCallback` trait
    ```rust
    pub trait ProgressCallback: FnMut(u32, u32) + Send {}
    ```
  - [ ] 在子集化过程中定期调用回调（每处理 100 个字形）
  - [ ] 支持取消操作（通过返回 `Result<(), Cancelled>`）
  
- [ ] **Feature 标志**
  - [ ] 将异步支持作为可选 feature (`async`)
  - [ ] 默认不启用，避免增加依赖

#### 技术选型
- **运行时**: `tokio` (更流行) 或 `async-std` (更轻量)
- **文件大小**: tokio ~1MB, async-std ~500KB
- **推荐**: 使用 `tokio`，生态更成熟

**依赖新增**: `tokio = { version = "1.35", features = ["fs"], optional = true }`

---

### 5. 错误处理增强
**优先级**: 🟡 中  
**预计工作量**: 6-8 小时  
**影响范围**: `crates/rfont-types/src/error.rs`, 所有解析逻辑

#### 当前问题
虽然已使用 thiserror，但错误信息不够详细，缺少上下文。

#### 改进方案
- [ ] **结构化错误枚举**
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum FontError {
      #[error("Invalid magic number: expected {expected:#010X}, got {actual:#010X}")]
      InvalidMagicNumber { expected: u32, actual: u32 },
      
      #[error("Table '{tag}' not found in font")]
      TableNotFound { tag: String },
      
      #[error("Invalid offset in table '{table}': offset={offset}, max={max}")]
      InvalidOffset { 
          table: String, 
          offset: u32, 
          max: u32 
      },
      
      #[error("Failed to parse {table} at offset {offset}: {reason}")]
      ParseError {
          table: String,
          offset: u64,
          reason: String,
      },
      
      // ... 更多具体错误类型
  }
  ```

- [ ] **错误上下文**
  - [ ] 每个错误包含足够的调试信息
  - [ ] 支持错误链（source）
  - [ ] 提供人类可读的错误消息
  
- [ ] **错误恢复建议**
  - [ ] 对于常见错误，提供修复建议
  - [ ] 例如："文件可能损坏，请尝试重新下载"

---

### 6. 格式检测增强
**优先级**: 🟡 中  
**预计工作量**: 4-6 小时  
**影响范围**: `crates/rfont/src/font/load.rs`

#### 当前状态
已支持 TTF/WOFF/WOFF2 自动检测，但信息有限。

#### 改进方向
- [ ] **详细格式信息**
  - [ ] 区分 TTF (TrueType) 和 OTF (OpenType with CFF)
  - [ ] 检测 WOFF/WOFF2 的压缩级别
  - [ ] 识别字体变体（Variable Font）
  
- [ ] **格式验证**
  - [ ] 校验 SFNT 结构完整性
  - [ ] 检查必需表是否存在（head, hhea, maxp, cmap）
  - [ ] 验证表偏移量和长度的一致性
  
- [ ] **API 扩展**
  ```rust
  pub struct FontFormatInfo {
      pub format: FontFormat,  // TTF, OTF, WOFF, WOFF2
      pub version: String,
      pub is_variable: bool,
      pub compression: Option<CompressionType>,
      pub required_tables: Vec<Tag>,
      pub optional_tables: Vec<Tag>,
  }
  
  impl Font {
      pub fn detect_format(data: &[u8]) -> Result<FontFormatInfo, FontError>;
  }
  ```

---

### 7. 批量处理 API
**优先级**: 🟡 中  
**预计工作量**: 8-10 小时  
**影响范围**: `crates/rfont/src/lib.rs`

#### 功能设计
- [ ] **批量子集化方法**
  ```rust
  impl Font {
      pub fn batch_subset(
          &self,
          inputs: Vec<SubsetInput>,
          options: &BatchSubsetOptions,
      ) -> Result<Vec<SubsetResult>, FontError>;
  }
  
  pub struct SubsetInput {
      pub text: String,
      pub output_path: PathBuf,
      pub format: String,  // "ttf", "woff", "woff2"
  }
  
  pub struct SubsetResult {
      pub input_text: String,
      pub output_path: PathBuf,
      pub original_size: u64,
      pub subset_size: u64,
      pub glyph_count: u32,
  }
  ```

- [ ] **并行处理支持**
  - [ ] 使用 `rayon` 进行并行处理
  - [ ] 可配置的线程数
  - [ ] 进度回调支持
  
- [ ] **统计信息**
  - [ ] 总处理时间
  - [ ] 平均压缩率
  - [ ] 失败任务统计

**依赖新增**: `rayon = { version = "1.8", optional = true }`

---

## 🟢 低优先级任务（1-2 月内）

### 8. 文档注释完善
**优先级**: 🟢 低  
**预计工作量**: 8-10 小时  
**影响范围**: 所有公共 API

#### 任务清单
- [ ] **核心库文档**
  - [ ] 为所有 `pub` 函数添加 rustdoc 注释
  - [ ] 为所有 `pub` 结构体和枚举添加说明
  - [ ] 添加使用示例到关键 API
  
- [ ] **示例代码**
  - [ ] 每个主要功能至少一个示例
  - [ ] 示例应该可直接运行（`cargo test --doc`）
  
- [ ] **文档质量检查**
  - [ ] 运行 `cargo doc --open` 检查渲染效果
  - [ ] 确保没有 broken links
  - [ ] 添加模块级别的概述文档

#### 文档标准
```rust
/// 将文本转换为字形 ID 列表
///
/// # 参数
/// * `text` - 要转换的 Unicode 文本
///
/// # 返回值
/// 返回字形 ID 列表，顺序与输入文本对应
///
/// # 示例
/// ```
/// use rfont::Font;
///
/// let font = Font::load("test.ttf").unwrap();
/// let glyph_ids = font.text_to_glyph_ids("Hello");
/// assert_eq!(glyph_ids.len(), 5);
/// ```
///
/// # 错误
/// 如果字体不支持某个字符，返回 `FontError::CharacterNotSupported`
pub fn text_to_glyph_ids(&self, text: &str) -> Result<Vec<u16>, FontError>;
```

---

### 9. 属性测试（Property-based Testing）
**优先级**: 🟢 低  
**预计工作量**: 6-8 小时  
**影响范围**: 测试代码

#### 引入 proptest
- [ ] **依赖添加**
  ```toml
  [dev-dependencies]
  proptest = "1.4"
  ```

- [ ] **测试场景**
  - [ ] 校验和计算：任意字节数组的校验和应保持一致
  - [ ] 字节序转换：u16/u32 的大端/小端转换往返一致
  - [ ] 解析-序列化：任意有效字体的解析后再序列化应与原文件一致
  - [ ] Base128 编码：任意 u32 值的编码解码往返一致
  
- [ ] **模糊测试**
  - [ ] 随机生成的字节数组不应导致 panic
  - [ ] 边界值测试（空文件、超大文件、损坏文件）

**收益**: 发现边缘情况的 bug，提高代码健壮性

---

### 10. 性能基准测试
**优先级**: 🟢 低  
**预计工作量**: 4-6 小时  
**影响范围**: `benches/` 目录

#### Criterion 集成
已配置 criterion，需要添加实际基准测试：

- [ ] **字体加载基准**
  - [ ] 小字体 (< 1MB)
  - [ ] 中等字体 (1-5MB)
  - [ ] 大字体 (> 5MB)
  
- [ ] **子集化性能**
  - [ ] 少量字符（< 100）
  - [ ] 中等字符（100-1000）
  - [ ] 大量字符（> 1000）
  
- [ ] **格式转换**
  - [ ] TTF → WOFF
  - [ ] TTF → WOFF2
  - [ ] 不同压缩级别的影响

- [ ] **缓存效果**
  - [ ] 有缓存 vs 无缓存的查询性能
  - [ ] 缓存命中率对性能的影响

**示例**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_font_loading(c: &mut Criterion) {
    c.bench_function("load_small_font", |b| {
        b.iter(|| {
            Font::load(black_box("small.ttf")).unwrap()
        })
    });
}
```

---

### 11. 代码覆盖率报告
**优先级**: 🟢 低  
**预计工作量**: 2-3 小时  

#### tarpaulin 集成
- [ ] **安装工具**
  ```bash
  cargo install cargo-tarpaulin
  ```

- [ ] **生成报告**
  ```bash
  cargo tarpaulin --out Html --output-dir coverage
  ```

- [ ] **目标设定**
  - [ ] 核心解析逻辑 > 80% 覆盖率
  - [ ] 整体覆盖率 > 70%
  - [ ] 识别未覆盖的代码路径并补充测试

---

### 12. CI/CD 集成
**优先级**: 🟢 低  
**预计工作量**: 4-6 小时  
**影响范围**: `.github/workflows/`

#### GitHub Actions 配置
- [ ] **自动化测试**
  - [ ] 每次 push 运行 `cargo test --workspace`
  - [ ] 每次 PR 运行 clippy 检查
  - [ ] 生成代码覆盖率报告
  
- [ ] **自动化文档**
  - [ ] 每次 release 生成 rustdoc
  - [ ] 部署到 GitHub Pages
  
- [ ] **发布流程**
  - [ ] 打 tag 时自动发布到 crates.io
  - [ ] 构建多平台二进制（Windows, macOS, Linux）
  - [ ] 创建 GitHub Release

**示例 workflow**:
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test --workspace
      - run: cargo clippy -- -D warnings
```

---

### 13. 日志系统优化
**优先级**: 🟢 低  
**预计工作量**: 2-3 小时  
**影响范围**: `crates/rfont/examples/test_alimama.rs`

#### 评估与配置
- [ ] **性能评估**
  - [ ] 测量 tracing 在生产环境的开销
  - [ ] 对比开启/关闭日志的性能差异
  
- [ ] **日志级别配置**
  - [ ] 开发环境：DEBUG 级别
  - [ ] 生产环境：WARN 级别
  - [ ] 支持环境变量配置（`RUST_LOG`）
  
- [ ] **结构化日志**
  - [ ] 考虑是否需要 JSON 格式输出
  - [ ] 添加性能追踪 span（可选）
  - [ ] 集成到 CLI 工具的 verbose 模式

---

### 14. API 设计改进
**优先级**: 🟢 低  
**预计工作量**: 8-12 小时  
**影响范围**: `crates/rfont/src/lib.rs` 公共 API

#### 改进方向
虽然已有 Builder 模式，但仍有优化空间：

- [ ] **流式子集化**
  - [ ] 针对超大字体（> 50MB）的流式处理
  - [ ] 避免一次性加载所有字形到内存
  - [ ] 支持边读边写的子集化
  
- [ ] **批量字符 API**
  - [ ] 更友好的批量字符输入方式
  - [ ] 支持从多个来源合并字符集
  - [ ] 字符去重和排序优化
  
- [ ] **预设模板扩展**
  - [ ] 更多预设（mobile, print, ebook）
  - [ ] 允许用户自定义预设
  - [ ] 预设的组合和继承

---

## 🖥️ 桌面应用规划（3-6 个月）

> **注意**: 桌面应用是独立项目，建议在 CLI 工具稳定后启动

### Phase 1: MVP 基础框架（2-3 周）
- [ ] 初始化 Tauri + Vue3 项目结构
- [ ] 配置 Rust 后端依赖
- [ ] 搭建前端 UI 框架
- [ ] 实现字体文件导入功能
- [ ] 实现文本输入模块
- [ ] 实现基础预览功能
- [ ] 集成 rfont 子集化功能
- [ ] 实现 TTF 格式导出

### Phase 2: 功能完善（2-3 周）
- [ ] WOFF/WOFF2 格式导出支持
- [ ] 用户体验优化（进度条、快捷键、撤销/重做）
- [ ] 高级预览选项

### Phase 3: 高级功能（2-3 周）
- [ ] 批量处理支持
- [ ] 字体对比模式
- [ ] 导出预设模板
- [ ] 国际化支持
- [ ] 主题切换

### Phase 4: 优化与发布（1-2 周）
- [ ] 性能优化（缓存、后台处理）
- [ ] 测试覆盖（单元 + E2E）
- [ ] 打包配置（Windows/macOS/Linux）
- [ ] 文档编写
- [ ] 发布流程

**技术栈**: Tauri + Vue3 + TypeScript + unocss  
**预计总工作量**: 7-11 周（全职开发）

---

## 🌐 长期愿景（6+ 个月）

### Web 版本（可选）
- 基于 WebAssembly 的在线工具
- 无需安装，浏览器直接使用
- 适合快速体验和分享

### 移动端应用（可选）
- React Native 或 Flutter 实现
- 随时随地进行字体子集化

### 生态建设
- 插件系统（自定义导出格式）
- API SDK（供其他项目集成）
- 社区贡献指南

---

## 📈 产品演进路线

```
核心库 (rfont) ✅ 已完成
    ↓
命令行工具 (rfont-cli) ✅ 已完成
    ↓
桌面应用 (rfont-desktop) 🎯 下一阶段
    ↓
Web 应用 + 生态系统 🔮 未来愿景
```

**核心理念**：
1. **渐进式开发**：从核心库到 CLI，再到 GUI，逐步完善
2. **代码复用**：所有上层应用都基于 rfont 核心库
3. **用户导向**：每个阶段都提供可用的产品形态
4. **质量优先**：每步都确保测试覆盖和文档完善

---

## 💡 快速胜利（Quick Wins）

以下任务可以在 **1 小时内** 完成且收益明显：

- [ ] 运行 `cargo fmt` 统一代码风格
- [ ] 运行 `cargo clippy --fix` 修复警告
- [ ] 删除未使用的导入（`cargo +nightly udeps`）
- [ ] 为关键 API 添加缺失的 rustdoc 注释
- [ ] 更新 README 中的安装和使用说明

---

*本文档采用"已完成简洁记录，未完成详细说明"的结构，便于快速了解项目状态和工作重点。*
