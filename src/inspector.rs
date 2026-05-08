//! FIT 文件内容检查器：读取一个 FIT 文件，按 session（会话）逐个输出统计摘要，
//! 包括距离、耗时、计时、热量、上升/下降、功率/踏频/心率 的平均与最大值。
//!
//! 如果文件不包含 session 消息（极少见），则退化为遍历 record 消息做实时聚合。

use crate::fit_parser::read_fit_file;
use crate::fit_types::*;
use std::collections::HashMap;

// FIT 全局消息号
const MSG_SESSION: u16 = 18;
const MSG_RECORD: u16 = 20;

/// 单个会话的摘要数据（原始单位为 FIT 协议单位，打印时换算）。
#[derive(Debug, Default, Clone)]
pub struct SessionSummary {
    pub start_time: Option<u32>,       // FIT 时间戳（1989-12-31 00:00:00 UTC 起的秒数）
    pub total_distance: Option<u32>,   // 1/100 m
    pub total_elapsed_time: Option<u32>, // 1/1000 s
    pub total_timer_time: Option<u32>,   // 1/1000 s
    pub total_calories: Option<u16>,   // kcal
    pub total_ascent: Option<u16>,     // m
    pub total_descent: Option<u16>,    // m
    pub avg_speed: Option<u16>,        // 1/1000 m/s
    pub max_speed: Option<u16>,        // 1/1000 m/s
    pub avg_heart_rate: Option<u8>,
    pub max_heart_rate: Option<u8>,
    pub avg_cadence: Option<u8>,
    pub max_cadence: Option<u8>,
    pub avg_power: Option<u16>,
    pub max_power: Option<u16>,
}

impl SessionSummary {
    /// 从 session 消息的 definition + payload 中提取所有关心的字段。
    /// 字段定义号严格遵循 FIT 协议规范（来源：FIT SDK profile）。
    pub fn from_session(def: &DefinitionMessage, payload: &[u8]) -> Self {
        Self {
            start_time: read_u32(def, payload, 2),
            total_elapsed_time: read_u32(def, payload, 7),
            total_timer_time: read_u32(def, payload, 8),
            total_distance: read_u32(def, payload, 9),
            total_calories: read_u16(def, payload, 11),
            avg_speed: read_u16(def, payload, 14).filter(|&v| v != 0xFFFF),
            max_speed: read_u16(def, payload, 15).filter(|&v| v != 0xFFFF),
            avg_heart_rate: read_u8(def, payload, 16).filter(|&v| v != 0xFF),
            max_heart_rate: read_u8(def, payload, 17).filter(|&v| v != 0xFF),
            avg_cadence: read_u8(def, payload, 18).filter(|&v| v != 0xFF),
            max_cadence: read_u8(def, payload, 19).filter(|&v| v != 0xFF),
            avg_power: read_u16(def, payload, 20).filter(|&v| v != 0xFFFF),
            max_power: read_u16(def, payload, 21).filter(|&v| v != 0xFFFF),
            total_ascent: read_u16(def, payload, 22),
            total_descent: read_u16(def, payload, 23),
        }
    }
}

/// 格式化一行摘要输出。
pub fn format_summary(idx: usize, s: &SessionSummary) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = write!(out, "会话 {}:", idx);

    if let Some(v) = s.total_distance {
        // 协议单位：1/100 m → 转换为 km
        let _ = write!(out, " 距离={:.3} km", v as f64 / 100_000.0);
    }
    if let Some(v) = s.total_elapsed_time {
        let _ = write!(out, ", 耗时={}", format_duration_ms(v));
    }
    if let Some(v) = s.total_timer_time {
        let _ = write!(out, ", 计时={}", format_duration_ms(v));
    }
    if let Some(v) = s.total_calories {
        let _ = write!(out, ", 卡路里={} kcal", v);
    }
    if let Some(v) = s.total_ascent {
        let _ = write!(out, ", 上升={} m", v);
    }
    if let Some(v) = s.total_descent {
        let _ = write!(out, ", 下降={} m", v);
    }
    if let Some(v) = s.avg_speed {
        // 协议单位：1/1000 m/s → m/s = v/1000；km/h = m/s * 3.6
        let _ = write!(out, ", 平均速度={:.2} km/h", v as f64 * 3.6 / 1000.0);
    }
    if let Some(v) = s.max_speed {
        let _ = write!(out, ", 最大速度={:.2} km/h", v as f64 * 3.6 / 1000.0);
    }
    if let Some(v) = s.avg_heart_rate {
        let _ = write!(out, ", 平均心率={} bpm", v);
    }
    if let Some(v) = s.max_heart_rate {
        let _ = write!(out, ", 最大心率={} bpm", v);
    }
    if let Some(v) = s.avg_cadence {
        let _ = write!(out, ", 平均踏频={} rpm", v);
    }
    if let Some(v) = s.max_cadence {
        let _ = write!(out, ", 最大踏频={} rpm", v);
    }
    if let Some(v) = s.avg_power {
        let _ = write!(out, ", 平均功率={} W", v);
    }
    if let Some(v) = s.max_power {
        let _ = write!(out, ", 最大功率={} W", v);
    }
    out
}

/// 从给定 FIT 文件提取所有会话摘要。
pub fn summarize_file(path: &str) -> Result<Vec<SessionSummary>, String> {
    let file = read_fit_file(path)?;

    // 收集所有 LMT -> 当前定义（按出现顺序，后出现的 Definition 会覆盖旧的）
    let mut defs: HashMap<u8, DefinitionMessage> = HashMap::new();
    let mut sessions: Vec<SessionSummary> = Vec::new();

    for msg in &file.messages {
        match msg {
            FitMessage::Definition(d) => {
                defs.insert(d.local_message_type, d.clone());
            }
            FitMessage::Data(d) if d.global_message_number == MSG_SESSION => {
                if let Some(def) = defs.get(&d.local_message_type) {
                    sessions.push(SessionSummary::from_session(def, &d.payload));
                }
            }
            _ => {}
        }
    }

    if !sessions.is_empty() {
        return Ok(sessions);
    }

    // 退化路径：没有 session 消息，通过 record 计算一个"虚拟会话"
    eprintln!("警告：{} 中未发现 session 消息，基于 record 计算摘要。", path);
    let mut agg = RecordAggregator::default();
    for msg in &file.messages {
        if let FitMessage::Data(d) = msg {
            if d.global_message_number == MSG_RECORD {
                if let Some(def) = defs.get(&d.local_message_type) {
                    agg.absorb(def, &d.payload);
                }
            }
        }
    }
    Ok(vec![agg.to_summary()])
}

// ==== record 聚合器（退化路径使用） ====

#[derive(Default)]
struct RecordAggregator {
    first_ts: Option<u32>,
    last_ts: Option<u32>,
    first_dist: Option<u32>,
    last_dist: Option<u32>,
    hr_sum: u64,
    hr_cnt: u64,
    hr_max: u8,
    cad_sum: u64,
    cad_cnt: u64,
    cad_max: u8,
    pw_sum: u64,
    pw_cnt: u64,
    pw_max: u16,
    speed_max: u16,
}

impl RecordAggregator {
    fn absorb(&mut self, def: &DefinitionMessage, payload: &[u8]) {
        if let Some(ts) = read_u32(def, payload, 253) {
            self.first_ts.get_or_insert(ts);
            self.last_ts = Some(ts);
        }
        if let Some(dist) = read_u32(def, payload, 5) {
            self.first_dist.get_or_insert(dist);
            self.last_dist = Some(dist);
        }
        if let Some(hr) = read_u8(def, payload, 3).filter(|&v| v != 0xFF) {
            self.hr_sum += hr as u64;
            self.hr_cnt += 1;
            self.hr_max = self.hr_max.max(hr);
        }
        if let Some(cad) = read_u8(def, payload, 4).filter(|&v| v != 0xFF) {
            self.cad_sum += cad as u64;
            self.cad_cnt += 1;
            self.cad_max = self.cad_max.max(cad);
        }
        if let Some(pw) = read_u16(def, payload, 7).filter(|&v| v != 0xFFFF) {
            self.pw_sum += pw as u64;
            self.pw_cnt += 1;
            self.pw_max = self.pw_max.max(pw);
        }
        if let Some(sp) = read_u16(def, payload, 6).filter(|&v| v != 0xFFFF) {
            self.speed_max = self.speed_max.max(sp);
        }
    }

    fn to_summary(&self) -> SessionSummary {
        let elapsed = match (self.first_ts, self.last_ts) {
            (Some(a), Some(b)) if b >= a => Some((b - a).saturating_mul(1000)),
            _ => None,
        };
        let dist = match (self.first_dist, self.last_dist) {
            (Some(a), Some(b)) if b >= a => Some(b - a),
            _ => None,
        };
        SessionSummary {
            start_time: self.first_ts,
            total_distance: dist,
            total_elapsed_time: elapsed,
            total_timer_time: elapsed,
            total_calories: None,
            total_ascent: None,
            total_descent: None,
            avg_speed: None,
            max_speed: if self.speed_max > 0 { Some(self.speed_max) } else { None },
            avg_heart_rate: if self.hr_cnt > 0 { Some((self.hr_sum / self.hr_cnt) as u8) } else { None },
            max_heart_rate: if self.hr_max > 0 { Some(self.hr_max) } else { None },
            avg_cadence: if self.cad_cnt > 0 { Some((self.cad_sum / self.cad_cnt) as u8) } else { None },
            max_cadence: if self.cad_max > 0 { Some(self.cad_max) } else { None },
            avg_power: if self.pw_cnt > 0 { Some((self.pw_sum / self.pw_cnt) as u16) } else { None },
            max_power: if self.pw_max > 0 { Some(self.pw_max) } else { None },
        }
    }
}

// ==== 字段字节级读取小工具 ====

fn locate<'a>(def: &DefinitionMessage, payload: &'a [u8], num: u8) -> Option<(&'a [u8], Architecture)> {
    let mut off = 0usize;
    for f in &def.fields {
        let sz = f.size as usize;
        if f.field_definition_number == num {
            if off + sz > payload.len() {
                return None;
            }
            return Some((&payload[off..off + sz], def.architecture));
        }
        off += sz;
    }
    None
}

fn read_u8(def: &DefinitionMessage, payload: &[u8], num: u8) -> Option<u8> {
    locate(def, payload, num).and_then(|(s, _)| s.get(0).copied())
}
fn read_u16(def: &DefinitionMessage, payload: &[u8], num: u8) -> Option<u16> {
    let (s, arch) = locate(def, payload, num)?;
    if s.len() < 2 { return None; }
    Some(match arch {
        Architecture::LittleEndian => u16::from_le_bytes([s[0], s[1]]),
        Architecture::BigEndian => u16::from_be_bytes([s[0], s[1]]),
    })
}
/// 把 1/1000 s 单位的时长格式化为 `hh:mm:ss`（超过 24 小时正常进位到 hh）。
fn format_duration_ms(ms: u32) -> String {
    let total_secs = (ms as u64 + 500) / 1000; // 四舍五入到秒
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

#[cfg(test)]
mod tests {
    use super::format_duration_ms;

    #[test]
    fn duration_formatting() {
        assert_eq!(format_duration_ms(0), "00:00:00");
        assert_eq!(format_duration_ms(1_500), "00:00:02"); // 1.5s 四舍五入
        assert_eq!(format_duration_ms(3_600_000), "01:00:00");
        assert_eq!(format_duration_ms(20_867_280), "05:47:47"); // ≈5h47m
        assert_eq!(format_duration_ms(162_420_380), "45:07:00"); // 合并耗时
    }
}

fn read_u32(def: &DefinitionMessage, payload: &[u8], num: u8) -> Option<u32> {
    let (s, arch) = locate(def, payload, num)?;
    if s.len() < 4 { return None; }
    Some(match arch {
        Architecture::LittleEndian => u32::from_le_bytes([s[0], s[1], s[2], s[3]]),
        Architecture::BigEndian => u32::from_be_bytes([s[0], s[1], s[2], s[3]]),
    })
}