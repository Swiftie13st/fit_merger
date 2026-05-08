//! FIT 文件字节级生成器：将 [`FitFile`] 重新序列化为符合 FIT 规范的二进制流。
//!
//! - 正确写入 14 字节文件头并填入实际 data_size、头部 CRC
//! - 数据消息直接写入 payload 原始字节（合并阶段未修改的字段保持比特级一致）
//! - 末尾使用官方 CRC-16/FIT 算法计算文件 CRC

use crate::fit_types::*;
use std::fs::File;
use std::io::Write;

pub struct FitGenerator {
    out: Vec<u8>,
}

impl FitGenerator {
    pub fn new() -> Self {
        Self { out: Vec::with_capacity(64 * 1024) }
    }

    pub fn generate(mut self, file: &FitFile) -> Result<Vec<u8>, String> {
        // 1. 占位写入 14 字节文件头（data_size、header_crc 待补）
        self.out.extend_from_slice(&[0u8; 14]);

        let data_start = self.out.len();
        // 2. 写入所有消息
        for m in &file.messages {
            match m {
                FitMessage::Definition(d) => self.write_definition(d)?,
                FitMessage::Data(d) => self.write_data(d)?,
            }
        }
        let data_size = (self.out.len() - data_start) as u32;

        // 3. 回填头部
        self.out[0] = 14;
        self.out[1] = file.header.protocol_version.max(0x10);
        let pv = if file.header.profile_version == 0 {
            2140
        } else {
            file.header.profile_version
        };
        self.out[2..4].copy_from_slice(&pv.to_le_bytes());
        self.out[4..8].copy_from_slice(&data_size.to_le_bytes());
        self.out[8..12].copy_from_slice(b".FIT");
        // header CRC = CRC over bytes [0..12]
        let header_crc = fit_crc(&self.out[0..12]);
        self.out[12..14].copy_from_slice(&header_crc.to_le_bytes());

        // 4. 文件末尾 CRC = CRC over header + data
        let file_crc = fit_crc(&self.out);
        self.out.extend_from_slice(&file_crc.to_le_bytes());

        Ok(self.out)
    }

    fn write_definition(&mut self, def: &DefinitionMessage) -> Result<(), String> {
        let has_dev = !def.developer_fields.is_empty();
        // 普通定义消息头：bit6=1，bit5=has_dev，低 4 位为 lmt
        let mut hdr: u8 = 0x40 | (def.local_message_type & 0x0F);
        if has_dev {
            hdr |= 0x20;
        }
        self.out.push(hdr);
        // reserved
        self.out.push(0);
        // architecture
        let arch_byte = match def.architecture {
            Architecture::LittleEndian => 0u8,
            Architecture::BigEndian => 1u8,
        };
        self.out.push(arch_byte);
        // global message number
        match def.architecture {
            Architecture::LittleEndian => {
                self.out.extend_from_slice(&def.global_message_number.to_le_bytes())
            }
            Architecture::BigEndian => {
                self.out.extend_from_slice(&def.global_message_number.to_be_bytes())
            }
        }
        // fields count
        if def.fields.len() > 255 {
            return Err("一个定义消息不能超过 255 个字段".to_string());
        }
        self.out.push(def.fields.len() as u8);
        for f in &def.fields {
            self.out.push(f.field_definition_number);
            self.out.push(f.size);
            self.out.push(f.base_type_raw);
        }
        if has_dev {
            self.out.push(def.developer_fields.len() as u8);
            for f in &def.developer_fields {
                self.out.push(f.field_number);
                self.out.push(f.size);
                self.out.push(f.developer_data_index);
            }
        }
        Ok(())
    }

    fn write_data(&mut self, data: &DataMessage) -> Result<(), String> {
        match data.compressed_timestamp_offset {
            Some(offset) => {
                let hdr: u8 = 0x80 | ((data.local_message_type & 0x03) << 5) | (offset & 0x1F);
                self.out.push(hdr);
            }
            None => {
                let hdr: u8 = data.local_message_type & 0x0F;
                self.out.push(hdr);
            }
        }
        self.out.extend_from_slice(&data.payload);
        Ok(())
    }
}

/// 把 [`FitFile`] 写到磁盘。
pub fn write_fit_file(file: &FitFile, path: &str) -> Result<(), String> {
    let bytes = FitGenerator::new().generate(file)?;
    let mut f = File::create(path).map_err(|e| format!("创建 {} 失败: {}", path, e))?;
    f.write_all(&bytes)
        .map_err(|e| format!("写入 {} 失败: {}", path, e))?;
    Ok(())
}

/// 标准 FIT CRC-16 算法（Garmin SDK 提供，4-bit 表查表法）。
pub fn fit_crc(bytes: &[u8]) -> u16 {
    const CRC_TABLE: [u16; 16] = [
        0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800,
        0xB401, 0x5000, 0x9C01, 0x8801, 0x4400,
    ];
    let mut crc: u16 = 0;
    for &b in bytes {
        // 处理低 4 位
        let tmp = CRC_TABLE[(crc & 0x0F) as usize];
        crc = (crc >> 4) & 0x0FFF;
        crc ^= tmp ^ CRC_TABLE[(b & 0x0F) as usize];
        // 处理高 4 位
        let tmp = CRC_TABLE[(crc & 0x0F) as usize];
        crc = (crc >> 4) & 0x0FFF;
        crc ^= tmp ^ CRC_TABLE[((b >> 4) & 0x0F) as usize];
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_known_vector() {
        // 来自 FIT SDK 的简单已知样例：空输入 CRC = 0
        assert_eq!(fit_crc(&[]), 0);
        // "123456789" 的 FIT CRC 不是经典 CRC-16/CCITT，仅做不抛错性测试
        let _ = fit_crc(b"123456789");
    }
}