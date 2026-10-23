//! GPX → FIT 转换：解析 GPX 轨迹点，生成可被 Garmin Connect / Strava 等平台导入的
//! 单会话 FIT 活动文件。
//!
//! 解析仅依赖标准库，支持常见命名空间前缀（`gpxtpx:hr`、`gpxtpx:cad`、`gpxtpx:atemp`、
//! `power` 等）；距离由经纬度 haversine 累加，速度由相邻点时间差计算。

use crate::fit_generator::write_fit_file;
use crate::fit_types::*;
use std::fs;

/// FIT 时间戳起点：1989-12-31 00:00:00 UTC。
const FIT_EPOCH_OFFSET: i64 = 631_065_600;
/// 海拔变化超过该死区才计入累计爬升 / 下降，避免 GPS 高程抖动。
const ELE_HYSTERESIS_M: f64 = 2.0;

// FIT 全局消息号
const MSG_FILE_ID: u16 = 0;
const MSG_SESSION: u16 = 18;
const MSG_LAP: u16 = 19;
const MSG_RECORD: u16 = 20;
const MSG_EVENT: u16 = 21;
const MSG_ACTIVITY: u16 = 34;

// 本文件内部使用的 local message type
const LMT_FILE_ID: u8 = 0;
const LMT_EVENT: u8 = 1;
const LMT_RECORD: u8 = 2;
const LMT_LAP: u8 = 3;
const LMT_SESSION: u8 = 4;
const LMT_ACTIVITY: u8 = 5;

/// 一个 GPX 轨迹点（可选字段来自 GPX extension）。
#[derive(Debug, Clone, Default)]
pub struct GpxPoint {
    pub lat: f64,
    pub lon: f64,
    pub ele: Option<f64>,
    /// Unix 时间戳（秒）
    pub time: Option<i64>,
    pub hr: Option<f64>,
    pub cadence: Option<f64>,
    pub power: Option<f64>,
    pub temperature: Option<f64>,
    /// GPX 扩展中的累计距离（米）
    pub distance: Option<f64>,
    /// GPX 扩展中的速度（米 / 秒）
    pub speed: Option<f64>,
}

/// GPX 解析结果。
#[derive(Debug, Clone, Default)]
pub struct GpxData {
    pub points: Vec<GpxPoint>,
    /// `<trk><type>` 中的运动类型，如 cycling / running / hiking
    pub track_type: Option<String>,
    /// GPX 中的总卡路里（优先 lap 级求和，其次 metadata 级单值）
    pub calories: Option<u32>,
    /// lat/lon 缺失或越界而跳过的 trkpt 数量
    pub invalid_points: usize,
}

/// 转换统计信息。
#[derive(Debug, Clone, Default)]
pub struct ConvertSummary {
    pub input_points: usize,
    pub records: usize,
    pub skipped_no_time: usize,
    pub skipped_non_increasing: usize,
    pub distance_m: f64,
    pub elapsed_s: u32,
    /// 计时时间：剔除自动暂停产生的长时间采样间隔后的累计秒数
    pub timer_s: u32,
    pub sport: u8,
    pub calories: Option<u32>,
    pub has_hr: bool,
    pub has_cadence: bool,
    pub has_power: bool,
}

impl ConvertSummary {
    pub fn sport_name(&self) -> &'static str {
        match self.sport {
            1 => "running",
            2 => "cycling",
            5 => "swimming",
            11 => "walking",
            12 => "cross_country_skiing",
            15 => "rowing",
            16 => "mountaineering",
            17 => "hiking",
            _ => "generic",
        }
    }
}

/// 读取并转换 GPX 文件，写出 FIT 文件。
pub fn convert_gpx_file(input: &str, output: &str) -> Result<ConvertSummary, String> {
    let xml = fs::read_to_string(input).map_err(|e| format!("读取 {} 失败: {}", input, e))?;
    let gpx = parse_gpx(&xml)?;
    let (fit, summary) = gpx_to_fit(&gpx)?;
    write_fit_file(&fit, output)?;
    Ok(summary)
}

// ---------------------------------------------------------------------------
// GPX 解析
// ---------------------------------------------------------------------------

/// 解析 GPX 文本，提取全部 `<trkpt>` 与 `<trk><type>`。
pub fn parse_gpx(xml: &str) -> Result<GpxData, String> {
    let mut points: Vec<GpxPoint> = Vec::new();
    let mut track_type: Option<String> = None;
    let mut invalid_points = 0usize;
    let mut current: Option<GpxPoint> = None;
    let mut stack: Vec<String> = Vec::new();
    let mut capture: Option<String> = None;
    let mut capture_in_lap = false;
    let mut lap_calories: u64 = 0;
    let mut metadata_calories: Option<u32> = None;
    let mut in_trk = false;
    let mut pos = 0usize;

    while pos < xml.len() {
        let Some(rel) = xml[pos..].find('<') else {
            break;
        };
        let lt = pos + rel;

        // 上一个标签之后、当前标签之前的文本
        if let Some(name) = capture.take() {
            let raw_text = &xml[pos..lt];
            apply_capture(
                &name,
                capture_in_lap,
                raw_text,
                &mut current,
                &mut track_type,
                &mut lap_calories,
                &mut metadata_calories,
            );
            capture_in_lap = false;
        }
        pos = lt;

        if xml[pos..].starts_with("<!--") {
            pos = find_after(xml, pos + 4, "-->");
            continue;
        }
        if xml[pos..].starts_with("<?") {
            pos = find_after(xml, pos + 2, "?>");
            continue;
        }
        if xml[pos..].starts_with("<![CDATA[") {
            pos = find_after(xml, pos + 9, "]]>");
            continue;
        }
        if xml[pos..].starts_with("<!") {
            pos = find_tag_end(xml, pos + 2)
                .map(|e| e + 1)
                .unwrap_or(xml.len());
            continue;
        }

        let Some(end) = find_tag_end(xml, pos + 1) else {
            return Err(format!("GPX 标签未闭合（位置 {}）", pos));
        };
        let inner = xml[pos + 1..end].trim();
        pos = end + 1;
        if inner.is_empty() {
            continue;
        }

        // 结束标签
        if let Some(rest) = inner.strip_prefix('/') {
            let name = local_name(rest.trim());
            if stack.last().map(|s| s.as_str()) == Some(name) {
                stack.pop();
            }
            if name == "trkpt" {
                if let Some(p) = current.take() {
                    points.push(p);
                }
            }
            if name == "trk" {
                in_trk = false;
            }
            continue;
        }

        let self_closing = inner.ends_with('/');
        let inner = inner.trim_end_matches('/').trim_end();
        let (name_part, attr_part) = match inner.find(char::is_whitespace) {
            Some(i) => (&inner[..i], &inner[i..]),
            None => (inner, ""),
        };
        let local = local_name(name_part);

        match local {
            "trk" => in_trk = true,
            "trkpt" => {
                let attrs = parse_attributes(attr_part);
                let lat = attr_value(&attrs, "lat").and_then(|v| v.parse::<f64>().ok());
                let lon = attr_value(&attrs, "lon").and_then(|v| v.parse::<f64>().ok());
                match (lat, lon) {
                    (Some(lat), Some(lon))
                        if lat.is_finite()
                            && lon.is_finite()
                            && (-90.0..=90.0).contains(&lat)
                            && (-180.0..=180.0).contains(&lon) =>
                    {
                        current = Some(GpxPoint {
                            lat,
                            lon,
                            ..Default::default()
                        });
                        if self_closing {
                            if let Some(p) = current.take() {
                                points.push(p);
                            }
                        }
                    }
                    _ => {
                        invalid_points += 1;
                        current = None;
                    }
                }
            }
            _ => {}
        }

        if current.is_some() {
            if matches!(
                local,
                "ele"
                    | "time"
                    | "hr"
                    | "cad"
                    | "cadence"
                    | "power"
                    | "atemp"
                    | "temp"
                    | "distance"
                    | "speed"
            ) {
                capture = Some(local.to_string());
                capture_in_lap = false;
            }
        } else if in_trk && local == "type" && stack.last().map(|s| s.as_str()) == Some("trk") {
            capture = Some("type".to_string());
            capture_in_lap = false;
        } else if local == "calories" && !stack.iter().any(|s| s == "trkpt") {
            // 逐点 calories 不处理，只采集 lap / metadata 级总量
            capture = Some("calories".to_string());
            capture_in_lap = stack.iter().any(|s| s == "lap");
        }

        if !self_closing {
            stack.push(local.to_string());
        }
    }

    if points.is_empty() {
        return Err("GPX 中没有找到有效的 <trkpt> 轨迹点".to_string());
    }
    let calories = if lap_calories > 0 {
        Some(lap_calories.min(u32::MAX as u64) as u32)
    } else {
        metadata_calories
    };
    Ok(GpxData {
        points,
        track_type,
        calories,
        invalid_points,
    })
}

fn apply_capture(
    name: &str,
    in_lap: bool,
    raw_text: &str,
    current: &mut Option<GpxPoint>,
    track_type: &mut Option<String>,
    lap_calories: &mut u64,
    metadata_calories: &mut Option<u32>,
) {
    let text = unescape(raw_text.trim());
    if text.is_empty() {
        return;
    }
    if let Some(p) = current.as_mut() {
        let number = || text.parse::<f64>().ok().filter(|v| v.is_finite());
        match name {
            "ele" => p.ele = number(),
            "time" => p.time = parse_iso8601_utc(&text),
            "hr" => p.hr = number(),
            "cad" | "cadence" => p.cadence = number(),
            "power" => p.power = number(),
            "atemp" | "temp" => p.temperature = number(),
            "distance" => p.distance = number(),
            "speed" => p.speed = number(),
            _ => {}
        }
    } else if name == "type" && track_type.is_none() {
        *track_type = Some(text);
    } else if name == "calories" {
        if let Ok(value) = text.parse::<f64>() {
            if value.is_finite() && value >= 0.0 {
                let value = value.round().min(u32::MAX as f64) as u32;
                if in_lap {
                    *lap_calories = lap_calories.saturating_add(value as u64);
                } else if metadata_calories.is_none() {
                    *metadata_calories = Some(value);
                }
            }
        }
    }
}

fn local_name(name: &str) -> &str {
    match name.rfind(':') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

fn find_after(s: &str, start: usize, needle: &str) -> usize {
    match s[start..].find(needle) {
        Some(i) => start + i + needle.len(),
        None => s.len(),
    }
}

/// 找到标签结束的 `>`，忽略引号内的 `>`。
fn find_tag_end(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut quote: Option<u8> = None;
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i];
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                b'"' | b'\'' => quote = Some(c),
                b'>' => return Some(i),
                _ => {}
            },
        }
        i += 1;
    }
    None
}

fn parse_attributes(s: &str) -> Vec<(String, String)> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if key_start == i {
            i += 1;
            continue;
        }
        let key = &s[key_start..i];
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'=') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let quote = bytes[i];
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        i += 1;
        let value_start = i;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        let value = &s[value_start..i];
        i += 1;
        out.push((key.to_string(), unescape(value)));
    }
    out
}

fn attr_value<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// 解析 ISO 8601 / RFC 3339 时间（如 `2026-05-01T08:23:27Z`、`...+08:00`），返回 Unix 秒。
pub fn parse_iso8601_utc(s: &str) -> Option<i64> {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() < 19 {
        return None;
    }
    if b[4] != b'-' || b[7] != b'-' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    if b[10] != b'T' && b[10] != b't' && b[10] != b' ' {
        return None;
    }
    let num = |range: std::ops::Range<usize>| -> Option<i64> { s.get(range)?.parse::<i64>().ok() };
    let year = num(0..4)?;
    let month = num(5..7)?;
    let day = num(8..10)?;
    let hour = num(11..13)?;
    let minute = num(14..16)?;
    let second = num(17..19)?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let mut secs = days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second;
    let mut idx = 19usize;
    if b.get(idx) == Some(&b'.') {
        idx += 1;
        while idx < b.len() && b[idx].is_ascii_digit() {
            idx += 1;
        }
    }
    if idx < b.len() {
        match b[idx] {
            b'Z' | b'z' => {}
            b'+' | b'-' => {
                let sign = if b[idx] == b'-' { -1 } else { 1 };
                let rest = s.get(idx + 1..)?;
                let (hh, mm) = if rest.len() >= 5 && rest.as_bytes()[2] == b':' {
                    (
                        rest.get(0..2)?.parse::<i64>().ok()?,
                        rest.get(3..5)?.parse::<i64>().ok()?,
                    )
                } else if rest.len() >= 4 {
                    (
                        rest.get(0..2)?.parse::<i64>().ok()?,
                        rest.get(2..4)?.parse::<i64>().ok()?,
                    )
                } else if rest.len() >= 2 {
                    (rest.get(0..2)?.parse::<i64>().ok()?, 0)
                } else {
                    return None;
                };
                if hh > 23 || mm > 59 {
                    return None;
                }
                secs -= sign * (hh * 3_600 + mm * 60);
            }
            _ => return None,
        }
    }
    Some(secs)
}

/// Howard Hinnant 的 civil date → days since 1970-01-01 算法。
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

// ---------------------------------------------------------------------------
// GPX → FIT
// ---------------------------------------------------------------------------

struct BuiltRecord {
    fit_ts: u32,
    lat: f64,
    lon: f64,
    ele: Option<f64>,
    hr: Option<f64>,
    cadence: Option<f64>,
    power: Option<f64>,
    temperature: Option<f64>,
    distance_m: f64,
    speed_mps: f64,
}

/// 把解析后的 GPX 转换为单会话 FIT 文件（内存结构，不含 IO）。
pub fn gpx_to_fit(gpx: &GpxData) -> Result<(FitFile, ConvertSummary), String> {
    let sport = sport_from_track_type(gpx.track_type.as_deref());
    let mut records: Vec<BuiltRecord> = Vec::with_capacity(gpx.points.len());
    let mut skipped_no_time = 0usize;
    let mut skipped_non_increasing = 0usize;
    let mut prev_ts: Option<i64> = None;
    let mut prev_pos: Option<(f64, f64)> = None;
    let mut prev_ele: Option<f64> = None;
    let mut prev_output_distance = 0.0f64;
    let mut deltas: Vec<i64> = Vec::with_capacity(gpx.points.len());
    let mut ascent = 0.0f64;
    let mut descent = 0.0f64;

    for p in &gpx.points {
        let Some(ts) = p.time else {
            skipped_no_time += 1;
            continue;
        };
        if let Some(prev) = prev_ts {
            if ts <= prev {
                skipped_non_increasing += 1;
                continue;
            }
        }
        let fit_ts_i64 = ts - FIT_EPOCH_OFFSET;
        if fit_ts_i64 < 0 || fit_ts_i64 > u32::MAX as i64 {
            return Err(format!("轨迹点时间超出 FIT 可表示范围: {}", ts));
        }

        let seg_distance = match prev_pos {
            Some((plat, plon)) => haversine_m(plat, plon, p.lat, p.lon),
            None => 0.0,
        };
        // 优先使用设备导出的累计距离，缺失时用相邻点 haversine 累加
        let distance_m = match p.distance {
            Some(d) if d.is_finite() && d >= 0.0 => d,
            _ => prev_output_distance + seg_distance,
        };
        let dt = prev_ts.map(|prev| ts - prev);
        // 优先使用设备导出的速度，缺失时按距离差 / 时间差计算
        let speed_mps = match p.speed {
            Some(v) if v.is_finite() && v >= 0.0 => v,
            _ => match dt {
                Some(dt) if dt > 0 => (distance_m - prev_output_distance) / dt as f64,
                _ => 0.0,
            },
        };
        if let Some(dt) = dt {
            deltas.push(dt);
        }

        if let Some(ele) = p.ele {
            match prev_ele {
                None => prev_ele = Some(ele),
                Some(reference) => {
                    if ele - reference >= ELE_HYSTERESIS_M {
                        ascent += ele - reference;
                        prev_ele = Some(ele);
                    } else if reference - ele >= ELE_HYSTERESIS_M {
                        descent += reference - ele;
                        prev_ele = Some(ele);
                    }
                }
            }
        }

        records.push(BuiltRecord {
            fit_ts: fit_ts_i64 as u32,
            lat: p.lat,
            lon: p.lon,
            ele: p.ele,
            hr: p.hr,
            cadence: p.cadence,
            power: p.power,
            temperature: p.temperature,
            distance_m,
            speed_mps,
        });
        prev_ts = Some(ts);
        prev_pos = Some((p.lat, p.lon));
        prev_output_distance = distance_m;
    }

    if records.len() < 2 {
        return Err(format!(
            "有效轨迹点不足：需要至少 2 个带时间戳且时间递增的点（当前 {} 个，缺时间 {} 个，时间未递增 {} 个）",
            records.len(),
            skipped_no_time,
            skipped_non_increasing
        ));
    }

    let start_ts = records.first().unwrap().fit_ts;
    let end_ts = records.last().unwrap().fit_ts;
    let elapsed_s = end_ts - start_ts;
    if elapsed_s == 0 {
        return Err("首尾轨迹点时间相同，无法生成有效会话".to_string());
    }
    let total_distance = records.last().unwrap().distance_m;
    let timer_s = moving_time_s(&deltas, elapsed_s);

    let has_alt = records.iter().any(|r| r.ele.is_some());
    let has_hr = records.iter().any(|r| r.hr.is_some());
    let has_cadence = records.iter().any(|r| r.cadence.is_some());
    let has_power = records.iter().any(|r| r.power.is_some());
    let has_temp = records.iter().any(|r| r.temperature.is_some());

    let max_speed = records.iter().map(|r| r.speed_mps).fold(0.0f64, f64::max);
    let avg_speed = total_distance / timer_s.max(1) as f64;
    let avg_hr = average(records.iter().filter_map(|r| r.hr));
    let max_hr = max_opt(records.iter().filter_map(|r| r.hr));
    // 设备平均踏频不计入滑行时的 0 值
    let avg_cadence = average_nonzero(records.iter().filter_map(|r| r.cadence));
    let max_cadence = max_opt(records.iter().filter_map(|r| r.cadence));
    let avg_power = average(records.iter().filter_map(|r| r.power));
    let max_power = max_opt(records.iter().filter_map(|r| r.power));

    let elapsed_ms = elapsed_s.saturating_mul(1000);
    let timer_ms = timer_s.saturating_mul(1000);
    let distance_raw = scaled_u32(total_distance, 100.0);

    let mut messages: Vec<FitMessage> = Vec::with_capacity(records.len() + 12);

    // file_id
    let mut file_id = MsgBuilder::new(MSG_FILE_ID);
    file_id.enum8(0, 4); // activity
    file_id.u16f(1, 255); // manufacturer = development
    file_id.u16f(2, 0); // product
    file_id.u32f(4, start_ts);
    push_messages(&mut messages, LMT_FILE_ID, file_id);

    // event: timer start
    let mut event_def = MsgBuilder::new(MSG_EVENT);
    event_def.u32f(253, start_ts);
    event_def.enum8(0, 0); // timer
    event_def.enum8(1, 0); // start
    event_def.u8f(4, 0); // event_group
    let (ev_def, ev_start) = event_def.build(LMT_EVENT);
    messages.push(FitMessage::Definition(ev_def));
    messages.push(FitMessage::Data(ev_start));

    // records
    let record_def_pos = messages.len();
    let mut record_def: Option<DefinitionMessage> = None;
    for r in &records {
        let mut b = MsgBuilder::new(MSG_RECORD);
        b.u32f(253, r.fit_ts);
        b.s32f(0, semicircles(r.lat));
        b.s32f(1, semicircles(r.lon));
        if has_alt {
            b.u16f(2, altitude_raw(r.ele));
        }
        if has_hr {
            b.u8f(3, u8_or_invalid(r.hr));
        }
        if has_cadence {
            b.u8f(4, u8_or_invalid(r.cadence));
        }
        b.u32f(5, scaled_u32(r.distance_m, 100.0));
        b.u16f(6, speed_raw(r.speed_mps));
        if has_power {
            b.u16f(7, u16_or_invalid(r.power));
        }
        if has_temp {
            b.s8f(13, temperature_raw(r.temperature));
        }
        let (def, data) = b.build(LMT_RECORD);
        if record_def.is_none() {
            record_def = Some(def);
        }
        messages.push(FitMessage::Data(data));
    }
    if let Some(def) = record_def {
        // 定义消息必须出现在其数据消息之前
        messages.insert(record_def_pos, FitMessage::Definition(def));
    }

    // event: timer stop（复用定义，写一条带停止时间的数据）
    let mut stop = MsgBuilder::new(MSG_EVENT);
    stop.u32f(253, end_ts);
    stop.enum8(0, 0);
    stop.enum8(1, 1); // stop
    stop.u8f(4, 0);
    let (_, stop_data) = stop.build(LMT_EVENT);
    messages.push(FitMessage::Data(stop_data));

    // lap
    let mut lap = MsgBuilder::new(MSG_LAP);
    lap.u32f(253, end_ts);
    lap.enum8(0, 9); // lap
    lap.enum8(1, 1); // stop
    lap.u32f(2, start_ts);
    lap.s32f(3, semicircles(records.first().unwrap().lat));
    lap.s32f(4, semicircles(records.first().unwrap().lon));
    lap.s32f(5, semicircles(records.last().unwrap().lat));
    lap.s32f(6, semicircles(records.last().unwrap().lon));
    lap.u32f(7, elapsed_ms);
    lap.u32f(8, timer_ms);
    lap.u32f(9, distance_raw);
    if let Some(cal) = gpx.calories {
        lap.u16f(11, cal.min(65_534) as u16);
    }
    lap.u16f(13, speed_raw(avg_speed));
    lap.u16f(14, speed_raw(max_speed));
    if has_hr {
        lap.u8f(15, u8_or_invalid(Some(avg_hr)));
        lap.u8f(16, u8_or_invalid(Some(max_hr)));
    }
    if has_cadence {
        lap.u8f(17, u8_or_invalid(Some(avg_cadence)));
        lap.u8f(18, u8_or_invalid(Some(max_cadence)));
    }
    if has_power {
        lap.u16f(19, u16_or_invalid(Some(avg_power)));
        lap.u16f(20, u16_or_invalid(Some(max_power)));
    }
    lap.u16f(21, scaled_u16(ascent, 1.0, 0.0));
    lap.u16f(22, scaled_u16(descent, 1.0, 0.0));
    lap.enum8(23, 0); // intensity = active
    lap.enum8(24, 7); // lap_trigger = session_end
    lap.enum8(25, sport);
    push_messages(&mut messages, LMT_LAP, lap);

    // session
    let mut session = MsgBuilder::new(MSG_SESSION);
    session.u32f(253, end_ts);
    session.enum8(0, 8); // event = session
    session.enum8(1, 1); // event_type = stop
    session.u32f(2, start_ts);
    session.enum8(5, sport);
    session.enum8(6, 0); // sub_sport = generic
    session.u32f(7, elapsed_ms);
    session.u32f(8, timer_ms);
    session.u32f(9, distance_raw);
    if let Some(cal) = gpx.calories {
        session.u16f(11, cal.min(65_534) as u16);
    }
    session.u16f(14, speed_raw(avg_speed));
    session.u16f(15, speed_raw(max_speed));
    if has_hr {
        session.u8f(16, u8_or_invalid(Some(avg_hr)));
        session.u8f(17, u8_or_invalid(Some(max_hr)));
    }
    if has_cadence {
        session.u8f(18, u8_or_invalid(Some(avg_cadence)));
        session.u8f(19, u8_or_invalid(Some(max_cadence)));
    }
    if has_power {
        session.u16f(20, u16_or_invalid(Some(avg_power)));
        session.u16f(21, u16_or_invalid(Some(max_power)));
    }
    session.u16f(22, scaled_u16(ascent, 1.0, 0.0));
    session.u16f(23, scaled_u16(descent, 1.0, 0.0));
    session.u16f(25, 0); // first_lap_index
    session.u16f(26, 1); // num_laps
    push_messages(&mut messages, LMT_SESSION, session);

    // activity
    let mut activity = MsgBuilder::new(MSG_ACTIVITY);
    activity.u32f(253, end_ts);
    activity.u32f(0, timer_ms);
    activity.u16f(1, 1); // num_sessions
    activity.enum8(2, 0); // type = manual
    activity.enum8(3, 26); // event = activity
    activity.enum8(4, 1); // event_type = stop
    activity.u32f(5, end_ts); // local_timestamp
    activity.u8f(6, 0); // event_group
    push_messages(&mut messages, LMT_ACTIVITY, activity);

    let fit = FitFile {
        header: FitHeader::default(),
        messages,
        crc: 0,
    };
    let summary = ConvertSummary {
        input_points: gpx.points.len(),
        records: records.len(),
        skipped_no_time,
        skipped_non_increasing,
        distance_m: total_distance,
        elapsed_s: elapsed_s as u32,
        timer_s,
        sport,
        calories: gpx.calories,
        has_hr,
        has_cadence,
        has_power,
    };
    Ok((fit, summary))
}

fn average<I: Iterator<Item = f64>>(values: I) -> f64 {
    let (sum, count) = values.fold((0.0, 0usize), |(sum, n), v| (sum + v, n + 1));
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

fn average_nonzero<I: Iterator<Item = f64>>(values: I) -> f64 {
    let (sum, count) = values
        .filter(|v| *v > 0.0)
        .fold((0.0, 0usize), |(sum, n), v| (sum + v, n + 1));
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// 计时时间：只累计不超过典型采样间隔若干倍的相邻点间隔，剔除自动暂停的长空档。
fn moving_time_s(deltas: &[i64], elapsed_s: u32) -> u32 {
    if deltas.is_empty() {
        return elapsed_s;
    }
    let mut sorted = deltas.to_vec();
    sorted.sort_unstable();
    let median = sorted[sorted.len() / 2].max(1);
    let max_gap = (median * 3).clamp(10, 300);
    let moving: i64 = deltas.iter().filter(|d| **d <= max_gap).sum();
    if moving <= 0 {
        elapsed_s
    } else {
        (moving as u32).min(elapsed_s)
    }
}

fn max_opt<I: Iterator<Item = f64>>(values: I) -> f64 {
    values.fold(0.0f64, f64::max)
}

fn sport_from_track_type(track_type: Option<&str>) -> u8 {
    let t = track_type.unwrap_or("").to_ascii_lowercase();
    match t.as_str() {
        "running" | "run" => 1,
        "cycling" | "biking" | "bike" | "ride" => 2,
        "swimming" | "swim" => 5,
        "walking" | "walk" => 11,
        "cross_country_skiing" | "crosscountryskiing" | "skiing" => 12,
        "rowing" => 15,
        "mountaineering" => 16,
        "hiking" | "hike" => 17,
        _ => 0,
    }
}

fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0;
    let p1 = lat1.to_radians();
    let p2 = lat2.to_radians();
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * a.sqrt().asin()
}

fn semicircles(deg: f64) -> i32 {
    let raw = (deg * (2_147_483_648.0 / 180.0)).round();
    raw.clamp(-2_147_483_648.0, 2_147_483_647.0) as i32
}

/// 通用定点缩放，越界返回 0xFFFF。
fn scaled_u16(value: f64, scale: f64, offset: f64) -> u16 {
    let raw = ((value + offset) * scale).round();
    if !raw.is_finite() || raw < 0.0 || raw > 65_534.0 {
        0xFFFF
    } else {
        raw as u16
    }
}

fn scaled_u32(value: f64, scale: f64) -> u32 {
    let raw = (value * scale).round();
    if !raw.is_finite() || raw <= 0.0 {
        0
    } else if raw >= u32::MAX as f64 {
        u32::MAX
    } else {
        raw as u32
    }
}

fn altitude_raw(ele: Option<f64>) -> u16 {
    match ele {
        Some(v) => scaled_u16(v, 5.0, 500.0),
        None => 0xFFFF,
    }
}

fn speed_raw(speed_mps: f64) -> u16 {
    scaled_u16(speed_mps, 1000.0, 0.0)
}

fn u8_or_invalid(value: Option<f64>) -> u8 {
    match value {
        Some(v) => {
            let raw = v.round();
            if !raw.is_finite() || raw < 0.0 || raw > 254.0 {
                0xFF
            } else {
                raw as u8
            }
        }
        None => 0xFF,
    }
}

fn u16_or_invalid(value: Option<f64>) -> u16 {
    match value {
        Some(v) => {
            let raw = v.round();
            if !raw.is_finite() || raw < 0.0 || raw > 65_534.0 {
                0xFFFF
            } else {
                raw as u16
            }
        }
        None => 0xFFFF,
    }
}

fn temperature_raw(value: Option<f64>) -> u8 {
    match value {
        Some(v) => {
            let raw = v.round();
            if !raw.is_finite() || !(-127.0..=126.0).contains(&raw) {
                0x7F
            } else {
                raw as i8 as u8
            }
        }
        None => 0x7F,
    }
}

// ---------------------------------------------------------------------------
// FIT 消息构造
// ---------------------------------------------------------------------------

struct MsgBuilder {
    global: u16,
    fields: Vec<FieldDefinition>,
    payload: Vec<u8>,
}

fn push_messages(messages: &mut Vec<FitMessage>, lmt: u8, builder: MsgBuilder) {
    let (def, data) = builder.build(lmt);
    messages.push(FitMessage::Definition(def));
    messages.push(FitMessage::Data(data));
}

impl MsgBuilder {
    fn new(global: u16) -> Self {
        Self {
            global,
            fields: Vec::new(),
            payload: Vec::new(),
        }
    }

    fn add_field(&mut self, num: u8, size: u8, base_type_raw: u8) {
        self.fields.push(FieldDefinition {
            field_definition_number: num,
            size,
            base_type_raw,
        });
    }

    fn enum8(&mut self, num: u8, value: u8) {
        self.add_field(num, 1, 0x00);
        self.payload.push(value);
    }

    fn u8f(&mut self, num: u8, value: u8) {
        self.add_field(num, 1, 0x02);
        self.payload.push(value);
    }

    fn s8f(&mut self, num: u8, value: u8) {
        self.add_field(num, 1, 0x01);
        self.payload.push(value);
    }

    fn u16f(&mut self, num: u8, value: u16) {
        self.add_field(num, 2, 0x84);
        self.payload.extend_from_slice(&value.to_le_bytes());
    }

    fn s32f(&mut self, num: u8, value: i32) {
        self.add_field(num, 4, 0x85);
        self.payload.extend_from_slice(&value.to_le_bytes());
    }

    fn u32f(&mut self, num: u8, value: u32) {
        self.add_field(num, 4, 0x86);
        self.payload.extend_from_slice(&value.to_le_bytes());
    }

    fn build(self, lmt: u8) -> (DefinitionMessage, DataMessage) {
        let def = DefinitionMessage {
            local_message_type: lmt,
            global_message_number: self.global,
            architecture: Architecture::LittleEndian,
            fields: self.fields,
            developer_fields: Vec::new(),
        };
        let data = DataMessage {
            local_message_type: lmt,
            global_message_number: self.global,
            compressed_timestamp_offset: None,
            payload: self.payload,
        };
        (def, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
    <gpx version="1.1" creator="test" xmlns="http://www.topografix.com/GPX/1/1"
        xmlns:gpxtpx="http://www.garmin.com/xmlschemas/TrackPointExtension/v1">
    <trk>
        <name>Morning Ride</name>
        <type>cycling</type>
        <trkseg>
        <trkpt lat="39.900000" lon="116.400000">
            <ele>50.0</ele>
            <time>2026-05-01T08:00:00Z</time>
            <extensions><gpxtpx:TrackPointExtension>
            <gpxtpx:hr>120</gpxtpx:hr><gpxtpx:cad>80</gpxtpx:cad>
            <gpxtpx:atemp>22.5</gpxtpx:atemp><power>180</power>
            </gpxtpx:TrackPointExtension></extensions>
        </trkpt>
        <trkpt lat="39.901000" lon="116.400000">
            <ele>53.0</ele>
            <time>2026-05-01T08:00:10Z</time>
            <extensions><gpxtpx:TrackPointExtension>
            <gpxtpx:hr>125</gpxtpx:hr><gpxtpx:cad>82</gpxtpx:cad>
            <gpxtpx:atemp>23.0</gpxtpx:atemp><power>190</power>
            </gpxtpx:TrackPointExtension></extensions>
        </trkpt>
        <trkpt lat="39.902000" lon="116.400000">
            <ele>49.0</ele>
            <time>2026-05-01T08:00:20Z</time>
        </trkpt>
        </trkseg>
    </trk>
    </gpx>"#;

    #[test]
    fn parses_points_and_extensions() {
        let gpx = parse_gpx(SAMPLE).unwrap();
        assert_eq!(gpx.points.len(), 3);
        assert_eq!(gpx.track_type.as_deref(), Some("cycling"));
        let p = &gpx.points[0];
        assert!((p.ele.unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(p.hr, Some(120.0));
        assert_eq!(p.cadence, Some(80.0));
        assert_eq!(p.power, Some(180.0));
        assert_eq!(p.temperature, Some(22.5));
        assert!(gpx.points[2].hr.is_none());
    }

    #[test]
    fn parses_iso8601_with_offsets() {
        let base = parse_iso8601_utc("2026-05-01T08:00:00Z").unwrap();
        assert_eq!(parse_iso8601_utc("2026-05-01T16:00:00+08:00"), Some(base));
        assert_eq!(parse_iso8601_utc("2026-05-01T08:00:00.250Z"), Some(base));
        assert_eq!(parse_iso8601_utc("2026-05-01T00:00:00-08:00"), Some(base));
    }

    #[test]
    fn haversine_matches_known_distance() {
        // 纬度 1 度约 111.19 km
        let d = haversine_m(39.0, 116.0, 40.0, 116.0);
        assert!((d - 111_195.0).abs() < 200.0, "distance={}", d);
    }

    #[test]
    fn builds_single_session_fit() {
        let gpx = parse_gpx(SAMPLE).unwrap();
        let (fit, summary) = gpx_to_fit(&gpx).unwrap();
        assert_eq!(summary.records, 3);
        assert_eq!(summary.sport, 2);
        assert_eq!(summary.elapsed_s, 20);
        assert!(summary.distance_m > 200.0 && summary.distance_m < 300.0);

        let count = |global: u16| {
            fit.messages
                .iter()
                .filter(|m| matches!(m, FitMessage::Data(d) if d.global_message_number == global))
                .count()
        };
        assert_eq!(count(MSG_FILE_ID), 1);
        assert_eq!(count(MSG_RECORD), 3);
        assert_eq!(count(MSG_LAP), 1);
        assert_eq!(count(MSG_SESSION), 1);
        assert_eq!(count(MSG_ACTIVITY), 1);

        // 定义消息必须在其数据消息之前
        let first_record_data = fit
            .messages
            .iter()
            .position(|m| matches!(m, FitMessage::Data(d) if d.global_message_number == MSG_RECORD))
            .unwrap();
        let record_def = fit
            .messages
            .iter()
            .position(
                |m| matches!(m, FitMessage::Definition(d) if d.global_message_number == MSG_RECORD),
            )
            .unwrap();
        assert!(record_def < first_record_data);
    }

    #[test]
    fn uses_gpx_distance_speed_and_skips_pauses() {
        let xml = r#"<?xml version="1.0"?>
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
    </gpx>"#;
        let gpx = parse_gpx(xml).unwrap();
        assert_eq!(gpx.points[0].distance, Some(0.0));
        assert_eq!(gpx.points[3].speed, Some(5.0));

        let (_, summary) = gpx_to_fit(&gpx).unwrap();
        assert_eq!(summary.distance_m, 30.0);
        assert_eq!(summary.elapsed_s, 100);
        assert_eq!(summary.timer_s, 2);
        assert_eq!(summary.sport, 2);
    }

    #[test]
    fn parses_lap_and_metadata_calories() {
        let xml = r#"<?xml version="1.0"?>
    <gpx xmlns="http://www.topografix.com/GPX/1/1" xmlns:gpxdata="http://www.cluetrust.com/XML/GPXDATA/1/0">
    <metadata><extensions><gpxdata:calories>999</gpxdata:calories></extensions></metadata>
    <trk><trkseg>
        <trkpt lat="39.9000" lon="116.4000"><time>2026-05-01T08:00:00Z</time>
        <extensions><gpxdata:calories>5</gpxdata:calories></extensions></trkpt>
        <trkpt lat="39.9001" lon="116.4000"><time>2026-05-01T08:00:01Z</time></trkpt>
    </trkseg></trk>
    <extensions><gpxdata:laps>
        <gpxdata:lap><gpxdata:calories>120</gpxdata:calories></gpxdata:lap>
        <gpxdata:lap><gpxdata:calories>80</gpxdata:calories></gpxdata:lap>
    </gpxdata:laps></extensions>
    </gpx>"#;
        let gpx = parse_gpx(xml).unwrap();
        // lap 级总和优先于 metadata 级单值，逐点 calories 忽略
        assert_eq!(gpx.calories, Some(200));

        let metadata_only = r#"<gpx xmlns:gpxdata="http://www.cluetrust.com/XML/GPXDATA/1/0">
    <metadata><extensions><gpxdata:calories>42</gpxdata:calories></extensions></metadata>
    <trk><trkseg>
        <trkpt lat="1" lon="2"><time>2026-05-01T08:00:00Z</time></trkpt>
        <trkpt lat="1.1" lon="2"><time>2026-05-01T08:00:01Z</time></trkpt>
    </trkseg></trk>
    </gpx>"#;
        assert_eq!(parse_gpx(metadata_only).unwrap().calories, Some(42));
    }
}
