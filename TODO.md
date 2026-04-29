# rfont 项目优化 TODO List

## 📊 当前状态

| 指标 | 数值 |
|------|------|
| 单元测试总数 | 40 (rfont: 6, rfont-core: 34) |
| 测试通过率 | 100% ✅ |
| 编译状态 | 无警告 ✅ |
| 性能优化 | 已完成 ✅ |
| glyf/hmtx 测试覆盖 | 已完成 ✅ |
| CLI 工具开发 | 已完成 ✅ |
| WOFF2 支持 | 已完成 ✅ |
| Cargo.toml 优化 | 已完成 ✅ |
| 最后更新 | 2026-04-29 (Cargo.toml 优化完成) |

---

## 🔴 高优先级（核心功能完善）

### 1. 命令行工具 (CLI) 开发 ✅ COMPLETED
- [x] 创建 `rfont-cli` crate 结构
- [x] 集成 `clap` v4 作为 CLI 框架
- [x] 实现 `info` 命令：字体信息查询
  - [x] 显示基本元数据（格式、版本、字形数等）
  - [x] 列出所有表及其大小
  - [x] 支持 `--verbose` 详细模式
  - [x] 支持 `--json` JSON 输出格式
- [x] 实现 `subset` 命令：字体子集化
  - [x] 支持直接传入文本参数
  - [x] 支持从文件读取字符列表 (`--text-file`)
  - [x] 支持 Unicode 范围指定 (`--range`)
  - [x] 集成进度条显示处理进度
  - [x] 显示压缩率统计
  - [x] 支持 post 表优化选项 (`--strip-post-names`)
- [x] 实现 `convert` 命令：格式转换
  - [x] TTF ↔ WOFF 双向转换
  - [x] TTF ↔ WOFF2 双向转换
  - [x] 支持压缩级别配置
  - [x] 批量文件转换支持
- [ ] 实现 `batch` 命令：批量处理
  - [ ] 批量子集化处理
  - [ ] 批量格式转换
  - [ ] 并行处理支持 (`--jobs`)
- [x] 用户体验优化
  - [x] 彩色输出（成功/警告/错误）
  - [x] 友好的错误提示和建议
  - [x] 完善的 `--help` 文档和示例
  - [x] 进度条和状态反馈

**技术栈**:
- `clap` v4 (CLI 解析)
- `indicatif` (进度条)
- `colored` (彩色输出)
- `serde_json` (JSON 支持)
- `brotli` (WOFF2 压缩)

**影响范围**: 新建 `crates/rfont-cli/`  
**预计工作量**: 16-24 小时  
**优先级说明**: CLI 是用户直接接触的入口，应优先开发以提升可用性

---

### 2. WOFF2 格式支持 ✅ COMPLETED
- [x] 添加 `brotli` 依赖到 rfont-core 和 rfont-cli
- [x] 创建 woff2.rs 模块，定义 WOFF2 Header 和表结构
  - [x] Woff2Header 结构（签名、flavor、长度等）
  - [x] Woff2TableDirectoryEntry 结构（变长编码）
  - [x] Base128 编码解码实现
  - [x] 63 个预定义标签常量
- [x] 实现 WOFF2 解压缩逻辑（Brotli）
  - [x] 更新 Font::load 支持 WOFF2 格式检测
  - [x] 解析 WOFF2 Header 和表目录
  - [x] 使用 Brotli 解压缩表数据
  - [x] 重组为 SFNT 数据结构
- [x] 实现 WOFF2 写入/压缩功能
  - [x] convert_to_woff() 方法（TTF → WOFF）
  - [x] convert_to_woff2() 方法（TTF → WOFF2）
  - [x] 支持压缩级别配置（0-11）
  - [x] 集成到 subset_with_options
- [x] 更新 CLI convert 命令支持 WOFF2 格式
  - [x] 验证目标格式（ttf/woff/woff2）
  - [x] TTF → WOFF2 转换
  - [x] WOFF2 → TTF 转换
  - [x] 文件大小统计和压缩率显示
- [x] 添加 WOFF2 单元测试
  - [x] WOFF2 Header 读取测试
  - [x] Header 验证测试
  - [x] Base128 编码测试
  - [x] 预定义标签测试

**技术栈**:
- `brotli` v6.0 (WOFF2 压缩/解压缩)
- Base128 变长编码

**影响范围**: 
- `crates/rfont-core/src/tables/woff2.rs` (新建)
- `crates/rfont/src/font/load.rs`
- `crates/rfont/src/font/subset.rs`
- `crates/rfont-cli/src/commands/convert.rs`

**测试结果**:
- 4 个 WOFF2 单元测试全部通过
- 实际测试：4.9 MB TTF → 2.8 MB WOFF2 (压缩率 43%)

**预计工作量**: 已完成

---

### 3. glyf 表单元测试
- [ ] 添加简单字形（Simple Glyph）解析测试
- [ ] 添加复合字形（Composite Glyph）解析测试
- [ ] 验证坐标增量编码的正确性
- [ ] 测试边界情况（空字形、单点字形等）

**影响范围**: `crates/rfont-core/src/tables/glyf.rs`  
**预计工作量**: 6-8 小时

---

### 4. hmtx 表完整测试
- [ ] 添加水平度量数据解析测试
- [ ] 验证 advanceWidth 和 lsb 的计算
- [ ] 测试 numberOfHMetrics 与实际数据的匹配

**影响范围**: `crates/rfont-core/src/tables/hmtx.rs`  
**预计工作量**: 2-3 小时

---

### 5. rfont 核心库优化（适配 CLI 和桌面应用）

#### A. API 层优化
- [ ] **异步支持**
  - [ ] 为耗时操作添加 async/await 支持（字体加载、子集化）
  - [ ] 使用 `tokio` 或 `async-std` 运行时
  - [ ] 提供同步和异步两套 API
  
- [ ] **进度反馈机制**
  - [ ] 定义 `ProgressCallback` trait
    ```rust
    pub trait ProgressCallback: FnMut(u32, u32) + Send {}
    ```
  - [ ] 在子集化过程中定期调用回调
  - [ ] 支持取消操作（通过返回 `Result<(), Cancelled>`）

#### B. 性能优化
- [x] **懒加载机制** ✅ COMPLETED
  - [x] 使用 `OnceCell` 缓存已解析的表
  - [x] 按需加载大型表（glyf、cmap）
  - [x] 提供 `preload_tables()` 方法预加载常用表
  
- [x] **查询缓存** ✅ COMPLETED
  - [x] 为 Cmap 添加 LRU Cache（256 条目）
  - [x] 实现 `get_glyph_id_mut()` 带缓存版本
  - [x] 提供缓存清除接口 `clear_cache()`
  - [x] 批量查询优化 `get_glyph_ids()`

- [x] **流式处理支持** ✅ COMPLETED
  - [x] 实现 `GlyphIterator` 用于逐字形处理
  - [x] 支持分块读取大型 glyf 表 `get_glyphs_chunked()`
  - [x] 减少内存峰值占用

#### C. 功能增强
- [x] **字体元数据查询 API** ✅ COMPLETED
  - [x] `get_font_info()` - 返回字体基本信息（字形数、units per EM、边界框等）
  - [x] `get_table_list()` - 列出所有表及其大小、偏移量、校验和
  - [x] `get_supported_characters()` - 返回字体支持的所有 Unicode 字符
  - [x] `supports_character()` - 检查是否支持特定字符
  - [x] `text_to_glyph_ids()` - 将文本转换为字形 ID 列表
  
  **新增结构体**:
  - `FontInfo` - 包含完整的字体元数据
  - `TableInfo` - 表信息（标签、校验和、偏移量、长度）
  
  **影响文件**: `crates/rfont/src/lib.rs`  
  **示例代码**: `crates/rfont/examples/font_info_demo.rs`

- [ ] **批量处理支持**
  - [ ] 提供 `batch_subset()` 方法
  - [ ] 支持并行处理（使用 `rayon`）
  - [ ] 返回处理统计信息

- [ ] **格式检测增强**
  - [ ] 自动检测 TTF/OTF/WOFF/WOFF2
  - [ ] 提供更详细的格式信息

#### D. 配置选项
- [ ] **子集化配置结构体**
  ```rust
  pub struct SubsetOptions {
      pub optimize_post_table: bool,     // 是否优化 post 表
      pub strip_glyph_names: bool,       // 是否移除字形名称
      pub compression_level: u8,         // WOFF 压缩级别 (0-9)
      pub keep_hinting: bool,            // 是否保留 hinting 数据
      pub output_format: String,         // 输出格式（"ttf" 或 "woff"）
  }
  ```

- [x] **Builder 模式 API** ✅ COMPLETED
  ```rust
  let subset_data = font.subset_builder()
      .text("你好世界")
      .optimize_post(true)
      .compression_level(9)
      .output_format("ttf")
      .build()?;
  ```
  
  **新增功能**:
  - `FontSubsetBuilder` - Builder 结构体
  - `SubsetOptions` - 配置选项结构体
  - 链式调用接口：`.text()`, `.glyph_ids()`, `.unicode_range()`
  - 配置方法：`.optimize_post()`, `.strip_glyph_names()`, `.compression_level()`
  - 预设模板：`.preset("web")`, `.preset("print")`
  - 灵活输入：支持文本、字形 ID、Unicode 范围
  
  **影响文件**: `crates/rfont/src/lib.rs`  
  **示例代码**: `crates/rfont/examples/builder_demo.rs`

**依赖新增**:
- `tokio` 或 `async-std` - 异步运行时（可选 feature）
- `lru` - LRU 缓存
- `rayon` - 并行处理（可选 feature）

**影响范围**: `crates/rfont/src/lib.rs`, `crates/rfont-core/src/`  
**预计工作量**: 12-16 小时  
**优先级说明**: 这是 CLI 和桌面应用的基础，应优先完成

---

### 5. 桌面应用程序 (Desktop App)
- [ ] **Phase 1: MVP 基础框架** (2-3 周)
  - [ ] 初始化 Tauri + Vue3 项目结构
  - [ ] 配置 Rust 后端依赖（rfont、image、font-kit）
  - [ ] 搭建前端 UI 框架（Vue3 + TypeScript + unocss）
  - [ ] 实现字体文件导入功能
    - [ ] 拖拽上传支持
    - [ ] 文件选择对话框
    - [ ] 字体列表展示
  - [ ] 实现文本输入模块
    - [ ] 多行文本输入框
    - [ ] 字符统计显示
    - [ ] 从文件导入文本
  - [ ] 实现基础预览功能
    - [ ] Canvas 实时渲染
    - [ ] 字号调节（8px - 200px）
    - [ ] 背景色/文字颜色设置
  - [ ] 集成 rfont 子集化功能
    - [ ] Tauri 命令封装
    - [ ] 异步处理支持
    - [ ] 进度反馈
  - [ ] 实现 TTF 格式导出
    - [ ] 文件保存对话框
    - [ ] 导出成功提示

- [ ] **Phase 2: 功能完善** (2-3 周)
  <!-- - [ ] Unicode 范围选择器
    - [ ] 常用汉字（CJK Unified Ideographs）
    - [ ] 拉丁字母（Latin）
    - [ ] 自定义范围输入
  - [ ] 高级预览选项
    - [ ] 字重调节（如果支持）
    - [ ] 行高和字间距
    - [ ] 单字/段落预览模式切换
    - [ ] 缩放和平移 -->
  - [ ] WOFF 格式导出支持
    - [ ] 压缩级别选择
    - [ ] 格式对比显示
  - [ ] 用户体验优化
    - [ ] 进度条动画
    - [ ] 错误提示优化
    - [ ] 快捷键支持（Ctrl+O/S/R）
    - [ ] 撤销/重做功能

- [ ] **Phase 3: 高级功能** (2-3 周)
  - [ ] 批量处理支持
    - [ ] 多字体同时子集化
    - [ ] 批量导出
  - [ ] 字体对比模式
    - [ ] 原始字体 vs 子集字体
    - [ ] 并排预览
  - [ ] 导出预设模板
    - [ ] Web 优化预设
    - [ ] 打印优化预设
    - [ ] 移动端优化预设
  <!-- - [ ] 历史记录和收藏
    - [ ] 最近使用的字体
    - [ ] 常用字符集保存 -->
  - [ ] 国际化支持
    - [ ] 中文界面
    - [ ] 英文界面
  - [ ] 主题切换
    - [ ] 亮色主题
    - [ ] 暗色主题

- [ ] **Phase 4: 优化与发布** (1-2 周)
  - [ ] 性能优化
    - [ ] 字体缓存机制（LRU Cache）
    - [ ] Web Worker 后台处理
    - [ ] 内存管理优化
  - [ ] 测试覆盖
    - [ ] 单元测试（Rust + TypeScript）
    - [ ] E2E 测试（Playwright）
  - [ ] 打包配置
    - [ ] Windows (.exe, .msi)
    - [ ] macOS (.dmg, .app)
    - [ ] Linux (.deb, .AppImage)
  - [ ] 应用美化
    - [ ] 应用图标设计
    - [ ] 启动画面
    - [ ] 关于页面
  - [ ] 文档编写
    - [ ] 用户使用手册
    - [ ] 常见问题解答
    <!-- - [ ] 视频教程 -->
  - [ ] 发布流程
    - [ ] GitHub Releases 配置
    - [ ] 自动更新支持
    - [ ] 应用商店提交（可选）

**技术栈**:
- **后端**: Tauri + Rust (rfont 库)
- **前端**: Vue3 + TypeScript + Vite
- **状态管理**: Zustand
- **UI 框架**: unocss + Framer Motion
- **字体渲染**: Canvas API / WebGL
- **测试**: Vitest + Playwright

**影响范围**: 新建 `rfont-desktop/` 目录  
**预计总工作量**: 7-11 周（全职开发）  
**优先级说明**: 桌面应用是产品的最终形态，建议在 CLI 工具稳定后启动

---

## 🟡 中优先级（性能与健壮性）

### 6. 懒加载机制实现
- [ ] 为大型表（glyf、cmap）实现真正的懒加载
- [ ] 使用 `OnceCell` 或 `Lazy` 缓存解析结果
- [ ] 避免重复解析相同的表数据

**收益**: 减少内存占用，提升大字体文件加载速度  
**影响范围**: `crates/rfont-core/src/reader.rs`, 各表模块  
**预计工作量**: 8-12 小时

---

### 7. 错误处理增强
- [ ] 定义结构化的 `FontError` 枚举类型
- [ ] 添加错误上下文信息（表名、偏移量、期望值等）
- [ ] 实现 `std::error::Error` trait
- [ ] 提供人类可读的错误消息

**示例**:
``rust
#[derive(Debug)]
pub enum FontError {
    InvalidMagicNumber { expected: u32, actual: u32 },
    TableNotFound { tag: String },
    InvalidOffset { table: String, offset: u32, max: u32 },
    // ...
}
```

**影响范围**: `crates/rfont-types/src/lib.rs`, 所有解析逻辑  
**预计工作量**: 6-8 小时

---

### 8. WOFF2 格式支持
- [ ] 添加 brotli 解压缩依赖
- [ ] 实现 WOFF2 签名检测
- [ ] 实现 WOFF2 到 SFNT 的转换
- [ ] 添加 WOFF2 加载测试

**依赖**: `brotli` crate  
**影响范围**: `crates/rfont-core/src/tables/woff.rs`  
**预计工作量**: 12-16 小时

---

### 9. post 表子集化优化
- [ ] 在子集化时自动将 post format 2.0 降级为 3.0
- [ ] 移除不必要的字形名称数组
- [ ] 减小子集字体体积（可减少数十 KB）

**参考记忆**: `cba61cf5-82a8-4844-b307-28abae70537f`  
**影响范围**: `crates/rfont/src/lib.rs` (subset_and_serialize)  
**预计工作量**: 3-4 小时

---

## 🟢 低优先级（代码质量与生态）

### 10. 清理废弃代码
- [ ] 删除或重构 `src/a.rs` 和 `src/tables1.rs`（如果仍存在）
- [ ] 移除未使用的导入和变量
- [ ] 统一代码风格（使用 `cargo fmt`）

**影响范围**: 根目录 src/  
**预计工作量**: 1-2 小时

---

### 11. 文档注释完善
- [ ] 为所有公共 API 添加 rustdoc 注释
- [ ] 添加使用示例到文档中
- [ ] 生成并检查文档质量（`cargo doc --open`）

**影响范围**: 所有 pub 函数和类型  
**预计工作量**: 8-10 小时

---

### 12. 属性测试（Property-based Testing）
- [ ] 引入 `proptest` 或 `quickcheck`
- [ ] 为校验和计算添加随机测试
- [ ] 为字节序转换添加模糊测试
- [ ] 验证解析-序列化往返一致性

**依赖**: `proptest` crate  
**影响范围**: 测试代码  
**预计工作量**: 6-8 小时

---

### 13. 性能基准测试
- [ ] 引入 `criterion` 基准测试框架
- [ ] 测量字体加载时间
- [ ] 测量子集化性能
- [ ] 建立性能回归监控

**依赖**: `criterion` crate  
**影响范围**: `benches/` 目录  
**预计工作量**: 4-6 小时

---

### 14. 代码覆盖率报告
- [ ] 集成 `tarpaulin` 或 `grcov`
- [ ] 生成 HTML 覆盖率报告
- [ ] 识别未覆盖的代码路径
- [ ] 目标：核心解析逻辑 > 80% 覆盖率

**依赖**: `cargo-tarpaulin`  
**预计工作量**: 2-3 小时

---

### 17. API 设计改进
- [ ] 考虑提供更友好的 Builder 模式 API
- [ ] 支持流式子集化（针对超大字体）
- [ ] 添加字体元数据查询接口
- [ ] 支持批量字符的子集化

**影响范围**: `crates/rfont/src/lib.rs` 公共 API  
**预计工作量**: 8-12 小时

---

### 20. CI/CD 集成
- [ ] 配置 GitHub Actions
- [ ] 自动化运行测试
- [ ] 自动化运行 clippy 检查
- [ ] 自动化生成文档
- [ ] 发布到 crates.io 的工作流

**影响范围**: `.github/workflows/`  
**预计工作量**: 4-6 小时

---

### 18. Cargo.toml 优化 ✅ COMPLETED
- [x] 添加 workspace 级别的依赖管理
  - [x] 统一版本控制（chrono, brotli, lru, tracing 等）
  - [x] 减少重复声明，提升可维护性
  - [x] 加快编译速度（共享依赖只编译一次）
- [x] 添加 resolver = "2"
  - [x] Rust 2021 edition 推荐配置
  - [x] 更好的依赖解析算法
- [x] 更新所有 crate 使用 workspace 依赖
  - [x] rfont-types: chrono, thiserror
  - [x] rfont-core: chrono, encoding_rs, lru, brotli
  - [x] rfont: flate2, brotli, chrono, tracing, lru 等
  - [x] rfont-cli: clap, colored, indicatif, serde_json 等
  - [x] font_macros: proc-macro2, quote, syn
- [x] 添加构建优化配置
  - [x] release profile: LTO, strip, panic=abort
  - [x] dev profile: 依赖包 opt-level=2
- [x] 完善 package metadata
  - [x] 添加 description, authors, license
  - [x] 添加 keywords, categories (rfont, rfont-cli)
  - [x] 添加 repository URL

**优化效果**:
- ✅ 统一版本管理，避免版本冲突
- ✅ 减少重复代码，提升可维护性
- ✅ 加快编译速度（共享依赖缓存）
- ✅ 更小的发布二进制（strip + LTO）
- ✅ 完整的元数据信息

**影响范围**: 
- `Cargo.toml` (根目录)
- `crates/*/Cargo.toml` (所有子 crate)

**测试结果**: 
- 编译成功，无警告
- 45 个测试全部通过

**预计工作量**: 已完成

---

### 19. 日志系统优化
- [ ] 评估 tracing 在生产环境的开销
- [ ] 配置不同环境的日志级别
- [ ] 添加性能追踪 span（可选）
- [ ] 考虑是否需要结构化日志输出

**当前状态**: 已集成 tracing，需评估实际效果  
**影响范围**: `crates/rfont/examples/test_alimama.rs`  
**预计工作量**: 2-3 小时

---

### 17. API 设计改进
- [ ] 考虑提供更友好的 Builder 模式 API
- [ ] 支持流式子集化（针对超大字体）
- [ ] 添加字体元数据查询接口
- [ ] 支持批量字符的子集化

**影响范围**: `crates/rfont/src/lib.rs` 公共 API  
**预计工作量**: 8-12 小时

---

## 📊 优先级说明

| 优先级 | 标准 | 建议执行顺序 |
|--------|------|-------------|
| 🔴 高 | 影响核心功能、测试完整性 | 立即执行 |
| 🟡 中 | 提升性能、健壮性、用户体验 | 1-2 周内 |
| 🟢 低 | 代码质量、长期维护 | 1-2 月内 |

---

## 🎯 推荐执行路线

### 短期目标（1-2 个月）

**第一阶段（本周）**：
1. ✅ **rfont 核心库优化** - 结构化错误处理、模块分离已完成
2. ✅ **命令行工具基础框架** - 创建 crate 结构，实现 `info` 命令
3. ✅ **glyf 表单元测试** - 14个测试用例全部通过

**第二阶段（下周）**：
4. ✅ **CLI 核心功能** - 实现 `subset` 和 `convert` 命令
5. ✅ **rfont 性能优化** - 懒加载机制、查询缓存、流式处理已完成
6. ✅ **hmtx 表完整测试** - 7个测试用例全部通过

**第三阶段（本月）**：
7. ⭐ **CLI 高级功能** - 实现 `batch` 命令和用户体验优化
8. ✅ **rfont 字体元数据 API** - get_font_info(), get_table_list() 等已完成
9. ✅ **rfont Builder 模式 API** - FontSubsetBuilder, SubsetOptions 已完成
10. ✅ **rfont 性能优化** - 懒加载、LRU 缓存、流式迭代器已完成
11. ✅ **CLI 工具开发** - info, subset, convert 命令全部完成
12. ✅ **Cargo.toml 优化** - workspace 依赖管理、profiles、metadata 已完成
13. 完善文档注释
14. 配置 CI/CD

**第四阶段（下月）**：
15. ✅ **WOFF2 支持** - 已完成，压缩率 43%
16. 属性测试
17. API 设计改进

---

### 中期目标（3-6 个月）

**第五阶段（第 2-3 月）**：
18. 🖥️ **桌面应用 Phase 1 (MVP)** 
    - Tauri + React 项目初始化
    - 基础字体导入和文本输入
    - Canvas 预览功能
    - TTF 格式导出

**第六阶段（第 4-5 月）**：
19. 🖥️ **桌面应用 Phase 2 (功能完善)**
    - Unicode 范围选择器
    - 高级预览选项
    - WOFF 格式支持
    - 用户体验优化

**第七阶段（第 6 月）**：
20. 🖥️ **桌面应用 Phase 3 & 4**
    - 批量处理和对比模式
    - 性能优化和测试
    - 打包发布准备

---

### 长期愿景（6+ 个月）

21. 🌐 **Web 版本**（可选）
    - 基于 WebAssembly 的在线工具
    - 无需安装，浏览器直接使用
    - 适合快速体验和分享

22. 📱 **移动端应用**（可选）
    - React Native 或 Flutter 实现
    - 随时随地进行字体子集化

23. 🤝 **生态建设**
    - 插件系统（自定义导出格式）
    - API SDK（供其他项目集成）
    - 社区贡献指南

---

## 💡 快速胜利（Quick Wins）

以下任务可以在 **1 小时内** 完成且收益明显：

- ✅ 运行 `cargo fmt` 统一代码风格
- ✅ 运行 `cargo clippy` 修复警告
- ✅ 删除未使用的导入
- ✅ 添加缺失的 rustdoc 注释（关键 API）
- ✅ 更新 README 中的测试章节
- ✅ **结构化错误处理** - 已完成，使用 thiserror 定义 FontError
- ⭐ **创建 CLI 项目骨架** - 初始化 `rfont-cli` crate 和基础结构

---

## 📈 产品演进路线

```
核心库 (rfont) 
    ↓
命令行工具 (rfont-cli) ← 当前阶段
    ↓
桌面应用 (rfont-desktop) ← 下一阶段
    ↓
Web 应用 + 生态系统 ← 未来愿景
```

**核心理念**：
1. **渐进式开发**：从核心库到 CLI，再到 GUI，逐步完善
2. **代码复用**：所有上层应用都基于 rfont 核心库
3. **用户导向**：每个阶段都提供可用的产品形态
4. **质量优先**：每步都确保测试覆盖和文档完善

---

*最后更新: 2026-04-29 (Cargo.toml 优化完成)*
