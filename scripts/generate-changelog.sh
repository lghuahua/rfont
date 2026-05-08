#!/bin/bash
# Shell 脚本：自动生成 CHANGELOG
# 使用方法: ./scripts/generate-changelog.sh

set -e

# 颜色定义
CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
GRAY='\033[0;37m'
NC='\033[0m' # No Color

echo -e "${CYAN}🔧 正在生成 CHANGELOG...${NC}"

# 检查 git-cliff 是否安装
if ! command -v git-cliff &> /dev/null; then
    echo -e "${RED}❌ 错误: git-cliff 未安装${NC}"
    echo -e "${YELLOW}请运行: cargo install git-cliff${NC}"
    exit 1
fi

# 解析参数
UNRELEASED=false
PREVIEW=false
OUTPUT="CHANGELOG.md"

while [[ $# -gt 0 ]]; do
    case $1 in
        --unreleased|-u)
            UNRELEASED=true
            shift
            ;;
        --preview|-p)
            PREVIEW=true
            shift
            ;;
        --output|-o)
            OUTPUT="$2"
            shift 2
            ;;
        --help|-h)
            echo "用法: $0 [选项]"
            echo ""
            echo "选项:"
            echo "  -u, --unreleased  只生成未发布的变更"
            echo "  -p, --preview     预览而不保存"
            echo "  -o, --output FILE 指定输出文件路径 (默认: CHANGELOG.md)"
            echo "  -h, --help        显示帮助信息"
            exit 0
            ;;
        *)
            echo -e "${RED}未知参数: $1${NC}"
            exit 1
            ;;
    esac
done

# 构建命令
CMD="git-cliff"
ARGS=()

if [ "$UNRELEASED" = true ]; then
    ARGS+=("--unreleased")
    echo -e "${GREEN}📝 生成未发布的变更...${NC}"
else
    echo -e "${GREEN}📝 生成完整 CHANGELOG...${NC}"
fi

if [ "$PREVIEW" = true ]; then
    # 预览模式：直接输出到控制台
    $CMD "${ARGS[@]}"
else
    # 保存到文件
    ARGS+=("--output" "$OUTPUT")
    $CMD "${ARGS[@]}"
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ CHANGELOG 已生成: $OUTPUT${NC}"
        
        # 显示文件大小
        SIZE=$(du -h "$OUTPUT" | cut -f1)
        echo -e "${CYAN}📊 文件大小: $SIZE${NC}"
        
        # 显示行数
        LINES=$(wc -l < "$OUTPUT")
        echo -e "${CYAN}📄 总行数: $LINES${NC}"
    else
        echo -e "${RED}❌ 生成失败${NC}"
        exit 1
    fi
fi

echo ""
echo -e "${YELLOW}💡 提示:${NC}"
echo -e "${GRAY}  - 使用 -u/--unreleased 参数只生成未发布的变更${NC}"
echo -e "${GRAY}  - 使用 -p/--preview 参数预览而不保存${NC}"
echo -e "${GRAY}  - 使用 -o/--output 指定输出文件路径${NC}"
