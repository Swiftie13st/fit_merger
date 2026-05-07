use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use fit_merger::fit_generator::{FitFile, FileIdMessage, SessionMessage, ActivityMessage, RecordMessage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("简单FIT文件合并器");
    
    // 定义要合并的文件路径
    let file_paths = vec![
        "../fit_files/公路骑行20260430061420.fit",
        "../fit_files/公路骑行20260501082327.fit",
        "../fit_files/公路骑行20260502140948.fit",
        "../fit_files/公路骑行20260503064808.fit",
        "../fit_files/公路骑行20260504094905.fit",
        "../fit_files/公路骑行20260505103329.fit",
    ];
    
    // 尝试读取所有文件
    let mut all_records = Vec::new();
    let mut total_distance = 0.0;
    let mut start_time = None;
    let mut end_time = None;
    let mut total_elapsed_time = 0.0;
    let mut total_timer_time = 0.0;
    
    for file_path in &file_paths {
        println!("正在处理: {}", file_path);
        
        match read_fit_file(file_path) {
            Ok(records) => {
                println!("  成功读取 {} 条记录", records.len());
                
                if !records.is_empty() {
                    // 更新开始时间
                    if start_time.is_none() || records[0].timestamp < start_time.unwrap() {
                        start_time = Some(records[0].timestamp);
                    }
                    
                    // 更新结束时间
                    if end_time.is_none() || records.last().unwrap().timestamp > end_time.unwrap() {
                        end_time = Some(records.last().unwrap().timestamp);
                    }
                    
                    // 调整距离（累积）
                    for record in &records {
                        let adjusted_record = RecordMessage {
                            timestamp: record.timestamp,
                            position_lat: record.position_lat,
                            position_long: record.position_long,
                            distance: record.distance + total_distance,
                            altitude: record.altitude,
                            speed: record.speed,
                            heart_rate: record.heart_rate,
                            cadence: record.cadence,
                            power: record.power,
                            temperature: record.temperature,
                        };
                        all_records.push(adjusted_record);
                    }
                    
                    // 更新总距离
                    if let Some(last_record) = records.last() {
                        total_distance = last_record.distance + total_distance;
                    }
                }
            }
            Err(e) => {
                println!("  跳过文件 {}: {}", file_path, e);
            }
        }
    }
    
    if all_records.is_empty() {
        return Err("没有成功读取任何文件".into());
    }
    
    println!("总共读取 {} 条记录", all_records.len());
    
    // 计算总时间
    if let (Some(start), Some(end)) = (start_time, end_time) {
        total_elapsed_time = (end - start) as f32 / 1000.0; // 转换为秒
        total_timer_time = total_elapsed_time;
    }
    
    // 创建合并后的FIT文件
    let output_path = "../fit_files/merged_rides.fit";
    
    let mut fit_file = FitFile::new();
    
    // 添加文件ID消息
    fit_file.add_file_id_message(FileIdMessage {
        file_type: 4, // activity
        manufacturer: 1, // garmin
        product: 0,
        serial_number: 12345,
        time_created: start_time.unwrap_or(0),
        number: 0,
    });
    
    // 添加会话消息
    fit_file.add_session_message(SessionMessage {
        timestamp: end_time.unwrap_or(0),
        start_time: start_time.unwrap_or(0),
        start_position_lat: all_records.first().and_then(|r| r.position_lat),
        start_position_long: all_records.first().and_then(|r| r.position_long),
        total_elapsed_time,
        total_timer_time,
        total_distance: total_distance,
        sport: 2, // cycling
        sub_sport: 0,
        avg_speed: total_distance / total_timer_time.max(1.0),
        avg_heart_rate: all_records.iter()
            .filter_map(|r| r.heart_rate)
            .sum::<u8>() as f32 / all_records.iter().filter(|r| r.heart_rate.is_some()).count() as f32,
        avg_cadence: all_records.iter()
            .filter_map(|r| r.cadence)
            .sum::<u8>() as f32 / all_records.iter().filter(|r| r.cadence.is_some()).count() as f32,
        avg_power: all_records.iter()
            .filter_map(|r| r.power)
            .sum::<u16>() as f32 / all_records.iter().filter(|r| r.power.is_some()).count() as f32,
    });
    
    // 添加活动消息
    fit_file.add_activity_message(ActivityMessage {
        timestamp: start_time.unwrap_or(0),
        total_timer_time,
        num_sessions: 1,
        type_: 0, // manual
        event: 0,
        event_type: 0,
    });
    
    // 添加所有记录消息
    for record in all_records {
        fit_file.add_record_message(record);
    }
    
    // 生成FIT文件
    fit_file.generate_file(output_path)?;
    
    println!("合并完成！输出文件: {}", output_path);
    println!("总记录数: {}", fit_file.record_messages.len());
    println!("总距离: {:.2} 米", total_distance);
    println!("总时间: {:.2} 秒", total_elapsed_time);
    
    Ok(())
}

// 简化的FIT文件读取函数
fn read_fit_file(path: &str) -> Result<Vec<RecordMessage>, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    // 跳过头部（14字节）
    let data = &buffer[14..];
    
    let mut records = Vec::new();
    let mut offset = 0;
    
    // 简单的记录消息解析（假设时间戳压缩偏移为0）
    while offset + 12 <= data.len() {
        // 检查是否是记录消息头部
        if data[offset] & 0x40 != 0 { // 定义消息
            offset += 5;
            continue;
        }
        
        if data[offset] & 0x0F == 0 { // 记录消息类型
            offset += 1; // 跳过消息头部
            
            let timestamp = u32::from_le_bytes([
                data[offset], data[offset+1], data[offset+2], data[offset+3]
            ]) as i64 * 1000; // 转换为毫秒
            
            offset += 4;
            
            let position_lat = if offset + 4 <= data.len() {
                Some(i32::from_le_bytes([
                    data[offset], data[offset+1], data[offset+2], data[offset+3]
                ]))
            } else {
                None
            };
            offset += 4;
            
            let position_long = if offset + 4 <= data.len() {
                Some(i32::from_le_bytes([
                    data[offset], data[offset+1], data[offset+2], data[offset+3]
                ]))
            } else {
                None
            };
            offset += 4;
            
            let distance = if offset + 4 <= data.len() {
                u32::from_le_bytes([
                    data[offset], data[offset+1], data[offset+2], data[offset+3]
                ]) as f32 / 100.0 // 转换为米
            } else {
                0.0
            };
            offset += 4;
            
            let altitude = if offset + 2 <= data.len() {
                Some(u16::from_le_bytes([data[offset], data[offset+1]]) as f32 / 5.0 - 500.0)
            } else {
                None
            };
            offset += 2;
            
            let speed = if offset + 2 <= data.len() {
                Some(u16::from_le_bytes([data[offset], data[offset+1]]) as f32 / 1000.0)
            } else {
                None
            };
            offset += 2;
            
            let heart_rate = if offset < data.len() {
                Some(data[offset])
            } else {
                None
            };
            offset += 1;
            
            let cadence = if offset < data.len() {
                Some(data[offset])
            } else {
                None
            };
            offset += 1;
            
            let power = if offset + 2 <= data.len() {
                Some(u16::from_le_bytes([data[offset], data[offset+1]]))
            } else {
                None
            };
            offset += 2;
            
            let temperature = if offset < data.len() {
                Some(data[offset] as i8)
            } else {
                None
            };
            offset += 1;
            
            records.push(RecordMessage {
                timestamp,
                position_lat,
                position_long,
                distance,
                altitude,
                speed,
                heart_rate,
                cadence,
                power,
                temperature,
            });
        } else {
            // 跳过其他消息类型
            offset += 1;
        }
    }
    
    Ok(records)
}