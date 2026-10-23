//! 用第三方 fitparser 校验 GPX → FIT 转换结果。
//!
//! 验证项：
//! - fitparser 默认 CRC 严格校验通过
//! - 单一 file_id / session / lap / activity，record 数与轨迹点一致
//! - record 保留 GPS / 海拔 / 心率 / 踏频 / 功率 / 温度
//! - session 距离、爬升、下降、运动类型正确

use fit_merger::fit_generator::FitGenerator;
use fit_merger::gpx::{gpx_to_fit, parse_gpx};
use fitparser::profile::MesgNum;
use fitparser::{from_bytes, Value};

const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test" xmlns="http://www.topografix.com/GPX/1/1"
     xmlns:gpxtpx="http://www.garmin.com/xmlschemas/TrackPointExtension/v1">
  <trk>
    <name>Morning Ride</name>
    <type>cycling</type>
    <trkseg>
      <trkpt lat="39.900000" lon="116.400000">
        <ele>50.0</ele><time>2026-05-01T08:00:00Z</time>
        <extensions><gpxtpx:TrackPointExtension>
          <gpxtpx:hr>120</gpxtpx:hr><gpxtpx:cad>80</gpxtpx:cad>
          <gpxtpx:atemp>22.5</gpxtpx:atemp><power>180</power>
        </gpxtpx:TrackPointExtension></extensions>
      </trkpt>
      <trkpt lat="39.901000" lon="116.400000">
        <ele>53.0</ele><time>2026-05-01T08:00:10Z</time>
        <extensions><gpxtpx:TrackPointExtension>
          <gpxtpx:hr>125</gpxtpx:hr><gpxtpx:cad>82</gpxtpx:cad>
          <gpxtpx:atemp>23.0</gpxtpx:atemp><power>190</power>
        </gpxtpx:TrackPointExtension></extensions>
      </trkpt>
      <trkpt lat="39.902000" lon="116.400000">
        <ele>49.0</ele><time>2026-05-01T08:00:20Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

fn count(recs: &[fitparser::FitDataRecord], kind: MesgNum) -> usize {
    recs.iter().filter(|r| r.kind() == kind).count()
}

fn numeric(rec: &fitparser::FitDataRecord, name: &str) -> Option<f64> {
    rec.fields()
        .iter()
        .find(|f| f.name() == name)
        .and_then(|f| match f.value() {
            Value::Byte(v) => Some(*v as f64),
            Value::Enum(v) => Some(*v as f64),
            Value::SInt8(v) => Some(*v as f64),
            Value::UInt8(v) => Some(*v as f64),
            Value::SInt16(v) => Some(*v as f64),
            Value::UInt16(v) => Some(*v as f64),
            Value::SInt32(v) => Some(*v as f64),
            Value::UInt32(v) => Some(*v as f64),
            Value::Float32(v) => Some(*v as f64),
            Value::Float64(v) => Some(*v),
            _ => None,
        })
}

fn text(rec: &fitparser::FitDataRecord, name: &str) -> Option<String> {
    rec.fields()
        .iter()
        .find(|f| f.name() == name)
        .map(|f| f.value().to_string())
}

#[test]
fn gpx_conversion_is_valid_fit() {
    let gpx = parse_gpx(SAMPLE).expect("GPX 解析失败");
    let (fit, summary) = gpx_to_fit(&gpx).expect("GPX → FIT 转换失败");
    let bytes = FitGenerator::new().generate(&fit).expect("FIT 序列化失败");

    // fitparser 默认校验头部 CRC 与文件 CRC
    let recs = from_bytes(&bytes).expect("fitparser 校验 FIT 失败");

    assert_eq!(count(&recs, MesgNum::FileId), 1);
    assert_eq!(count(&recs, MesgNum::Record), 3);
    assert_eq!(count(&recs, MesgNum::Lap), 1);
    assert_eq!(count(&recs, MesgNum::Session), 1);
    assert_eq!(count(&recs, MesgNum::Activity), 1);
    assert_eq!(count(&recs, MesgNum::Event), 2);
    assert_eq!(summary.records, 3);

    let session = recs
        .iter()
        .find(|r| r.kind() == MesgNum::Session)
        .expect("缺少 session");
    assert_eq!(text(session, "sport").as_deref(), Some("cycling"));
    assert_eq!(numeric(session, "num_laps"), Some(1.0));
    let distance = numeric(session, "total_distance").expect("缺少 total_distance");
    assert!(
        (200.0..300.0).contains(&distance),
        "session 距离异常: {}",
        distance
    );
    let elapsed = numeric(session, "total_elapsed_time").expect("缺少 total_elapsed_time");
    assert!((elapsed - 20.0).abs() < 0.01, "elapsed={}", elapsed);
    assert_eq!(numeric(session, "total_ascent"), Some(3.0));
    assert_eq!(numeric(session, "total_descent"), Some(4.0));

    let record = recs
        .iter()
        .find(|r| r.kind() == MesgNum::Record)
        .expect("缺少 record");
    for name in [
        "position_lat",
        "position_long",
        "heart_rate",
        "cadence",
        "distance",
        "power",
        "temperature",
    ] {
        assert!(numeric(record, name).is_some(), "record 缺少字段 {}", name);
    }
    // fitparser 默认把复合字段 altitude/speed 解码为 enhanced_altitude/enhanced_speed
    assert!(
        numeric(record, "altitude").is_some() || numeric(record, "enhanced_altitude").is_some(),
        "record 缺少海拔字段"
    );
    let altitude = numeric(record, "enhanced_altitude")
        .or_else(|| numeric(record, "altitude"))
        .unwrap();
    assert!((altitude - 50.0).abs() < 0.2, "altitude={}", altitude);
    assert!(
        numeric(record, "speed").is_some() || numeric(record, "enhanced_speed").is_some(),
        "record 缺少速度字段"
    );
    let lat = numeric(record, "position_lat").unwrap();
    let lat_deg = lat * 180.0 / 2_147_483_648.0;
    assert!(
        (lat_deg - 39.9).abs() < 1e-5,
        "position_lat={} ({})",
        lat,
        lat_deg
    );
}

#[test]
fn gpx_extensions_and_pauses_are_applied() {
    const GAP_SAMPLE: &str = r#"<?xml version="1.0"?>
<gpx xmlns="http://www.topografix.com/GPX/1/1" xmlns:gpxdata="http://www.cluetrust.com/XML/GPXDATA/1/0">
  <trk><type>cycling</type><trkseg>
    <trkpt lat="39.9000" lon="116.4000"><ele>0</ele><time>2026-05-01T08:00:00Z</time>
      <extensions><gpxdata:distance>0</gpxdata:distance><gpxdata:speed>10</gpxdata:speed></extensions></trkpt>
    <trkpt lat="39.9001" lon="116.4000"><ele>2</ele><time>2026-05-01T08:00:01Z</time>
      <extensions><gpxdata:distance>10</gpxdata:distance><gpxdata:speed>10</gpxdata:speed></extensions></trkpt>
    <trkpt lat="39.9002" lon="116.4000"><ele>4</ele><time>2026-05-01T08:00:02Z</time>
      <extensions><gpxdata:distance>20</gpxdata:distance><gpxdata:speed>10</gpxdata:speed></extensions></trkpt>
    <trkpt lat="39.9003" lon="116.4000"><ele>6</ele><time>2026-05-01T08:01:40Z</time>
      <extensions><gpxdata:distance>30</gpxdata:distance><gpxdata:speed>5</gpxdata:speed></extensions></trkpt>
  </trkseg></trk>
  <extensions><gpxdata:laps>
    <gpxdata:lap><gpxdata:calories>123</gpxdata:calories></gpxdata:lap>
  </gpxdata:laps></extensions>
</gpx>"#;

    let gpx = parse_gpx(GAP_SAMPLE).expect("GPX 解析失败");
    let (fit, summary) = gpx_to_fit(&gpx).expect("GPX → FIT 转换失败");
    assert_eq!(summary.distance_m, 30.0);
    assert_eq!(summary.elapsed_s, 100);
    assert_eq!(summary.timer_s, 2);
    assert_eq!(summary.calories, Some(123));

    let bytes = FitGenerator::new().generate(&fit).expect("FIT 序列化失败");
    let recs = from_bytes(&bytes).expect("fitparser 校验 FIT 失败");
    let session = recs
        .iter()
        .find(|r| r.kind() == MesgNum::Session)
        .expect("缺少 session");
    assert_eq!(numeric(session, "total_distance"), Some(30.0));
    assert_eq!(numeric(session, "total_elapsed_time"), Some(100.0));
    assert_eq!(numeric(session, "total_timer_time"), Some(2.0));
    assert_eq!(
        numeric(session, "enhanced_avg_speed").or_else(|| numeric(session, "avg_speed")),
        Some(15.0)
    );
    assert_eq!(numeric(session, "total_ascent"), Some(6.0));
    assert_eq!(numeric(session, "total_calories"), Some(123.0));
    let lap = recs
        .iter()
        .find(|r| r.kind() == MesgNum::Lap)
        .expect("缺少 lap");
    assert_eq!(numeric(lap, "total_calories"), Some(123.0));
}
