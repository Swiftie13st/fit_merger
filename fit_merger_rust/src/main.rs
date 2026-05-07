use fit_merger::merger::merge_fit_files;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();

    let (inputs, output): (Vec<String>, String) = if args.len() >= 3 {
        let output = args[args.len() - 1].clone();
        let inputs = args[1..args.len() - 1].iter().cloned().collect();
        (inputs, output)
    } else {
        // 默认：扫描 ../fit_files 目录中的所有 .fit 文件，输出到 merged.fit
        println!("未指定参数，使用默认模式：扫描 ../fit_files 下的 .fit 文件");
        let dir = PathBuf::from("../fit_files");
        if !dir.exists() {
            eprintln!(
                "用法: {} <input1.fit> <input2.fit> ... <output.fit>",
                args[0]
            );
            std::process::exit(1);
        }
        let mut paths: Vec<String> = match fs::read_dir(&dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.eq_ignore_ascii_case("fit"))
                        .unwrap_or(false)
                })
                .filter(|p| {
                    // 跳过之前生成的 merged 文件，避免循环包含
                    !p.file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.starts_with("merged"))
                        .unwrap_or(false)
                })
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            Err(e) => {
                eprintln!("读取目录失败: {}", e);
                std::process::exit(1);
            }
        };
        paths.sort();
        if paths.is_empty() {
            eprintln!("../fit_files 中没有 .fit 文件");
            std::process::exit(1);
        }
        (paths, "../fit_files/merged.fit".to_string())
    };

    let refs: Vec<&str> = inputs.iter().map(|s| s.as_str()).collect();
    println!("正在合并 {} 个 FIT 文件 → {}", refs.len(), output);
    match merge_fit_files(&refs, &output) {
        Ok(()) => println!("✅ 合并成功：{}", output),
        Err(e) => {
            eprintln!("❌ 合并失败：{}", e);
            std::process::exit(1);
        }
    }
}