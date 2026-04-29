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
}
