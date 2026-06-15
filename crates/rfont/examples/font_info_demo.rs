use rfont::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📖 字体元数据 API 演示\n");

    // 加载字体
    let font = Font::load("crates/rfont/src/AlimamaDaoLiTi.ttf")?;

    // 获取字体信息
    let info = font.get_font_info();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📄 字体基本信息");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("字形总数:       {}", info.glyph_count);
    println!("每 EM 单位数:   {}", info.units_per_em);
    println!(
        "边界框:         ({}, {}) - ({}, {})",
        info.x_min, info.y_min, info.x_max, info.y_max
    );
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📏 水平度量信息");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("度量数量:       {}", info.number_of_h_metrics);
    println!("上升高度:       {}", info.ascender);
    println!("下降高度:       {}", info.descender);
    println!("行间距:         {}", info.line_gap);
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 字符支持统计");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("支持的字符数:   {}", info.supported_char_count);
    println!();

    // 检查特定字符支持
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔍 字符支持检测");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let test_chars = vec!['阿', '里', '妈', '妈', 'A', 'B', '1', '2'];
    for ch in test_chars {
        let supported = font.supports_character(ch);
        let status = if supported { "✅" } else { "❌" };
        println!("{} '{}' (U+{:04X})", status, ch, ch as u32);
    }
    println!();

    // 文本转字形 ID
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔤 文本到字形映射");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let test_text = "阿里";
    let glyph_ids = font.text_to_glyph_ids(test_text);
    println!("文本: \"{}\"", test_text);
    println!("字形 ID: {:?}", glyph_ids);
    println!();

    // 表列表
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 字体表列表");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let tables = font.get_table_list();
    for table in &tables {
        println!(
            "{:<6} {:>8} bytes  (offset: {}, checksum: {:#010X})",
            table.tag, table.length, table.offset, table.checksum
        );
    }
    println!();

    println!("🎉 字体元数据 API 演示完成！");

    Ok(())
}
