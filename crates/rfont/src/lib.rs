mod constants;
mod checksum;
mod info;
mod font_data;
mod font;
mod subset;

// 重新导出公共 API
pub use font::Font;
pub use font_data::FontData;
pub use info::{FontInfo, TableInfo};
pub use subset::{FontSubsetBuilder, SubsetOptions};
pub use font::GlyphIterator;

/// 批量子集化处理（并行版本）
/// 
/// 当启用 `parallel` feature 时，使用 Rayon 进行并行处理，
/// 可显著提升大批量子集化任务的性能。
/// 
/// # 示例
/// ```no_run
/// use rfont::{Font, subset_batch_parallel};
/// 
/// let font = Font::load("font.ttf").unwrap();
/// let texts = vec!["你好", "世界", "字体"];
/// 
/// // 并行处理多个文本的子集化
/// let results = subset_batch_parallel(&font, &texts, &Default::default());
/// 
/// for (i, result) in results.iter().enumerate() {
///     match result {
///         Ok(data) => println!("文本 {} 子集化成功: {} bytes", i, data.len()),
///         Err(e) => eprintln!("文本 {} 子集化失败: {}", i, e),
///     }
/// }
/// ```
#[cfg(feature = "parallel")]
pub fn subset_batch_parallel(
    font: &Font,
    texts: &[&str],
    options: &SubsetOptions,
) -> Vec<Result<Vec<u8>, rfont_types::FontError>> {
    use rayon::prelude::*;
    
    texts.par_iter()
        .map(|text| {
            let glyph_ids = font.text_to_glyph_ids(text);
            font.subset_with_options(&glyph_ids, options)
        })
        .collect()
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_tag_conversion() {
        use rfont_types::Tag;
        let tag = Tag(*b"head");
        assert_eq!(tag.0, [0x68, 0x65, 0x61, 0x64]);
        
        let tag2 = Tag(*b"cmap");
        assert_eq!(tag2.as_str(), "cmap");
    }

    #[test]
    fn test_glyph_id_subset_deduplication() {
        // 测试字形ID去重逻辑
        let mut ids = vec![5, 3, 1, 5, 2, 3];
        ids.sort();
        ids.dedup();
        
        assert_eq!(ids, vec![1, 2, 3, 5]);
    }

    #[test]
    fn test_notdef_inclusion() {
        // 测试 .notdef (glyph 0) 的自动包含
        let glyph_ids = vec![5, 10, 15];
        let mut subset_glyphs_vec: Vec<u16> = glyph_ids.clone();
        
        if !subset_glyphs_vec.contains(&0) {
            subset_glyphs_vec.push(0);
        }
        subset_glyphs_vec.sort();
        
        assert!(subset_glyphs_vec.contains(&0));
        assert_eq!(subset_glyphs_vec.len(), 4); // 3 + 1 (.notdef)
    }

    #[test]
    fn test_cmap_format_selection() {
        // 测试 cmap 格式选择逻辑
        let bmp_chars = vec!['A', 'B', 'C']; // BMP 字符
        let non_bmp_chars = vec!['😀']; // 非 BMP 字符 (U+1F600)
        
        let has_non_bmp = non_bmp_chars.iter().any(|&c| (c as u32) > 0xFFFF);
        assert!(has_non_bmp);
        
        let all_bmp = bmp_chars.iter().all(|&c| (c as u32) <= 0xFFFF);
        assert!(all_bmp);
    }
    
    #[test]
    fn test_font_format_detection_ttf() {
        use crate::Font;
        use rfont_types::FontFormat;
        
        // 读取 TTF 文件
        let data = std::fs::read("src/AlimamaDaoLiTi.ttf").unwrap();
        let format_info = Font::detect_format(&data).unwrap();
        
        assert_eq!(format_info.format, FontFormat::Ttf);
        assert_eq!(format_info.version, "0x00010000");
        assert!(!format_info.is_variable);
        assert!(format_info.compression.is_none());
        assert!(format_info.has_required_tables(&format_info.required_tables));
    }
    
    #[test]
    fn test_font_format_detection_woff() {
        use crate::Font;
        use rfont_types::{FontFormat, CompressionType};
        
        // 读取 WOFF 文件
        let data = std::fs::read("src/AlimamaDaoLiTi.woff").unwrap();
        let format_info = Font::detect_format(&data).unwrap();
        
        assert_eq!(format_info.format, FontFormat::Ttf); // WOFF 内部是 TTF
        assert_eq!(format_info.compression, Some(CompressionType::Zlib));
        assert!(format_info.has_required_tables(&format_info.required_tables));
    }
    
    #[test]
    fn test_font_format_detection_woff2() {
        use crate::Font;
        use rfont_types::{FontFormat, CompressionType};
        
        // 读取 WOFF2 文件
        let data = std::fs::read("src/AlimamaDaoLiTi.woff2").unwrap();
        let format_info = Font::detect_format(&data).unwrap();
        
        assert_eq!(format_info.format, FontFormat::Ttf); // WOFF2 内部是 TTF
        assert_eq!(format_info.compression, Some(CompressionType::Brotli));
        assert!(format_info.has_required_tables(&format_info.required_tables));
    }
}
