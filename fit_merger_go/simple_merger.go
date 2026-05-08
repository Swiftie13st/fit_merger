package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

func main() {
	// 定义命令行参数
	output := flag.String("o", "merged.fit", "输出文件名")
	help := flag.Bool("h", false, "显示帮助信息")

	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "简单FIT文件合并工具\n")
		fmt.Fprintf(os.Stderr, "用法: %s [选项] 文件1.fit 文件2.fit ...\n", filepath.Base(os.Args[0]))
		fmt.Fprintf(os.Stderr, "选项:\n")
		flag.PrintDefaults()
		fmt.Fprintf(os.Stderr, "示例:\n")
		fmt.Fprintf(os.Stderr, "  %s -o merged.fit file1.fit file2.fit file3.fit\n", filepath.Base(os.Args[0]))
		fmt.Fprintf(os.Stderr, "\n注意: 这个工具使用简单的文件合并方法，适用于无法解析的FIT文件\n")
	}

	flag.Parse()

	if *help {
		flag.Usage()
		os.Exit(0)
	}

	// 检查是否有输入文件
	if flag.NArg() == 0 {
		fmt.Fprintf(os.Stderr, "错误: 请提供至少一个FIT文件\n")
		flag.Usage()
		os.Exit(1)
	}

	fmt.Printf("开始合并 %d 个FIT文件到 %s\n", len(flag.Args()), *output)
	start := time.Now()

	// 尝试合并文件
	if err := mergeFITFiles(flag.Args(), *output); err != nil {
		fmt.Fprintf(os.Stderr, "合并失败: %v\n", err)
		os.Exit(1)
	}

	elapsed := time.Since(start)
	fmt.Printf("合并完成，耗时: %v\n", elapsed)
}

// mergeFITFiles 合并多个FIT文件
func mergeFITFiles(files []string, output string) error {
	if len(files) == 0 {
		return fmt.Errorf("没有要合并的文件")
	}

	// 验证所有文件是否存在
	for _, file := range files {
		if _, err := os.Stat(file); os.IsNotExist(err) {
			return fmt.Errorf("文件不存在: %s", file)
		}
	}

	// 创建输出文件
	outFile, err := os.Create(output)
	if err != nil {
		return fmt.Errorf("创建输出文件失败: %v", err)
	}
	defer outFile.Close()

	// 写入文件头
	var totalSize int64
	for i, file := range files {
		fmt.Printf("处理文件 %d/%d: %s\n", i+1, len(files), file)

		// 读取文件内容
		data, err := os.ReadFile(file)
		if err != nil {
			fmt.Printf("警告: 读取文件 %s 失败: %v，跳过此文件\n", file, err)
			continue
		}

		// 如果是第一个文件，写入完整的文件内容（包括头部）
		if i == 0 {
			if _, err := outFile.Write(data); err != nil {
				return fmt.Errorf("写入文件失败: %v", err)
			}
			totalSize = int64(len(data))
		} else {
			// 对于后续文件，尝试跳过头部，只写入数据部分
			// 这是一个简单的合并方法，可能不适用于所有情况
			headerSize := 12 // FIT文件头部大小
			if len(data) > headerSize {
				dataPart := data[headerSize:]
				if _, err := outFile.Write(dataPart); err != nil {
					return fmt.Errorf("写入文件数据失败: %v", err)
				}
				totalSize += int64(len(dataPart))
			}
		}
	}

	fmt.Printf("合并完成，总大小: %d 字节\n", totalSize)
	return nil
}
