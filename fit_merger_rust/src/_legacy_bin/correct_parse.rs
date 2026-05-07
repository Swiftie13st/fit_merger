use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: cargo run --bin correct_parse -- <文件路径>");
        std::process::exit(1);
    }
    
    let file_path = &args[1];
    
    // 读取文件
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    // 解析FIT文件
    let fit = fitparser::from_bytes(&buffer)?;
    
    println!("FIT文件正确解析器");
    println!("文件: {}", file_path);
    println!("消息数量: {}", fit.len());
    
    // 提取记录消息
    let mut total_distance = 0.0;
    let mut record_count = 0;
    
    for data in fit {
        // 检查是否是数据消息
        if let Some(fields) = data.as_data_message() {
            if data.kind().as_u16() == 20 { // 20是记录消息的编号
                record_count += 1;
                
                // 提取距离
                for field in fields {
                    if field.name() == "distance" {
                        if let fitparser::Value::Float64(distance) = field.value() {
                            total_distance = distance;
                            println!("记录 {}: 距离 = {:.2} 米", record_count, distance);
                        }
                    }
                }
            }
        }
    }
    
    println!("\n汇总信息:");
    println!("总记录数: {}", record_count);
    println!("总距离: {:.2} 米 ({:.2} 公里)", total_distance, total_distance / 1000.0);
    
    Ok(())
}