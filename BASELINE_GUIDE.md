# rfont 基准测试基线管理指南

## 📊 当前状态

### ✅ 已有基线支持

rfont 项目**已经具备基线测试能力**，Criterion 框架自动维护基线数据：

```
target/criterion/
├── <benchmark_name>/
│   ├── base/          ← 基线数据（上次运行结果）
│   ├── new/           ← 新数据（本次运行结果）
│   ├── change/        ← 变化分析（对比结果）
│   └── report/        ← HTML 报告
```

### ⚠️ 当前问题

1. **基线自动覆盖**：每次运行后，`new` 会自动覆盖 `base`
2. **缺少明确基线管理**：无法长期跟踪性能趋势
3. **采样配置不足**：默认采样数可能不够准确
4. **环境因素影响**：系统负载、CPU 频率等会影响结果

---

## 🎯 基线管理最佳实践

### 1. 建立稳定基线

```bash
# 清除旧数据，建立新的干净基线
rm -rf target/criterion

# 首次运行（建立基线）
cargo bench -p rfont

# 验证基线已创建
ls target/criterion/*/base/estimates.json
```

### 2. 使用命名基线（推荐）

```bash
# 保存特定版本的基线
cargo bench -p rfont -- --save-baseline v0.1.0

# 后续对比该基线
cargo bench -p rfont -- --baseline v0.1.0

# 保存多个版本进行对比
cargo bench -p rfont -- --save-baseline before-optimization
# ... 进行优化 ...
cargo bench -p rfont -- --baseline before-optimization --save-baseline after-optimization
```

### 3. CI/CD 中的基线管理

```yaml
# .github/workflows/benchmark.yml
name: Performance Benchmark

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Cache baseline
        uses: actions/cache@v3
        with:
          path: target/criterion
          key: ${{ runner.os }}-criterion-baseline-${{ hashFiles('Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-criterion-baseline-
      
      - name: Run benchmarks
        run: cargo bench -p rfont -- --save-baseline main
      
      - name: Compare with baseline
        if: github.event_name == 'pull_request'
        run: cargo bench -p rfont -- --baseline main
      
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: target/criterion/report/
```

---

## 🔧 配置优化

### 已完成优化

#### 1. criterion.toml 配置文件

位置：`crates/rfont/criterion.toml`

```toml
# 采样配置
sample_size = 100              # 默认采样次数
warm_up_time = "3s"            # 预热时间
measurement_time = "5s"        # 测量时间
noise_threshold = 0.05         # 噪声阈值（5%）

# 置信区间
confidence_level = 0.95        # 95% 置信水平
```

#### 2. 增强的基准测试代码

位置：`crates/rfont/benches/performance.rs`

每个基准测试现在都使用 `BenchmarkGroup` 进行精细控制：

```rust
let mut group = c.benchmark_group("cmap_lookup");
group.sample_size(200);  // cmap 查找很快，需要更多采样
group.warm_up_time(Duration::from_secs(5));
group.measurement_time(Duration::from_secs(15));

group.bench_function("without_cache", |b| {
    // ...
});

group.finish();
```

**各测试的采样配置**：

| 测试组 | sample_size | warm_up | measurement | 说明 |
|--------|-------------|---------|-------------|------|
| font_loading | 150 | 3s | 10s | 字体加载 |
| cmap_lookup | 200 | 5s | 15s | Cmap 查找（快速操作，需高采样） |
| text_conversion | 150 | 3s | 10s | 文本转字形 |
| glyph_iteration | 100 | 3s | 10s | 字形迭代 |
| subset_creation | 100 | 5s | 15s | 子集化（慢速操作） |
| chunked_processing | 150 | 3s | 10s | 分块处理 |

---

## 📈 解读性能报告

### 1. 理解 change 值

```
change: [+3.8614% +4.2562% +4.6362%] (p = 0.00 < 0.05)
Performance has regressed.
```

**含义**：
- **正值（+）**：性能退化（时间增加，变慢）
- **负值（-）**：性能提升（时间减少，变快）
- **三个值**：置信区间的下界、点估计、上界
- **p-value**：统计显著性（< 0.05 表示显著）

### 2. 判断标准

| 变化幅度 | 建议操作 |
|---------|---------|
| < 2% | ✅ 忽略（噪声范围内） |
| 2-5% | ⚠️ 观察（多次运行确认） |
| 5-10% | 🔍 调查（检查代码变更） |
| > 10% | 🚨 警报（必须调查原因） |

### 3. 常见误报原因

#### ❌ 系统环境因素
- CPU 频率波动（节能模式 vs 性能模式）
- 后台进程干扰
- 内存压力
- 磁盘缓存状态

#### ❌ 基线不准确
- 在不同环境下生成的基线
- 采样次数不足
- 预热时间不够

#### ❌ 统计误差
- p-value > 0.05 的变化不显著
- 置信区间过宽
- 异常值影响

---

## 🛠️ 实用命令

### 建立和管理基线

```bash
# 1. 清除所有基线数据
rm -rf target/criterion

# 2. 建立新基线
cargo bench -p rfont

# 3. 保存命名基线
cargo bench -p rfont -- --save-baseline my-baseline

# 4. 对比命名基线
cargo bench -p rfont -- --baseline my-baseline

# 5. 列出所有基线
ls target/criterion/*/base/
```

### 提高准确性

```bash
# 1. 关闭其他应用程序
# 2. 设置 CPU 为性能模式
# 3. 多次运行取平均
for i in {1..3}; do
    cargo bench -p rfont
done

# 4. 只运行特定测试
cargo bench -p rfont cmap_lookup
```

### 查看报告

```bash
# 打开 HTML 报告
open target/criterion/report/index.html  # macOS
xdg-open target/criterion/report/index.html  # Linux
start target/criterion/report/index.html  # Windows

# 查看原始数据
cat target/criterion/cmap_lookup/change/estimates.json | jq
```

---

## 📝 工作流程示例

### 场景 1：日常开发中的性能检查

```bash
# 1. 开发前建立基线
cargo bench -p rfont -- --save-baseline before-change

# 2. 进行代码修改
# ... coding ...

# 3. 对比性能
cargo bench -p rfont -- --baseline before-change

# 4. 如果性能下降 > 5%，调查原因
# 5. 如果可接受，更新基线
cargo bench -p rfont -- --save-baseline before-change
```

### 场景 2：优化前后的对比

```bash
# 1. 优化前
cargo bench -p rfont -- --save-baseline before-optimization

# 2. 实施优化
# ... optimization ...

# 3. 优化后对比
cargo bench -p rfont -- --baseline before-optimization

# 4. 生成详细报告
# 查看 target/criterion/*/report/index.html
```

### 场景 3：CI/CD 自动化检查

```bash
# 在 CI 中
if [ -d "target/criterion" ]; then
    # 有基线，进行对比
    cargo bench -p rfont -- --baseline main
    
    # 检查是否有显著退化
    python check_performance_regression.py
else
    # 无基线，建立基线
    cargo bench -p rfont -- --save-baseline main
fi
```

---

## ⚡ 快速参考

### 关键文件

- `crates/rfont/criterion.toml` - Criterion 配置
- `crates/rfont/benches/performance.rs` - 基准测试代码
- `target/criterion/*/base/estimates.json` - 基线数据
- `target/criterion/*/change/estimates.json` - 变化分析
- `target/criterion/report/index.html` - HTML 报告

### 常用命令

```bash
# 建立基线
cargo bench -p rfont -- --save-baseline <name>

# 对比基线
cargo bench -p rfont -- --baseline <name>

# 清除基线
rm -rf target/criterion

# 查看报告
start target/criterion/report/index.html  # Windows
```

### 判断标准

- **< 2%**：忽略（噪声）
- **2-5%**：观察（多次运行）
- **5-10%**：调查（检查代码）
- **> 10%**：警报（必须修复）

---

## 🔗 相关资源

- [Criterion 官方文档](https://bheisler.github.io/criterion.rs/)
- [Criterion 用户指南](https://bheisler.github.io/criterion.rs/book/user_guide/user_guide.html)
- [性能测试最佳实践](https://bheisler.github.io/criterion.rs/book/faq.html)
- [统计学基础](https://bheisler.github.io/criterion.rs/book/statistical_tests.html)
