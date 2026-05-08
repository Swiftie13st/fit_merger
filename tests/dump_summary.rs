//! 输出合并文件的会话汇总信息，便于人工核对。
use fitparser::profile::MesgNum;
use fitparser::{from_bytes, Value};
use std::fs;

#[test]
fn dump_merged_summary() {
    let merged_path = "../fit_files/merged.fit";
    if !std::path::Path::new(merged_path).exists() {
        eprintln!("跳过：{} 不存在", merged_path);
        return;
    }
    let bytes = fs::read(merged_path).unwrap();
    let recs = from_bytes(&bytes).unwrap();

    let size_kb = bytes.len() as f64 / 1024.0;
    println!("\n==== 合并文件汇总 ({:.1} KB) ====", size_kb);

    let session = recs
        .iter()
        .find(|r| r.kind() == MesgNum::Session)
        .expect("no session");
    println!("--- session 字段 ---");
    for f in session.fields() {
        println!("  {} = {} {}", f.name(), f.value(), f.units());
    }

    let activity = recs
        .iter()
        .find(|r| r.kind() == MesgNum::Activity)
        .expect("no activity");
    println!("--- activity 字段 ---");
    for f in activity.fields() {
        println!("  {} = {} {}", f.name(), f.value(), f.units());
    }

    // 按消息类型计数
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    for r in &recs {
        *counts.entry(format!("{:?}", r.kind())).or_default() += 1;
    }
    println!("--- 消息类型统计 ---");
    for (k, v) in counts {
        println!("  {} = {}", k, v);
    }

    // 示例：第一条 record 的所有字段
    if let Some(first_rec) = recs.iter().find(|r| r.kind() == MesgNum::Record) {
        println!("--- 首条 record 所含字段 ---");
        for f in first_rec.fields() {
            if !matches!(f.value(), Value::Invalid) {
                println!("  {} = {} {}", f.name(), f.value(), f.units());
            }
        }
    }
}