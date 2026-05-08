#!/bin/bash

# FIT文件智能转换器
# 使用统一合并器，支持多种合并策略，保留所有详细数据

set -e

# 显示使用帮助
show_help() {
    echo "FIT文件智能转换器 - 保留所有详细数据（功率、踏频、心率等）"
    echo "用法: $0 [选项] [输出文件] [输入文件1] [输入文件2] ..."
    echo "选项:"
    echo "  -t, --type TYPE  合并器类型: simple, modern, enhanced (默认: enhanced)"
    echo "  -h, --help       显示帮助"
    echo "示例: $0 merged.fit file1.fit file2.fit file3.fit"
    echo "      $0 -t enhanced merged.fit *.fit"
    exit 1
}

# 默认参数
MERGER_TYPE="enhanced"

# 解析参数
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--type)
            MERGER_TYPE="$2"
            shift 2
            ;;
        -h|--help)
            show_help
            ;;
        -*)
            echo "错误: 未知选项 $1"
            show_help
            ;;
        *)
            break
            ;;
    esac
done

# 检查参数
if [ $# -lt 1 ]; then
    show_help
fi

OUTPUT_FILE="$1"
shift
INPUT_FILES=("$@")

echo "FIT文件合并工具 - 统一合并器（保留所有详细数据）"
echo "合并器类型: $MERGER_TYPE"
echo "输入文件数量: ${#INPUT_FILES[@]}"
echo "输出文件: $OUTPUT_FILE"
echo "注意：将合并为单一会话的连续活动，保留功率、踏频、心率等所有详细数据"

# 检查是否安装了Go
if ! command -v go &> /dev/null; then
    echo "错误: 未找到Go，请先安装Go"
    exit 1
fi

# 检查依赖
echo "检查依赖..."
if [ ! -f go.mod ]; then
    echo "初始化Go模块..."
    go mod init fit_merger
fi

# 检查是否已安装依赖
if ! grep -q "github.com/muktihari/fit" go.mod 2>/dev/null; then
    echo "安装依赖..."
    go get github.com/muktihari/fit@latest
fi

# 构建统一合并器
echo "构建统一合并器..."
go build -o unified_fit_merger unified_fit_merger.go

# 使用统一合并器
echo "使用统一合并器（$MERGER_TYPE）..."
if ./unified_fit_merger -type "$MERGER_TYPE" -o "$OUTPUT_FILE" "${INPUT_FILES[@]}"; then
    echo "✓ 统一合并器成功完成（类型: $MERGER_TYPE）"
    echo "✓ 已保留所有详细数据：功率、踏频、心率、速度、距离、海拔"
else
    echo "✗ 统一合并器失败"
    exit 1
fi

echo "合并完成: $OUTPUT_FILE"