# rfont 测试套件

本文档记录了 rfont 项目的单元测试覆盖情况。

## 📊 测试统计

| Crate | 测试数量 | 状态 |
|-------|---------|------|
| rfont-types | 130 | ✅ 全部通过 |
| rfont-core | 62 | ✅ 全部通过 |
| rfont | 52 | ✅ 全部通过 |
| rfont-cli | 18 (doctest) | ✅ 全部通过 |
| **总计** | **262** | **✅ 100% 通过** |

*最后更新: 2026-05-08*

## 🧪 测试覆盖详情

### 1. rfont-types (基础类型层)

**文件**: `crates/rfont-types/src/`

#### 主要测试模块

- **primitives.rs** - 基本类型测试（~50 个测试）
  - Tag 类型创建和转换
  - Fixed/FWord/UFixed/UFWord 定点数转换
  - TableRecord/LONGDATETIME 解析
  - 字节序处理和大端序验证

- **io.rs** - I/O 操作测试（~80 个测试）
  - Reader/Writer 读写操作
  - 边界条件检查
  - 错误处理验证
  - 100% 代码覆盖率

### 2. rfont-core (核心逻辑层)

**文件**: `crates/rfont-core/src/tables/`

#### 字体表测试（62 个测试）

- **cmap.rs** - Unicode 映射表测试
  - Format 0/4/12 格式解析
  - Unicode 字符查找
  - 平台/编码智能选择

- **head/maxp/hhea/hmtx** - 字体元数据表测试
  - 魔数验证和字段解析
  - Units per EM 验证
  - 版本 0.5/1.0 格式支持
  - 水平度量计数

- **glyf/loca** - 字形数据表测试
  - 字形轮廓解析
  - 位置索引计算
  - 复合字形支持

- **woff/woff2** - Web 字体格式测试
  - WOFF Header 和表目录
  - WOFF2 Base128 编码
  - 预定义标签验证
  - Brotli 压缩/解压缩

### 3. rfont (应用层)

**文件**: `crates/rfont/src/`

#### 核心功能测试（52 个测试）

- **字体加载** - TTF/WOFF/WOFF2 格式支持
  - 格式自动检测
  - 表解析和验证
  - 懒加载缓存机制

- **子集化** - 字体子集化功能
  - Builder 模式 API
  - 文本/Unicode 范围/字形 ID 输入
  - .notdef 自动包含
  - 字形 ID 去重和排序

- **格式转换** - TTF ↔ WOFF ↔ WOFF2
  - zlib/Brotli 压缩
  - 校验和计算
  - 表目录重组

- **性能优化**
  - Cmap 查找缓存
  - 流式迭代器
  - 分块处理（parallel feature）

- **文档测试** - 19 个 doctest
  - Font API 示例
  - FontSubsetBuilder 用法
  - SubsetOptions 配置

## 🚀 运行测试

### 运行所有测试
```bash
cargo test --workspace
```

### 运行特定 crate 的测试
```bash
# 基础类型测试
cargo test -p rfont-types

# 核心逻辑测试
cargo test -p rfont-core

# 应用层测试
cargo test -p rfont
```

### 运行单个测试
```bash
cargo test test_sfnt_checksum_calculation
```

### 显示详细输出
```bash
cargo test -- --nocapture
```

## 📝 测试策略

### 单元测试原则

1. **独立性**：每个测试用例独立运行，不依赖外部状态
2. **确定性**：测试结果可重复，无随机性
3. **快速执行**：单个测试应在毫秒级完成
4. **清晰断言**：失败时提供明确的错误信息

### 覆盖重点

- ✅ **二进制解析**：验证 Big-Endian 字节序的正确处理
- ✅ **边界条件**：测试空数据、最小值、最大值的处理
- ✅ **错误路径**：验证无效输入的错误返回
- ✅ **业务逻辑**：测试子集化、校验和计算等核心算法

### 集成测试

除了单元测试，项目还包含示例程序作为集成测试：

```bash
# 测试 TTF 和 WOFF 加载及子集化
cargo run --example test_alimama -p rfont
```

## 🔧 未来改进方向

- [ ] **属性测试** - 使用 `proptest` 进行随机化测试
- [ ] **模糊测试** - 对畸形字体文件的鲁棒性测试
- [ ] **更多表测试** - 为 name、OS/2、post 等表添加单元测试
- [ ] **集成测试** - 增加真实字体文件的端到端测试
- [ ] **CI/CD 集成** - 自动化运行测试和覆盖率检查

## ⚠️ 已知限制

- **WOFF2 实现** - 当前使用简化的 Brotli 压缩，未实现完整的表转换（glyf/loca 重构）
- **并行处理** - parallel feature 需要显式启用（`--features parallel`）
- **大字体文件** - 超大字体（>50MB）可能需要更多内存

---

*最后更新: 2026-05-08 (更新测试统计：20 → 262 个测试)*
