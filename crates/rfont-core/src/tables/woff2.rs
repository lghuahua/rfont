use font_macros::ReadBytes;
use rfont_types::{FontError, ReadBytes, Reader, Tag};

/// WOFF2 Header 结构
/// 参考: <https://www.w3.org/TR/WOFF2/#woff20Header>
#[derive(Debug, Clone, ReadBytes)]
pub struct Woff2Header {
    pub signature: u32,             // 0x774F4632 ('wOF2')
    pub flavor: u32,                // Original font signature (0x00010000 for TrueType)
    pub length: u32,                // Total size of WOFF2 file
    pub num_tables: u16,            // Number of tables
    pub reserved: u16, // Reserved (set to 0)  We don't care about these fields of the header
    pub total_sfnt_size: u32, // Uncompressed size of the entire font we don't believe this, will compute later
    pub total_compressed_size: u32, // Compressed size including table directory
    pub major_version: u16,   // Major version of WOFF2 format
    pub minor_version: u16,   // Minor version of WOFF2 format
    pub meta_offset: u32,     // Offset to metadata block (0 if none)
    pub meta_length: u32,     // Length of compressed metadata (0 if none)
    pub meta_orig_length: u32, // Uncompressed length of metadata (0 if none)
    pub priv_offset: u32,     // Offset to private data block (0 if none)
    pub priv_length: u32,     // Length of private data block (0 if none)
}

impl Woff2Header {
    pub fn validate(&self) -> Result<(), FontError> {
        if self.signature != 0x774F4632 {
            // "wOF2"
            return Err(FontError::InvalidMagicNumber {
                expected: 0x774F4632,
                actual: self.signature,
            });
        }

        if self.num_tables == 0 {
            return Err(FontError::Generic("WOFF2: No tables in font".to_string()));
        }

        Ok(())
    }

    pub fn to_ttf_offset_table(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(10);
        data.extend_from_slice(&self.flavor.to_be_bytes());
        data.extend_from_slice(&self.num_tables.to_be_bytes());

        // 计算 searchRange, entrySelector, rangeShift
        let num_tables = self.num_tables as u32;
        let max_pow2: u32 = if num_tables > 0 {
            1 << (31 - num_tables.leading_zeros())
        } else {
            1
        };
        let search_range = max_pow2 * 16;
        let entry_selector = max_pow2.trailing_zeros() as u16;
        let range_shift = (num_tables * 16).saturating_sub(search_range) as u16;

        data.extend_from_slice(&(search_range as u16).to_be_bytes());
        data.extend_from_slice(&entry_selector.to_be_bytes());
        data.extend_from_slice(&range_shift.to_be_bytes());
        data
    }
}

/// WOFF2 表目录项
/// 注意：WOFF2 的表目录格式与 WOFF 不同，使用变长编码
#[derive(Debug, Clone)]
pub struct Woff2TableDirectoryEntry {
    pub flags: u8,
    pub tag: Tag,                      // None 表示使用预定义标签
    pub orig_length: u32,              // 原始长度（未压缩）
    pub transform_length: Option<u32>, // 转换后的长度（如果应用了转换）
}

impl Woff2TableDirectoryEntry {
    /// 从 reader 中读取一个表目录项
    pub fn read_from(reader: &mut Reader, known_tags: &[Tag]) -> Result<Self, FontError> {
        let flags = reader.read_u8()?;

        // 提取 table type (低 6 bits)
        // flags 结构: [transform_version(2 bits, high) | table_type(6 bits, low)]
        // bits 7-6: transform_version
        // bits 5-0: table_type (index)
        let table_type = flags & 0x3F;

        let tag = if table_type < 63 {
            // 使用预定义标签
            if table_type as usize >= known_tags.len() {
                return Err(FontError::Generic(format!(
                    "WOFF2: Invalid predefined tag index: {}",
                    table_type
                )));
            }
            known_tags[table_type as usize]
        } else {
            // 读取 4 字节自定义标签
            Tag::read_from(reader)?
        };

        // 读取原始长度（使用 Base128 编码）
        let orig_length = reader.read_base128()?;

        // 检查是否有转换长度（glyf 和 loca 表）
        // 只有当 transform_version != 3 时才需要读取 transformLength
        let transform_version = (flags >> 6) & 0x03;
        let transform_length =
            if (tag.as_str() == "glyf" || tag.as_str() == "loca") && transform_version != 3 {
                // glyf 和 loca 表有转换，且不是 null transform
                Some(reader.read_base128()?)
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
}

/// WOFF2 预定义标签列表（按规范顺序）
pub const WOFF2_KNOWN_TAGS: [Tag; 63] = [
    Tag(*b"cmap"),
    Tag(*b"head"),
    Tag(*b"hhea"),
    Tag(*b"hmtx"),
    Tag(*b"maxp"),
    Tag(*b"name"),
    Tag(*b"OS/2"),
    Tag(*b"post"),
    Tag(*b"cvt "),
    Tag(*b"fpgm"),
    Tag(*b"glyf"),
    Tag(*b"loca"),
    Tag(*b"prep"),
    Tag(*b"CFF "),
    Tag(*b"VORG"),
    Tag(*b"EBDT"),
    Tag(*b"EBLC"),
    Tag(*b"gasp"),
    Tag(*b"hdmx"),
    Tag(*b"kern"),
    Tag(*b"LTSH"),
    Tag(*b"PCLT"),
    Tag(*b"VDMX"),
    Tag(*b"vhea"),
    Tag(*b"vmtx"),
    Tag(*b"BASE"),
    Tag(*b"GDEF"),
    Tag(*b"GPOS"),
    Tag(*b"GSUB"),
    Tag(*b"EBSC"),
    Tag(*b"JSTF"),
    Tag(*b"MATH"),
    Tag(*b"CBDT"),
    Tag(*b"CBLC"),
    Tag(*b"COLR"),
    Tag(*b"CPAL"),
    Tag(*b"SVG "),
    Tag(*b"sbix"),
    Tag(*b"acnt"),
    Tag(*b"avar"),
    Tag(*b"bdat"),
    Tag(*b"bloc"),
    Tag(*b"bsln"),
    Tag(*b"cvar"),
    Tag(*b"fdsc"),
    Tag(*b"feat"),
    Tag(*b"fmtx"),
    Tag(*b"fvar"),
    Tag(*b"gvar"),
    Tag(*b"hsty"),
    Tag(*b"just"),
    Tag(*b"lcar"),
    Tag(*b"mort"),
    Tag(*b"morx"),
    Tag(*b"opbd"),
    Tag(*b"prop"),
    Tag(*b"trak"),
    Tag(*b"Zapf"),
    Tag(*b"Silf"),
    Tag(*b"Glat"),
    Tag(*b"Gloc"),
    Tag(*b"Feat"),
    Tag(*b"Sill"),
];

pub fn read_table_directory(
    reader: &mut Reader,
    num_tables: u16,
) -> Result<Vec<Woff2TableDirectoryEntry>, FontError> {
    let mut table_directory = Vec::with_capacity(num_tables as usize);
    for _ in 0..num_tables {
        let entry = Woff2TableDirectoryEntry::read_from(reader, &WOFF2_KNOWN_TAGS)?;
        table_directory.push(entry);
    }
    Ok(table_directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::Reader;

    #[test]
    fn test_woff2_header_read() {
        // 测试 WOFF2 Header 读取（完整的 48 字节）
        let data = vec![
            // signature: 'wOF2' (0x774F4632)
            0x77, 0x4F, 0x46, 0x32, // flavor: TrueType (0x00010000)
            0x00, 0x01, 0x00, 0x00, // length: 1000 bytes
            0x00, 0x00, 0x03, 0xE8, // num_tables: 5
            0x00, 0x05, // reserved: 0
            0x00, 0x00, // total_sfnt_size: 2000 bytes
            0x00, 0x00, 0x07, 0xD0, // total_compressed_size: 800 bytes
            0x00, 0x00, 0x03, 0x20, // major_version: 1
            0x00, 0x01, // minor_version: 0
            0x00, 0x00, // meta_offset: 0
            0x00, 0x00, 0x00, 0x00, // meta_length: 0
            0x00, 0x00, 0x00, 0x00, // meta_orig_length: 0
            0x00, 0x00, 0x00, 0x00, // priv_offset: 0
            0x00, 0x00, 0x00, 0x00, // priv_length: 0
            0x00, 0x00, 0x00, 0x00,
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
        // 测试有效的 WOFF2 Header（完整的 48 字节）
        let valid_data = vec![
            0x77, 0x4F, 0x46, 0x32, // signature
            0x00, 0x01, 0x00, 0x00, // flavor
            0x00, 0x00, 0x03, 0xE8, // length
            0x00, 0x05, // num_tables
            0x00, 0x00, // reserved
            0x00, 0x00, 0x07, 0xD0, // total_sfnt_size
            0x00, 0x00, 0x03, 0x20, // total_compressed_size
            0x00, 0x01, // major_version
            0x00, 0x00, // minor_version
            0x00, 0x00, 0x00, 0x00, // meta_offset
            0x00, 0x00, 0x00, 0x00, // meta_length
            0x00, 0x00, 0x00, 0x00, // meta_orig_length
            0x00, 0x00, 0x00, 0x00, // priv_offset
            0x00, 0x00, 0x00, 0x00, // priv_length
        ];

        let mut reader = Reader::new(&valid_data);
        let header = Woff2Header::read_from(&mut reader).unwrap();
        assert!(header.validate().is_ok());

        // 测试无效的签名
        let invalid_data = vec![
            0x77, 0x4F, 0x46, 0x46, // 'wOFF' instead of 'wOF2'
            0x00, 0x01, 0x00, 0x00, // flavor
            0x00, 0x00, 0x03, 0xE8, // length
            0x00, 0x05, // num_tables
            0x00, 0x00, // reserved
            0x00, 0x00, 0x07, 0xD0, // total_sfnt_size
            0x00, 0x00, 0x03, 0x20, // total_compressed_size
            0x00, 0x01, // major_version
            0x00, 0x00, // minor_version
            0x00, 0x00, 0x00, 0x00, // meta_offset
            0x00, 0x00, 0x00, 0x00, // meta_length
            0x00, 0x00, 0x00, 0x00, // meta_orig_length
            0x00, 0x00, 0x00, 0x00, // priv_offset
            0x00, 0x00, 0x00, 0x00, // priv_length
        ];

        let mut reader2 = Reader::new(&invalid_data);
        let header2 = Woff2Header::read_from(&mut reader2).unwrap();
        assert!(header2.validate().is_err());
    }

    #[test]
    fn test_woff2_known_tags() {
        // 验证预定义标签数量
        assert_eq!(WOFF2_KNOWN_TAGS.len(), 63);

        // 验证一些常见标签
        assert_eq!(WOFF2_KNOWN_TAGS[1].as_str(), "head");
        assert_eq!(WOFF2_KNOWN_TAGS[10].as_str(), "glyf");
        assert_eq!(WOFF2_KNOWN_TAGS[11].as_str(), "loca");
        assert_eq!(WOFF2_KNOWN_TAGS[2].as_str(), "hhea");
        assert_eq!(WOFF2_KNOWN_TAGS[3].as_str(), "hmtx");
    }

    #[test]
    fn test_woff2_table_directory_predefined_tag() {
        // 测试使用预定义标签的表目录项
        // hhea 在 WOFF2_KNOWN_TAGS 中的索引是 2
        // flags: transform_version = 0 (高 2 bits), table_type = 2 (低 6 bits)
        // flags = 0b00_000010 = 0x02
        let data = vec![
            0x02, // flags: table_type = 2 (hhea), transform_version = 0
            0x81, 0x00, // orig_length = 128 (Base128: 0x81 0x00)
        ];

        let mut reader = Reader::new(&data);
        let entry = Woff2TableDirectoryEntry::read_from(&mut reader, &WOFF2_KNOWN_TAGS).unwrap();

        assert_eq!(entry.tag.as_str(), "hhea");
        assert_eq!(entry.orig_length, 128);
        assert_eq!(entry.transform_length, None);
    }

    #[test]
    fn test_woff2_table_directory_custom_tag() {
        // 测试使用自定义标签的表目录项（table_type >= 63）
        // flags: table_type = 63 << 2 = 0xFC, + 0x03 (custom tag flag) = 0xFF
        let data = vec![
            0xFF, // flags: table_type = 63 (custom tag)
            b'c', b'u', b's', b't', // custom tag = 'cust'
            0x40, // orig_length = 64 (single byte Base128)
        ];

        let mut reader = Reader::new(&data);
        let entry = Woff2TableDirectoryEntry::read_from(&mut reader, &WOFF2_KNOWN_TAGS).unwrap();

        assert_eq!(entry.tag.as_str(), "cust");
        assert_eq!(entry.orig_length, 64);
    }

    #[test]
    fn test_woff2_table_directory_glyf_with_transform() {
        // glyf 表应该有转换长度
        // glyf 在 WOFF2_KNOWN_TAGS 中的索引是 10
        // flags: table_type = 10 (低 6 bits), transform_version = 0 (高 2 bits)
        // flags = 0b00_001010 = 0x0A
        let data = vec![
            0x0A, // flags: table_type = 10 (glyf), transform_version = 0
            0x84, 0x00, // orig_length = 512 (Base128: 0x84 0x00)
            0x82, 0x00, // transform_length = 256 (Base128: 0x82 0x00)
        ];

        let mut reader = Reader::new(&data);
        let entry = Woff2TableDirectoryEntry::read_from(&mut reader, &WOFF2_KNOWN_TAGS).unwrap();

        assert_eq!(entry.tag.as_str(), "glyf");
        assert_eq!(entry.orig_length, 512);
        assert_eq!(entry.transform_length, Some(256));
    }

    #[test]
    fn test_base128_write() {
        use rfont_types::Writer;

        // 测试 Base128 写入
        // 值 0
        let mut writer = Writer::new();
        writer.write_base128(0).unwrap();
        assert_eq!(writer.data, vec![0x00]);

        // 值 127 (单字节)
        let mut writer = Writer::new();
        writer.write_base128(127).unwrap();
        assert_eq!(writer.data, vec![0x7F]);

        // 值 128 (双字节)
        let mut writer = Writer::new();
        writer.write_base128(128).unwrap();
        assert_eq!(writer.data, vec![0x81, 0x00]);

        // 值 300 (双字节)
        let mut writer = Writer::new();
        writer.write_base128(300).unwrap();
        assert_eq!(writer.data, vec![0x82, 0x2C]);
    }
}
