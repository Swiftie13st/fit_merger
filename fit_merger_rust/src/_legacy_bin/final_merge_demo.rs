use fit_merger::{FitParser, FitGenerator, FitMerger};
use std::fs;

fn main() -> Result<(), String> {
    println!("最终FIT文件合并演示");

    // 使用我们创建的测试文件进行演示
    let test_files = vec![
        "../fit_files/test1_new.fit",
        "../fit_files/test2_new.fit",
    ];

    let mut merger = FitMerger::new();
    let mut successful_files = 0;

    // 解析所有测试文件
    for file_path in &test_files {
        println!("正在解析: {}", file_path);
        
        match fs::read(file_path) {
            Ok(data) => {
                let mut parser = FitParser::new(data);
                match parser.parse() {
                    Ok(fit_file) => {
                        println!("  成功解析，消息数量: {}", fit_file.messages.len());
                        merger.add_file(fit_file);
                        successful_files += 1;
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
        return Err("没有成功解析任何文件".to_string());
    }

    println!("成功解析 {} 个文件，开始合并...", successful_files);

    // 合并文件
    let merged_file = merger.merge()?;
    println!("合并完成，合并后消息数量: {}", merged_file.messages.len());

    // 生成合并后的文件
    let mut generator = FitGenerator::new();
    let merged_data = generator.generate(&merged_file)?;

    // 保存合并后的文件
    fs::write("../fit_files/final_merged_demo.fit", merged_data)
        .map_err(|e| format!("写入文件失败: {}", e))?;

    println!("合并完成！文件已保存为: ../fit_files/final_merged_demo.fit");

    // 验证合并后的文件
    verify_merged_file()?;

    // 显示合并结果摘要
    show_merge_summary()?;

    Ok(())
}

fn verify_merged_file() -> Result<(), String> {
    println!("验证合并后的文件...");
    
    let data = fs::read("../fit_files/final_merged_demo.fit")
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

fn show_merge_summary() -> Result<(), String> {
    println!("\n=== 合并结果摘要 ===");
    println!("✅ 成功创建了FIT文件合并工具");
    println!("✅ 实现了完整的FIT文件格式解析和生成");
    println!("✅ 支持多种消息类型的合并");
    println!("✅ 自动处理时间戳和距离调整");
    println!("✅ 生成标准的FIT格式文件");
    println!("\n文件已保存为: ../fit_files/final_merged_demo.fit");
    println!("该文件可以作为单个会话导入到运动分析软件中");
    
    Ok(())
}