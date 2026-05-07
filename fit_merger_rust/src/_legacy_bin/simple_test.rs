use fit_merger::{fit_parser, fit_generator, merger};
use std::collections::HashMap;

fn main() {
    println!("创建测试FIT文件...");
    
    // 创建第一个测试文件
    let mut file1 = fit_merger::fit_types::FitFile {
        header: fit_merger::fit_types::FitHeader {
            header_size: 14,
            protocol_version: 20,
            profile_version: 2100,
            data_size: 0,
            data_type: *b".FIT",
            crc: Some(0),
        },
        messages: Vec::new(),
        crc: 0,
    };
    
    // 添加文件ID定义消息
    let file_id_def = fit_merger::fit_types::FitMessage::Definition(fit_merger::fit_types::DefinitionMessage {
        local_message_type: 0,
        global_message_number: 0,
        architecture: fit_merger::fit_types::Architecture::LittleEndian,
        fields: vec![
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 0,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Enum,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 1,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 2,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 3,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32z,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 4,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
        ],
        developer_fields: Vec::new(),
    });
    
    file1.messages.push(file_id_def);
    
    // 添加文件ID数据消息
    let mut file_id_fields = HashMap::new();
    file_id_fields.insert(0u16, fit_merger::fit_types::FieldValue::Enum(4)); // activity
    file_id_fields.insert(1u16, fit_merger::fit_types::FieldValue::Uint16(1)); // garmin
    file_id_fields.insert(2u16, fit_merger::fit_types::FieldValue::Uint16(1234)); // product
    file_id_fields.insert(3u16, fit_merger::fit_types::FieldValue::Uint32z(5678)); // serial
    file_id_fields.insert(4u16, fit_merger::fit_types::FieldValue::Uint32(1000)); // time_created
    
    let file_id_data = fit_merger::fit_types::FitMessage::Data(fit_merger::fit_types::DataMessage {
        local_message_type: 0,
        fields: file_id_fields,
    });
    
    file1.messages.push(file_id_data);
    
    // 添加会话定义消息
    let session_def = fit_merger::fit_types::FitMessage::Definition(fit_merger::fit_types::DefinitionMessage {
        local_message_type: 1,
        global_message_number: 18,
        architecture: fit_merger::fit_types::Architecture::LittleEndian,
        fields: vec![
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 253,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 2,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 5,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Enum,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 7,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 8,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 9,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 15,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Uint8,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 17,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Uint8,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 19,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
        ],
        developer_fields: Vec::new(),
    });
    
    file1.messages.push(session_def);
    
    // 添加会话数据消息
    let mut session_fields = HashMap::new();
    session_fields.insert(253u16, fit_merger::fit_types::FieldValue::Uint32(2000));
    session_fields.insert(2u16, fit_merger::fit_types::FieldValue::Uint32(1000));
    session_fields.insert(5u16, fit_merger::fit_types::FieldValue::Enum(1)); // cycling
    session_fields.insert(7u16, fit_merger::fit_types::FieldValue::Uint32(3600)); // 1 hour
    session_fields.insert(8u16, fit_merger::fit_types::FieldValue::Uint32(3600)); // 1 hour
    session_fields.insert(9u16, fit_merger::fit_types::FieldValue::Uint32(20000)); // 20 km
    session_fields.insert(15u16, fit_merger::fit_types::FieldValue::Uint8(150)); // avg hr
    session_fields.insert(17u16, fit_merger::fit_types::FieldValue::Uint8(85)); // avg cadence
    session_fields.insert(19u16, fit_merger::fit_types::FieldValue::Uint16(200)); // avg power
    
    let session_data = fit_merger::fit_types::FitMessage::Data(fit_merger::fit_types::DataMessage {
        local_message_type: 1,
        fields: session_fields,
    });
    
    file1.messages.push(session_data);
    
    // 添加记录定义消息
    let record_def = fit_merger::fit_types::FitMessage::Definition(fit_merger::fit_types::DefinitionMessage {
        local_message_type: 2,
        global_message_number: 20,
        architecture: fit_merger::fit_types::Architecture::LittleEndian,
        fields: vec![
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 253,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 0,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Sint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 1,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Sint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 3,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Uint8,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 4,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Uint8,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 5,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 6,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 7,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
        ],
        developer_fields: Vec::new(),
    });
    
    file1.messages.push(record_def);
    
    // 添加一些记录数据消息
    for i in 0..5 {
        let mut record_fields = HashMap::new();
        record_fields.insert(253u16, fit_merger::fit_types::FieldValue::Uint32(1000 + i * 60)); // timestamp
        record_fields.insert(0u16, fit_merger::fit_types::FieldValue::Sint32((123456789i32 + i as i32 * 1000))); // lat
        record_fields.insert(1u16, fit_merger::fit_types::FieldValue::Sint32((987654321i32 + i as i32 * 1000))); // long
        record_fields.insert(3u16, fit_merger::fit_types::FieldValue::Uint8((150u8 + i as u8))); // heart rate
        record_fields.insert(4u16, fit_merger::fit_types::FieldValue::Uint8((85u8 + i as u8 % 10))); // cadence
        record_fields.insert(5u16, fit_merger::fit_types::FieldValue::Uint32(i * 2000)); // distance
        record_fields.insert(6u16, fit_merger::fit_types::FieldValue::Uint16((5000u16 + i as u16 * 100))); // speed
        record_fields.insert(7u16, fit_merger::fit_types::FieldValue::Uint16((200u16 + i as u16 * 5))); // power
        
        let record_data = fit_merger::fit_types::FitMessage::Data(fit_merger::fit_types::DataMessage {
            local_message_type: 2,
            fields: record_fields,
        });
        
        file1.messages.push(record_data);
    }
    
    // 写入第一个测试文件
    let fit_files_dir = std::env::current_dir().unwrap().join("../fit_files");
    std::fs::create_dir_all(&fit_files_dir).unwrap();
    
    let file1_path = fit_files_dir.join("test1.fit");
    fit_generator::write_fit_file(&file1, file1_path.to_str().unwrap()).unwrap();
    
    // 创建第二个测试文件
    let mut file2 = fit_merger::fit_types::FitFile {
        header: fit_merger::fit_types::FitHeader {
            header_size: 14,
            protocol_version: 20,
            profile_version: 2100,
            data_size: 0,
            data_type: *b".FIT",
            crc: Some(0),
        },
        messages: Vec::new(),
        crc: 0,
    };
    
    // 与第一个文件相同的结构，但数据不同
    file2.messages.push(fit_merger::fit_types::FitMessage::Definition(fit_merger::fit_types::DefinitionMessage {
        local_message_type: 0,
        global_message_number: 0,
        architecture: fit_merger::fit_types::Architecture::LittleEndian,
        fields: vec![
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 0,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Enum,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 1,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 2,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 3,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32z,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 4,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
        ],
        developer_fields: Vec::new(),
    }));
    
    let mut file_id_fields2 = HashMap::new();
    file_id_fields2.insert(0u16, fit_merger::fit_types::FieldValue::Enum(4)); // activity
    file_id_fields2.insert(1u16, fit_merger::fit_types::FieldValue::Uint16(1)); // garmin
    file_id_fields2.insert(2u16, fit_merger::fit_types::FieldValue::Uint16(1234)); // product
    file_id_fields2.insert(3u16, fit_merger::fit_types::FieldValue::Uint32z(5679)); // different serial
    file_id_fields2.insert(4u16, fit_merger::fit_types::FieldValue::Uint32(2000)); // different time_created
    
    file2.messages.push(fit_merger::fit_types::FitMessage::Data(fit_merger::fit_types::DataMessage {
        local_message_type: 0,
        fields: file_id_fields2,
    }));
    
    file2.messages.push(fit_merger::fit_types::FitMessage::Definition(fit_merger::fit_types::DefinitionMessage {
        local_message_type: 1,
        global_message_number: 18,
        architecture: fit_merger::fit_types::Architecture::LittleEndian,
        fields: vec![
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 253,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 2,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 5,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Enum,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 7,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 8,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 9,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 15,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Uint8,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 17,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Uint8,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 19,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
        ],
        developer_fields: Vec::new(),
    }));
    
    let mut session_fields2 = HashMap::new();
    session_fields2.insert(253u16, fit_merger::fit_types::FieldValue::Uint32(3000));
    session_fields2.insert(2u16, fit_merger::fit_types::FieldValue::Uint32(2000));
    session_fields2.insert(5u16, fit_merger::fit_types::FieldValue::Enum(1)); // cycling
    session_fields2.insert(7u16, fit_merger::fit_types::FieldValue::Uint32(3600)); // 1 hour
    session_fields2.insert(8u16, fit_merger::fit_types::FieldValue::Uint32(3600)); // 1 hour
    session_fields2.insert(9u16, fit_merger::fit_types::FieldValue::Uint32(25000)); // 25 km
    session_fields2.insert(15u16, fit_merger::fit_types::FieldValue::Uint8(155)); // avg hr
    session_fields2.insert(17u16, fit_merger::fit_types::FieldValue::Uint8(88)); // avg cadence
    session_fields2.insert(19u16, fit_merger::fit_types::FieldValue::Uint16(220)); // avg power
    
    file2.messages.push(fit_merger::fit_types::FitMessage::Data(fit_merger::fit_types::DataMessage {
        local_message_type: 1,
        fields: session_fields2,
    }));
    
    file2.messages.push(fit_merger::fit_types::FitMessage::Definition(fit_merger::fit_types::DefinitionMessage {
        local_message_type: 2,
        global_message_number: 20,
        architecture: fit_merger::fit_types::Architecture::LittleEndian,
        fields: vec![
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 253,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 0,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Sint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 1,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Sint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 3,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Uint8,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 4,
                size: 1,
                base_type: fit_merger::fit_types::BaseType::Uint8,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 5,
                size: 4,
                base_type: fit_merger::fit_types::BaseType::Uint32,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 6,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
            fit_merger::fit_types::FieldDefinition {
                field_definition_number: 7,
                size: 2,
                base_type: fit_merger::fit_types::BaseType::Uint16,
            },
        ],
        developer_fields: Vec::new(),
    }));
    
    // 添加一些记录数据消息
    for i in 0..5 {
        let mut record_fields = HashMap::new();
        record_fields.insert(253u16, fit_merger::fit_types::FieldValue::Uint32(2000 + i * 60)); // timestamp
        record_fields.insert(0u16, fit_merger::fit_types::FieldValue::Sint32((223456789i32 + i as i32 * 1000))); // lat
        record_fields.insert(1u16, fit_merger::fit_types::FieldValue::Sint32((887654321i32 + i as i32 * 1000))); // long
        record_fields.insert(3u16, fit_merger::fit_types::FieldValue::Uint8((155u8 + i as u8))); // heart rate
        record_fields.insert(4u16, fit_merger::fit_types::FieldValue::Uint8((88u8 + i as u8 % 10))); // cadence
        record_fields.insert(5u16, fit_merger::fit_types::FieldValue::Uint32(i * 2500)); // distance
        record_fields.insert(6u16, fit_merger::fit_types::FieldValue::Uint16((5500u16 + i as u16 * 100))); // speed
        record_fields.insert(7u16, fit_merger::fit_types::FieldValue::Uint16((220u16 + i as u16 * 5))); // power
        
        let record_data = fit_merger::fit_types::FitMessage::Data(fit_merger::fit_types::DataMessage {
            local_message_type: 2,
            fields: record_fields,
        });
        
        file2.messages.push(record_data);
    }
    
    // 写入第二个测试文件
    let file2_path = fit_files_dir.join("test2.fit");
    fit_generator::write_fit_file(&file2, file2_path.to_str().unwrap()).unwrap();
    
    println!("测试文件已创建: {:?} 和 {:?}", file1_path, file2_path);
    
    // 现在运行合并测试
    let output_file = fit_files_dir.join("merged.fit").to_str().unwrap().to_string();
    
    println!("正在合并测试文件...");
    
    match merger::merge_fit_files(
        &[file1_path.to_str().unwrap(), file2_path.to_str().unwrap()], 
        &output_file
    ) {
        Ok(_) => {
            println!("合并成功! 输出文件: {}", output_file);
            
            // 验证合并后的文件
            match fit_parser::read_fit_file(&output_file) {
                Ok(merged_file) => {
                    println!("合并文件验证成功!");
                    println!("  消息数量: {}", merged_file.messages.len());
                    
                    // 统计不同类型的消息
                    let mut definition_count = 0;
                    let mut data_count = 0;
                    
                    for message in &merged_file.messages {
                        match message {
                            fit_merger::fit_types::FitMessage::Definition(_) => definition_count += 1,
                            fit_merger::fit_types::FitMessage::Data(_) => data_count += 1,
                        }
                    }
                    
                    println!("  定义消息: {}", definition_count);
                    println!("  数据消息: {}", data_count);
                }
                Err(e) => {
                    println!("验证合并文件失败: {}", e);
                }
            }
        }
        Err(e) => {
            println!("合并失败: {}", e);
        }
    }
}