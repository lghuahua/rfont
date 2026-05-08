#!/bin/bash
# 快速生成覆盖率报告脚本（使用 cargo-llvm-cov）
# 
# 用法:
#   ./scripts/coverage.sh              # 生成完整覆盖率报告
#   ./scripts/coverage.sh --html       # 生成 HTML 报告
#   ./scripts/coverage.sh --open       # 生成并打开 HTML 报告
#   ./scripts/coverage.sh --json       # 生成 JSON 报告

set -e

# 检查 cargo-llvm-cov 是否安装
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "❌ 错误: cargo-llvm-cov 未安装"
    echo "💡 请运行: cargo install cargo-llvm-cov --locked"
    exit 1
fi

echo "🚀 开始生成覆盖率报告..."
echo ""

# 解析参数
GENERATE_HTML=false
OPEN_HTML=false
GENERATE_JSON=false

for arg in "$@"; do
    case $arg in
        --html)
            GENERATE_HTML=true
            ;;
        --open)
            GENERATE_HTML=true
            OPEN_HTML=true
            ;;
        --json)
            GENERATE_JSON=true
            ;;
    esac
done

# 生成 LCOV 格式（用于 Codecov）
echo "📊 生成 LCOV 报告..."
cargo llvm-cov \
    --workspace \
    --lcov \
    --output-path lcov.info \
    --ignore-filename-regex "tests|benches|examples" \
    --no-fail-fast \
    --quiet

echo "✅ LCOV 报告已生成: lcov.info"
echo ""

# 生成 HTML 报告
if [ "$GENERATE_HTML" = true ]; then
    echo "📊 生成 HTML 报告..."
    cargo llvm-cov \
        --workspace \
        --html \
        --output-dir coverage/html \
        --ignore-filename-regex "tests|benches|examples" \
        --no-fail-fast \
        --quiet
    
    echo "✅ HTML 报告已生成: coverage/html/index.html"
    
    if [ "$OPEN_HTML" = true ]; then
        echo "🌐 正在打开 HTML 报告..."
        if command -v xdg-open &> /dev/null; then
            xdg-open coverage/html/index.html
        elif command -v open &> /dev/null; then
            open coverage/html/index.html
        elif command -v start &> /dev/null; then
            start coverage/html/index.html
        else
            echo "⚠️  无法自动打开浏览器，请手动打开: coverage/html/index.html"
        fi
    fi
    echo ""
fi

# 生成 JSON 报告
if [ "$GENERATE_JSON" = true ]; then
    echo "📊 生成 JSON 报告..."
    cargo llvm-cov \
        --workspace \
        --json \
        --output-path coverage/report.json \
        --ignore-filename-regex "tests|benches|examples" \
        --no-fail-fast \
        --quiet
    
    echo "✅ JSON 报告已生成: coverage/report.json"
    echo ""
fi

# 显示覆盖率摘要
echo "📈 覆盖率摘要:"
cargo llvm-cov \
    --workspace \
    --summary-only \
    --ignore-filename-regex "tests|benches|examples" \
    --no-fail-fast

echo ""
echo "✨ 完成！"
