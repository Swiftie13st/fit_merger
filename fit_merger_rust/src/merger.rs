//! FIT 文件合并器：合并多个骑行 FIT 文件为单一会话，保留全部数据字段。
//!
//! 合并策略：
//! 1. 时间序列消息（record/event/lap/length/hr/device_info/...）按原文件顺序串接，
//!    其原始字节 payload 完整保留 —— 心率、踏频、功率、海拔、距离、温度、GPS 等
//!    所有字段（包括 manufacturer 私有字段）都不会丢失。
//! 2. 三种"摘要类"消息会被重写为单条记录：
//!    - file_id：取首个文件的元数据，更新 time_created 为最早开始时间
//!    - session：聚合 6 个文件的 totals（距离累加、耗时累加、avg 加权、max 取最大）
//!    - activity：累加 total_timer_time，num_sessions = 1
//! 3. 通过重新分配 local_message_type，避免不同文件之间 LMT 冲突。
//! 4. 输出文件含合规的头部 CRC 与文件末尾 CRC。

use crate::fit_generator::write_fit_file;
use crate::fit_parser::read_fit_file;
use crate::fit_types::*;

// FIT 全局消息号常量（来自 FIT 协议）
const MSG_FILE_ID: u16 = 0;
const MSG_SESSION: u16 = 18;
const MSG_ACTIVITY: u16 = 34;

/// 主入口：合并多个 FIT 文件，写入 `output_path`。
pub fn merge_fit_files(input_paths: &[&str], output_path: &str) -> Result<(), String> {
    if input_paths.is_empty() {
        return Err("没有输入文件".to_string());
    }
    let mut files = Vec::with_capacity(input_paths.len());
    for p in input_paths {
        let f = read_fit_file(p)?;
        println!(
            "  解析 {} → {} 条消息（含 {} 条数据消息）",
            p,
            f.messages.len(),
            f.messages
                .iter()
                .filter(|m| matches!(m, FitMessage::Data(_)))
                .count()
        );
        files.push(f);
    }
    let merged = merge(&files)?;
    write_fit_file(&merged, output_path)?;
    Ok(())
}

/// 合并 FIT 文件结构（不含 IO）。
pub fn merge(files: &[FitFile]) -> Result<FitFile, String> {
    if files.is_empty() {
        return Err("没有输入文件".to_string());
    }
    if files.len() == 1 {
        return Ok(files[0].clone());
    }

    // 基础统计量聚合
    let mut session_acc = SessionAccumulator::new();
    let mut activity_total_timer_time: u64 = 0;
    let mut earliest_start_time: Option<u32> = None;
    let mut earliest_time_created: Option<u32> = None;

    let mut first_file_id_def: Option<DefinitionMessage> = None;
    let mut first_file_id_data: Option<DataMessage> = None;
    let mut first_session_def: Option<DefinitionMessage> = None;
    let mut first_session_data: Option<DataMessage> = None;
    let mut first_activity_def: Option<DefinitionMessage> = None;
    let mut first_activity_data: Option<DataMessage> = None;

    // 输出消息列表（先收集时间序列消息，最后追加 session/activity）
    let mut out_messages: Vec<FitMessage> = Vec::new();
    // 全局 LMT 分配器（0..=12 循环；13/14/15 保留给 activity/session/file_id）
    let mut next_lmt: u8 = 0;

    for (file_idx, file) in files.iter().enumerate() {
        // 局部 LMT (in this file) -> 全局 LMT 映射
        let mut lmt_remap: std::collections::HashMap<u8, u8> = std::collections::HashMap::new();
        // 每条 LMT 对应的本文件定义
        let mut local_defs: std::collections::HashMap<u8, DefinitionMessage> =
            std::collections::HashMap::new();

        for msg in &file.messages {
            match msg {
                FitMessage::Definition(def) => {
                    local_defs.insert(def.local_message_type, def.clone());

                    // 跳过 file_id/session/activity 的定义（我们最后统一重写）
                    let g = def.global_message_number;
                    if g == MSG_FILE_ID || g == MSG_SESSION || g == MSG_ACTIVITY {
                        // 但保留 first 文件的定义模板供之后重写使用
                        if file_idx == 0 {
                            match g {
                                MSG_FILE_ID => first_file_id_def = Some(def.clone()),
                                MSG_SESSION => first_session_def = Some(def.clone()),
                                MSG_ACTIVITY => first_activity_def = Some(def.clone()),
                                _ => {}
                            }
                        }
                        continue;
                    }

                    // 分配新的 LMT（0..=12 循环；13/14/15 保留给 activity/session/file_id）
                    let new_lmt = next_lmt;
                    next_lmt = (next_lmt + 1) % 13;
                    lmt_remap.insert(def.local_message_type, new_lmt);

                    let mut new_def = def.clone();
                    new_def.local_message_type = new_lmt;
                    out_messages.push(FitMessage::Definition(new_def));
                }
                FitMessage::Data(data) => {
                    let g = data.global_message_number;
                    // 提取 file_id：第一个文件的保存以备重写
                    if g == MSG_FILE_ID {
                        if file_idx == 0 && first_file_id_data.is_none() {
                            first_file_id_data = Some(data.clone());
                        }
                        // 提取 time_created（field 4, uint32）以更新最早时间
                        if let Some(def) = local_defs.get(&data.local_message_type) {
                            if let Some(v) = read_field_u32(def, &data.payload, 4) {
                                earliest_time_created = Some(match earliest_time_created {
                                    Some(e) => e.min(v),
                                    None => v,
                                });
                            }
                        }
                        continue;
                    }

                    if g == MSG_SESSION {
                        if let Some(def) = local_defs.get(&data.local_message_type) {
                            session_acc.absorb(def, &data.payload);
                            if let Some(st) = read_field_u32(def, &data.payload, 2) {
                                earliest_start_time = Some(match earliest_start_time {
                                    Some(e) => e.min(st),
                                    None => st,
                                });
                            }
                        }
                        if file_idx == 0 && first_session_data.is_none() {
                            first_session_data = Some(data.clone());
                        }
                        continue;
                    }

                    if g == MSG_ACTIVITY {
                        if let Some(def) = local_defs.get(&data.local_message_type) {
                            // total_timer_time field=0
                            if let Some(v) = read_field_u32(def, &data.payload, 0) {
                                activity_total_timer_time += v as u64;
                            }
                        }
                        if file_idx == 0 && first_activity_data.is_none() {
                            first_activity_data = Some(data.clone());
                        }
                        continue;
                    }

                    // 普通数据消息：将原始 payload 原封不动写出
                    let &new_lmt = lmt_remap.get(&data.local_message_type).ok_or_else(|| {
                        format!(
                            "文件 {} 的 LMT={} 没有先前的定义消息",
                            file_idx, data.local_message_type
                        )
                    })?;
                    // 仅文件级首条消息要求是 file_id —— 我们已用合并后的 file_id 替代
                    let mut new_data = data.clone();
                    new_data.local_message_type = new_lmt;
                    out_messages.push(FitMessage::Data(new_data));
                }
            }
        }
    }

    // ---- 现在装配最终消息序列：file_id → ...时间序列... → session → activity ----

    // 1. file_id 放在最前
    let (fid_def, fid_data) = build_file_id(
        first_file_id_def.as_ref(),
        first_file_id_data.as_ref(),
        earliest_time_created,
    )?;

    let mut final_msgs: Vec<FitMessage> = Vec::with_capacity(out_messages.len() + 6);
    final_msgs.push(FitMessage::Definition(fid_def));
    final_msgs.push(FitMessage::Data(fid_data));
    final_msgs.extend(out_messages);

    // 2. session
    let (sess_def, sess_data) = build_session(
        first_session_def.as_ref(),
        first_session_data.as_ref(),
        &session_acc,
        earliest_start_time,
    )?;
    final_msgs.push(FitMessage::Definition(sess_def));
    final_msgs.push(FitMessage::Data(sess_data));

    // 3. activity
    // 使用合并后的 session.total_timer_time（所有分段的计时时间之和）作为 activity 的 total_timer_time。
    // 这比简单累加源 activity.total_timer_time 更准确，也与合并后的 session 保持一致。
    let act_timer = if session_acc.total_timer_time > 0 {
        session_acc.total_timer_time as u32
    } else {
        activity_total_timer_time as u32
    };
    let (act_def, act_data) = build_activity(
        first_activity_def.as_ref(),
        first_activity_data.as_ref(),
        act_timer,
    )?;
    final_msgs.push(FitMessage::Definition(act_def));
    final_msgs.push(FitMessage::Data(act_data));

    Ok(FitFile {
        header: FitHeader::default(),
        messages: final_msgs,
        crc: 0,
    })
}

/// session 聚合器：累加 totals，记录 max/min，按时间加权平均 avg
struct SessionAccumulator {
    total_elapsed_time: u64, // field 7, uint32, 1/1000 s
    total_timer_time: u64,   // field 8
    total_distance: u64,     // field 9, uint32, /100 m
    total_cycles: u64,       // field 10, uint32
    total_calories: u32,     // field 11, uint16
    total_ascent: u32,       // field 22, uint16
    total_descent: u32,      // field 23, uint16
    max_speed: u16,          // field 14, uint16, /1000 m/s
    avg_speed_num: u64,      // 加权
    avg_speed_w: u64,
    max_hr: u8,    // field 16
    avg_hr_num: u64, // weighted by timer_time
    avg_hr_w: u64,
    max_cadence: u8, // field 18
    avg_cadence_num: u64,
    avg_cadence_w: u64,
    max_power: u16, // field 20
    avg_power_num: u64,
    avg_power_w: u64,
}

impl SessionAccumulator {
    fn new() -> Self {
        Self {
            total_elapsed_time: 0,
            total_timer_time: 0,
            total_distance: 0,
            total_cycles: 0,
            total_calories: 0,
            total_ascent: 0,
            total_descent: 0,
            max_speed: 0,
            avg_speed_num: 0,
            avg_speed_w: 0,
            max_hr: 0,
            avg_hr_num: 0,
            avg_hr_w: 0,
            max_cadence: 0,
            avg_cadence_num: 0,
            avg_cadence_w: 0,
            max_power: 0,
            avg_power_num: 0,
            avg_power_w: 0,
        }
    }

    fn absorb(&mut self, def: &DefinitionMessage, payload: &[u8]) {
        let timer = read_field_u32(def, payload, 8).unwrap_or(0) as u64; // 1/1000s
        let elapsed = read_field_u32(def, payload, 7).unwrap_or(0) as u64;
        self.total_timer_time += timer;
        self.total_elapsed_time += elapsed;
        if let Some(v) = read_field_u32(def, payload, 9) {
            self.total_distance += v as u64;
        }
        if let Some(v) = read_field_u32(def, payload, 10) {
            self.total_cycles += v as u64;
        }
        if let Some(v) = read_field_u16(def, payload, 11) {
            self.total_calories += v as u32;
        }
        if let Some(v) = read_field_u16(def, payload, 22) {
            self.total_ascent += v as u32;
        }
        if let Some(v) = read_field_u16(def, payload, 23) {
            self.total_descent += v as u32;
        }
        if let Some(v) = read_field_u16(def, payload, 14) {
            self.max_speed = self.max_speed.max(v);
        }
        if let Some(v) = read_field_u16(def, payload, 13) {
            // avg_speed
            self.avg_speed_num += v as u64 * timer.max(1);
            self.avg_speed_w += timer.max(1);
        }
        if let Some(v) = read_field_u8(def, payload, 16) {
            if v != 0xFF {
                self.max_hr = self.max_hr.max(v);
            }
        }
        if let Some(v) = read_field_u8(def, payload, 15) {
            if v != 0xFF {
                self.avg_hr_num += v as u64 * timer.max(1);
                self.avg_hr_w += timer.max(1);
            }
        }
        if let Some(v) = read_field_u8(def, payload, 18) {
            if v != 0xFF {
                self.max_cadence = self.max_cadence.max(v);
            }
        }
        if let Some(v) = read_field_u8(def, payload, 17) {
            if v != 0xFF {
                self.avg_cadence_num += v as u64 * timer.max(1);
                self.avg_cadence_w += timer.max(1);
            }
        }
        if let Some(v) = read_field_u16(def, payload, 20) {
            if v != 0xFFFF {
                self.max_power = self.max_power.max(v);
            }
        }
        if let Some(v) = read_field_u16(def, payload, 19) {
            if v != 0xFFFF {
                self.avg_power_num += v as u64 * timer.max(1);
                self.avg_power_w += timer.max(1);
            }
        }
    }

    fn avg_speed(&self) -> u16 {
        if self.avg_speed_w == 0 {
            0xFFFF
        } else {
            (self.avg_speed_num / self.avg_speed_w) as u16
        }
    }
    fn avg_hr(&self) -> u8 {
        if self.avg_hr_w == 0 {
            0xFF
        } else {
            (self.avg_hr_num / self.avg_hr_w) as u8
        }
    }
    fn avg_cadence(&self) -> u8 {
        if self.avg_cadence_w == 0 {
            0xFF
        } else {
            (self.avg_cadence_num / self.avg_cadence_w) as u8
        }
    }
    fn avg_power(&self) -> u16 {
        if self.avg_power_w == 0 {
            0xFFFF
        } else {
            (self.avg_power_num / self.avg_power_w) as u16
        }
    }
}

// ---------- 字段读取工具 ----------

/// 在 payload 中按定义查找指定字段，返回其字节切片。
fn locate_field<'a>(
    def: &DefinitionMessage,
    payload: &'a [u8],
    field_num: u8,
) -> Option<(&'a [u8], Architecture)> {
    let mut off = 0usize;
    for f in &def.fields {
        let sz = f.size as usize;
        if f.field_definition_number == field_num {
            if off + sz > payload.len() {
                return None;
            }
            return Some((&payload[off..off + sz], def.architecture));
        }
        off += sz;
    }
    None
}

fn read_field_u8(def: &DefinitionMessage, payload: &[u8], num: u8) -> Option<u8> {
    locate_field(def, payload, num).and_then(|(s, _)| s.get(0).copied())
}

fn read_field_u16(def: &DefinitionMessage, payload: &[u8], num: u8) -> Option<u16> {
    let (s, arch) = locate_field(def, payload, num)?;
    if s.len() < 2 {
        return None;
    }
    Some(match arch {
        Architecture::LittleEndian => u16::from_le_bytes([s[0], s[1]]),
        Architecture::BigEndian => u16::from_be_bytes([s[0], s[1]]),
    })
}

fn read_field_u32(def: &DefinitionMessage, payload: &[u8], num: u8) -> Option<u32> {
    let (s, arch) = locate_field(def, payload, num)?;
    if s.len() < 4 {
        return None;
    }
    Some(match arch {
        Architecture::LittleEndian => u32::from_le_bytes([s[0], s[1], s[2], s[3]]),
        Architecture::BigEndian => u32::from_be_bytes([s[0], s[1], s[2], s[3]]),
    })
}

fn write_field_u8(def: &DefinitionMessage, payload: &mut [u8], num: u8, val: u8) {
    let mut off = 0usize;
    for f in &def.fields {
        let sz = f.size as usize;
        if f.field_definition_number == num && sz >= 1 {
            payload[off] = val;
            return;
        }
        off += sz;
    }
}

fn write_field_u16(def: &DefinitionMessage, payload: &mut [u8], num: u8, val: u16) {
    let mut off = 0usize;
    for f in &def.fields {
        let sz = f.size as usize;
        if f.field_definition_number == num && sz >= 2 {
            let bytes = match def.architecture {
                Architecture::LittleEndian => val.to_le_bytes(),
                Architecture::BigEndian => val.to_be_bytes(),
            };
            payload[off..off + 2].copy_from_slice(&bytes);
            return;
        }
        off += sz;
    }
}

fn write_field_u32(def: &DefinitionMessage, payload: &mut [u8], num: u8, val: u32) {
    let mut off = 0usize;
    for f in &def.fields {
        let sz = f.size as usize;
        if f.field_definition_number == num && sz >= 4 {
            let bytes = match def.architecture {
                Architecture::LittleEndian => val.to_le_bytes(),
                Architecture::BigEndian => val.to_be_bytes(),
            };
            payload[off..off + 4].copy_from_slice(&bytes);
            return;
        }
        off += sz;
    }
}

// ---------- 重建 file_id / session / activity ----------

fn build_file_id(
    def: Option<&DefinitionMessage>,
    data: Option<&DataMessage>,
    earliest_time_created: Option<u32>,
) -> Result<(DefinitionMessage, DataMessage), String> {
    let mut def = def.cloned().ok_or("缺少 file_id 定义消息")?;
    let mut data = data.cloned().ok_or("缺少 file_id 数据消息")?;
    def.local_message_type = 15; // 用最高 LMT 给重写消息，避免与时间序列冲突
    data.local_message_type = 15;
    if let Some(t) = earliest_time_created {
        write_field_u32(&def, &mut data.payload, 4, t);
    }
    Ok((def, data))
}

fn build_session(
    def: Option<&DefinitionMessage>,
    data: Option<&DataMessage>,
    acc: &SessionAccumulator,
    earliest_start_time: Option<u32>,
) -> Result<(DefinitionMessage, DataMessage), String> {
    let mut def = def.cloned().ok_or("缺少 session 定义消息")?;
    let mut data = data.cloned().ok_or("缺少 session 数据消息")?;
    def.local_message_type = 14;
    data.local_message_type = 14;

    if let Some(st) = earliest_start_time {
        write_field_u32(&def, &mut data.payload, 2, st);
    }
    write_field_u32(&def, &mut data.payload, 7, acc.total_elapsed_time as u32);
    write_field_u32(&def, &mut data.payload, 8, acc.total_timer_time as u32);
    write_field_u32(&def, &mut data.payload, 9, acc.total_distance as u32);
    write_field_u32(&def, &mut data.payload, 10, acc.total_cycles as u32);
    write_field_u16(&def, &mut data.payload, 11, acc.total_calories.min(0xFFFF) as u16);
    write_field_u16(&def, &mut data.payload, 22, acc.total_ascent.min(0xFFFF) as u16);
    write_field_u16(&def, &mut data.payload, 23, acc.total_descent.min(0xFFFF) as u16);
    write_field_u16(&def, &mut data.payload, 13, acc.avg_speed());
    write_field_u16(&def, &mut data.payload, 14, acc.max_speed);
    write_field_u8(&def, &mut data.payload, 15, acc.avg_hr());
    write_field_u8(&def, &mut data.payload, 16, acc.max_hr);
    write_field_u8(&def, &mut data.payload, 17, acc.avg_cadence());
    write_field_u8(&def, &mut data.payload, 18, acc.max_cadence);
    write_field_u16(&def, &mut data.payload, 19, acc.avg_power());
    write_field_u16(&def, &mut data.payload, 20, acc.max_power);

    Ok((def, data))
}

fn build_activity(
    def: Option<&DefinitionMessage>,
    data: Option<&DataMessage>,
    total_timer_time: u32,
) -> Result<(DefinitionMessage, DataMessage), String> {
    let mut def = def.cloned().ok_or("缺少 activity 定义消息")?;
    let mut data = data.cloned().ok_or("缺少 activity 数据消息")?;
    def.local_message_type = 13;
    data.local_message_type = 13;
    write_field_u32(&def, &mut data.payload, 0, total_timer_time);
    write_field_u16(&def, &mut data.payload, 1, 1); // num_sessions = 1
    Ok((def, data))
}

/// 兼容旧 API：提供 FitMerger 类型壳（已废弃，仅保留以避免破坏外部引用）。
pub struct FitMerger {
    files: Vec<FitFile>,
}

impl FitMerger {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }
    pub fn add_file(&mut self, file: FitFile) {
        self.files.push(file);
    }
    pub fn merge(&self) -> Result<FitFile, String> {
        merge(&self.files)
    }
}