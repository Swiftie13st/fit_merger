# fit_merger

一个用 Rust 编写的 FIT 文件合并 / 检视工具，专为骑行多日记录合并而设计。

## 功能

1. **合并多个 FIT 文件为单一会话**
   - 完整保留所有原始字段：距离 / 时间 / 速度 / 海拔 / 心率 / 踏频 / 功率 / GPS / 温度 / 厂商私有字段……
   - 重写 `file_id` / `session` / `activity` 三类摘要消息：
     - 单一 `session`：累加距离 / 计时 / 耗时 / 卡路里 / 爬升 / 下降；按 `timer_time` 加权得出 avg；逐项取 max
     - 单一 `activity`：`num_sessions = 1`，`total_timer_time` 与合并后 `session` 一致
     - `file_id`：以最早 `time_created` 为准
   - 输出文件含合规的 14 字节文件头 CRC + 文件末尾 CRC，可被 Garmin Connect / Strava / fitparser 等工具直接读取

2. **inspect 子命令**：打印任意 FIT 文件中的会话摘要，便于对照核查
   - 距离（km）、耗时 / 计时（hh:mm:ss）、卡路里、爬升 / 下降
   - 平均 / 最大速度（km/h）、心率（bpm）、踏频（rpm）、功率（W）

## 构建

```bash
cd fit_merger
cargo build --release
```

可执行文件位于 `target/release/fit_merger`。

## 使用

### 合并

显式指定输入与输出：

```bash
./target/release/fit_merger \
    in1.fit in2.fit in3.fit ... merged.fit
```

无参数模式（自动扫描 `../fit_files/*.fit`，跳过文件名以 `merged` 开头者，输出到 `../fit_files/merged.fit`）：

```bash
./target/release/fit_merger
```

### 查看会话摘要

```bash
./target/release/fit_merger inspect file.fit [file2.fit ...]
```

输出示例：

```text
📄 文件: ../fit_files/公路骑行20260504094905.fit
  会话 1: 距离=160.906 km, 耗时=05:47:47, 计时=05:04:52, 卡路里=2688 kcal,
          上升=270 m, 下降=217 m, 平均速度=31.67 km/h, 最大速度=46.68 km/h,
          平均心率=137 bpm, 最大心率=176 bpm, 平均踏频=85 rpm, 最大踏频=111 rpm,
          平均功率=135 W, 最大功率=799 W

📄 文件: ../fit_files/merged.fit
  会话 1: 距离=1020.109 km, 耗时=45:07:00, 计时=37:45:23, 卡路里=17500 kcal,
          上升=1050 m, 下降=953 m, 平均速度=27.01 km/h, 最大速度=48.56 km/h,
          平均心率=134 bpm, 最大心率=179 bpm, 平均踏频=80 rpm, 最大踏频=123 rpm,
          平均功率=118 W, 最大功率=799 W
```

### 合并效果

原始的码表数据：
![fit_files](https://raw.githubusercontent.com/Swiftie13st/Figurebed/main/img/2026050704.png)
合并后fit文件上传到strava:
![before](https://raw.githubusercontent.com/Swiftie13st/Figurebed/main/img/2026050701.png)

![after](https://raw.githubusercontent.com/Swiftie13st/Figurebed/main/img/2026050702.png)
最终的Strava结果:
![strava](https://raw.githubusercontent.com/Swiftie13st/Figurebed/main/img/2026050703.png)

### GPX 转 FIT

显式指定输出：

```bash
./target/release/fit_merger gpx2fit ride.gpx ride.fit
```

省略输出时，默认与输入同目录同名，仅扩展名改为 `.fit`：

```bash
./target/release/fit_merger gpx2fit ride.gpx     # 生成 ride.fit
```

转换说明：

- 必须包含 `<time>` 的轨迹点；缺失时间的点会被跳过，无法组成递增时间序列的点也会被跳过（命令会输出跳过数量）
- 时间按 UTC 写入 FIT；带 `+08:00` 等时区偏移的 GPX 会先换算为 UTC
- 若 GPX 含 `gpxdata:distance` / `gpxdata:speed`，记录与汇总优先采用设备值；否则用 haversine 累加距离、相邻点时间差计算速度
- `session.total_timer_time` 会剔除长采样空档（自动暂停），`session.total_elapsed_time` 保留完整首尾时间差；平均速度 = 总距离 / 计时时间
- 平均踏频只统计非 0 值；累计爬升 / 下降使用 2 m 死区，减少 1 m 高程量化带来的抖动
- 卡路里只读取 lap / metadata 级 `<gpxdata:calories>`（逐点 calories 不处理）：多 lap 时求和，无 lap 值时使用 metadata 单值，写入 FIT 的 session / lap `total_calories`
- GPX 中没有功率扩展时，输出 FIT 也不会包含功率字段
- FIT 速度 / 距离 / 海拔使用协议规定的缩放：速度 1/1000 m/s、距离 1/100 m、海拔 1/5 m + 500 m 偏移

输出示例：

```text
正在转换 GPX → FIT: ride.gpx → ride.fit
✅ 转换成功：3 个轨迹点 → 生成 3 条 record（跳过缺时间 0 个 / 时间未递增 0 个）
   距离=0.222 km, 耗时=00:00:20, 计时=00:00:20, 运动类型=cycling
```

### 帮助

```bash
./target/release/fit_merger --help
```

## 项目结构

```text
fit_merger/
├── Cargo.toml
├── src/
│   ├── fit_types.rs        # FIT 协议字节级数据结构
│   ├── fit_parser.rs       # 解析器（保留原始 payload 字节）
│   ├── fit_generator.rs    # 生成器（含正确的 FIT CRC-16）
│   ├── gpx.rs              # GPX 解析与 GPX → FIT 转换
│   ├── merger.rs           # 多文件合并逻辑
│   ├── inspector.rs        # session 摘要提取与格式化
│   ├── lib.rs
│   └── main.rs             # CLI 入口
└── tests/
    ├── validate_merged.rs  # 用 fitparser 第三方库严格校验合并文件
    ├── gpx_to_fit.rs       # 用 fitparser 严格校验 GPX 转换结果（CRC / 字段 / 汇总）
    └── dump_summary.rs     # 输出合并文件全部 session/activity 字段
```

## 设计要点

- **字节级合并**：parser 仅解析文件头 / 记录头 / 定义消息，数据消息整体复制原始字节，避免任何字段缩放 / 单位转换的失真。
- **LMT 重映射**：跨文件合并时，每条 Definition 重新分配 `local_message_type`（0..=12 循环，13/14/15 保留给 activity / session / file_id），避免不同源文件 LMT 冲突。
- **CRC**：严格按 FIT SDK 4-bit 表查表算法计算文件头 CRC 与末尾 CRC，符合协议要求。
- **session 字段号**：严格遵循 FIT SDK profile 定义（`avg_speed=14`，`max_speed=15`，`avg_heart_rate=16`，`max_heart_rate=17`，`avg_cadence=18`，`max_cadence=19`，`avg_power=20`，`max_power=21` 等）。


## 测试

```bash
# 单元测试（CRC 算法、时长格式化）
cargo test --release --lib

# 集成测试：用 fitparser 0.10 校验 merged.fit 是 CRC 合法、单一 session
cargo test --release --test validate_merged -- --nocapture

# 输出合并文件 session/activity 全部字段
cargo test --release --test dump_summary -- --nocapture

# GPX → FIT 集成测试（CRC、record 数、session 汇总、扩展字段）
cargo test --release --test gpx_to_fit -- --nocapture
```

## 兼容性

合并产出的 `.fit` 文件已通过：

- [fitparser 0.10](https://crates.io/crates/fitparser) 严格 CRC 校验
- 字段保留检查：record 中 distance / heart_rate / cadence / power / altitude 全部存在

可直接导入 Garmin Connect、Strava、TrainingPeaks 等支持 FIT 格式的运动平台。
