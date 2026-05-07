use fit_merger::{FitParser, fit_types::FitMessage};
use std::fs;

fn main() -> Result<(), String> {
    println!("验证FIT文件格式");

    // 验证合并后的文件
    println!("验证合并后的文件...");
    let data = fs::read("../fit_files/merged_simple.fit").map_err(|e| format!("读取文件失败: {}", e))?;
    let mut parser = FitParser::new(data);
    let file = parser.parse()?;

    println!("文件头信息:");
    println!("  头大小: {}", file.header.header_size);
    println!("  协议版本: {}", file.header.protocol_version);
    println!("  配置文件版本: {}", file.header.profile_version);
    println!("  数据大小: {}", file.header.data_size);
    println!("  数据类型: {:?}", std::str::from_utf8(&file.header.data_type).unwrap_or("无效"));
    println!("  CRC: {:?}", file.header.crc);

    println!("消息数量: {}", file.messages.len());

    // 统计不同类型的消息
    let mut definition_count = 0;
    let mut data_count = 0;
    let mut message_types = std::collections::HashMap::new();

    for message in &file.messages {
        match message {
            FitMessage::Definition(def) => {
                definition_count += 1;
                *message_types.entry(def.global_message_number).or_insert(0) += 1;
            }
            FitMessage::Data(data) => {
                data_count += 1;
                // 这里我们需要找到对应的定义来获取全局消息号
                // 简化处理，不统计具体类型
            }
        }
    }

    println!("定义消息数量: {}", definition_count);
    println!("数据消息数量: {}", data_count);
    
    println!("定义消息类型:");
    for (msg_type, count) in message_types {
        println!("  全局消息号 {}: {} 个", msg_type, count);
    }

    Ok(())
}