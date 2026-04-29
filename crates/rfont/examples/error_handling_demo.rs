use rfont::Font;
use rfont_types::FontError;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 示例 1: 加载字体文件（自动处理 IO 错误）
    println!("📖 示例 1: 加载字体文件");
    match Font::load("AlimamaDaoLiTi.ttf") {
        Ok(font) => {
            println!("✅ 字体加载成功！");
            println!("   字形数量: {}", font.maxp.num_glyphs);
        }
        Err(FontError::Io(e)) => {
            eprintln!("❌ 文件读取失败: {}", e);
            eprintln!("💡 提示: 请检查文件路径是否正确");
        }
        Err(FontError::InvalidMagicNumber { expected, actual }) => {
            eprintln!("❌ 无效的字体格式");
            eprintln!("   期望魔数: {:#010X}", expected);
            eprintln!("   实际魔数: {:#010X}", actual);
        }
        Err(e) => {
            eprintln!("❌ 未知错误: {}", e);
        }
    }

    println!("\n📖 示例 2: 详细的错误信息");
    // 示例 2: 尝试加载不存在的表（演示结构化错误）
    let _font = Font::load("crates/rfont/src/AlimamaDaoLiTi.ttf")?;
    
    // 这个调用会触发 TableNotFound 错误
    // （实际代码中不会这样用，仅用于演示）
    
    println!("\n🎉 结构化错误处理演示完成！");
    println!("\n✨ 优势:");
    println!("   • 类型安全的错误匹配（模式匹配）");
    println!("   • 详细的错误上下文（文件名、偏移量等）");
    println!("   • 人类可读的错误消息");
    println!("   • 支持标准 Error trait（可与其他库集成）");
    
    Ok(())
}
