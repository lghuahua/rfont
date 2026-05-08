use rfont_types::{FontError, Reader, ReadBytes, Tag};
use font_macros::ReadBytes;

/// WOFF2 Header 结构
/// 参考: <https://www.w3.org/TR/WOFF2/#woff20Header>
#[derive(Debug, Clone, ReadBytes)]
pub struct Woff2Header {
    pub signature: u32,           // 0x774F4632 ('wOF2')
    pub flavor: u32,              // Original font signature (0x00010000 for TrueType)
    pub length: u32,              // Total size of WOFF2 file
    pub num_tables: u16,          // Number of tables
    pub reserved: u16,            // Reserved (set to 0)
    pub total_sfnt_size: u32,     // Uncompressed size of the entire font
}

impl Woff2Header {
    pub fn validate(&self) -> Result<(), FontError> {
        if self.signature != 0x774F4632 { // "wOF2"
            return Err(FontError::InvalidMagicNumber { 
                expected: 0x774F4632, 
                actual: self.signature 
            });
        }
        
        if self.num_tables == 0 {
            return Err(FontError::Generic("WOFF2: No tables in font".to_string()));
        }
        
        Ok(())
    }
}

/// WOFF2 表类型枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Woff2TableType {
    Woff2TransformedGlyf = 0xCFFF,
    Woff2TransformedLoca = 0xCF,
    Woff2OriginalFormat = 0x3F,
}

/// WOFF2 表目录项
/// 注意：WOFF2 的表目录格式与 WOFF 不同，使用变长编码
#[derive(Debug, Clone)]
pub struct Woff2TableDirectoryEntry {
    pub flags: u8,
    pub tag: Option<Tag>,         // None 表示使用预定义标签
    pub orig_length: u32,         // 原始长度（未压缩）
    pub transform_length: Option<u32>, // 转换后的长度（如果应用了转换）
}

impl Woff2TableDirectoryEntry {
    /// 从 reader 中读取一个表目录项
    pub fn read_from(reader: &mut Reader, known_tags: &[Tag]) -> Result<Self, FontError> {
        let flags = reader.read_u8()?;
        
        // 提取 table type (高 6 bits)
        let table_type = (flags >> 2) & 0x3F;
        
        let tag = if table_type < 63 {
            // 使用预定义标签
            if table_type as usize >= known_tags.len() {
                return Err(FontError::Generic(format!(
                    "WOFF2: Invalid predefined tag index: {}",
                    table_type
                )));
            }
            Some(known_tags[table_type as usize])
        } else {
            // 读取 4 字节自定义标签
            let tag_bytes = [
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
            ];
            Some(Tag(tag_bytes))
        };
        
        // 读取原始长度（使用 Base128 编码）
        let orig_length = Self::read_base128(reader)?;
        
        // 检查是否有转换长度（glyf 和 loca 表）
        // 在 WOFF2 中，通过标签判断是否需要转换长度
        let transform_length = if let Some(tag) = tag {
            if tag.as_str() == "glyf" || tag.as_str() == "loca" {
                // glyf 和 loca 表有转换
                Some(Self::read_base128(reader)?)
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(Woff2TableDirectoryEntry {
            flags,
            tag,
            orig_length,
            transform_length,
        })
    }
    
    /// 读取 Base128 编码的整数
    fn read_base128(reader: &mut Reader) -> Result<u32, FontError> {
        let mut result: u32 = 0;
        
        loop {
            if result > 0x0FFFFFFF {
                return Err(FontError::Generic("WOFF2: Base128 overflow".to_string()));
            }
            
            let byte = reader.read_u8()?;
            result = (result << 7) | ((byte & 0x7F) as u32);
            
            if byte & 0x80 == 0 {
                break;
            }
        }
        
        Ok(result)
    }
}

/// WOFF2 预定义标签列表（按规范顺序）
pub const WOFF2_KNOWN_TAGS: [Tag; 63] = [
    Tag(*b"gasp"), Tag(*b"cmap"), Tag(*b"hhea"), Tag(*b"hmtx"),
    Tag(*b"maxp"), Tag(*b"name"), Tag(*b"OS/2"), Tag(*b"post"),
    Tag(*b"cvt "), Tag(*b"fpgm"), Tag(*b"glyf"), Tag(*b"loca"),
    Tag(*b"prep"), Tag(*b"CFF "), Tag(*b"VDMX"), Tag(*b"hdmx"),
    Tag(*b"kern"), Tag(*b"LTSH"), Tag(*b"PCLT"), Tag(*b"DSIG"),
    Tag(*b"EBDT"), Tag(*b"EBLC"), Tag(*b"EBSC"), Tag(*b"BASE"),
    Tag(*b"GDEF"), Tag(*b"GPOS"), Tag(*b"GSUB"), Tag(*b"JSTF"),
    Tag(*b"MATH"), Tag(*b"CBLC"), Tag(*b"COLR"), Tag(*b"CPAL"),
    Tag(*b"SVG "), Tag(*b"sbix"), Tag(*b"acnt"), Tag(*b"ankr"),
    Tag(*b"bhed"), Tag(*b"bloc"), Tag(*b"cvar"), Tag(*b"fdsc"),
    Tag(*b"feat"), Tag(*b"fmtx"), Tag(*b"fvar"), Tag(*b"gvar"),
    Tag(*b"hsty"), Tag(*b"just"), Tag(*b"lcar"), Tag(*b"mort"),
    Tag(*b"morx"), Tag(*b"opbd"), Tag(*b"prop"), Tag(*b"trak"),
    Tag(*b"Zapf"), Tag(*b"SILF"), Tag(*b"SILL"), Tag(*b"Silf"),
    Tag(*b"Fea "), Tag(*b"Glat"), Tag(*b"Jstf"), Tag(*b"Ltag"),
    Tag(*b"Prop"), Tag(*b"Sill"), Tag(*b"bsln"),
];

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
        // 值 127 (0x7F) - 单字节，最高位为 0 表示结束
        let data1 = vec![0x7F];
        let mut reader1 = Reader::new(&data1);
        let value1 = Woff2TableDirectoryEntry::read_base128_test(&mut reader1).unwrap();
        assert_eq!(value1, 127);
        
        // 值 128 - 双字节编码: (1 << 7) | 0x80, 0x00
        // 第一个字节: 0x81 (继续位=1, 值=1)
        // 第二个字节: 0x00 (继续位=0, 值=0)
        // 结果: (1 << 7) | 0 = 128
        let data2 = vec![0x81, 0x00];
        let mut reader2 = Reader::new(&data2);
        let value2 = Woff2TableDirectoryEntry::read_base128_test(&mut reader2).unwrap();
        assert_eq!(value2, 128);
        
        // 值 300 - 双字节编码
        // 300 = 2*128 + 44 = 0x02 0x2C
        // 第一个字节: 0x82 (继续位=1, 值=2)
        // 第二个字节: 0x2C (继续位=0, 值=44)
        let data3 = vec![0x82, 0x2C];
        let mut reader3 = Reader::new(&data3);
        let value3 = Woff2TableDirectoryEntry::read_base128_test(&mut reader3).unwrap();
        assert_eq!(value3, 300);
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

    #[test]
    fn test_woff2_table_directory_predefined_tag() {
        // 测试使用预定义标签的表目录项
        // table_type = 2 (hhea), flags = 2 << 2 = 0x08
        let data = vec![
            0x08,                   // flags: table_type = 2 (hhea)
            0x81, 0x00,             // orig_length = 128 (Base128: 0x81 0x00)
        ];
        
        let mut reader = Reader::new(&data);
        let entry = Woff2TableDirectoryEntry::read_from(&mut reader, &WOFF2_KNOWN_TAGS).unwrap();
        
        assert_eq!(entry.tag.unwrap().as_str(), "hhea");
        assert_eq!(entry.orig_length, 128);
        assert_eq!(entry.transform_length, None);
    }

    #[test]
    fn test_woff2_table_directory_custom_tag() {
        // 测试使用自定义标签的表目录项（table_type >= 63）
        // flags: table_type = 63 << 2 = 0xFC, + 0x03 (custom tag flag) = 0xFF
        let data = vec![
            0xFF,                   // flags: table_type = 63 (custom tag)
            b'c', b'u', b's', b't', // custom tag = 'cust'
            0x40,                   // orig_length = 64 (single byte Base128)
        ];
        
        let mut reader = Reader::new(&data);
        let entry = Woff2TableDirectoryEntry::read_from(&mut reader, &WOFF2_KNOWN_TAGS).unwrap();
        
        assert_eq!(entry.tag.unwrap().as_str(), "cust");
        assert_eq!(entry.orig_length, 64);
    }

    #[test]
    fn test_woff2_table_directory_glyf_with_transform() {
        // glyf 表应该有转换长度
        // flags: table_type = 10 (glyf) << 2 = 0x28
        let data = vec![
            0x28,                   // flags: table_type = 10 (glyf)
            0x84, 0x00,             // orig_length = 512 (Base128: 0x84 0x00)
            0x82, 0x00,             // transform_length = 256 (Base128: 0x82 0x00)
        ];
        
        let mut reader = Reader::new(&data);
        let entry = Woff2TableDirectoryEntry::read_from(&mut reader, &WOFF2_KNOWN_TAGS).unwrap();
        
        assert_eq!(entry.tag.unwrap().as_str(), "glyf");
        assert_eq!(entry.orig_length, 512);
        assert_eq!(entry.transform_length, Some(256));
    }

    #[test]
    fn test_base128_single_byte() {
        // 单字节 Base128 编码（0-127）
        for value in [0, 1, 64, 127] {
            let data = vec![value as u8];
            let mut reader = Reader::new(&data);
            let result = Woff2TableDirectoryEntry::read_base128_test(&mut reader).unwrap();
            assert_eq!(result, value as u32);
        }
    }

    #[test]
    fn test_base128_multi_byte() {
        // 多字节 Base128 编码
        // 值 128 = 0x80
        // 编码: 0x81 (继续位=1, 值=1), 0x00 (继续位=0, 值=0)
        // 结果: (1 << 7) | 0 = 128
        let data = vec![0x81, 0x00];
        let mut reader = Reader::new(&data);
        let value = Woff2TableDirectoryEntry::read_base128_test(&mut reader).unwrap();
        assert_eq!(value, 128);
    }

    #[test]
    fn test_base128_large_value() {
        // 大数值测试
        // 值 300 = 2*128 + 44
        // 编码: 0x82 (继续位=1, 值=2), 0x2C (继续位=0, 值=44)
        let data = vec![0x82, 0x2C];
        let mut reader = Reader::new(&data);
        let value = Woff2TableDirectoryEntry::read_base128_test(&mut reader).unwrap();
        assert_eq!(value, 300);
    }
}

// 测试辅助函数
impl Woff2TableDirectoryEntry {
    #[cfg(test)]
    pub fn read_base128_test(reader: &mut Reader) -> Result<u32, FontError> {
        Self::read_base128(reader)
    }
}
