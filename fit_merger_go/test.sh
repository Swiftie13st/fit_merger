#!/bin/bash

# FIT文件合并工具测试脚本

echo "=== FIT文件合并工具测试 ==="

# 检查工具是否已编译
if [ ! -f "fit_merger" ]; then
    echo "正在编译FIT合并工具..."
    go build -o fit_merger fit_merger.go
    if [ $? -ne 0 ]; then
        echo "编译失败！"
        exit 1
    fi
fi

# 显示帮助信息
echo "显示帮助信息:"
./fit_merger -h

# 检查是否有测试文件
if [ -z "$(ls *.fit 2>/dev/null)" ]; then
    echo "没有找到FIT测试文件。"
    echo "请提供一些FIT文件进行测试。"
    echo "示例: ./test.sh file1.fit file2.fit"
    exit 0
fi

# 如果有参数，使用参数作为测试文件
if [ $# -gt 0 ]; then
    echo "使用提供的文件进行测试: $@"
    ./fit_merger -o test_merged.fit "$@"
    if [ $? -eq 0 ]; then
        echo "测试成功！合并后的文件: test_merged.fit"
        ls -lh test_merged.fit
    else
        echo "测试失败！"
        exit 1
    fi
else
    echo "没有找到命令行参数，跳过实际合并测试。"
    echo "要使用实际文件测试，请运行: ./test.sh file1.fit file2.fit ..."
fi

echo "=== 测试完成 ==="