# Justfile for rfont project
# 使用 just 命令管理器替代手动脚本
# 安装: cargo install just
# 使用: just <recipe-name>

# 默认配方
default:
    just --list

# ==================== 开发工作流 ====================

# 快速开始 - 格式化、检查、测试（自动修复格式）
dev:
    just fmt
    just clippy-fix
    just test

# 代码质量检查（不修改代码，用于 CI/预提交）
check:
    just fmt-check
    just clippy-strict
    just test-fast

# 全面检查（包含完整测试，用于推送前）
check-all:
    just fmt-check
    just clippy-strict
    just test
    @echo "✅ 所有检查通过"

# ==================== Git 操作 ====================

# 推送前全面检查
push-check:
    @echo "🔍 开始推送前检查..."
    just check-all
    @echo "✅ 可以安全推送"

# 推送到远程仓库（带检查）
push:
    @echo "🚀 准备推送到远程仓库..."
    just push-check
    @echo ""
    @echo "📤 正在推送..."
    git push
    @echo "✅ 推送成功！"

# 强制推送（危险操作，需确认）
push-force:
    @echo "⚠️  警告：即将执行强制推送！"
    @echo "这会覆盖远程历史，请确保你了解后果。"
    @echo ""
    just check-all
    @echo ""
    @read -p "确认强制推送？(yes/no): " confirm && [ "$${confirm}" = "yes" ]
    git push --force
    @echo "✅ 强制推送完成"

# 推送到指定分支
push-branch branch:
    @echo "🚀 推送到分支: {{branch}}"
    just check-all
    git push origin {{branch}}
    @echo "✅ 推送到 {{branch}} 成功"

# ==================== 代码质量 ====================

# 格式化所有代码
fmt:
    cargo fmt --all

# 检查代码格式（不修改）
fmt-check:
    cargo fmt --all -- --check

# 运行 clippy 并自动修复
clippy-fix:
    cargo clippy --fix --allow-dirty --allow-staged
    cargo fmt --all

# 运行 clippy 严格检查（CI 使用）
clippy-strict:
    cargo clippy --workspace -- -D warnings

# 运行 clippy（宽松模式，显示警告）
clippy:
    cargo clippy --workspace

# ==================== 测试 ====================

# 运行所有测试
test:
    cargo test --workspace

# 快速测试（跳过 doc tests）
test-fast:
    cargo test --workspace --lib

# 运行特定 crate 的测试
test-crate crate:
    cargo test -p {{crate}}

# 查看详细测试输出
test-verbose:
    cargo test --workspace -- --nocapture

# 运行基准测试
bench:
    cargo bench --workspace

# ==================== 覆盖率 ====================

# 生成代码覆盖率报告（使用 cargo-llvm-cov）
coverage:
    cargo llvm-cov --workspace --lcov --output-path lcov.info
    @echo "✅ LCOV 报告已生成: lcov.info"

# 生成 HTML 覆盖率报告
coverage-html:
    cargo llvm-cov --workspace --html
    @echo "✅ HTML 报告已生成: target/llvm-cov/html/index.html"

# 生成并打开 HTML 报告
coverage-open:
    cargo llvm-cov --workspace --html --open

# 生成 JSON 覆盖率报告
coverage-json:
    cargo llvm-cov --workspace --json --output-path coverage/report.json
    @echo "✅ JSON 报告已生成: coverage/report.json"

# 显示覆盖率摘要
coverage-summary:
    cargo llvm-cov --workspace --summary-only

# ==================== 文档 ====================

# 构建文档
doc:
    cargo doc --workspace --no-deps

# 构建并打开文档
doc-open:
    cargo doc --workspace --no-deps --open

# 检查文档链接
doc-check:
    cargo doc --workspace --no-deps --document-private-items

# ==================== 构建 ====================

# Debug 构建（不包含桌面应用）
build:
    cargo build --workspace

# Release 构建（不包含桌面应用）
build-release:
    cargo build --release --workspace

# 只构建 CLI 工具
alias cli := build-cli
build-cli:
    cargo build --release -p rfont-cli

# 构建完整的桌面应用（前端 + 后端）
alias app := build-desktop
build-desktop:
    cd rfont-desktop && pnpm tauri build

# 清理构建产物
clean:
    cargo clean

# ==================== CHANGELOG ====================

# 生成完整 CHANGELOG
changelog:
    git-cliff --output CHANGELOG.md
    @echo "✅ CHANGELOG.md 已生成"

# 生成未发布的变更
changelog-unreleased:
    git-cliff --unreleased
    @echo "✅ 未发布变更已显示"

# 预览 CHANGELOG（不保存）
changelog-preview:
    git-cliff

# ==================== 发布流程 ====================

# 预发布检查
pre-release-check:
    just check-all
    just coverage-summary
    @echo "✅ 所有检查通过，可以发布"

# 完整发布流程
release version="patch":
    @echo "⚠️  即将执行发布流程..."
    @echo "版本: {{version}}"
    just pre-release-check
    @echo ""
    @echo "📦 开始发布..."
    cargo release --workspace {{version}} --execute --no-confirm -v

# ==================== 实用工具 ====================

# 查看项目信息
info:
    @echo "项目名称: rfont"
    @echo "版本: $(cargo metadata --format-version 1 | jq '.packages[] | select(.name == "rfont") | .version')"
    @echo "Rust 版本: $(rustc --version)"
    @echo "Cargo 版本: $(cargo --version)"

# 更新依赖
update:
    cargo update

# 检查过时依赖
outdated:
    cargo outdated

# 审计安全漏洞
audit:
    cargo audit

# 运行所有示例
examples:
    cargo run --example font_info -p rfont
    cargo run --example builder_demo -p rfont
    cargo run --example error_handling_demo -p rfont
    cargo run --example format_detection_demo -p rfont
    cargo run --example perf_comparison -p rfont
    cargo run --example test_alimama -p rfont

# ==================== CI/CD 相关 ====================

# 模拟 CI 检查
ci-check:
    just check
    just doc-check
    @echo "✅ CI 检查全部通过"

# ==================== 帮助 ====================

# 显示常用命令
help:
    @echo "rfont 项目管理工具 (just)"
    @echo ""
    @echo "📦 开发工作流:"
    @echo "  just dev              - 格式化、修复、测试（自动修复）"
    @echo "  just check            - 代码质量检查（快速，用于 CI）"
    @echo "  just check-all        - 全面检查（完整测试，用于推送）"
    @echo ""
    @echo "🔍 代码质量:"
    @echo "  just fmt              - 格式化代码"
    @echo "  just clippy-fix       - 修复 clippy 警告"
    @echo "  just clippy-strict    - 严格检查（CI）"
    @echo ""
    @echo "🧪 测试:"
    @echo "  just test             - 运行所有测试"
    @echo "  just test-fast        - 快速测试"
    @echo "  just bench            - 运行基准测试"
    @echo ""
    @echo "📊 覆盖率:"
    @echo "  just coverage         - 生成 LCOV 报告"
    @echo "  just coverage-html    - 生成 HTML 报告"
    @echo "  just coverage-open    - 生成并打开 HTML"
    @echo ""
    @echo "📝 文档:"
    @echo "  just doc              - 构建文档"
    @echo "  just doc-open         - 构建并打开文档"
    @echo ""
    @echo "🔧 构建:"
    @echo "  just build                  - Debug 构建（不含桌面应用）"
    @echo "  just build-release          - Release 构建（不含桌面应用）"
    @echo "  just build-cli              - 只构建 CLI"
    @echo "  just build-desktop          - 构建完整桌面应用（前端+后端）"
    @echo ""
    @echo "📋 CHANGELOG:"
    @echo "  just changelog        - 生成 CHANGELOG"
    @echo "  just changelog-preview - 预览变更"
    @echo ""
    @echo "🚀 发布:"
    @echo "  just pre-release-check   - 发布前检查"
    @echo "  just release             - 完整发布流程（默认 patch）"
    @echo "  just release minor       - 次要版本更新"
    @echo "  just release major       - 主要版本更新"
    @echo "  just release 0.2.0       - 指定具体版本号"
    @echo ""
    @echo "📤 Git 操作:"
    @echo "  just push-check       - 推送前全面检查"
    @echo "  just push             - 推送到远程仓库（带检查）"
    @echo "  just push-branch main - 推送到指定分支"
    @echo ""
    @echo "💡 提示: 运行 'just --list' 查看所有可用命令"
