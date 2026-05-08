# rfont 性能基准测试

本目录包含 rfont 库的性能基准测试，使用 Criterion 框架进行精确的性能测量。

## 🎯 基线管理

### 当前状态

✅ **已配置基线支持**
- Criterion 自动维护 `base/`、`new/`、`change/` 数据
- 配置文件：`crates/rfont/criterion.toml`
- 增强采样：各测试组使用不同的 sample_size 和 warm_up_time

### 常用命令

```bash
# 建立新基线
cargo bench -p rfont -- --save-baseline v0.1.0

# 对比基线
cargo bench -p rfont -- --baseline v0.1.0

# 清除旧基线
rm -rf target/criterion

# 查看 HTML 报告
start target/criterion/report/index.html  # Windows
```

### 详细指南

完整的基线管理指南请参考：[BASELINE_GUIDE.md](../../BASELINE_GUIDE.md)

---

## 📊 基准测试列表

### performance.rs - 综合性能基准测试

**运行方式**:
```bash
# 运行所有基准测试
cargo bench -p rfont

# 运行特定基准测试
cargo bench -p rfont load_ttf_font
cargo bench -p rfont cmap_lookup_without_cache
cargo bench -p rfont text_to_glyph_ids
```

**测试场景**:

1. **字体加载性能** (`load_ttf_font`)
   - 测试从磁盘加载 TTF 字体的速度
   - 衡量文件 I/O 和解析性能

2. **Cmap 查找性能** (`cmap_lookup_without_cache`)
   - 测试 Unicode 到字形 ID 的映射查找
   - 评估 cmap 表的查询效率

3. **文本转字形性能** (`text_to_glyph_ids`)
   - 测试将文本转换为字形 ID 列表的速度
   - 衡量批量字符映射的性能

4. **字形迭代性能** (`glyph_iterator`)
   - 测试流式遍历所有字形的速度
   - 评估迭代器实现的效率

5. **子集化性能** (`subset_with_text`)
   - 测试基于文本的字体子集化处理速度
   - 衡量核心子集化算法的性能

6. **分块处理性能** (`chunked_processing`)
   - 测试分块处理字形的性能
   - 评估内存优化策略的效果

7. **并行处理对比** (`parallel_vs_serial`)
   - 对比串行和并行处理的性能差异
   - 需要启用 `parallel` feature

---

## 🚀 运行基准测试

### 基本要求

```bash
# 确保安装了 Rust toolchain
rustup update

# 运行所有基准测试（可能需要几分钟）
cargo bench -p rfont
```

### 启用并行特性

要测试并行处理的性能，需要启用 `parallel` feature：

```bash
cargo bench -p rfont --features parallel
```

### 查看结果

Criterion 会生成详细的 HTML 报告：

```bash
# 报告位于 target/criterion/ 目录
open target/criterion/report/index.html  # macOS
start target/criterion/report/index.html  # Windows
xdg-open target/criterion/report/index.html  # Linux
```

---

## 📈 性能指标

每个基准测试会测量以下指标：

- **平均时间** (Mean) - 多次运行的平均执行时间
- **中位数** (Median) - 执行时间的中位数
- **标准差** (Std Dev) - 执行时间的波动程度
- **变化范围** (Change) - 与上次运行的对比

---

## 💡 优化建议

根据基准测试结果，可以考虑以下优化方向：

### 1. 字体加载优化
- 使用懒加载机制减少初始加载时间
- 实现字体缓存避免重复加载
- 预加载常用表数据

### 2. Cmap 查找优化
- 使用 LRU 缓存热点字符映射
- 优化哈希表实现
- 考虑使用更高效的查找算法

### 3. 子集化优化
- 并行处理多个文本的子集化
- 优化字形提取算法
- 减少内存分配次数

### 4. 迭代器优化
- 使用流式处理减少内存峰值
- 预取数据提高缓存命中率
- 优化数据结构布局

---

## 🔬 基准测试最佳实践

### 1. 稳定的测试环境
- 关闭其他应用程序
- 禁用 CPU 频率调节
- 使用相同的测试数据

### 2. 多次运行
- 至少运行 3 次取平均值
- 观察标准差判断稳定性
- 排除异常值

### 3. 对比测试
- 修改前后都要运行基准测试
- 使用相同的硬件和环境
- 记录系统配置信息

### 4. 关注关键路径
- 优先优化热点代码
- 测量实际使用场景
- 考虑用户体验影响

---

## 📝 添加新的基准测试

要添加新的基准测试函数：

```rust
fn benchmark_my_feature(c: &mut Criterion) {
    // 准备测试数据
    let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();
    
    // 定义基准测试
    c.bench_function("my_feature", |b| {
        b.iter(|| {
            // 要测试的代码
            black_box(font.some_method())
        })
    });
}

// 注册到 criterion_group
criterion_group!(
    benches,
    benchmark_font_loading,
    benchmark_cmap_lookup,
    // ... 其他测试
    benchmark_my_feature,  // 添加新测试
);
```

---

## 🔗 相关文档

- [Criterion 用户指南](https://bheisler.github.io/criterion.rs/book/)
- [rfont API 文档](https://docs.rs/rfont)
- [examples/README.md](../examples/README.md) - 示例程序说明
- [COVERAGE_REPORT.md](../../COVERAGE_REPORT.md) - 代码覆盖率报告

---

## ⚠️ 注意事项

1. **基准测试 vs 单元测试**
   - 基准测试用于性能测量，不验证正确性
   - 单元测试用于功能验证，不关注性能
   - 两者互补，都需要维护

2. **结果解读**
   - 小幅波动（< 5%）通常是正常的
   - 关注趋势而非单次结果
   - 结合实际使用场景评估

3. **硬件影响**
   - 不同 CPU 的结果可能差异很大
   - SSD vs HDD 会影响 I/O 测试
   - 内存大小影响缓存效果

4. **编译优化**
   - 基准测试使用 release 模式编译
   - 确保 LTO 和其他优化已启用
   - 不要对比 debug 和 release 的结果
