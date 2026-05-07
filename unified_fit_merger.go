package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"time"

	"github.com/muktihari/fit/decoder"
	"github.com/muktihari/fit/encoder"
	"github.com/muktihari/fit/profile/filedef"
	"github.com/muktihari/fit/profile/mesgdef"
	"github.com/muktihari/fit/profile/typedef"
)

// UnifiedFITMerger 统一版FIT合并器，整合所有功能
type UnifiedFITMerger struct {
	files      []string
	output     string
	mergerType string
	records    []*mesgdef.Record
	sessions   []*mesgdef.Session
	laps       []*mesgdef.Lap
	events     []*mesgdef.Event
}

// NewUnifiedFITMerger 创建新的统一FIT合并器
func NewUnifiedFITMerger(output string, mergerType string) *UnifiedFITMerger {
	return &UnifiedFITMerger{
		files:      make([]string, 0),
		output:     output,
		mergerType: mergerType,
		records:    make([]*mesgdef.Record, 0),
		sessions:   make([]*mesgdef.Session, 0),
		laps:       make([]*mesgdef.Lap, 0),
		events:     make([]*mesgdef.Event, 0),
	}
}

// AddFile 添加要合并的FIT文件
func (fm *UnifiedFITMerger) AddFile(file string) {
	fm.files = append(fm.files, file)
}

// Merge 执行合并操作
func (fm *UnifiedFITMerger) Merge() error {
	if len(fm.files) == 0 {
		return fmt.Errorf("没有要合并的文件")
	}

	// 验证所有文件是否存在
	for _, file := range fm.files {
		if _, err := os.Stat(file); os.IsNotExist(err) {
			return fmt.Errorf("文件不存在: %s", file)
		}
	}

	// 解析所有文件
	if err := fm.parseAllFiles(); err != nil {
		return fmt.Errorf("解析文件失败: %v", err)
	}

	// 根据合并器类型选择合并策略
	switch fm.mergerType {
	case "enhanced":
		if err := fm.mergeEnhanced(); err != nil {
			return fmt.Errorf("增强合并失败: %v", err)
		}
	case "modern":
		if err := fm.mergeModern(); err != nil {
			return fmt.Errorf("现代合并失败: %v", err)
		}
	case "simple":
		if err := fm.mergeSimple(); err != nil {
			return fmt.Errorf("简单合并失败: %v", err)
		}
	default:
		return fmt.Errorf("未知的合并器类型: %s", fm.mergerType)
	}

	// 写入合并后的文件
	if err := fm.writeMergedFile(); err != nil {
		return fmt.Errorf("写入合并文件失败: %v", err)
	}

	return nil
}

// parseAllFiles 解析所有FIT文件
func (fm *UnifiedFITMerger) parseAllFiles() error {
	successCount := 0
	for _, file := range fm.files {
		fmt.Printf("解析文件: %s\n", file)

		// 打开文件
		f, err := os.Open(file)
		if err != nil {
			fmt.Printf("警告: 打开文件 %s 失败: %v，跳过此文件\n", file, err)
			continue
		}
		defer f.Close()

		// 创建监听器
		listener := filedef.NewListener()
		defer listener.Close()

		// 创建解码器
		dec := decoder.New(f,
			decoder.WithMesgListener(listener),
			decoder.WithBroadcastOnly(),
		)

		// 解码文件
		_, err = dec.Decode()
		if err != nil {
			fmt.Printf("警告: 无法解码文件 %s: %v，跳过此文件\n", file, err)
			continue
		}

		// 获取活动文件
		activity, ok := listener.File().(*filedef.Activity)
		if !ok {
			fmt.Printf("警告: 文件 %s 不是活动文件，跳过此文件\n", file)
			continue
		}

		// 提取数据
		recordCount := len(activity.Records)
		sessionCount := len(activity.Sessions)
		lapCount := len(activity.Laps)
		eventCount := len(activity.Events)

		// 添加到合并数据
		fm.records = append(fm.records, activity.Records...)
		fm.sessions = append(fm.sessions, activity.Sessions...)
		fm.laps = append(fm.laps, activity.Laps...)
		fm.events = append(fm.events, activity.Events...)

		fmt.Printf("  成功提取: 记录=%d, 会话=%d, 圈数=%d, 事件=%d\n",
			recordCount, sessionCount, lapCount, eventCount)

		// 检查详细数据字段
		if recordCount > 0 {
			firstRecord := activity.Records[0]
			fmt.Printf("  记录数据字段: ")
			if firstRecord.Power != 0xFFFF { // 0xFFFF 表示无效值
				fmt.Printf("功率 ")
			}
			if firstRecord.Cadence != 0xFF { // 0xFF 表示无效值
				fmt.Printf("踏频 ")
			}
			if firstRecord.HeartRate != 0xFF { // 0xFF 表示无效值
				fmt.Printf("心率 ")
			}
			if firstRecord.Speed != 0xFFFF { // 0xFFFF 表示无效值
				fmt.Printf("速度 ")
			}
			if firstRecord.Distance != 0xFFFFFFFF { // 0xFFFFFFFF 表示无效值
				fmt.Printf("距离 ")
			}
			if firstRecord.Altitude != 0xFFFF { // 0xFFFF 表示无效值
				fmt.Printf("海拔 ")
			}
			fmt.Printf("\n")
		}
		successCount++
	}

	if successCount == 0 {
		return fmt.Errorf("没有成功解析任何文件")
	}

	return nil
}

// mergeEnhanced 增强版合并（针对上传平台优化）
func (fm *UnifiedFITMerger) mergeEnhanced() error {
	fmt.Printf("使用增强合并器（针对上传平台优化）\n")
	fmt.Printf("记录总数: %d\n", len(fm.records))
	fmt.Printf("原始会话数: %d\n", len(fm.sessions))
	fmt.Printf("圈数: %d\n", len(fm.laps))
	fmt.Printf("事件数: %d\n", len(fm.events))

	// 按时间戳排序记录
	sort.Slice(fm.records, func(i, j int) bool {
		return fm.records[i].Timestamp.Before(fm.records[j].Timestamp)
	})

	// 合并所有会话为一个单一会话
	if len(fm.sessions) > 0 {
		// 创建一个新的单一会话，合并所有数据
		mergedSession := mesgdef.NewSession(nil)

		// 设置基本信息
		if len(fm.records) > 0 {
			mergedSession.SetTimestamp(fm.records[len(fm.records)-1].Timestamp)
			mergedSession.SetStartTime(fm.records[0].Timestamp)
		}

		// 累计所有会话的数据
		var totalDistance, totalElapsedTime, totalTimerTime float64
		var totalCalories float64
		var totalAscent, totalDescent float64
		var avgHeartRate, avgCadence, avgSpeed float64
		var maxHeartRate, maxSpeed uint8
		var countHeartRate, countCadence, countSpeed int

		// 显示每个会话的原始数据用于调试
		fmt.Printf("\n=== 原始会话数据 ===\n")
		for i, session := range fm.sessions {
			distance := session.TotalDistanceScaled()
			elapsed := session.TotalElapsedTimeScaled()
			timer := session.TotalTimerTimeScaled()
			calories := float64(session.TotalCalories)
			ascent := float64(session.TotalAscent)
			descent := float64(session.TotalDescent)

			fmt.Printf("会话 %d: 距离=%.2f m, 耗时=%.2f s, 计时=%.2f s, 卡路里=%.0f kcal, 上升=%.0f m, 下降=%.0f m\n",
				i+1, distance, elapsed, timer, calories, ascent, descent)

			// 累加所有数据
			totalDistance += distance
			totalElapsedTime += elapsed
			totalTimerTime += timer
			totalCalories += calories
			totalAscent += ascent
			totalDescent += descent

			// 心率 - 只累加有效值
			if session.AvgHeartRate > 0 {
				avgHeartRate += float64(session.AvgHeartRate)
				countHeartRate++
			}
			if session.MaxHeartRate > maxHeartRate {
				maxHeartRate = session.MaxHeartRate
			}

			// 踏频 - 只累加有效值
			if session.AvgCadence > 0 {
				avgCadence += float64(session.AvgCadence)
				countCadence++
			}

			// 速度 - 只累加有效值
			if session.AvgSpeedScaled() > 0 {
				avgSpeed += session.AvgSpeedScaled()
				countSpeed++
			}
			if session.MaxSpeedScaled() > float64(maxSpeed) {
				maxSpeed = uint8(session.MaxSpeedScaled())
			}
		}

		// 设置合并后的数据
		mergedSession.SetTotalDistanceScaled(totalDistance)
		mergedSession.SetTotalElapsedTimeScaled(totalElapsedTime)
		mergedSession.SetTotalTimerTimeScaled(totalTimerTime)
		mergedSession.SetTotalCalories(uint16(totalCalories))
		mergedSession.SetTotalAscent(uint16(totalAscent))
		mergedSession.SetTotalDescent(uint16(totalDescent))

		// 设置运动类型为骑行
		mergedSession.SetSport(typedef.SportCycling)
		mergedSession.SetSubSport(typedef.SubSportRoad)

		// 计算平均值
		if countHeartRate > 0 {
			mergedSession.SetAvgHeartRate(uint8(avgHeartRate / float64(countHeartRate)))
		}
		if maxHeartRate > 0 {
			mergedSession.SetMaxHeartRate(maxHeartRate)
		}

		if countCadence > 0 {
			mergedSession.SetAvgCadence(uint8(avgCadence / float64(countCadence)))
		}

		if countSpeed > 0 {
			mergedSession.SetAvgSpeedScaled(avgSpeed / float64(countSpeed))
			mergedSession.SetMaxSpeedScaled(float64(maxSpeed))
		}

		// 设置其他重要字段
		mergedSession.SetEvent(typedef.EventSession)
		mergedSession.SetEventType(typedef.EventTypeStop)
		mergedSession.SetFirstLapIndex(0)
		mergedSession.SetNumLaps(uint16(len(fm.laps)))

		// 替换会话列表为单一合并会话
		fm.sessions = []*mesgdef.Session{mergedSession}

		fmt.Printf("\n=== 增强合并结果 ===\n")
		fmt.Printf("合并后会话数: %d (合并为单一会话)\n", len(fm.sessions))
		fmt.Printf("合并后总距离: %.2f m (%.2f km)\n", totalDistance, totalDistance/1000)
		fmt.Printf("合并后总时间: %.2f s (%.2f min)\n", totalElapsedTime, totalElapsedTime/60)
		fmt.Printf("合并后总卡路里: %.0f kcal\n", totalCalories)
		fmt.Printf("合并后总上升: %.0f m\n", totalAscent)
		fmt.Printf("合并后总下降: %.0f m\n", totalDescent)

		// 统计详细数据
		fm.analyzeDetailedData()
	}

	// 合并圈数，重新编号
	for i, lap := range fm.laps {
		lap.SetMessageIndex(typedef.MessageIndex(i))
	}

	return nil
}

// mergeModern 现代版合并（单一会话）
func (fm *UnifiedFITMerger) mergeModern() error {
	fmt.Printf("使用现代合并器（单一会话）\n")
	fmt.Printf("记录总数: %d\n", len(fm.records))
	fmt.Printf("原始会话数: %d\n", len(fm.sessions))
	fmt.Printf("圈数: %d\n", len(fm.laps))
	fmt.Printf("事件数: %d\n", len(fm.events))

	// 按时间戳排序记录
	sort.Slice(fm.records, func(i, j int) bool {
		return fm.records[i].Timestamp.Before(fm.records[j].Timestamp)
	})

	// 合并所有会话为一个单一会话
	if len(fm.sessions) > 0 {
		// 创建一个新的单一会话，合并所有数据
		mergedSession := mesgdef.NewSession(nil)

		// 设置基本信息
		if len(fm.records) > 0 {
			mergedSession.SetTimestamp(fm.records[len(fm.records)-1].Timestamp)
			mergedSession.SetStartTime(fm.records[0].Timestamp)
		}

		// 累计所有会话的数据
		var totalDistance, totalElapsedTime, totalTimerTime float64
		var totalCalories float64
		var avgHeartRate, avgCadence, avgSpeed float64
		var maxHeartRate, maxSpeed uint8
		var countHeartRate, countCadence, countSpeed int

		// 显示每个会话的原始数据用于调试
		fmt.Printf("\n=== 原始会话数据 ===\n")
		for i, session := range fm.sessions {
			distance := session.TotalDistanceScaled()
			elapsed := session.TotalElapsedTimeScaled()
			timer := session.TotalTimerTimeScaled()
			calories := float64(session.TotalCalories)

			fmt.Printf("会话 %d: 距离=%.2f m, 耗时=%.2f s, 计时=%.2f s, 卡路里=%.0f kcal\n",
				i+1, distance, elapsed, timer, calories)

			// 累加所有数据
			totalDistance += distance
			totalElapsedTime += elapsed
			totalTimerTime += timer
			totalCalories += calories

			// 心率 - 只累加有效值
			if session.AvgHeartRate > 0 {
				avgHeartRate += float64(session.AvgHeartRate)
				countHeartRate++
			}
			if session.MaxHeartRate > maxHeartRate {
				maxHeartRate = session.MaxHeartRate
			}

			// 踏频 - 只累加有效值
			if session.AvgCadence > 0 {
				avgCadence += float64(session.AvgCadence)
				countCadence++
			}

			// 速度 - 只累加有效值
			if session.AvgSpeedScaled() > 0 {
				avgSpeed += session.AvgSpeedScaled()
				countSpeed++
			}
			if session.MaxSpeedScaled() > float64(maxSpeed) {
				maxSpeed = uint8(session.MaxSpeedScaled())
			}
		}

		// 设置合并后的数据
		mergedSession.SetTotalDistanceScaled(totalDistance)
		mergedSession.SetTotalElapsedTimeScaled(totalElapsedTime)
		mergedSession.SetTotalTimerTimeScaled(totalTimerTime)
		mergedSession.SetTotalCalories(uint16(totalCalories))

		// 设置运动类型为骑行
		mergedSession.SetSport(typedef.SportCycling)
		mergedSession.SetSubSport(typedef.SubSportRoad)

		// 计算平均值
		if countHeartRate > 0 {
			mergedSession.SetAvgHeartRate(uint8(avgHeartRate / float64(countHeartRate)))
		}
		if maxHeartRate > 0 {
			mergedSession.SetMaxHeartRate(maxHeartRate)
		}

		if countCadence > 0 {
			mergedSession.SetAvgCadence(uint8(avgCadence / float64(countCadence)))
		}

		if countSpeed > 0 {
			mergedSession.SetAvgSpeedScaled(avgSpeed / float64(countSpeed))
			mergedSession.SetMaxSpeedScaled(float64(maxSpeed))
		}

		// 设置其他重要字段
		mergedSession.SetEvent(typedef.EventSession)
		mergedSession.SetEventType(typedef.EventTypeStop)
		mergedSession.SetFirstLapIndex(0)
		mergedSession.SetNumLaps(uint16(len(fm.laps)))

		// 替换会话列表为单一合并会话
		fm.sessions = []*mesgdef.Session{mergedSession}

		fmt.Printf("\n=== 现代合并结果 ===\n")
		fmt.Printf("合并后会话数: %d (合并为单一会话)\n", len(fm.sessions))
		fmt.Printf("合并后总距离: %.2f m (%.2f km)\n", totalDistance, totalDistance/1000)
		fmt.Printf("合并后总时间: %.2f s (%.2f min)\n", totalElapsedTime, totalElapsedTime/60)
		fmt.Printf("合并后总卡路里: %.0f kcal\n", totalCalories)
	}

	// 合并圈数，重新编号
	for i, lap := range fm.laps {
		lap.SetMessageIndex(typedef.MessageIndex(i))
	}

	return nil
}

// mergeSimple 简单版合并（保留原始会话）
func (fm *UnifiedFITMerger) mergeSimple() error {
	fmt.Printf("使用简单合并器（保留原始会话）\n")
	fmt.Printf("记录总数: %d\n", len(fm.records))
	fmt.Printf("原始会话数: %d\n", len(fm.sessions))
	fmt.Printf("圈数: %d\n", len(fm.laps))
	fmt.Printf("事件数: %d\n", len(fm.events))

	// 按时间戳排序记录
	sort.Slice(fm.records, func(i, j int) bool {
		return fm.records[i].Timestamp.Before(fm.records[j].Timestamp)
	})

	// 重新编号圈数
	for i, lap := range fm.laps {
		lap.SetMessageIndex(typedef.MessageIndex(i))
	}

	fmt.Printf("\n=== 简单合并结果 ===\n")
	fmt.Printf("会话数: %d\n", len(fm.sessions))
	fmt.Printf("记录数: %d\n", len(fm.records))
	fmt.Printf("圈数: %d\n", len(fm.laps))

	return nil
}

// writeMergedFile 写入合并后的FIT文件
func (fm *UnifiedFITMerger) writeMergedFile() error {
	// 创建输出文件
	file, err := os.Create(fm.output)
	if err != nil {
		return fmt.Errorf("创建输出文件失败: %v", err)
	}
	defer file.Close()

	// 创建活动文件
	activity := filedef.NewActivity()

	// 设置文件ID
	activity.FileId.SetType(typedef.FileActivity)
	activity.FileId.SetTimeCreated(time.Now())
	activity.FileId.SetManufacturer(typedef.ManufacturerDevelopment)
	activity.FileId.SetProduct(0)
	activity.FileId.SetSerialNumber(uint32(time.Now().Unix()))

	// 添加所有数据
	activity.Records = fm.records
	activity.Sessions = fm.sessions
	activity.Laps = fm.laps
	activity.Events = fm.events

	// 转换为FIT格式
	fit := activity.ToFIT(nil)

	// 创建编码器
	enc := encoder.New(file)

	// 编码并写入
	if err := enc.Encode(&fit); err != nil {
		return fmt.Errorf("编码FIT文件失败: %v", err)
	}

	fmt.Printf("成功写入合并后的文件: %s\n", fm.output)
	return nil
}

// analyzeDetailedData 分析详细数据字段
func (fm *UnifiedFITMerger) analyzeDetailedData() {
	if len(fm.records) == 0 {
		return
	}

	var hasPower, hasCadence, hasHeartRate, hasSpeed, hasDistance, hasAltitude bool
	powerCount, cadenceCount, heartRateCount, speedCount, distanceCount, altitudeCount := 0, 0, 0, 0, 0, 0

	for _, record := range fm.records {
		if record.Power != 0xFFFF { // 0xFFFF 表示无效值
			hasPower = true
			powerCount++
		}
		if record.Cadence != 0xFF { // 0xFF 表示无效值
			hasCadence = true
			cadenceCount++
		}
		if record.HeartRate != 0xFF { // 0xFF 表示无效值
			hasHeartRate = true
			heartRateCount++
		}
		if record.Speed != 0xFFFF { // 0xFFFF 表示无效值
			hasSpeed = true
			speedCount++
		}
		if record.Distance != 0xFFFFFFFF { // 0xFFFFFFFF 表示无效值
			hasDistance = true
			distanceCount++
		}
		if record.Altitude != 0xFFFF { // 0xFFFF 表示无效值
			hasAltitude = true
			altitudeCount++
		}
	}

	fmt.Printf("\n=== 详细数据分析 ===\n")
	fmt.Printf("总记录数: %d\n", len(fm.records))
	if hasPower {
		fmt.Printf("功率数据: ✅ 有 (%d 条记录)\n", powerCount)
	} else {
		fmt.Printf("功率数据: ❌ 无\n")
	}
	if hasCadence {
		fmt.Printf("踏频数据: ✅ 有 (%d 条记录)\n", cadenceCount)
	} else {
		fmt.Printf("踏频数据: ❌ 无\n")
	}
	if hasHeartRate {
		fmt.Printf("心率数据: ✅ 有 (%d 条记录)\n", heartRateCount)
	} else {
		fmt.Printf("心率数据: ❌ 无\n")
	}
	if hasSpeed {
		fmt.Printf("速度数据: ✅ 有 (%d 条记录)\n", speedCount)
	} else {
		fmt.Printf("速度数据: ❌ 无\n")
	}
	if hasDistance {
		fmt.Printf("距离数据: ✅ 有 (%d 条记录)\n", distanceCount)
	} else {
		fmt.Printf("距离数据: ❌ 无\n")
	}
	if hasAltitude {
		fmt.Printf("海拔数据: ✅ 有 (%d 条记录)\n", altitudeCount)
	} else {
		fmt.Printf("海拔数据: ❌ 无\n")
	}
}

func main() {
	// 定义命令行参数
	output := flag.String("o", "merged.fit", "输出文件名")
	mergerType := flag.String("type", "enhanced", "合并器类型: simple, modern, enhanced")
	help := flag.Bool("h", false, "显示帮助信息")

	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "统一版FIT文件合并工具\n")
		fmt.Fprintf(os.Stderr, "用法: %s [选项] 文件1.fit 文件2.fit ...\n", filepath.Base(os.Args[0]))
		fmt.Fprintf(os.Stderr, "选项:\n")
		flag.PrintDefaults()
		fmt.Fprintf(os.Stderr, "合并器类型:\n")
		fmt.Fprintf(os.Stderr, "  simple   - 简单合并器 (保留原始会话)\n")
		fmt.Fprintf(os.Stderr, "  modern   - 现代合并器 (单一会话)\n")
		fmt.Fprintf(os.Stderr, "  enhanced - 增强合并器 (针对上传平台优化)\n")
		fmt.Fprintf(os.Stderr, "示例:\n")
		fmt.Fprintf(os.Stderr, "  %s -o merged.fit file1.fit file2.fit file3.fit\n", filepath.Base(os.Args[0]))
		fmt.Fprintf(os.Stderr, "  %s -type enhanced -o merged.fit *.fit\n", filepath.Base(os.Args[0]))
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

	// 创建合并器
	merger := NewUnifiedFITMerger(*output, *mergerType)

	// 添加所有输入文件
	for _, file := range flag.Args() {
		merger.AddFile(file)
	}

	// 执行合并
	fmt.Printf("开始合并 %d 个FIT文件到 %s\n", len(flag.Args()), *output)
	fmt.Printf("使用合并器类型: %s\n", *mergerType)
	start := time.Now()

	if err := merger.Merge(); err != nil {
		fmt.Fprintf(os.Stderr, "合并失败: %v\n", err)
		os.Exit(1)
	}

	elapsed := time.Since(start)
	fmt.Printf("合并完成，耗时: %v\n", elapsed)
}
