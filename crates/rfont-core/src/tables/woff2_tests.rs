#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::Reader;

    #[test]
    fn test_woff2_header_read() {
        // 测试 WOFF2 Header 读取
        let data = vec![
            // signature: 'wOF2' (0x774F4632)
            0x77, 0x4F, 0x46, 0x32,
            // flavor: TrueType (0x00010000)
            0x00, 0x01, 0x00, 0x00,
            // length: 1000 bytes
            0x00, 0x00, 0x03, 0xE8,
            // num_tables: 5
            0x00, 0x05,
            // reserved: 0
            0x00, 0x00,
            // total_sfnt_size: 2000 bytes
            0x00, 0x00, 0x07, 0xD0,
        ];
        
        let mut reader = Reader::new(&data);
        let header = Woff2Header::read_from(&mut reader).unwrap();
        
        assert_eq!(header.signature, 0x774F4632);
        assert_eq!(header.flavor, 0x00010000);
        assert_eq!(header.length, 1000);
        assert_eq!(header.num_tables, 5);
        assert_eq!(header.total_sfnt_size, 2000);
    }

    #[test]
    fn test_woff2_header_validation() {
        // 测试有效的 WOFF2 Header
        let valid_data = vec![
            0x77, 0x4F, 0x46, 0x32, // signature
            0x00, 0x01, 0x00, 0x00, // flavor
            0x00, 0x00, 0x03, 0xE8, // length
            0x00, 0x05,             // num_tables
            0x00, 0x00,             // reserved
            0x00, 0x00, 0x07, 0xD0, // total_sfnt_size
        ];
        
        let mut reader = Reader::new(&valid_data);
        let header = Woff2Header::read_from(&mut reader).unwrap();
        assert!(header.validate().is_ok());
        
        // 测试无效的签名
        let invalid_data = vec![
            0x77, 0x4F, 0x46, 0x46, // 'wOFF' instead of 'wOF2'
            0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x05,
            0x00, 0x00,
            0x00, 0x00, 0x07, 0xD0,
        ];
        
        let mut reader2 = Reader::new(&invalid_data);
        let header2 = Woff2Header::read_from(&mut reader2).unwrap();
        assert!(header2.validate().is_err());
    }

    #[test]
    fn test_base128_encoding() {
        // 测试 Base128 编码读取
        // 值 127 (0x7F) - 单字节
        let data1 = vec![0x7F];
        let mut reader1 = Reader::new(&data1);
        let value1 = Woff2TableDirectoryEntry::read_base128_internal(&mut reader1).unwrap();
        assert_eq!(value1, 127);
        
        // 值 128 (0x80 0x00) - 双字节
        let data2 = vec![0x81, 0x00];
        let mut reader2 = Reader::new(&data2);
        let value2 = Woff2TableDirectoryEntry::read_base128_internal(&mut reader2).unwrap();
        assert_eq!(value2, 128);
        
        // 值 16384 (0x81 0x80 0x00) - 三字节
        let data3 = vec![0x81, 0x80, 0x00];
        let mut reader3 = Reader::new(&data3);
        let value3 = Woff2TableDirectoryEntry::read_base128_internal(&mut reader3).unwrap();
        assert_eq!(value3, 16384);
    }

    #[test]
    fn test_woff2_known_tags() {
        // 验证预定义标签数量
        assert_eq!(WOFF2_KNOWN_TAGS.len(), 63);
        
        // 验证一些常见标签
        assert_eq!(WOFF2_KNOWN_TAGS[1].as_str(), "cmap");
        assert_eq!(WOFF2_KNOWN_TAGS[10].as_str(), "glyf");
        assert_eq!(WOFF2_KNOWN_TAGS[11].as_str(), "loca");
        assert_eq!(WOFF2_KNOWN_TAGS[2].as_str(), "hhea");
        assert_eq!(WOFF2_KNOWN_TAGS[3].as_str(), "hmtx");
    }
}

// 辅助函数用于测试（需要公开 read_base128）
impl Woff2TableDirectoryEntry {
    #[cfg(test)]
    pub fn read_base128_internal(reader: &mut Reader) -> Result<u32, FontError> {
        Self::read_base128(reader)
    }
}
