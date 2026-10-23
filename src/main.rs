use fit_merger::gpx::convert_gpx_file;
use fit_merger::inspector::{format_summary, summarize_file};
use fit_merger::merger::merge_fit_files;
use std::env;
use std::fs;
use std::path::PathBuf;

fn print_usage(prog: &str) {
    eprintln!("用法:");
    eprintln!("  {} <input1.fit> <input2.fit> ... <output.fit>     # 合并多个 FIT 文件", prog);
    eprintln!("  {} inspect <file.fit> [<file2.fit> ...]            # 输出 FIT 文件的会话摘要", prog);
    eprintln!("  {} gpx2fit <input.gpx> [output.fit]                # 将 GPX 转换为 FIT", prog);
    eprintln!("  {}                                                  # 默认：扫描 fit_files 合并", prog);
}

/// 默认数据目录：优先当前工作目录下的 `fit_files`，兼容旧布局的 `../fit_files`。
fn default_fit_dir() -> Option<PathBuf> {
    ["fit_files", "../fit_files"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_dir())
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

fn run_gpx2fit(input: &str, output: Option<&str>) -> i32 {
    let output = match output {
        Some(p) => p.to_string(),
        None => PathBuf::from(input)
            .with_extension("fit")
            .to_string_lossy()
            .to_string(),
    };
    println!("正在转换 GPX → FIT: {} → {}", input, output);
    match convert_gpx_file(input, &output) {
        Ok(s) => {
            let hms = |total: u32| {
                format!(
                    "{:02}:{:02}:{:02}",
                    total / 3600,
                    (total % 3600) / 60,
                    total % 60
                )
            };
            println!(
                "✅ 转换成功：{} 个轨迹点 → 生成 {} 条 record（跳过缺时间 {} 个 / 时间未递增 {} 个）",
                s.input_points, s.records, s.skipped_no_time, s.skipped_non_increasing
            );
            let calories = s
                .calories
                .map(|c| format!(", 卡路里={} kcal", c))
                .unwrap_or_default();
            println!(
                "   距离={:.3} km{}, 耗时={}, 计时={}, 运动类型={}",
                s.distance_m / 1000.0,
                calories,
                hms(s.elapsed_s),
                hms(s.timer_s),
                s.sport_name()
            );
            0
        }
        Err(e) => {
            eprintln!("❌ 转换失败：{}", e);
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

    // gpx2fit 子命令
    if args.len() >= 2 && args[1] == "gpx2fit" {
        match args.len() {
            3 => std::process::exit(run_gpx2fit(&args[2], None)),
            4 => std::process::exit(run_gpx2fit(&args[2], Some(&args[3]))),
            _ => {
                eprintln!("gpx2fit 用法: {} gpx2fit <input.gpx> [output.fit]", args[0]);
                std::process::exit(1);
            }
        }
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

    // 默认：扫描 fit_files 目录中的所有 .fit 文件，输出到 merged.fit
    let dir = match default_fit_dir() {
        Some(dir) => dir,
        None => {
            print_usage(&args[0]);
            std::process::exit(1);
        }
    };
    println!(
        "未指定参数，使用默认模式：扫描 {} 下的 .fit 文件",
        dir.display()
    );
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
        eprintln!("{} 中没有 .fit 文件", dir.display());
        std::process::exit(1);
    }
    let output = dir.join("merged.fit").to_string_lossy().to_string();
    std::process::exit(run_merge(&paths, &output));
}
gpx_to_fit.rs