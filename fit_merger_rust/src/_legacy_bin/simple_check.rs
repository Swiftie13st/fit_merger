use fit_merger::FitParser;
use std::fs;

fn main() -> Result<(), String> {
    println!("简单文件检查");

    let files = vec![
        "../fit_files/公路骑行20260430061420.fit",
        "../fit_files/公路骑行20260501082327.fit",
        "../fit_files/公路骑行20260502140948.fit",
    ];

    for file_path in files {
        println!("\n检查文件: {}", file_path);
        
        match fs::metadata(file_path) {
            Ok(metadata) => {
                println!("  文件大小: {} 字节", metadata.len());
                
                // 只读取前几个字节来检查文件头
                match fs::read(file_path) {
                    Ok(data) => {
                        if data.len() >= 14 {
                            let header_size = data[0];
                            let protocol_version = data[1];
                            let profile_version = u16::from_le_bytes([data[2], data[3]]);
                            let data_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                            let data_type = &data[8..12];
                            
                            println!("  头部大小: {}", header_size);
                            println!("  协议版本: {}", protocol_version);
                            println!("  配置文件版本: {}", profile_version);
                            println!("  数据大小: {}", data_size);
                            println!("  数据类型: {:?}", std::str::from_utf8(data_type).unwrap_or("无效"));
                            
                            // 尝试解析
                            let mut parser = FitParser::new(data);
                            match parser.parse() {
                                Ok(fit_file) => {
                                    println!("  解析成功，消息数量: {}", fit_file.messages.len());
                                    
                                    // 统计定义消息
                                    let mut def_count = 0;
                                    let mut data_count = 0;
                                    for msg in &fit_file.messages {
                                        match msg {
                                            fit_merger::fit_types::FitMessage::Definition(_) => def_count += 1,
                                            fit_merger::fit_types::FitMessage::Data(_) => data_count += 1,
                                        }
                                    }
                                    println!("  定义消息: {}, 数据消息: {}", def_count, data_count);
                                }
                                Err(e) => {
                                    println!("  解析失败: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("  读取文件失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  获取文件信息失败: {}", e);
            }
        }
    }

    Ok(())
}