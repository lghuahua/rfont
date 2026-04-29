use rfont::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔨 Builder 模式 API 演示\n");
    
    // 加载字体
    let font = Font::load("crates/rfont/src/AlimamaDaoLiTi.ttf")?;
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 示例 1: 使用文本进行子集化");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let subset_data = font.subset_builder()
        .text("阿里妈妈刀隶体")
        .optimize_post(true)
        .strip_glyph_names(true)
        .build()?;
    
    println!("✅ 子集化完成！");
    println!("   原始大小: {} bytes", 
             std::fs::metadata("crates/rfont/src/AlimamaDaoLiTi.ttf")?.len());
    println!("   子集大小: {} bytes", subset_data.len());
    println!("   压缩率: {:.2}%", 
             (subset_data.len() as f64 / std::fs::metadata("crates/rfont/src/AlimamaDaoLiTi.ttf")?.len() as f64) * 100.0);
    println!();
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 示例 2: 使用 Unicode 范围");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let subset_data = font.subset_builder()
        .unicode_range(0x4E00, 0x4E10)  // CJK 基本汉字
        .unicode_range(0x0041, 0x005A)  // 大写字母 A-Z
        .output_format("ttf")
        .build()?;
    
    println!("✅ 子集化完成！");
    println!("   子集大小: {} bytes", subset_data.len());
    println!();
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 示例 3: 使用预设配置");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Web 优化预设
    let web_subset = font.subset_builder()
        .text("Hello World 你好世界")
        .preset("web")  // Web 优化：WOFF 格式，最大压缩
        .build()?;
    
    println!("✅ Web 优化子集完成！");
    println!("   子集大小: {} bytes", web_subset.len());
    println!();
    
    // 打印优化预设
    let print_subset = font.subset_builder()
        .text("测试文字")
        .preset("print")  // 打印优化：保留元数据
        .build()?;
    
    println!("✅ 打印优化子集完成！");
    println!("   子集大小: {} bytes", print_subset.len());
    println!();
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 示例 4: 链式调用完整示例");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let custom_subset = font.subset_builder()
        .text("自定义文本")
        .unicode_range(0x0030, 0x0039)  // 数字 0-9
        .optimize_post(true)
        .strip_glyph_names(true)
        .compression_level(9)
        .keep_hinting(false)
        .output_format("ttf")
        .build()?;
    
    println!("✅ 自定义子集完成！");
    println!("   子集大小: {} bytes", custom_subset.len());
    println!();
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 示例 5: 直接使用字形 ID");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let glyph_ids = vec![0, 1, 2, 100, 200];  // 包含 .notdef (ID 0)
    let subset_data = font.subset_builder()
        .glyph_ids(glyph_ids.clone())
        .build()?;
    
    println!("✅ 字形 ID 子集完成！");
    println!("   字形 ID: {:?}", glyph_ids);
    println!("   子集大小: {} bytes", subset_data.len());
    println!();
    
    println!("🎉 Builder 模式 API 演示完成！");
    println!("\n✨ 优势:");
    println!("   • 链式调用，代码简洁易读");
    println!("   • 灵活的配置选项");
    println!("   • 预设模板快速应用");
    println!("   • 支持多种输入方式（文本、字形 ID、Unicode 范围）");
    
    Ok(())
}
