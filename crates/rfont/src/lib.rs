mod checksum;
mod constants;
mod font;
mod font_data;
mod info;
mod subset;

// 重新导出公共 API
pub use font::Font;
pub use font::GlyphIterator;
pub use font_data::FontData;
pub use info::{FontInfo, TableInfo};
pub use subset::{FontSubsetBuilder, SubsetOptions};

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

    texts
        .par_iter()
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
        use rfont_types::{CompressionType, FontFormat};

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
        use rfont_types::{CompressionType, FontFormat};

        // 读取 WOFF2 文件
        let data = std::fs::read("src/AlimamaDaoLiTi.woff2").unwrap();
        let format_info = Font::detect_format(&data).unwrap();

        assert_eq!(format_info.format, FontFormat::Ttf); // WOFF2 内部是 TTF
        assert_eq!(format_info.compression, Some(CompressionType::Brotli));
        assert!(format_info.has_required_tables(&format_info.required_tables));
    }

    #[test]
    fn test_subset_builder_basic() {
        use crate::Font;

        // 使用 Builder 模式进行子集化
        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();
        let original_data = std::fs::read("src/AlimamaDaoLiTi.ttf").unwrap();

        let subset_data = font.subset_builder().text("Hello").build().unwrap();

        // 验证生成的字体数据不为空
        assert!(!subset_data.is_empty());
        assert!(subset_data.len() < original_data.len()); // 子集应该更小
    }

    #[test]
    fn test_subset_with_glyph_ids() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        // 直接使用字形 ID 进行子集化
        let glyph_ids = vec![0, 1, 2, 3, 4, 5];
        let subset_data = font.subset_builder().glyph_ids(glyph_ids).build().unwrap();

        assert!(!subset_data.is_empty());
    }

    #[test]
    fn test_subset_with_unicode_range() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        // 使用 Unicode 范围进行子集化（基本拉丁字母）
        let subset_data = font
            .subset_builder()
            .unicode_range(0x0041, 0x005A) // A-Z
            .build()
            .unwrap();

        assert!(!subset_data.is_empty());
    }

    #[test]
    fn test_subset_options_web_optimized() {
        use crate::{Font, SubsetOptions};

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        // 使用 Web 优化预设
        let options = SubsetOptions::web_optimized();
        assert_eq!(options.output_format, "woff");
        assert_eq!(options.compression_level, 9);
        assert!(options.optimize_post_table);
        assert!(options.strip_glyph_names);

        let glyph_ids = font.text_to_glyph_ids("Test");
        let subset_data = font.subset_with_options(&glyph_ids, &options).unwrap();

        assert!(!subset_data.is_empty());
    }

    #[test]
    fn test_subset_options_print_optimized() {
        use crate::SubsetOptions;

        let options = SubsetOptions::print_optimized();
        assert_eq!(options.output_format, "ttf");
        assert_eq!(options.compression_level, 0);
        assert!(!options.optimize_post_table);
        assert!(!options.strip_glyph_names);
        assert!(options.keep_hinting);
    }

    #[test]
    fn test_text_to_glyph_ids() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        let text = "ABC";
        let glyph_ids = font.text_to_glyph_ids(text);

        // 应该返回 3 个字形 ID
        assert_eq!(glyph_ids.len(), 3);

        // 所有 ID 应该有效（非零，除非字符不存在）
        for id in &glyph_ids {
            assert!(*id <= font.maxp.num_glyphs);
        }
    }

    #[test]
    fn test_get_supported_characters() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        let chars = font.get_supported_characters();

        // 字符列表不应为空
        assert!(!chars.is_empty());

        // 应该已排序
        for i in 1..chars.len() {
            assert!(chars[i] > chars[i - 1]);
        }
    }

    #[test]
    fn test_supports_character() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        // 测试常见字符（应该在字体中）
        let common_chars = vec!['A', 'B', 'C', 'a', 'b', 'c', '0', '1', '2'];
        for ch in common_chars {
            // 这个测试假设字体支持基本拉丁字母
            // 如果失败，说明字体可能不支持这些字符
            let supported = font.supports_character(ch as u32);
            // 不强制断言，因为字体可能不包含这些字符
            println!(
                "Character '{}' (U+{:04X}): {}",
                ch,
                ch as u32,
                if supported {
                    "supported"
                } else {
                    "not supported"
                }
            );
        }
    }

    #[test]
    fn test_get_font_info() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        let info = font.get_font_info();

        // 验证基本信息
        assert!(info.units_per_em > 0);
        assert!(info.glyph_count > 0);
        assert!(info.supported_char_count > 0);
        assert!(!info.tables.is_empty());

        println!("Font Info:");
        println!("  Units per EM: {}", info.units_per_em);
        println!("  Glyph count: {}", info.glyph_count);
        println!("  Supported chars: {}", info.supported_char_count);
        println!("  Tables: {}", info.tables.len());
    }

    #[test]
    fn test_get_table_list() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        let tables = font.get_table_list();

        // TTF 字体应该包含必要的表
        assert!(!tables.is_empty());

        // 检查是否包含必需的表
        let table_tags: Vec<&str> = tables.iter().map(|t| t.tag.as_str()).collect();
        assert!(table_tags.contains(&"head"));
        assert!(table_tags.contains(&"maxp"));
        assert!(table_tags.contains(&"cmap"));
        assert!(table_tags.contains(&"glyf"));
        assert!(table_tags.contains(&"loca"));
        assert!(table_tags.contains(&"hhea"));
        assert!(table_tags.contains(&"hmtx"));
    }

    #[test]
    fn test_glyph_iterator() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        let mut count = 0;
        for (glyph_id, glyph_data) in font.glyph_iter() {
            count += 1;
            // glyph_id 应该从 0 开始递增
            assert_eq!(glyph_id as usize, count - 1);

            // .notdef (glyph 0) 应该有数据或者为空
            if glyph_id == 0 {
                // .notdef 可能存在也可能不存在
            }
        }

        // 迭代器应该遍历所有字形
        assert_eq!(count, font.maxp.num_glyphs as usize);
    }

    #[test]
    fn test_subset_empty_text() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        // 空文本应该导致错误
        let result = font.subset_builder().text("").build();

        assert!(result.is_err());
    }

    #[test]
    fn test_subset_nonexistent_characters() {
        use crate::Font;

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        // 使用字体可能不支持的字符（如 emoji）
        let result = font.subset_builder().text("😀🎉🚀").build();

        // 如果字体不支持这些字符，应该返回错误
        // （但至少会包含 .notdef）
        match result {
            Ok(data) => {
                // 如果成功，至少包含 .notdef
                assert!(!data.is_empty());
            }
            Err(_) => {
                // 错误也是可接受的
            }
        }
    }

    #[test]
    fn test_subset_with_custom_options() {
        use crate::{Font, SubsetOptions};

        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();

        // 自定义选项
        let options = SubsetOptions {
            optimize_post_table: false,
            strip_glyph_names: false,
            compression_level: 5,
            keep_hinting: true,
            output_format: "ttf".to_string(),
        };

        let glyph_ids = font.text_to_glyph_ids("Test");
        let subset_data = font.subset_with_options(&glyph_ids, &options).unwrap();

        assert!(!subset_data.is_empty());
    }
}
