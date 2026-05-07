use fit_merger::{FitParser, FitGenerator, FitMerger};
use std::fs;

fn main() -> Result<(), String> {
    println!("合并真实骑行文件");

    // 获取所有真实骑行文件
    let fit_files = vec![
        "../fit_files/公路骑行20260430061420.fit",
        "../fit_files/公路骑行20260501082327.fit",
        "../fit_files/公路骑行20260502140948.fit",
        "../fit_files/公路骑行20260503064808.fit",
        "../fit_files/公路骑行20260504094905.fit",
        "../fit_files/公路骑行20260505103329.fit",
    ];

    let mut merger = FitMerger::new();
    let mut successful_files = 0;

    // 解析所有文件
    for file_path in &fit_files {
        println!("正在解析: {}", file_path);
        
        match fs::read(file_path) {
            Ok(data) => {
                let mut parser = FitParser::new(data);
                match parser.parse() {
                    Ok(fit_file) => {
                        println!("  成功解析，消息数量: {}", fit_file.messages.len());
                        
                        // 只添加有实际内容的文件
                        if !fit_file.messages.is_empty() {
                            merger.add_file(fit_file);
                            successful_files += 1;
                            println!("  已添加到合并器");
                        } else {
                            println!("  跳过空文件");
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

    if successful_files == 0 {
        return Err("没有成功解析任何文件。尝试使用更简单的合并方法...".to_string());
    }

    println!("成功解析 {} 个文件，开始合并...", successful_files);

    // 合并文件
    let merged_file = merger.merge()?;
    println!("合并完成，合并后消息数量: {}", merged_file.messages.len());

    // 生成合并后的文件
    let mut generator = FitGenerator::new();
    let merged_data = generator.generate(&merged_file)?;

    // 保存合并后的文件
    fs::write("../fit_files/all_rides_combined.fit", merged_data)
        .map_err(|e| format!("写入文件失败: {}", e))?;

    println!("合并完成！文件已保存为: ../fit_files/all_rides_combined.fit");

    // 验证合并后的文件
    verify_merged_file()?;

    Ok(())
}

fn verify_merged_file() -> Result<(), String> {
    println!("验证合并后的文件...");
    
    let data = fs::read("../fit_files/all_rides_combined.fit")
        .map_err(|e| format!("读取合并文件失败: {}", e))?;
    
    let mut parser = FitParser::new(data);
    let file = parser.parse()?;

    println!("验证结果:");
    println!("  文件大小: {} 字节", file.header.data_size + 16); // 数据大小 + 头部 + CRC
    println!("  消息数量: {}", file.messages.len());
    
    // 统计消息类型
    let mut definition_count = 0;
    let mut data_count = 0;
    let mut message_types = std::collections::HashMap::new();

    for message in &file.messages {
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

    Ok(())
}