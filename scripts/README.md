# Scripts 目录

本目录包含用于自动化项目维护的脚本工具。

---

## 📜 可用脚本

### 1. coverage.ps1 / coverage.sh

**功能**: 快速生成代码覆盖率报告（使用 cargo-llvm-cov）

**前置条件**:
- 已安装 `cargo-llvm-cov`: `cargo install cargo-llvm-cov --locked`

#### Windows PowerShell 使用

```powershell
# 生成完整覆盖率报告（LCOV 格式）
.\scripts\coverage.ps1

# 生成 HTML 报告
.\scripts\coverage.ps1 -Html

# 生成并打开 HTML 报告
.\scripts\coverage.ps1 -Open

# 生成 JSON 报告
.\scripts\coverage.ps1 -Json
```

#### Linux/Mac/Git Bash 使用

```bash
# 首次使用需要添加执行权限
chmod +x scripts/coverage.sh

# 生成完整覆盖率报告（LCOV 格式）
./scripts/coverage.sh

# 生成 HTML 报告
./scripts/coverage.sh --html

# 生成并打开 HTML 报告
./scripts/coverage.sh --open

# 生成 JSON 报告
./scripts/coverage.sh --json
```

**输出文件**:
- `lcov.info` - LCOV 格式（用于 Codecov）
- `coverage/html/index.html` - HTML 可视化报告
- `coverage/report.json` - JSON 格式报告

**优势**:
- ⚡ 比 cargo-tarpaulin 快 2-3 倍
- 🎯 更准确的分支覆盖率
- 💾 更低的内存占用
- 🔒 更好的稳定性

---

### 2. generate-changelog.ps1 / generate-changelog.sh

**功能**: 自动生成 CHANGELOG.md 文件

**前置条件**:
- 已安装 `git-cliff`: `cargo install git-cliff`
- Git 提交遵循 Conventional Commits 规范

#### Windows PowerShell 使用

```powershell
# 生成完整 CHANGELOG
.\scripts\generate-changelog.ps1

# 只生成未发布的变更
.\scripts\generate-changelog.ps1 -Unreleased

# 预览而不保存
.\scripts\generate-changelog.ps1 -Preview

# 指定输出文件
.\scripts\generate-changelog.ps1 -Output CHANGELOG_NEW.md
```

#### Linux/Mac/Git Bash 使用

```bash
# 首次使用需要添加执行权限
chmod +x scripts/generate-changelog.sh

# 生成完整 CHANGELOG
./scripts/generate-changelog.sh

# 只生成未发布的变更
./scripts/generate-changelog.sh --unreleased

# 预览而不保存
./scripts/generate-changelog.sh --preview

# 指定输出文件
./scripts/generate-changelog.sh --output CHANGELOG_NEW.md

# 查看帮助
./scripts/generate-changelog.sh --help
```

#### 直接使用 git-cliff

```bash
# 生成完整 CHANGELOG
git-cliff --output CHANGELOG.md

# 只生成未发布的变更
git-cliff --unreleased

# 预览
git-cliff

# 指定标签范围
git-cliff v0.1.0..HEAD

# 自定义配置
git-cliff --config .cliff.toml
```

---

## 🔧 Conventional Commits 规范

为了自动生成准确的 CHANGELOG，请使用规范的提交消息格式：

### 基本格式

```
type(scope): description

[optional body]

[optional footer(s)]
```

### Type 类型

| Type | 说明 | CHANGELOG 分组 |
|------|------|---------------|
| `feat` | 新功能 | Added |
| `fix` | Bug 修复 | Fixed |
| `perf` | 性能优化 | Changed |
| `refactor` | 代码重构 | Changed |
| `docs` | 文档更新 | Documentation |
| `test` | 测试相关 | Testing |
| `chore` | 杂项任务 | Chores |
| `style` | 代码格式 | Style |
| `ci` | CI/CD 相关 | CI/CD |

### Scope（可选）

表示影响的模块或组件，例如：
- `cli` - 命令行工具
- `font` - 字体处理核心
- `rfont` - 主库
- `cmap` - cmap 表
- `subset` - 子集化功能
- `build` - 构建配置

### 示例

```bash
# 新功能
git commit -m "feat: add WOFF2 compression support"
git commit -m "feat(cli): add batch processing command"
git commit -m "feat(rfont): implement parallel processing"

# Bug 修复
git commit -m "fix: correct cmap table parsing error"
git commit -m "fix(subset): fix glyph ID continuity check"

# 性能优化
git commit -m "perf: optimize glyph data processing"
git commit -m "perf(rfont): improve benchmark test accuracy"

# 重构
git commit -m "refactor: restructure font parsing logic"
git commit -m "refactor(cmap): simplify subtable parsing"

# 文档
git commit -m "docs: update API documentation"
git commit -m "docs(rfont): add detailed doc comments"

# 测试
git commit -m "test: add comprehensive unit tests"
git commit -m "test(font): add subset functionality tests"

# 杂项
git commit -m "chore: update dependencies"
git commit -m "chore(build): configure release profile"
git commit -m "chore(project): update license and author info"
```

---

## 📋 工作流程

### 日常开发

1. **提交代码时使用规范格式**
   ```bash
   git add .
   git commit -m "feat(cli): add new feature"
   ```

2. **定期生成 CHANGELOG**
   ```bash
   # Windows
   .\scripts\generate-changelog.ps1
   
   # Linux/Mac
   ./scripts/generate-changelog.sh
   ```

3. **审查并调整**
   - 检查生成的内容是否准确
   - 必要时手动调整分类或描述
   - 确保重要变更都被记录

### 发布新版本

1. **创建 Git Tag**
   ```bash
   git tag v0.2.0
   ```

2. **生成带版本的 CHANGELOG**
   ```bash
   git-cliff --output CHANGELOG.md
   ```

3. **提交 CHANGELOG**
   ```bash
   git add CHANGELOG.md
   git commit -m "docs: update CHANGELOG for v0.2.0"
   ```

4. **推送**
   ```bash
   git push origin main --tags
   ```

---

## ⚙️ 配置文件

### .cliff.toml

git-cliff 的配置文件，定义了：

- **Header/Footer**: CHANGELOG 的头部和尾部模板
- **Body**: 变更内容的格式化模板
- **Commit Parsers**: 如何将提交消息映射到 CHANGELOG 分组
- **Tag Pattern**: Git 标签的匹配规则

主要配置项：

```toml
[git]
conventional_commits = true  # 启用 Conventional Commits 解析

[git.commit_parsers]
{ message = "^feat", group = "Added" }
{ message = "^fix", group = "Fixed" }
{ message = "^perf", group = "Changed" }
{ message = "^refactor", group = "Changed" }
{ message = "^docs", group = "Documentation" }
{ message = "^test", group = "Testing" }
{ message = "^chore", group = "Chores" }
```

详细配置请参考 [.cliff.toml](../.cliff.toml) 和 [git-cliff 文档](https://git-cliff.org/docs/configuration)。

---

## 💡 最佳实践

### ✅ DO

1. **始终使用规范的提交消息**
   - 清晰的 type 和 scope
   - 简洁明了的描述
   - 必要时添加详细说明

2. **定期生成 CHANGELOG**
   - 每次 PR 合并后更新
   - 发布前必须生成

3. **审查自动生成的内容**
   - 检查分类是否正确
   - 补充必要的上下文
   - 标注破坏性变更

4. **保持提交原子性**
   - 一个提交只做一件事
   - 避免混合多个不相关的变更

### ❌ DON'T

1. **不要使用模糊的提交消息**
   ```bash
   # 不好
   git commit -m "fix bug"
   git commit -m "update code"
   
   # 好
   git commit -m "fix: correct cmap table parsing for Unicode characters"
   git commit -m "refactor: simplify font loading logic"
   ```

2. **不要忽略 scope**
   ```bash
   # 不够清晰
   git commit -m "feat: add new feature"
   
   # 更清晰
   git commit -m "feat(cli): add batch processing command"
   ```

3. **不要累积大量提交后再生成**
   - 及时生成，便于审查
   - 避免遗漏重要变更

---

## 🔗 相关资源

- [git-cliff 官方文档](https://git-cliff.org/)
- [Conventional Commits 规范](https://www.conventionalcommits.org/)
- [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)
- [语义化版本](https://semver.org/lang/zh-CN/)
- [CHANGELOG 维护指南](../CHANGELOG_GUIDE.md)

---

*最后更新: 2026-05-08*
