use fit_merger::FitParser;
use std::fs;

fn main() -> Result<(), String> {
    println!("调试FIT文件解析");

    // 读取文件
    let data = fs::read("../fit_files/merged_simple.fit").map_err(|e| format!("读取文件失败: {}", e))?;
    println!("文件大小: {} 字节", data.len());

    // 创建解析器
    let mut parser = FitParser::new(data);

    // 解析整个文件
    let file = parser.parse()?;
    
    println!("解析完成:");
    println!("  文件头大小: {}", file.header.header_size);
    println!("  协议版本: {}", file.header.protocol_version);
    println!("  配置文件版本: {}", file.header.profile_version);
    println!("  数据大小: {}", file.header.data_size);
    println!("  数据类型: {:?}", std::str::from_utf8(&file.header.data_type).unwrap_or("无效"));
    println!("  CRC: {:?}", file.header.crc);
    println!("  消息数量: {}", file.messages.len());

    Ok(())
}