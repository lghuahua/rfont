use rfont::Font;
use tracing_subscriber;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 tracing 订阅者
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    println!("=== AlimamaDaoLiTi 字体测试 ===\n");

    // 检查文件格式
    check_file_format("crates/rfont/src/AlimamaDaoLiTi.ttf")?;
    check_file_format("crates/rfont/src/AlimamaDaoLiTi.woff")?;
    
    println!("\n{}\n", "=".repeat(50));

    // 测试 TTF 格式
    test_font_format("crates/rfont/src/AlimamaDaoLiTi.ttf", "TTF")?;
    
    println!("\n{}\n", "=".repeat(50));
    
    // 测试 WOFF 格式
    test_font_format("crates/rfont/src/AlimamaDaoLiTi.woff", "WOFF")?;

    Ok(())
}

fn check_file_format(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(path).exists() {
        println!("❌ 文件不存在: {}", path);
        return Ok(());
    }
    
    let bytes = std::fs::read(path)?;
    println!("📄 文件: {}", path);
    println!("   大小: {} bytes", bytes.len());
    println!("   前4字节: {:02X} {:02X} {:02X} {:02X}", bytes[0], bytes[1], bytes[2], bytes[3]);
    
    if &bytes[0..4] == b"wOFF" {
        println!("   格式: WOFF ✓");
    } else if &bytes[0..4] == b"wOF2" {
        println!("   格式: WOFF2 (暂不支持)");
    } else {
        println!("   格式: TTF/OTF");
    }
    println!();
    
    Ok(())
}

fn test_font_format(path: &str, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(path).exists() {
        println!("❌ {} 文件不存在: {}", format, path);
        return Ok(());
    }

    println!("📖 正在加载 {} 格式字体: {}", format, path);
    let font = Font::load(path)?;
    println!("✅ {} 格式加载成功！", format);
    
    println!("\n📊 字体信息:");
    println!("   - 字形数量: {}", font.maxp.num_glyphs);
    println!("   - Units Per Em: {}", font.head.units_per_em);
    println!("   - index_to_loc_format: {}", font.head.index_to_loc_format);
    println!("   - Cmap 映射数: {}", font.cmap.unicode_map.len());

    // 测试一些常见汉字
    let test_texts = vec![
        "阿里妈妈刀隶体",
        "Hello World",
        "测试123",
    ];

    for text in test_texts {
        println!("\n🔤 测试文字: \"{}\"", text);
        let glyph_ids = font.get_glyph_ids_for_text(text);
        
        let mut valid_count = 0;
        let mut notdef_count = 0;
        
        for (i, ch) in text.chars().enumerate() {
            let gid = glyph_ids[i];
            if gid == 0 {
                notdef_count += 1;
                print!("  '{}' (U+{:04X}) -> .notdef | ", ch, ch as u32);
            } else {
                valid_count += 1;
                print!("  '{}' (U+{:04X}) -> GlyphID: {:3} | ", ch, ch as u32, gid);
            }
        }
        println!();
        println!("   有效字形: {}, 缺失字形: {}", valid_count, notdef_count);
    }

    // 生成子集字体测试
    let subset_text = "阿里妈妈";
    println!("\n🎯 生成子集字体测试: \"{}\"", subset_text);
    let glyph_ids = font.get_glyph_ids_for_text(subset_text);
    let subset_data = font.subset_and_serialize(&glyph_ids)?;
    
    let output_name = format!("subset_AlimamaDaoLiTi_{}.ttf", format.to_lowercase());
    std::fs::write(&output_name, &subset_data)?;
    println!("✅ 子集字体已保存: {} ({} bytes)", output_name, subset_data.len());
    
    // 对比原始文件大小
    let original_size = std::fs::metadata(path)?.len();
    let compression_ratio = (subset_data.len() as f64 / original_size as f64) * 100.0;
    println!("📦 压缩率: {:.2}% (原始: {} bytes, 子集: {} bytes)", 
             compression_ratio, original_size, subset_data.len());

    Ok(())
}
