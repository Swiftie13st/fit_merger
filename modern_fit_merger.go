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

// ModernFITMerger 使用现代FIT库合并多个FIT文件
type ModernFITMerger struct {
	files    []string
	output   string
	records  []*mesgdef.Record
	sessions []*mesgdef.Session
	laps     []*mesgdef.Lap
	events   []*mesgdef.Event
}

// NewModernFITMerger 创建新的现代FIT合并器
func NewModernFITMerger(output string) *ModernFITMerger {
	return &ModernFITMerger{
		files:    make([]string, 0),
		output:   output,
		records:  make([]*mesgdef.Record, 0),
		sessions: make([]*mesgdef.Session, 0),
		laps:     make([]*mesgdef.Lap, 0),
		events:   make([]*mesgdef.Event, 0),
	}
}

// AddFile 添加要合并的FIT文件
func (fm *ModernFITMerger) AddFile(file string) {
	fm.files = append(fm.files, file)
}

// Merge 执行合并操作
func (fm *ModernFITMerger) Merge() error {
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

	// 合并数据
	if err := fm.mergeData(); err != nil {
		return fmt.Errorf("合并数据失败: %v", err)
	}

	// 写入合并后的文件
	if err := fm.writeMergedFile(); err != nil {
		return fmt.Errorf("写入合并文件失败: %v", err)
	}

	return nil
}

// parseAllFiles 解析所有FIT文件
func (fm *ModernFITMerger) parseAllFiles() error {
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
		successCount++
	}

	if successCount == 0 {
		return fmt.Errorf("没有成功解析任何文件")
	}

	return nil
}

// mergeData 合并数据
func (fm *ModernFITMerger) mergeData() error {
	fmt.Printf("合并 %d 个文件的数据\n", len(fm.files))
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

		// 累计所有会话的数据 - 修复：不跳过任何数据，确保完整累加
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

			// 累加所有数据，不检查是否大于0
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

		if countHeartRate > 0 {
			mergedSession.SetAvgHeartRate(uint8(avgHeartRate / float64(countHeartRate)))
		}
		mergedSession.SetMaxHeartRate(maxHeartRate)

		if countCadence > 0 {
			mergedSession.SetAvgCadence(uint8(avgCadence / float64(countCadence)))
		}

		if countSpeed > 0 {
			mergedSession.SetAvgSpeedScaled(avgSpeed / float64(countSpeed))
			mergedSession.SetMaxSpeedScaled(float64(maxSpeed))
		}

		// 替换会话列表为单一合并会话
		fm.sessions = []*mesgdef.Session{mergedSession}

		fmt.Printf("\n=== 合并结果 ===\n")
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

// writeMergedFile 写入合并后的FIT文件
func (fm *ModernFITMerger) writeMergedFile() error {
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

func main() {
	// 定义命令行参数
	output := flag.String("o", "merged.fit", "输出文件名")
	help := flag.Bool("h", false, "显示帮助信息")

	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "现代FIT文件合并工具 (使用muktihari/fit库)\n")
		fmt.Fprintf(os.Stderr, "用法: %s [选项] 文件1.fit 文件2.fit ...\n", filepath.Base(os.Args[0]))
		fmt.Fprintf(os.Stderr, "选项:\n")
		flag.PrintDefaults()
		fmt.Fprintf(os.Stderr, "示例:\n")
		fmt.Fprintf(os.Stderr, "  %s -o merged.fit file1.fit file2.fit file3.fit\n", filepath.Base(os.Args[0]))
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
	merger := NewModernFITMerger(*output)

	// 添加所有输入文件
	for _, file := range flag.Args() {
		merger.AddFile(file)
	}

	// 执行合并
	fmt.Printf("开始合并 %d 个FIT文件到 %s\n", len(flag.Args()), *output)
	start := time.Now()

	if err := merger.Merge(); err != nil {
		fmt.Fprintf(os.Stderr, "合并失败: %v\n", err)
		os.Exit(1)
	}

	elapsed := time.Since(start)
	fmt.Printf("合并完成，耗时: %v\n", elapsed)
}
