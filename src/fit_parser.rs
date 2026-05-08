//! FIT 文件字节级解析器。
//!
//! 仅解析文件头、记录头与定义消息；数据消息仅按定义计算长度并以原始字节保存。
//! 这样可在保留 100% 原始字段的前提下完成跨文件合并。

use crate::fit_types::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

pub struct FitParser<'a> {
    data: &'a [u8],
    pos: usize,
    /// 当前 local_message_type -> 定义消息
    defs: HashMap<u8, DefinitionMessage>,
}

impl<'a> FitParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            defs: HashMap::new(),
        }
    }

    pub fn parse(&mut self) -> Result<FitFile, String> {
        let header = self.parse_header()?;
        let body_end = self.pos + header.data_size as usize;
        if body_end > self.data.len() {
            return Err(format!(
                "文件数据段越界：声明 data_size={}, 文件大小={}",
                header.data_size,
                self.data.len()
            ));
        }

        let mut messages = Vec::new();
        while self.pos < body_end {
            let msg = self.parse_record()?;
            messages.push(msg);
        }

        // 末尾 2 字节 CRC
        let file_crc = if self.data.len() >= self.pos + 2 {
            u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]])
        } else {
            0
        };

        Ok(FitFile {
            header,
            messages,
            crc: file_crc,
        })
    }

    fn parse_header(&mut self) -> Result<FitHeader, String> {
        if self.data.len() < 12 {
            return Err("文件过短，无法读取 FIT 头".to_string());
        }
        let header_size = self.data[0];
        if header_size != 12 && header_size != 14 {
            return Err(format!("不支持的 FIT 头大小: {}", header_size));
        }
        let protocol_version = self.data[1];
        let profile_version = u16::from_le_bytes([self.data[2], self.data[3]]);
        let data_size = u32::from_le_bytes([self.data[4], self.data[5], self.data[6], self.data[7]]);
        let mut data_type = [0u8; 4];
        data_type.copy_from_slice(&self.data[8..12]);
        if &data_type != b".FIT" {
            return Err("文件类型签名不是 .FIT".to_string());
        }
        let crc = if header_size == 14 {
            Some(u16::from_le_bytes([self.data[12], self.data[13]]))
        } else {
            None
        };
        self.pos = header_size as usize;
        Ok(FitHeader {
            header_size,
            protocol_version,
            profile_version,
            data_size,
            data_type,
            crc,
        })
    }

    fn parse_record(&mut self) -> Result<FitMessage, String> {
        let header_byte = self.read_u8()?;

        // bit7=1: 压缩时间戳 数据消息
        if header_byte & 0x80 != 0 {
            let lmt = (header_byte >> 5) & 0x03;
            let time_offset = header_byte & 0x1F;
            return self.parse_data_message(lmt, Some(time_offset));
        }

        // 普通头：bit6=1 -> definition
        let is_def = header_byte & 0x40 != 0;
        let has_dev_data = header_byte & 0x20 != 0;
        let lmt = header_byte & 0x0F;
        if is_def {
            self.parse_definition_message(lmt, has_dev_data)
        } else {
            self.parse_data_message(lmt, None)
        }
    }

    fn parse_definition_message(
        &mut self,
        lmt: u8,
        has_dev_data: bool,
    ) -> Result<FitMessage, String> {
        let _reserved = self.read_u8()?;
        let arch_byte = self.read_u8()?;
        let arch = if arch_byte == 0 {
            Architecture::LittleEndian
        } else {
            Architecture::BigEndian
        };
        let global_msg_num = match arch {
            Architecture::LittleEndian => self.read_u16_le()?,
            Architecture::BigEndian => self.read_u16_be()?,
        };
        let n_fields = self.read_u8()?;
        let mut fields = Vec::with_capacity(n_fields as usize);
        for _ in 0..n_fields {
            let num = self.read_u8()?;
            let size = self.read_u8()?;
            let bt = self.read_u8()?;
            fields.push(FieldDefinition {
                field_definition_number: num,
                size,
                base_type_raw: bt,
            });
        }
        let mut developer_fields = Vec::new();
        if has_dev_data {
            let n_dev = self.read_u8()?;
            for _ in 0..n_dev {
                let num = self.read_u8()?;
                let size = self.read_u8()?;
                let idx = self.read_u8()?;
                developer_fields.push(DeveloperFieldDefinition {
                    field_number: num,
                    size,
                    developer_data_index: idx,
                });
            }
        }

        let def = DefinitionMessage {
            local_message_type: lmt,
            global_message_number: global_msg_num,
            architecture: arch,
            fields,
            developer_fields,
        };
        self.defs.insert(lmt, def.clone());
        Ok(FitMessage::Definition(def))
    }

    fn parse_data_message(
        &mut self,
        lmt: u8,
        compressed_offset: Option<u8>,
    ) -> Result<FitMessage, String> {
        let def = self
            .defs
            .get(&lmt)
            .ok_or_else(|| format!("数据消息引用未知 local_message_type={}", lmt))?
            .clone();
        let payload_size = def.data_payload_size();
        if self.pos + payload_size > self.data.len() {
            return Err(format!(
                "数据消息越界：lmt={}, 需要 {} 字节, 剩余 {}",
                lmt,
                payload_size,
                self.data.len() - self.pos
            ));
        }
        let payload = self.data[self.pos..self.pos + payload_size].to_vec();
        self.pos += payload_size;
        Ok(FitMessage::Data(DataMessage {
            local_message_type: lmt,
            global_message_number: def.global_message_number,
            compressed_timestamp_offset: compressed_offset,
            payload,
        }))
    }

    // ==== 基本读取 ====
    fn read_u8(&mut self) -> Result<u8, String> {
        if self.pos >= self.data.len() {
            return Err("读取越界 (u8)".to_string());
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }
    fn read_u16_le(&mut self) -> Result<u16, String> {
        if self.pos + 2 > self.data.len() {
            return Err("读取越界 (u16)".to_string());
        }
        let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }
    fn read_u16_be(&mut self) -> Result<u16, String> {
        if self.pos + 2 > self.data.len() {
            return Err("读取越界 (u16)".to_string());
        }
        let v = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }
}

/// 从磁盘读取 FIT 文件并解析。
pub fn read_fit_file(path: &str) -> Result<FitFile, String> {
    let mut f = File::open(path).map_err(|e| format!("打开 {} 失败: {}", path, e))?;
    let mut data = Vec::new();
    f.read_to_end(&mut data)
        .map_err(|e| format!("读取 {} 失败: {}", path, e))?;
    let mut p = FitParser::new(&data);
    p.parse()
}