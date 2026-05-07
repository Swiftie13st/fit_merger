use fit_merger::FitParser;
use std::fs;

fn main() -> Result<(), String> {
    println!("检查现有的合并文件");

    let files_to_check = vec![
        "../fit_files/merged.fit",
        "../fit_files/direct_merged.fit",
        "../fit_files/merged_simple.fit",
    ];

    for file_path in files_to_check {
        println!("\n检查文件: {}", file_path);
        
        match fs::read(file_path) {
            Ok(data) => {
                let mut parser = FitParser::new(data);
                match parser.parse() {
                    Ok(fit_file) => {
                        println!("  文件大小: {} 字节", fit_file.header.data_size + 16); // 数据 + 头部 + CRC
                        println!("  消息数量: {}", fit_file.messages.len());
                        
                        // 统计消息类型
                        let mut definition_count = 0;
                        let mut data_count = 0;
                        let mut message_types = std::collections::HashMap::new();

                        for message in &fit_file.messages {
                            match message {
                                fit_merger::fit_types::FitMessage::Definition(def) => {
                                    definition_count += 1;
                                    *message_types.entry(def.global_message_number).or_insert(0) += 1;
                                }
                                fit_merger::fit_types::FitMessage::Data(_) => {
                                    data_count += 1;
                                }
                            }
                        }

                        println!("  定义消息: {}", definition_count);
                        println!("  数据消息: {}", data_count);
                        println!("  消息类型:");
                        for (msg_type, count) in message_types {
                            println!("    全局消息号 {}: {} 个定义", msg_type, count);
                        }
                    }
                    Err(e) => {
                        println!("  解析失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  读取文件失败: {}", e);
            }
        }
    }

    Ok(())
}