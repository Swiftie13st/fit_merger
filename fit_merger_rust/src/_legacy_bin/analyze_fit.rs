use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("FIT文件分析器");
    
    let file_path = "../fit_files/公路骑行20260430061420.fit";
    println!("分析文件: {}", file_path);
    
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    println!("文件大小: {} 字节", buffer.len());
    
    // 解析头部
    if buffer.len() >= 14 {
        let header_size = buffer[0];
        let protocol_version = buffer[1];
        let profile_version = u16::from_le_bytes([buffer[2], buffer[3]]);
        let data_size = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
        let data_type = std::str::from_utf8(&buffer[8..12]).unwrap_or("未知");
        
        println!("头部信息:");
        println!("  头部大小: {} 字节", header_size);
        println!("  协议版本: {}", protocol_version);
        println!("  配置文件版本: {}", profile_version);
        println!("  数据大小: {} 字节", data_size);
        println!("  数据类型: {}", data_type);
        
        // 分析数据部分
        let data_start = header_size as usize;
        let data_end = buffer.len() - 2; // 去掉CRC
        
        if data_start < data_end {
            let data = &buffer[data_start..data_end];
            analyze_data_messages(data)?;
        }
    }
    
    Ok(())
}

fn analyze_data_messages(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut offset = 0;
    let mut record_count = 0;
    let mut definition_count = 0;
    let mut max_distance = 0.0;
    
    println!("\n数据消息分析:");
    
    while offset < data.len() {
        if offset + 1 > data.len() {
            break;
        }
        
        let header = data[offset];
        
        // 检查是否是定义消息
        if header & 0x40 != 0 {
            // 定义消息
            definition_count += 1;
            
            if offset + 5 <= data.len() {
                let reserved = data[offset + 1];
                let architecture = data[offset + 2];
                let global_msg_num = u16::from_le_bytes([data[offset + 3], data[offset + 4]]);
                let field_count = data[offset + 5] as usize;
                
                println!("定义消息 {}: 全局消息号={}, 架构={}, 字段数={}", 
                         definition_count, global_msg_num, architecture, field_count);
                
                // 显示字段定义
                let mut field_offset = offset + 6;
                for i in 0..field_count {
                    if field_offset + 3 <= data.len() {
                        let field_def_num = data[field_offset];
                        let field_size = data[field_offset + 1];
                        let base_type = data[field_offset + 2];
                        
                        println!("  字段 {}: 定义号={}, 大小={}, 基础类型={}", 
                                 i, field_def_num, field_size, base_type);
                        
                        field_offset += 3;
                    }
                }
                
                // 开发者字段
                if field_offset < data.len() {
                    let dev_field_count = data[field_offset] as usize;
                    field_offset += 1;
                    
                    println!("  开发者字段数: {}", dev_field_count);
                    
                    for i in 0..dev_field_count {
                        if field_offset + 3 <= data.len() {
                            let field_num = data[field_offset];
                            let field_size = data[field_offset + 1];
                            let dev_data_index = data[field_offset + 2];
                            
                            println!("    开发者字段 {}: 字段号={}, 大小={}, 数据索引={}", 
                                     i, field_num, field_size, dev_data_index);
                            
                            field_offset += 3;
                        }
                    }
                }
                
                offset = field_offset;
            } else {
                break;
            }
        } else {
            // 数据消息
            let local_msg_type = header & 0x0F;
            let record_start = offset + 1;
            
            if local_msg_type == 0 {
                // 记录消息 - 假设这是我们要找的
                record_count += 1;
                
                // 我们需要知道定义消息来确定字段布局
                // 这里我们做一个简单的假设，基于常见的记录消息格式
                
                if record_start + 20 <= data.len() {
                    // 尝试解析一些常见字段
                    let timestamp_raw = u32::from_le_bytes([
                        data[record_start], data[record_start + 1], 
                        data[record_start + 2], data[record_start + 3]
                    ]);
                    
                    let position_lat = i32::from_le_bytes([
                        data[record_start + 4], data[record_start + 5], 
                        data[record_start + 6], data[record_start + 7]
                    ]);
                    
                    let position_long = i32::from_le_bytes([
                        data[record_start + 8], data[record_start + 9], 
                        data[record_start + 10], data[record_start + 11]
                    ]);
                    
                    let distance_raw = u32::from_le_bytes([
                        data[record_start + 12], data[record_start + 13], 
                        data[record_start + 14], data[record_start + 15]
                    ]);
                    
                    let distance = distance_raw as f32 / 100.0; // 转换为米
                    
                    if distance > max_distance {
                        max_distance = distance;
                    }
                    
                    if record_count <= 5 {
                        println!("记录消息 {}: 时间戳={}, 纬度={}, 经度={}, 距离={:.2}米", 
                                 record_count, timestamp_raw, position_lat, position_long, distance);
                    }
                }
                
                offset = record_start + 20; // 假设20字节记录
            } else {
                // 其他数据消息，跳过
                offset += 1;
            }
        }
    }
    
    println!("\n统计信息:");
    println!("  定义消息数: {}", definition_count);
    println!("  记录消息数: {}", record_count);
    println!("  最大距离: {:.2}米 ({:.2}公里)", max_distance, max_distance / 1000.0);
    
    Ok(())
}