//! FIT 文件协议的字节级数据结构。
//!
//! 为了在合并时完整保留所有字段（心率、踏频、功率、海拔、温度、GPS……），
//! 我们**不解析数据消息的字段值**，仅解析结构：文件头、记录头、定义消息，
//! 然后以原始字节块的形式保存数据消息负载。这样既不会丢失任何字段，
//! 也避免了缩放/单位转换导致的数据失真。

/// FIT 文件头
#[derive(Debug, Clone)]
pub struct FitHeader {
    pub header_size: u8,
    pub protocol_version: u8,
    pub profile_version: u16,
    pub data_size: u32,
    pub data_type: [u8; 4],
    pub crc: Option<u16>,
}

impl Default for FitHeader {
    fn default() -> Self {
        Self {
            header_size: 14,
            protocol_version: 0x20,
            profile_version: 2140,
            data_size: 0,
            data_type: *b".FIT",
            crc: Some(0),
        }
    }
}

/// 基础数据类型（FIT 协议 base type）
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BaseType {
    Enum,
    Sint8,
    Uint8,
    Sint16,
    Uint16,
    Sint32,
    Uint32,
    String,
    Float32,
    Float64,
    Uint8z,
    Uint16z,
    Uint32z,
    Byte,
    Sint64,
    Uint64,
    Uint64z,
    Unknown(u8),
}

impl BaseType {
    /// 按 FIT 规范编码（含 endian bit 0x80 保留为 0）
    pub fn to_u8(self) -> u8 {
        match self {
            BaseType::Enum => 0x00,
            BaseType::Sint8 => 0x01,
            BaseType::Uint8 => 0x02,
            BaseType::Sint16 => 0x83,
            BaseType::Uint16 => 0x84,
            BaseType::Sint32 => 0x85,
            BaseType::Uint32 => 0x86,
            BaseType::String => 0x07,
            BaseType::Float32 => 0x88,
            BaseType::Float64 => 0x89,
            BaseType::Uint8z => 0x0A,
            BaseType::Uint16z => 0x8B,
            BaseType::Uint32z => 0x8C,
            BaseType::Byte => 0x0D,
            BaseType::Sint64 => 0x8E,
            BaseType::Uint64 => 0x8F,
            BaseType::Uint64z => 0x90,
            BaseType::Unknown(v) => v,
        }
    }

    pub fn from_u8(raw: u8) -> Self {
        match raw & 0x1F {
            0 => BaseType::Enum,
            1 => BaseType::Sint8,
            2 => BaseType::Uint8,
            3 => BaseType::Sint16,
            4 => BaseType::Uint16,
            5 => BaseType::Sint32,
            6 => BaseType::Uint32,
            7 => BaseType::String,
            8 => BaseType::Float32,
            9 => BaseType::Float64,
            10 => BaseType::Uint8z,
            11 => BaseType::Uint16z,
            12 => BaseType::Uint32z,
            13 => BaseType::Byte,
            14 => BaseType::Sint64,
            15 => BaseType::Uint64,
            16 => BaseType::Uint64z,
            _ => BaseType::Unknown(raw),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Architecture {
    LittleEndian,
    BigEndian,
}

#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub field_definition_number: u8,
    pub size: u8,
    pub base_type_raw: u8,
}

#[derive(Debug, Clone)]
pub struct DeveloperFieldDefinition {
    pub field_number: u8,
    pub size: u8,
    pub developer_data_index: u8,
}

/// 定义消息（含字段列表）
#[derive(Debug, Clone)]
pub struct DefinitionMessage {
    pub local_message_type: u8,
    pub global_message_number: u16,
    pub architecture: Architecture,
    pub fields: Vec<FieldDefinition>,
    pub developer_fields: Vec<DeveloperFieldDefinition>,
}

impl DefinitionMessage {
    /// 数据消息的 payload 字节数 = sum(field.size)
    pub fn data_payload_size(&self) -> usize {
        let f: usize = self.fields.iter().map(|f| f.size as usize).sum();
        let d: usize = self.developer_fields.iter().map(|f| f.size as usize).sum();
        f + d
    }
}

/// 数据消息：以原始字节形式保存（不做字段解码）。
#[derive(Debug, Clone)]
pub struct DataMessage {
    pub local_message_type: u8,
    /// 全局消息号（来自其定义消息，便于合并阶段分类）
    pub global_message_number: u16,
    /// 是否为压缩时间戳消息（目前 6 个真实骑行文件未使用，但保留支持）
    pub compressed_timestamp_offset: Option<u8>,
    /// 数据字段的原始字节（不含 record header 本身）
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum FitMessage {
    Definition(DefinitionMessage),
    Data(DataMessage),
}

#[derive(Debug, Clone)]
pub struct FitFile {
    pub header: FitHeader,
    pub messages: Vec<FitMessage>,
    /// 文件末尾 CRC（解析时记录，生成时会重新计算）
    pub crc: u16,
}