use std::fs::File;
use std::io::{Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("改进版FIT文件合并器");
    
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
    let mut all_record_data = Vec::new();
    let mut total_distance = 0.0;
    let mut file_count = 0;
    
    for file_path in &file_paths {
        println!("正在处理: {}", file_path);
        
        match read_fit_records(file_path) {
            Ok((records, distance)) => {
                println!("  成功读取 {} 条记录", records.len());
                
                if !records.is_empty() {
                    // 调整距离（累积）
                    let mut adjusted_records = Vec::new();
                    for record in records {
                        let mut adjusted_record = record;
                        // 从分析器输出看，距离字段在记录末尾附近
                        // 假设记录格式包含时间戳、纬度、经度和距离
                        let record_len = adjusted_record.len();
                        if record_len >= 4 {
                            // 尝试从记录末尾提取距离（最后4字节）
                            let distance_offset = record_len - 4;
                            let original_distance = f32::from_le_bytes([
                                adjusted_record[distance_offset],
                                adjusted_record[distance_offset + 1],
                                adjusted_record[distance_offset + 2],
                                adjusted_record[distance_offset + 3],
                            ]);
                            
                            // 防止无效距离值
                            let original_distance = if original_distance.is_finite() && original_distance >= 0.0 && original_distance < 1000000.0 {
                                original_distance
                            } else {
                                0.0
                            };
                            
                            // 添加累积距离
                            let new_distance = original_distance + total_distance;
                            let new_distance_bytes = new_distance.to_le_bytes();
                            adjusted_record[distance_offset] = new_distance_bytes[0];
                            adjusted_record[distance_offset + 1] = new_distance_bytes[1];
                            adjusted_record[distance_offset + 2] = new_distance_bytes[2];
                            adjusted_record[distance_offset + 3] = new_distance_bytes[3];
                        }
                        adjusted_records.push(adjusted_record);
                    }
                    
                    all_record_data.extend(adjusted_records);
                    total_distance += distance;
                    file_count += 1;
                }
            }
            Err(e) => {
                println!("  跳过文件 {}: {}", file_path, e);
            }
        }
    }
    
    if all_record_data.is_empty() {
        return Err("没有成功读取任何文件".into());
    }
    
    println!("总共读取 {} 个文件，{} 条记录", file_count, all_record_data.len());
    
    // 创建合并后的FIT文件
    let output_path = "../fit_files/merged_basic.fit";
    
    // 使用第一个文件作为模板
    let template_path = file_paths[0];
    let mut template_data = Vec::new();
    File::open(template_path)?.read_to_end(&mut template_data)?;
    
    // 创建新的FIT文件数据
    let mut new_fit_data = Vec::new();
    
    // 复制头部（前14字节）
    new_fit_data.extend_from_slice(&template_data[..14]);
    
    // 添加记录数据
    new_fit_data.extend(all_record_data.iter().flatten());
    
    // 更新文件头中的数据大小
    let data_size = new_fit_data.len() - 14;
    let data_size_bytes = (data_size as u32).to_le_bytes();
    new_fit_data[4] = data_size_bytes[0];
    new_fit_data[5] = data_size_bytes[1];
    new_fit_data[6] = data_size_bytes[2];
    new_fit_data[7] = data_size_bytes[3];
    
    // 计算并添加CRC（最后2字节）
    let crc = calculate_crc(&new_fit_data[..new_fit_data.len()]);
    new_fit_data.extend_from_slice(&crc.to_le_bytes());
    
    // 写入文件
    let mut output_file = File::create(output_path)?;
    output_file.write_all(&new_fit_data)?;
    
    println!("合并完成！输出文件: {}", output_path);
    println!("总记录数: {}", all_record_data.len());
    println!("总距离: {:.2} 米", total_distance);
    
    Ok(())
}

// 改进的FIT文件读取函数，更准确地解析记录数据
fn read_fit_records(path: &str) -> Result<(Vec<Vec<u8>>, f32), Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    // 跳过头部（14字节）
    let data = &buffer[14..buffer.len().saturating_sub(2)]; // 去掉CRC
    
    let mut records = Vec::new();
    let mut offset = 0;
    let mut max_distance = 0.0;
    
    // 更准确的记录消息解析
    while offset < data.len() {
        if offset + 1 >= data.len() {
            break;
        }
        
        let header = data[offset];
        
        // 检查是否是定义消息
        if header & 0x40 != 0 {
            // 定义消息，跳过
            if offset + 5 < data.len() {
                let field_count = data[offset + 5] as usize;
                let dev_field_count = if offset + 6 < data.len() {
                    data[offset + 6] as usize
                } else {
                    0
                };
                let message_size = 7 + field_count * 3 + dev_field_count * 3;
                offset += message_size;
            } else {
                break;
            }
            continue;
        }
        
        // 检查是否是记录消息（本地消息类型0）
        if header & 0x0F == 0 {
            // 从分析器输出看，记录消息大小不固定
            // 尝试动态确定记录大小
            let record_size = if header & 0x80 != 0 {
                // 有开发者数据
                let dev_data_size = if offset + 1 < data.len() {
                    data[offset + 1] as usize
                } else {
                    0
                };
                1 + dev_data_size // 头部 + 开发者数据
            } else {
                // 标准记录消息，尝试从上下文推断大小
                // 从分析器输出看，记录包含时间戳、纬度、经度、距离等
                // 尝试使用更合理的记录大小
                let base_size = 20; // 基础大小
                let next_header_pos = find_next_header(&data, offset + 1);
                if let Some(next_pos) = next_header_pos {
                    next_pos - offset
                } else {
                    // 如果没有找到下一个头部，使用剩余数据
                    data.len() - offset
                }
            };
            
            if offset + record_size <= data.len() {
                let record_start = offset + 1;
                let record_end = offset + record_size;
                let record_data = data[record_start..record_end].to_vec();
                
                // 尝试从记录末尾提取距离（最后4字节）
                let record_len = record_data.len();
                if record_len >= 4 {
                    let distance_offset = record_len - 4;
                    let distance = f32::from_le_bytes([
                        record_data[distance_offset],
                        record_data[distance_offset + 1],
                        record_data[distance_offset + 2],
                        record_data[distance_offset + 3],
                    ]);
                    
                    // 防止无效距离值（更严格的检查）
                    if distance.is_finite() && distance >= 0.0 && distance < 100000.0 {
                        if distance > max_distance {
                            max_distance = distance;
                        }
                    }
                }
                
                records.push(record_data);
                offset = record_end;
            } else {
                break;
            }
        } else {
            // 其他消息类型，跳过
            offset += 1;
        }
    }
    
    Ok((records, max_distance))
}

// 查找下一个消息头部
fn find_next_header(data: &[u8], start: usize) -> Option<usize> {
    for i in start..data.len() {
        let byte = data[i];
        // 检查是否是定义消息或数据消息头部
        if (byte & 0x40 != 0) || (byte & 0x0F <= 0x0F) {
            return Some(i);
        }
    }
    None
}

/// 计算FIT文件的CRC
fn calculate_crc(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    
    for &byte in data {
        crc = fit_crc_get16(crc, byte);
    }
    
    crc
}

/// FIT CRC计算函数
fn fit_crc_get16(crc: u16, byte: u8) -> u16 {
    static CRC_TABLE: [u16; 16] = [
        0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401,
        0xA001, 0x6C00, 0x7800, 0xB401, 0x5000, 0x9C01, 0x8801, 0x4400
    ];
    
    let tmp = CRC_TABLE[(crc & 0xF) as usize];
    let crc = (crc >> 4) & 0x0FFF;
    let crc = crc ^ tmp ^ CRC_TABLE[(byte & 0xF) as usize];
    
    let tmp = CRC_TABLE[(crc & 0xF) as usize];
    let crc = (crc >> 4) & 0x0FFF;
    let crc = crc ^ tmp ^ CRC_TABLE[((byte >> 4) & 0xF) as usize];
    
    crc
}