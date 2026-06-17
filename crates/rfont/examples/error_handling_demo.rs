use rfont::Font;
use rfont_types::FontError;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 示例 1: 加载字体文件（自动处理 IO 错误）
    println!("📖 示例 1: 加载字体文件");
    match Font::load("AlimamaDaoLiTi.ttf") {
        Ok(font) => {
            println!("✅ 字体加载成功！");
            println!("   字形数量: {}", font.maxp().num_glyphs);
        }
        Err(FontError::Io(e)) => {
            eprintln!("❌ 文件读取失败: {}", e);
            let font_error = FontError::Io(std::io::Error::new(e.kind(), e.to_string()));
            if let Some(suggestion) = font_error.suggestion() {
                eprintln!("💡 建议: {}", suggestion);
            }
        }
        Err(FontError::InvalidMagicNumber { expected, actual }) => {
            eprintln!("❌ 无效的字体格式");
            eprintln!("   期望魔数: {:#010X}", expected);
            eprintln!("   实际魔数: {:#010X}", actual);
            let font_error = FontError::InvalidMagicNumber { expected, actual };
            if let Some(suggestion) = font_error.suggestion() {
                eprintln!("💡 建议: {}", suggestion);
            }
        }
        Err(e) => {
            eprintln!("❌ 未知错误: {}", e);
            if let Some(suggestion) = e.suggestion() {
                eprintln!("💡 建议: {}", suggestion);
            }
        }
    }

    println!("\n📖 示例 2: 结构化错误信息");
    // 示例 2: 演示 ParseError
    let parse_error = FontError::ParseError {
        table: "glyf".to_string(),
        offset: 0x1234,
        reason: "unexpected end of data".to_string(),
    };
    println!("   {}", parse_error);
    if let Some(suggestion) = parse_error.suggestion() {
        println!("   💡 建议: {}", suggestion);
    }

    println!("\n📖 示例 3: 错误链支持");
    // 示例 3: 演示 IO 错误链
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let font_error = FontError::from(io_error);
    println!("   {}", font_error);
    if let Some(suggestion) = font_error.suggestion() {
        println!("   💡 建议: {}", suggestion);
    }

    println!("\n🎉 结构化错误处理演示完成！");
    println!("\n✨ 优势:");
    println!("   • 类型安全的错误匹配（模式匹配）");
    println!("   • 详细的错误上下文（文件名、偏移量等）");
    println!("   • 人类可读的错误消息");
    println!("   • 支持标准 Error trait（可与其他库集成）");
    println!("   • 错误恢复建议（suggestion() 方法）");
    println!("   • 错误链支持（source() 方法）");

    Ok(())
}
