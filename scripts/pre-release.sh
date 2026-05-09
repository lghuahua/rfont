#!/usr/bin/env bash
# scripts/pre-release.sh
# cargo-release pre-release hook 脚本（纯 bash，跨平台）

set -e  # 遇到错误立即退出

# 获取脚本所在目录的父目录（workspace 根目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# 获取版本号（由 cargo-release 设置）
NEW_VERSION="${NEW_VERSION:-unknown}"
DATE=$(date +%Y-%m-%d)

# 防止重复执行：检查 CHANGELOG 中是否已包含当前版本
if [ -f "CHANGELOG.md" ] && grep -q "## \[$NEW_VERSION\]" CHANGELOG.md; then
    echo "⚠️  Version $NEW_VERSION already in CHANGELOG, skipping..."
    exit 0
fi

echo "=========================================="
echo "  Pre-release hook starting"
echo "  Version: v${NEW_VERSION}"
echo "  Project root: ${PROJECT_ROOT}"
echo "=========================================="

# 确保 PATH 包含 cargo bin 目录
export PATH="$PATH:$HOME/.cargo/bin"

# 步骤 1: 使用 git-cliff 生成 CHANGELOG
echo ""
echo "📝 Generating CHANGELOG from git history..."
if ! command -v git-cliff &> /dev/null; then
    echo "❌ git-cliff not found. Installing..."
    cargo install git-cliff
fi

# 生成未发布的内容到临时文件（不包含 header 和 footer）
git-cliff --unreleased --strip all --output CHANGELOG_UNRELEASED.md
echo "✅ CHANGELOG content generated"

# 步骤 2: 更新 CHANGELOG.md
echo ""
echo "🔖 Updating CHANGELOG.md..."

# 检查文件是否存在
if [ ! -f "CHANGELOG.md" ]; then
    echo "❌ Error: CHANGELOG.md not found"
    exit 1
fi

if [ ! -f "CHANGELOG_UNRELEASED.md" ]; then
    echo "❌ Error: CHANGELOG_UNRELEASED.md not found"
    exit 1
fi

# 读取临时文件内容并清理
NEW_CONTENT=$(cat CHANGELOG_UNRELEASED.md)

# 移除第一行（## [Unreleased]）和开头的空行
NEW_CONTENT=$(echo "$NEW_CONTENT" | sed '1{/^## \[Unreleased\]/d}' | sed '/./,$!d')

# 创建版本标题
VERSION_HEADER="## [$NEW_VERSION] - $DATE"

# 在 [Unreleased] 后面插入新版本
if grep -q "## \[Unreleased\]" CHANGELOG.md; then
    # 使用 awk 在 [Unreleased] 后插入内容
    awk -v header="$VERSION_HEADER" -v content="$NEW_CONTENT" '
    /## \[Unreleased\]/ {
        print $0
        print ""
        print header
        print ""
        print content
        next
    }
    {print}
    ' CHANGELOG.md > CHANGELOG_NEW.md
    
    mv CHANGELOG_NEW.md CHANGELOG.md
    echo "✅ CHANGELOG.md updated"
else
    echo "⚠️  Warning: [Unreleased] marker not found"
fi

# 清理临时文件
rm -f CHANGELOG_UNRELEASED.md

echo ""
echo "=========================================="
echo "  Pre-release hook completed successfully!"
echo "=========================================="