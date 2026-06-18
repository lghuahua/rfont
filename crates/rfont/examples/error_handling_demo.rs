use rfont::Font;
use rfont_types::FontError;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 示例 1: 加载字体文件（使用 anyhow 的错误链）
    println!("📖 示例 1: 加载字体文件");
    match Font::load("AlimamaDaoLiTi.ttf") {
        Ok(font) => {
            println!("✅ 字体加载成功！");
            println!("   字形数量: {}", font.maxp().num_glyphs);
        }
        Err(e) => {
            eprintln!("❌ 字体加载失败: {}", e);

            // 尝试获取底层原因并显示建议
            if let Some(source) = e.source()
                && let Some(font_error) = source.downcast_ref::<FontError>()
            {
                if let Some(suggestion) = font_error.suggestion() {
                    eprintln!("💡 建议: {}", suggestion);
                }

                // 演示模式匹配具体的错误类型
                match font_error {
                    FontError::Io(io_err) => {
                        eprintln!("   错误类型: IO 错误 - {}", io_err);
                    }
                    FontError::InvalidMagicNumber { expected, actual } => {
                        eprintln!("   错误类型: 无效的魔数");
                        eprintln!("   期望: {:#010X}, 实际: {:#010X}", expected, actual);
                    }
                    _ => {
                        eprintln!("   错误类型: {}", font_error);
                    }
                }
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

    println!("\n📖 示例 3: 带上下文的 UnexpectedEndOfData 错误");
    // 示例 3: 演示新增的上下文功能
    let error_with_context = FontError::unexpected_end(100, 50, "Reader::read_u8");
    println!("   {}", error_with_context);
    if let Some(suggestion) = error_with_context.suggestion() {
        println!("   💡 建议: {}", suggestion);
    }

    println!("\n📖 示例 4: 不带上下文的错误（向后兼容）");
    let error_without_context = FontError::UnexpectedEndOfData {
        offset: 200,
        needed: 30,
        context: None,
    };
    println!("   {}", error_without_context);

    println!("\n📖 示例 5: 错误链支持");
    // 示例 5: 演示 IO 错误链
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
    println!("   • 方法级追踪（context 字段显示错误来源）");

    Ok(())
}
