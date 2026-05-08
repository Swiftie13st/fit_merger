//! 用第三方 fitparser 库验证合并后的 FIT 文件。
//!
//! 验证项：
//! - CRC 通过（默认严格校验）
//! - 单一 session（合并要求）
//! - 单一 activity
//! - record 数量 ≈ 各源文件 record 之和
//! - record 中至少存在 distance/heart_rate/cadence/power/altitude 字段（至少各一条）

use fitparser::profile::MesgNum;
use fitparser::{from_bytes, Value};
use std::fs;

fn read_records(path: &str) -> Vec<fitparser::FitDataRecord> {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {} failed: {}", path, e));
    from_bytes(&bytes).unwrap_or_else(|e| panic!("parse {} failed: {}", path, e))
}

fn count_records(recs: &[fitparser::FitDataRecord], kind: MesgNum) -> usize {
    recs.iter().filter(|r| r.kind() == kind).count()
}

fn has_field(recs: &[fitparser::FitDataRecord], kind: MesgNum, name: &str) -> bool {
    recs.iter()
        .filter(|r| r.kind() == kind)
        .any(|r| r.fields().iter().any(|f| f.name() == name && !matches!(f.value(), Value::Invalid)))
}

#[test]
fn merged_file_is_valid_single_session() {
    let merged_path = "../fit_files/merged.fit";
    if !std::path::Path::new(merged_path).exists() {
        eprintln!("跳过：{} 不存在，请先运行 cargo run --release", merged_path);
        return;
    }
    let merged = read_records(merged_path);
    println!("merged 总记录数: {}", merged.len());

    let n_session = count_records(&merged, MesgNum::Session);
    let n_activity = count_records(&merged, MesgNum::Activity);
    let n_record = count_records(&merged, MesgNum::Record);
    let n_file_id = count_records(&merged, MesgNum::FileId);
    println!(
        "file_id={}, record={}, session={}, activity={}",
        n_file_id, n_record, n_session, n_activity
    );

    assert_eq!(n_file_id, 1, "应当仅有 1 条 file_id");
    assert_eq!(n_session, 1, "合并后应当仅有 1 条 session");
    assert_eq!(n_activity, 1, "合并后应当仅有 1 条 activity");
    assert!(n_record > 1000, "record 数量过少: {}", n_record);

    // 字段保留检查
    assert!(has_field(&merged, MesgNum::Record, "distance"), "丢失 distance 字段");
    assert!(has_field(&merged, MesgNum::Record, "heart_rate"), "丢失 heart_rate 字段");
    assert!(has_field(&merged, MesgNum::Record, "cadence"), "丢失 cadence 字段");
    assert!(has_field(&merged, MesgNum::Record, "power"), "丢失 power 字段");
    assert!(
        has_field(&merged, MesgNum::Record, "altitude")
            || has_field(&merged, MesgNum::Record, "enhanced_altitude"),
        "丢失 altitude 字段"
    );

    // 比较 record 总数（合并后约等于各源文件之和）
    let sources = [
        "../fit_files/公路骑行20260430061420.fit",
        "../fit_files/公路骑行20260501082327.fit",
        "../fit_files/公路骑行20260502140948.fit",
        "../fit_files/公路骑行20260503064808.fit",
        "../fit_files/公路骑行20260504094905.fit",
        "../fit_files/公路骑行20260505103329.fit",
    ];
    let mut total_src_records = 0usize;
    for p in sources {
        if let Ok(bytes) = fs::read(p) {
            if let Ok(recs) = from_bytes(&bytes) {
                total_src_records += count_records(&recs, MesgNum::Record);
            }
        }
    }
    println!("源 record 之和={}, 合并文件 record={}", total_src_records, n_record);
    let diff = (total_src_records as i64 - n_record as i64).abs();
    assert!(
        diff <= (total_src_records / 100) as i64 + 5,
        "合并后 record 数量与源文件相差过大: src={}, merged={}",
        total_src_records,
        n_record
    );
}