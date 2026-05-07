use rfont::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 字体格式检测增强测试\n");
    
    // 测试文件列表
    let test_files = vec![
        ("crates/rfont/src/AlimamaDaoLiTi.ttf", "TTF"),
        ("crates/rfont/src/AlimamaDaoLiTi.woff", "WOFF"),
        ("crates/rfont/src/AlimamaDaoLiTi.woff2", "WOFF2"),
    ];
    
    for (file_path, expected_format) in &test_files {
        println!("📄 测试文件: {}", file_path);
        println!("   预期格式: {}", expected_format);
        
        if !std::path::Path::new(file_path).exists() {
            println!("   ❌ 文件不存在\n");
            continue;
        }
        
        // 读取文件数据
        let data = std::fs::read(file_path)?;
        
        // 检测格式
        match Font::detect_format(&data) {
            Ok(format_info) => {
                println!("   ✅ 格式检测成功");
                println!("   格式类型: {}", format_info.format);
                println!("   SFNT 版本: {}", format_info.version);
                println!("   可变字体: {}", if format_info.is_variable { "是" } else { "否" });
                
                if let Some(compression) = format_info.compression {
                    println!("   压缩类型: {}", compression);
                } else {
                    println!("   压缩类型: 无");
                }
                
                println!("   表总数: {}", format_info.total_tables);
                println!("   必需表: {}", format_info.required_tables.join(", "));
                println!("   可选表数量: {}", format_info.optional_tables.len());
                
                // 验证必需表
                let available_tables: Vec<String> = format_info.required_tables
                    .iter()
                    .chain(format_info.optional_tables.iter())
                    .cloned()
                    .collect();
                
                if format_info.has_required_tables(&available_tables) {
                    println!("   ✅ 所有必需表都存在");
                } else {
                    let missing = format_info.missing_required_tables(&available_tables);
                    println!("   ⚠️  缺失必需表: {}", missing.join(", "));
                }
            }
            Err(e) => {
                println!("   ❌ 格式检测失败: {}", e);
            }
        }
        
        println!();
    }
    
    println!("✅ 测试完成");
    
    Ok(())
}
