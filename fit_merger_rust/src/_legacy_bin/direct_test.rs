use fit_merger::{fit_types, merger, fit_generator};

fn main() {
    println!("直接测试合并功能...");
    
    // 创建两个简单的FIT文件
    let mut file1 = fit_types::FitFile {
        header: fit_types::FitHeader {
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
    file1.messages.push(fit_types::FitMessage::Definition(fit_types::DefinitionMessage {
        local_message_type: 0,
        global_message_number: 0,
        architecture: fit_types::Architecture::LittleEndian,
        fields: vec![
            fit_types::FieldDefinition {
                field_definition_number: 0,
                size: 1,
                base_type: fit_types::BaseType::Enum,
            },
            fit_types::FieldDefinition {
                field_definition_number: 4,
                size: 4,
                base_type: fit_types::BaseType::Uint32,
            },
        ],
        developer_fields: Vec::new(),
    }));
    
    // 添加文件ID数据消息
    let mut file_id_fields = std::collections::HashMap::new();
    file_id_fields.insert(0u16, fit_types::FieldValue::Enum(4)); // activity
    file_id_fields.insert(4u16, fit_types::FieldValue::Uint32(1000)); // time_created
    
    file1.messages.push(fit_types::FitMessage::Data(fit_types::DataMessage {
        local_message_type: 0,
        fields: file_id_fields,
    }));
    
    // 创建第二个文件
    let mut file2 = fit_types::FitFile {
        header: fit_types::FitHeader {
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
    file2.messages.push(fit_types::FitMessage::Definition(fit_types::DefinitionMessage {
        local_message_type: 0,
        global_message_number: 0,
        architecture: fit_types::Architecture::LittleEndian,
        fields: vec![
            fit_types::FieldDefinition {
                field_definition_number: 0,
                size: 1,
                base_type: fit_types::BaseType::Enum,
            },
            fit_types::FieldDefinition {
                field_definition_number: 4,
                size: 4,
                base_type: fit_types::BaseType::Uint32,
            },
        ],
        developer_fields: Vec::new(),
    }));
    
    // 添加文件ID数据消息
    let mut file_id_fields2 = std::collections::HashMap::new();
    file_id_fields2.insert(0u16, fit_types::FieldValue::Enum(4)); // activity
    file_id_fields2.insert(4u16, fit_types::FieldValue::Uint32(2000)); // time_created
    
    file2.messages.push(fit_types::FitMessage::Data(fit_types::DataMessage {
        local_message_type: 0,
        fields: file_id_fields2,
    }));
    
    println!("文件1消息数量: {}", file1.messages.len());
    println!("文件2消息数量: {}", file2.messages.len());
    
    // 直接测试合并器
    let mut merger = merger::FitMerger::new();
    merger.add_file(file1.clone());
    merger.add_file(file2.clone());
    
    match merger.merge() {
        Ok(merged_file) => {
            println!("合并成功!");
            println!("合并后消息数量: {}", merged_file.messages.len());
            
            // 保存合并后的文件
            let fit_files_dir = std::env::current_dir().unwrap().join("../fit_files");
            std::fs::create_dir_all(&fit_files_dir).unwrap();
            let output_path = fit_files_dir.join("direct_merged.fit").to_str().unwrap().to_string();
            
            match fit_generator::write_fit_file(&merged_file, &output_path) {
                Ok(_) => println!("合并文件已保存到: {}", output_path),
                Err(e) => println!("保存合并文件失败: {}", e),
            }
        }
        Err(e) => {
            println!("合并失败: {}", e);
        }
    }
}