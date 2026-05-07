use fit_merger::{FitParser, FitGenerator, FitMerger};
use std::fs;

fn main() -> Result<(), String> {
    println!("简单FIT文件合并测试");

    // 解析第一个测试文件
    println!("解析第一个测试文件...");
    let data1 = fs::read("../fit_files/test1_new.fit").map_err(|e| format!("读取文件失败: {}", e))?;
    let mut parser1 = FitParser::new(data1);
    let file1 = parser1.parse()?;

    // 解析第二个测试文件
    println!("解析第二个测试文件...");
    let data2 = fs::read("../fit_files/test2_new.fit").map_err(|e| format!("读取文件失败: {}", e))?;
    let mut parser2 = FitParser::new(data2);
    let file2 = parser2.parse()?;

    println!("文件1消息数量: {}", file1.messages.len());
    println!("文件2消息数量: {}", file2.messages.len());

    // 创建合并器
    let mut merger = FitMerger::new();
    merger.add_file(file1);
    merger.add_file(file2);

    // 合并文件
    println!("合并文件...");
    let merged_file = merger.merge()?;

    println!("合并后消息数量: {}", merged_file.messages.len());

    // 生成合并后的文件
    println!("生成合并后的文件...");
    let mut generator = FitGenerator::new();
    let merged_data = generator.generate(&merged_file)?;

    // 保存合并后的文件
    fs::write("../fit_files/merged_simple.fit", merged_data).map_err(|e| format!("写入文件失败: {}", e))?;

    println!("合并完成！文件已保存为: ../fit_files/merged_simple.fit");

    Ok(())
}