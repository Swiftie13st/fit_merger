# FIT文件合并工具使用示例

## 项目集成

### 1. 将工具集成到主项目中

在项目的 `tools` 目录下已经创建了 `fit_merger` 子目录，包含完整的FIT文件合并功能。

### 2. 编译工具

```bash
cd tools/fit_merger
go build -o fit_merger fit_merger.go
```

### 3. 基本使用

```bash
# 合并多个FIT文件
./fit_merger file1.fit file2.fit file3.fit

# 指定输出文件名
./fit_merger -o combined_activity.fit morning_ride.fit afternoon_ride.fit
```

### 4. 在Go代码中使用

虽然主要设计为命令行工具，但也可以在Go代码中直接使用：

```go
package main

import (
    "fmt"
    "mockserver/tools/fit_merger"
)

func main() {
    // 创建合并器
    merger := fit_merger.NewFITMerger("output.fit")
    
    // 添加文件
    merger.AddFile("activity1.fit")
    merger.AddFile("activity2.fit")
    
    // 执行合并
    if err := merger.Merge(); err != nil {
        fmt.Printf("合并失败: %v\n", err)
        return
    }
    
    fmt.Println("合并成功！")
}
```

## 实际应用场景

### 场景1：多日骑行合并
```bash
./fit_merger -o tour_de_france.fit day1.fit day2.fit day3.fit day4.fit day5.fit
```

### 场景2：分段训练合并
```bash
./fit_merger -o complete_workout.fit warmup.fit main_set.fit cooldown.fit
```

### 场景3：多人活动合并
```bash
./fit_merger -o group_ride.fit rider1.fit rider2.fit rider3.fit
```

## 注意事项

1. **文件格式**：确保所有输入文件都是有效的FIT格式
2. **时间排序**：工具会自动按时间戳排序所有记录
3. **数据完整性**：合并后的文件将包含所有原始数据
4. **文件大小**：合并多个大文件可能会产生较大的输出文件

## 故障排除

- 如果遇到"文件不存在"错误，请检查文件路径是否正确
- 如果遇到"解码失败"错误，请确认文件是否为有效的FIT格式
- 如果遇到"内存不足"错误，请尝试减少一次合并的文件数量

## 扩展功能

未来可以考虑添加的功能：
- 数据过滤（按时间、距离、心率区间等）
- 数据清理（去除重复点、平滑轨迹等）
- 统计分析（总距离、总时间、平均速度等）
- 格式转换（FIT转GPX、TCX等）