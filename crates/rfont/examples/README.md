# rfont 示例程序

本目录包含 rfont 库的使用示例，展示各种功能和 API 的使用方法。

## 📚 示例列表

### 1. font_info_demo.rs - 字体元数据 API 演示
**功能**: 展示如何获取字体的基本信息和元数据

**运行方式**:
```bash
cargo run --example font_info_demo
```

**主要 API**:
- `Font::load()` - 加载字体文件
- `font.get_font_info()` - 获取字体基本信息
- `font.get_table_list()` - 获取所有表的列表
- `font.get_supported_characters()` - 获取支持的字符
- `font.supports_character()` - 检查是否支持特定字符

---

### 2. builder_demo.rs - Builder 模式 API 演示
**功能**: 展示如何使用 FontSubsetBuilder 进行字体子集化

**运行方式**:
```bash
cargo run --example builder_demo
```

**主要 API**:
- `font.subset_builder()` - 创建子集化构建器
- `.text()` - 基于文本创建子集
- `.unicode_range()` - 基于 Unicode 范围创建子集
- `.glyph_ids()` - 基于字形 ID 创建子集
- `.optimize_post()` - 优化 post 表
- `.strip_glyph_names()` - 移除字形名称
- `.compression_level()` - 设置压缩级别
- `.output_format()` - 设置输出格式
- `.preset()` - 使用预设配置（web/print）
- `.build()` - 执行子集化

---

### 3. error_handling_demo.rs - 错误处理演示
**功能**: 展示如何处理字体操作中的各种错误

**运行方式**:
```bash
cargo run --example error_handling_demo
```

**主要特性**:
- 结构化错误类型（FontError）
- 错误恢复建议（suggestion() 方法）
- 常见错误场景演示
- 优雅的错误处理模式

---

### 4. format_detection_demo.rs - 格式检测演示
**功能**: 展示如何检测和识别不同的字体格式

**运行方式**:
```bash
cargo run --example format_detection_demo
```

**主要 API**:
- `Font::detect_format()` - 检测字体格式
- 支持 TTF、OTF、WOFF、WOFF2 格式识别
- 可变字体检测
- 必需表验证

---

### 5. perf_comparison.rs - 性能对比演示
**功能**: 对比不同处理方式性能差异

**运行方式**:
```bash
cargo run --example perf_comparison
```

**对比场景**:
- 流式迭代 vs 批量处理
- 有缓存 vs 无缓存
- 串行 vs 并行处理（需要启用 parallel feature）

---

### 6. test_alimama.rs - 阿里妈妈字体测试
**功能**: 使用真实的阿里妈妈刀隶体字体进行测试

**运行方式**:
```bash
cargo run --example test_alimama
```

**测试内容**:
- 字体加载
- 子集化处理
- 格式转换
- 性能统计

---

## 🚀 快速开始

### 运行所有示例
```bash
# 列出所有可用示例
cargo run --example

# 运行特定示例
cargo run --example font_info_demo
cargo run --example builder_demo
cargo run --example error_handling_demo
cargo run --example format_detection_demo
cargo run --example perf_comparison
cargo run --example test_alimama
```

### 启用并行特性
某些示例需要启用 `parallel` feature 才能使用并行处理：
```bash
cargo run --example perf_comparison --features parallel
```

---

## 📖 学习路径

建议按以下顺序学习这些示例：

1. **font_info_demo** - 了解基本的字体加载和信息查询
2. **builder_demo** - 学习如何使用 Builder 模式进行子集化
3. **error_handling_demo** - 掌握错误处理技巧
4. **format_detection_demo** - 了解格式检测功能
5. **perf_comparison** - 理解性能优化方法
6. **test_alimama** - 综合应用所有功能

---

## 💡 提示

- 所有示例都使用 `crates/rfont/src/AlimamaDaoLiTi.ttf` 作为测试字体
- 示例代码可以直接复制到自己的项目中使用
- 每个示例都包含详细的注释说明
- 查看源代码可以了解更多高级用法

---

## 🔗 相关文档

- [rfont API 文档](https://docs.rs/rfont)
- [README.md](../../README.md) - 项目总览
- [LOGGING.md](../../LOGGING.md) - 日志系统说明
- [DOC_GUIDE.md](../../DOC_GUIDE.md) - 文档生成指南
