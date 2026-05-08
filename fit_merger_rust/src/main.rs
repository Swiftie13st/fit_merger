use fit_merger::inspector::{format_summary, summarize_file};
use fit_merger::merger::merge_fit_files;
use std::env;
use std::fs;
use std::path::PathBuf;

fn print_usage(prog: &str) {
    eprintln!("用法:");
    eprintln!("  {} <input1.fit> <input2.fit> ... <output.fit>     # 合并多个 FIT 文件", prog);
    eprintln!("  {} inspect <file.fit> [<file2.fit> ...]            # 输出 FIT 文件的会话摘要", prog);
    eprintln!("  {}                                                  # 默认：扫描 ../fit_files 合并", prog);
}

fn run_inspect(paths: &[String]) -> i32 {
    if paths.is_empty() {
        eprintln!("inspect: 请至少指定 1 个 .fit 文件");
        return 1;
    }
    let mut had_error = false;
    for path in paths {
        println!("📄 文件: {}", path);
        match summarize_file(path) {
            Ok(sessions) => {
                if sessions.is_empty() {
                    println!("  （无任何会话/记录数据）");
                } else {
                    for (i, s) in sessions.iter().enumerate() {
                        println!("  {}", format_summary(i + 1, s));
                    }
                }
            }
            Err(e) => {
                eprintln!("  ❌ 解析失败: {}", e);
                had_error = true;
            }
        }
    }
    if had_error { 1 } else { 0 }
}

fn run_merge(inputs: &[String], output: &str) -> i32 {
    let refs: Vec<&str> = inputs.iter().map(|s| s.as_str()).collect();
    println!("正在合并 {} 个 FIT 文件 → {}", refs.len(), output);
    match merge_fit_files(&refs, output) {
        Ok(()) => {
            println!("✅ 合并成功：{}", output);
            0
        }
        Err(e) => {
            eprintln!("❌ 合并失败：{}", e);
            1
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // inspect 子命令
    if args.len() >= 2 && args[1] == "inspect" {
        let paths: Vec<String> = args[2..].to_vec();
        std::process::exit(run_inspect(&paths));
    }

    // help
    if args.len() >= 2 && (args[1] == "-h" || args[1] == "--help" || args[1] == "help") {
        print_usage(&args[0]);
        return;
    }

    // 显式合并：N 个输入 + 1 个输出
    if args.len() >= 3 {
        let output = args[args.len() - 1].clone();
        let inputs: Vec<String> = args[1..args.len() - 1].iter().cloned().collect();
        std::process::exit(run_merge(&inputs, &output));
    }

    // 默认：扫描 ../fit_files 目录中的所有 .fit 文件，输出到 merged.fit
    println!("未指定参数，使用默认模式：扫描 ../fit_files 下的 .fit 文件");
    let dir = PathBuf::from("../fit_files");
    if !dir.exists() {
        print_usage(&args[0]);
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
    std::process::exit(run_merge(&paths, "../fit_files/merged.fit"));
}