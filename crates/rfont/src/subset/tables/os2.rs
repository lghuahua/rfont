use rfont_types::FontError;

const UL_UNICODE_RANGE_OFFSET: usize = 42;
const US_FIRST_CHAR_INDEX_OFFSET: usize = 64;
const US_LAST_CHAR_INDEX_OFFSET: usize = 66;
const MIN_OS2_LENGTH: usize = US_LAST_CHAR_INDEX_OFFSET + 2;

/// OS/2 `ulUnicodeRange1..4` 位与 Unicode 区段的对应关系
///
/// 数据来自 OpenType 规范中 "OS/2 ulUnicodeRange" 的定义。
/// 同一个位可能对应多个区段。
const UNICODE_RANGES: &[(u8, u32, u32)] = &[
    (0, 0x0000, 0x007F),      // Basic Latin
    (1, 0x0080, 0x00FF),      // Latin-1 Supplement
    (2, 0x0100, 0x017F),      // Latin Extended-A
    (3, 0x0180, 0x024F),      // Latin Extended-B
    (4, 0x0250, 0x02AF),      // IPA Extensions
    (4, 0x1D00, 0x1D7F),      // Phonetic Extensions
    (4, 0x1D80, 0x1DBF),      // Phonetic Extensions Supplement
    (5, 0x02B0, 0x02FF),      // Spacing Modifier Letters
    (5, 0xA700, 0xA71F),      // Modifier Tone Letters
    (6, 0x0300, 0x036F),      // Combining Diacritical Marks
    (6, 0x1DC0, 0x1DFF),      // Combining Diacritical Marks Supplement
    (7, 0x0370, 0x03FF),      // Greek and Coptic
    (8, 0x2C80, 0x2CFF),      // Coptic
    (9, 0x0400, 0x04FF),      // Cyrillic
    (9, 0x0500, 0x052F),      // Cyrillic Supplement
    (9, 0x2DE0, 0x2DFF),      // Cyrillic Extended-A
    (9, 0xA640, 0xA69F),      // Cyrillic Extended-B
    (10, 0x0530, 0x058F),     // Armenian
    (11, 0x0590, 0x05FF),     // Hebrew
    (12, 0xA500, 0xA63F),     // Vai
    (13, 0x0600, 0x06FF),     // Arabic
    (13, 0x0750, 0x077F),     // Arabic Supplement
    (14, 0x07C0, 0x07FF),     // NKo
    (15, 0x0900, 0x097F),     // Devanagari
    (16, 0x0980, 0x09FF),     // Bengali
    (17, 0x0A00, 0x0A7F),     // Gurmukhi
    (18, 0x0A80, 0x0AFF),     // Gujarati
    (19, 0x0B00, 0x0B7F),     // Oriya
    (20, 0x0B80, 0x0BFF),     // Tamil
    (21, 0x0C00, 0x0C7F),     // Telugu
    (22, 0x0C80, 0x0CFF),     // Kannada
    (23, 0x0D00, 0x0D7F),     // Malayalam
    (24, 0x0E00, 0x0E7F),     // Thai
    (25, 0x0E80, 0x0EFF),     // Lao
    (26, 0x10A0, 0x10FF),     // Georgian
    (26, 0x2D00, 0x2D2F),     // Georgian Supplement
    (27, 0x1B00, 0x1B7F),     // Balinese
    (28, 0x1100, 0x11FF),     // Hangul Jamo
    (29, 0x1E00, 0x1EFF),     // Latin Extended Additional
    (29, 0x2C60, 0x2C7F),     // Latin Extended-C
    (29, 0xA720, 0xA7FF),     // Latin Extended-D
    (30, 0x1F00, 0x1FFF),     // Greek Extended
    (31, 0x2000, 0x206F),     // General Punctuation
    (31, 0x2E00, 0x2E7F),     // Supplemental Punctuation
    (32, 0x2070, 0x209F),     // Superscripts And Subscripts
    (33, 0x20A0, 0x20CF),     // Currency Symbols
    (34, 0x20D0, 0x20FF),     // Combining Diacritical Marks For Symbols
    (35, 0x2100, 0x214F),     // Letterlike Symbols
    (36, 0x2150, 0x218F),     // Number Forms
    (37, 0x2190, 0x21FF),     // Arrows
    (37, 0x27F0, 0x27FF),     // Supplemental Arrows-A
    (37, 0x2900, 0x297F),     // Supplemental Arrows-B
    (37, 0x2B00, 0x2BFF),     // Miscellaneous Symbols and Arrows
    (38, 0x2200, 0x22FF),     // Mathematical Operators
    (38, 0x27C0, 0x27EF),     // Miscellaneous Mathematical Symbols-A
    (38, 0x2980, 0x29FF),     // Miscellaneous Mathematical Symbols-B
    (38, 0x2A00, 0x2AFF),     // Supplemental Mathematical Operators
    (39, 0x2300, 0x23FF),     // Miscellaneous Technical
    (40, 0x2400, 0x243F),     // Control Pictures
    (41, 0x2440, 0x245F),     // Optical Character Recognition
    (42, 0x2460, 0x24FF),     // Enclosed Alphanumerics
    (43, 0x2500, 0x257F),     // Box Drawing
    (44, 0x2580, 0x259F),     // Block Elements
    (45, 0x25A0, 0x25FF),     // Geometric Shapes
    (46, 0x2600, 0x26FF),     // Miscellaneous Symbols
    (47, 0x2700, 0x27BF),     // Dingbats
    (48, 0x3000, 0x303F),     // CJK Symbols And Punctuation
    (49, 0x3040, 0x309F),     // Hiragana
    (50, 0x30A0, 0x30FF),     // Katakana
    (50, 0x31F0, 0x31FF),     // Katakana Phonetic Extensions
    (51, 0x3100, 0x312F),     // Bopomofo
    (51, 0x31A0, 0x31BF),     // Bopomofo Extended
    (52, 0x3130, 0x318F),     // Hangul Compatibility Jamo
    (53, 0xA840, 0xA87F),     // Phags-pa
    (54, 0x3200, 0x32FF),     // Enclosed CJK Letters And Months
    (55, 0x3300, 0x33FF),     // CJK Compatibility
    (56, 0xAC00, 0xD7AF),     // Hangul Syllables
    (58, 0x10900, 0x1091F),   // Phoenician
    (59, 0x20000, 0x2A6DF),   // CJK Unified Ideographs Extension B
    (60, 0x2A700, 0x2B73F),   // CJK Unified Ideographs Extension C
    (61, 0x2B740, 0x2B81F),   // CJK Unified Ideographs Extension D
    (62, 0x2B820, 0x2CEAF),   // CJK Unified Ideographs Extension E
    (63, 0x2F800, 0x2FA1F),   // CJK Compatibility Ideographs Supplement
    (64, 0xE000, 0xF8FF),     // Private Use Area
    (65, 0x4E00, 0x9FFF),     // CJK Unified Ideographs
    (66, 0xFB00, 0xFB4F),     // Alphabetic Presentation Forms
    (67, 0xFB50, 0xFDFF),     // Arabic Presentation Forms-A
    (68, 0xFE20, 0xFE2F),     // Combining Half Marks
    (69, 0xFE10, 0xFE1F),     // Vertical Forms
    (69, 0xFE30, 0xFE4F),     // CJK Compatibility Forms
    (70, 0xFE50, 0xFE6F),     // Small Form Variants
    (71, 0xFE70, 0xFEFF),     // Arabic Presentation Forms-B
    (72, 0xFF00, 0xFFEF),     // Halfwidth And Fullwidth Forms
    (73, 0xFFF0, 0xFFFF),     // Specials
    (74, 0x0F00, 0x0FFF),     // Tibetan
    (75, 0x0700, 0x074F),     // Syriac
    (76, 0x0780, 0x07BF),     // Thaana
    (77, 0x0D80, 0x0DFF),     // Sinhala
    (78, 0x1000, 0x109F),     // Myanmar
    (79, 0x1200, 0x137F),     // Ethiopic
    (79, 0x1380, 0x139F),     // Ethiopic Supplement
    (79, 0x2D80, 0x2DDF),     // Ethiopic Extended
    (80, 0x13A0, 0x13FF),     // Cherokee
    (81, 0x1400, 0x167F),     // Unified Canadian Aboriginal Syllabics
    (82, 0x1680, 0x169F),     // Ogham
    (83, 0x16A0, 0x16FF),     // Runic
    (84, 0x1780, 0x17FF),     // Khmer
    (84, 0x19E0, 0x19FF),     // Khmer Symbols
    (85, 0x1800, 0x18AF),     // Mongolian
    (86, 0x2800, 0x28FF),     // Braille Patterns
    (87, 0xA000, 0xA48F),     // Yi Syllables
    (87, 0xA490, 0xA4CF),     // Yi Radicals
    (88, 0x1700, 0x171F),     // Tagalog
    (89, 0x10300, 0x1032F),   // Old Italic
    (90, 0x10330, 0x1034F),   // Gothic
    (91, 0x10400, 0x1044F),   // Deseret
    (92, 0x1D000, 0x1D0FF),   // Byzantine Musical Symbols
    (92, 0x1D100, 0x1D1FF),   // Musical Symbols
    (92, 0x1D200, 0x1D24F),   // Ancient Greek Musical Notation
    (93, 0x1D400, 0x1D7FF),   // Mathematical Alphanumeric Symbols
    (94, 0xFF000, 0xFFFFD),   // Private Use (plane 15)
    (94, 0x100000, 0x10FFFD), // Private Use (plane 16)
    (95, 0xFE00, 0xFE0F),     // Variation Selectors
    (95, 0xE0100, 0xE01EF),   // Variation Selectors Supplement
    (96, 0xE0000, 0xE007F),   // Tags
    (97, 0x1900, 0x194F),     // Limbu
    (98, 0x1950, 0x197F),     // Tai Le
    (99, 0x1980, 0x19DF),     // New Tai Lue
    (100, 0x1A00, 0x1A1F),    // Buginese
    (101, 0x2C00, 0x2C5F),    // Glagolitic
    (102, 0x2D30, 0x2D7F),    // Tifinagh
    (103, 0x4DC0, 0x4DFF),    // Yijing Hexagram Symbols
    (104, 0xA800, 0xA82F),    // Syloti Nagri
    (105, 0x10000, 0x1007F),  // Linear B Syllabary
    (105, 0x10080, 0x100FF),  // Linear B Ideograms
    (105, 0x10100, 0x1013F),  // Aegean Numbers
    (106, 0x10140, 0x1018F),  // Ancient Greek Numbers
    (107, 0x10380, 0x1039F),  // Ugaritic
    (108, 0x103A0, 0x103DF),  // Old Persian
    (109, 0x10450, 0x1047F),  // Shavian
    (110, 0x10480, 0x104AF),  // Osmanya
    (111, 0x10800, 0x1083F),  // Cypriot Syllabary
    (112, 0x10A00, 0x10A5F),  // Kharoshthi
    (113, 0x1D300, 0x1D35F),  // Tai Xuan Jing Symbols
    (114, 0x12000, 0x123FF),  // Cuneiform
    (114, 0x12400, 0x1247F),  // Cuneiform Numbers and Punctuation
    (115, 0x1D360, 0x1D37F),  // Counting Rod Numerals
    (116, 0x1B80, 0x1BBF),    // Sundanese
    (117, 0x1C00, 0x1C4F),    // Lepcha
    (118, 0x1C50, 0x1C7F),    // Ol Chiki
    (119, 0xA880, 0xA8DF),    // Saurashtra
    (120, 0xA900, 0xA92F),    // Kayah Li
    (121, 0xA930, 0xA95F),    // Rejang
    (122, 0xAA00, 0xAA5F),    // Cham
    (123, 0x10190, 0x101CF),  // Ancient Symbols
    (124, 0x101D0, 0x101FF),  // Phaistos Disc
    (125, 0x10280, 0x1029F),  // Lycian
    (125, 0x102A0, 0x102DF),  // Carian
    (125, 0x10920, 0x1093F),  // Lydian
    (126, 0x1F000, 0x1F02F),  // Mahjong Tiles
    (126, 0x1F030, 0x1F09F),  // Domino Tiles
];

/// 按子集字符集更新 OS/2 表
///
/// 更新内容：
/// - `usFirstCharIndex` / `usLastCharIndex`
/// - `ulUnicodeRange1..4`（按实际字符重新计算，非 BMP 置位 57）
///
/// 表长度不足时原样返回。
pub fn update_os2(original: &[u8], codepoints: &[u32]) -> Result<Vec<u8>, FontError> {
    if original.len() < MIN_OS2_LENGTH {
        return Ok(original.to_vec());
    }

    let mut data = original.to_vec();
    data[UL_UNICODE_RANGE_OFFSET..UL_UNICODE_RANGE_OFFSET + 16].fill(0);

    let mut first = u32::MAX;
    let mut last = 0u32;
    for &code in codepoints {
        first = first.min(code);
        last = last.max(code);

        if code > 0xFFFF {
            set_unicode_range_bit(&mut data, 57);
        }
        for &(bit, start, end) in UNICODE_RANGES {
            if code >= start && code <= end {
                set_unicode_range_bit(&mut data, bit as usize);
            }
        }
    }

    let (first, last) = if codepoints.is_empty() {
        (0u16, 0u16)
    } else {
        (first.min(0xFFFF) as u16, last.min(0xFFFF) as u16)
    };
    data[US_FIRST_CHAR_INDEX_OFFSET..US_FIRST_CHAR_INDEX_OFFSET + 2]
        .copy_from_slice(&first.to_be_bytes());
    data[US_LAST_CHAR_INDEX_OFFSET..US_LAST_CHAR_INDEX_OFFSET + 2]
        .copy_from_slice(&last.to_be_bytes());

    Ok(data)
}

fn set_unicode_range_bit(data: &mut [u8], bit: usize) {
    const BITS_PER_WORD: usize = 32;

    let word_offset = UL_UNICODE_RANGE_OFFSET + (bit / BITS_PER_WORD) * 4;
    let shift = BITS_PER_WORD - 1 - (bit % BITS_PER_WORD);
    let mut value = u32::from_be_bytes([
        data[word_offset],
        data[word_offset + 1],
        data[word_offset + 2],
        data[word_offset + 3],
    ]);
    value |= 1 << shift;
    data[word_offset..word_offset + 4].copy_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os2_table() -> Vec<u8> {
        let mut data = vec![0u8; 78];
        data[0..2].copy_from_slice(&4u16.to_be_bytes());
        data
    }

    fn unicode_range_mask(data: &[u8], bit: usize) -> bool {
        let offset = UL_UNICODE_RANGE_OFFSET + (bit / 32) * 4;
        let value = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        value & (1 << (31 - bit % 32)) != 0
    }

    #[test]
    fn test_update_os2_first_last_char() {
        let data = update_os2(&os2_table(), &[0x41, 0x4E2D, 0x1F600]).unwrap();
        assert_eq!(
            u16::from_be_bytes([
                data[US_FIRST_CHAR_INDEX_OFFSET],
                data[US_FIRST_CHAR_INDEX_OFFSET + 1]
            ]),
            0x41
        );
        assert_eq!(
            u16::from_be_bytes([
                data[US_LAST_CHAR_INDEX_OFFSET],
                data[US_LAST_CHAR_INDEX_OFFSET + 1]
            ]),
            0xFFFF
        );
    }

    #[test]
    fn test_update_os2_unicode_ranges() {
        let data = update_os2(&os2_table(), &[0x41, 0x4E2D, 0x1F600]).unwrap();

        assert!(unicode_range_mask(&data, 0)); // Basic Latin
        assert!(unicode_range_mask(&data, 65)); // CJK Unified Ideographs
        assert!(unicode_range_mask(&data, 57)); // Non-Plane 0
        assert!(!unicode_range_mask(&data, 7)); // 无 Greek
    }

    #[test]
    fn test_update_os2_clears_previous_bits() {
        let mut original = os2_table();
        original[UL_UNICODE_RANGE_OFFSET] = 0xFF; // 置位 bit 0..7（含 Greek）
        let data = update_os2(&original, &[0x41]).unwrap();

        assert!(unicode_range_mask(&data, 0));
        assert!(!unicode_range_mask(&data, 7)); // 原 Greek 位被清除
    }

    #[test]
    fn test_update_os2_short_table_is_unchanged() {
        let short = vec![0u8; 20];
        assert_eq!(update_os2(&short, &[0x41]).unwrap(), short);
    }
}
