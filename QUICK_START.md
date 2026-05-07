# FIT文件合并工具 - 快速开始指南

## 🚀 快速开始

### 1. 进入工具目录
```bash
cd tools/fit_merger
```

### 2. 安装依赖
```bash
go mod tidy
```

### 3. 使用智能转换器（推荐）
```bash
# 合并当前目录下的所有FIT文件
./fit_converter.sh merged.fit *.fit

# 合并指定文件
./fit_converter.sh merged.fit file1.fit file2.fit file3.fit
```

## 🔧 合并器类型选择

### 增强版合并器（推荐，针对上传平台优化）
```bash
# 使用增强版合并器（默认）
./fit_converter.sh -t enhanced merged.fit *.fit

# 或直接使用统一合并器
./unified_fit_merger -type enhanced -o merged.fit *.fit
```

### 现代合并器（单一会话）
```bash
./fit_converter.sh -t modern merged.fit *.fit
./unified_fit_merger -type modern -o merged.fit *.fit
```

### 简单合并器（保留原始会话）
```bash
./fit_converter.sh -t simple merged.fit *.fit
./unified_fit_merger -type simple -o merged.fit *.fit
```

## 📊 验证结果

### 查看合并结果
```bash
# 使用统一合并器查看详细信息
./unified_fit_merger -type enhanced -o merged.fit *.fit
```

### 预期输出示例
```
开始合并 6 个FIT文件到 merged.fit
使用合并器类型: enhanced
解析文件: fit_files/保定市公路骑行20260504094905.fit
  成功提取: 记录=18293, 会话=1, 圈数=4, 事件=92
  记录数据字段: 距离
...
=== 增强合并结果 ===
合并后会话数: 1 (合并为单一会话)
合并后总距离: 1020108.93 m (1020.11 km)
合并后总时间: 162420.38 s (2707.01 min)
合并后总卡路里: 17500 kcal
合并后总上升: 1050 m
合并后总下降: 953 m

=== 详细数据分析 ===
总记录数: 135930
功率数据: ✅ 有 (135228 条记录)
踏频数据: ✅ 有 (135651 条记录)
心率数据: ✅ 有 (135791 条记录)
速度数据: ✅ 有 (135924 条记录)
距离数据: ✅ 有 (135930 条记录)
海拔数据: ✅ 有 (135924 条记录)
合并完成，耗时: 206.1545ms
```

## 🎯 解决上传平台兼容性问题

如果上传平台显示的数据与预期不符，请使用增强版合并器：

```bash
# 增强版合并器专门针对上传平台兼容性优化
./fit_converter.sh -t enhanced merged.fit *.fit
```

## 🎯 解决功率、踏频等详细数据丢失问题

如果合并后发现功率、踏频等详细数据丢失，请使用最新版本的工具：

```bash
# 最新版本已修复详细数据保留问题
./fit_converter.sh -t enhanced merged.fit *.fit
```

## 📁 文件结构

```
fit_merger/
├── fit_converter.sh          # 智能转换器（推荐，保留所有详细数据）
├── unified_fit_merger.go       # 统一合并器（整合所有功能）
├── go.mod                    # Go依赖管理
├── README.md                 # 详细文档
└── fit_files/                # 测试用FIT文件
```

## ⚡ 一键测试

```bash
# 运行完整测试
./test_all.sh
```

## 🛠️ 故障排除

### 编译错误
```bash
# 确保依赖正确安装
go mod tidy
go build -o unified_fit_merger unified_fit_merger.go
```

### 功率、踏频等详细数据丢失
```bash
# 使用最新版本的增强版合并器
./fit_converter.sh -t enhanced merged.fit *.fit
```

### 上传平台不兼容
```bash
# 使用增强版合并器
./fit_converter.sh -t enhanced merged.fit *.fit
```

### 查看帮助
```bash
./fit_converter.sh -h
./unified_fit_merger -h