use fit_merger::FitGenerator;
use fit_merger::fit_types::{FitFile, FitHeader, FitMessage, DataMessage, DefinitionMessage, FieldDefinition, FieldValue, BaseType, Architecture};
use std::collections::HashMap;
use std::fs;

fn main() -> Result<(), String> {
    println!("创建测试FIT文件");

    // 创建第一个测试文件
    let file1 = create_simple_fit_file(1)?;
    let mut generator = FitGenerator::new();
    let data1 = generator.generate(&file1)?;
    fs::write("../fit_files/test1_new.fit", data1).map_err(|e| format!("写入文件失败: {}", e))?;

    // 创建第二个测试文件
    let file2 = create_simple_fit_file(2)?;
    let data2 = generator.generate(&file2)?;
    fs::write("../fit_files/test2_new.fit", data2).map_err(|e| format!("写入文件失败: {}", e))?;

    println!("测试文件创建完成！");
    Ok(())
}

fn create_simple_fit_file(id: u8) -> Result<FitFile, String> {
    let mut messages = Vec::new();

    // 添加文件ID定义消息
    let file_id_def = DefinitionMessage {
        local_message_type: 0,
        global_message_number: 0, // file_id消息
        architecture: Architecture::LittleEndian,
        fields: vec![
            FieldDefinition {
                field_definition_number: 0, // type
                size: 1,
                base_type: BaseType::Enum,
            },
            FieldDefinition {
                field_definition_number: 1, // manufacturer
                size: 2,
                base_type: BaseType::Uint16,
            },
        ],
        developer_fields: Vec::new(),
    };
    messages.push(FitMessage::Definition(file_id_def));

    // 添加文件ID数据消息
    let mut file_id_data = DataMessage {
        local_message_type: 0,
        fields: HashMap::new(),
    };
    file_id_data.fields.insert(0, FieldValue::Enum(4)); // activity
    file_id_data.fields.insert(1, FieldValue::Uint16(1)); // manufacturer
    messages.push(FitMessage::Data(file_id_data));

    // 添加记录定义消息
    let record_def = DefinitionMessage {
        local_message_type: 1,
        global_message_number: 20, // record消息
        architecture: Architecture::LittleEndian,
        fields: vec![
            FieldDefinition {
                field_definition_number: 253, // timestamp
                size: 4,
                base_type: BaseType::Uint32,
            },
            FieldDefinition {
                field_definition_number: 0, // position_lat
                size: 4,
                base_type: BaseType::Sint32,
            },
            FieldDefinition {
                field_definition_number: 1, // position_long
                size: 4,
                base_type: BaseType::Sint32,
            },
        ],
        developer_fields: Vec::new(),
    };
    messages.push(FitMessage::Definition(record_def));

    // 添加一些记录数据消息
    for i in 0..5 {
        let mut record_data = DataMessage {
            local_message_type: 1,
            fields: HashMap::new(),
        };
        record_data.fields.insert(253, FieldValue::Uint32(1000 + i as u32 * 10));
        record_data.fields.insert(0, FieldValue::Sint32(12345678 + i as i32 * 1000));
        record_data.fields.insert(1, FieldValue::Sint32(87654321 + i as i32 * 1000));
        messages.push(FitMessage::Data(record_data));
    }

    let header = FitHeader {
        header_size: 14,
        protocol_version: 20,
        profile_version: 2100,
        data_size: 0, // 将在生成时计算
        data_type: *b".FIT",
        crc: Some(0),
    };

    Ok(FitFile {
        header,
        messages,
        crc: 0,
    })
}