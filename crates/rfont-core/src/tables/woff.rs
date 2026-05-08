use font_macros::ReadBytes;
use rfont_types::{FontError, ReadBytes, Reader, Tag};

#[derive(Debug, Clone, ReadBytes)]
pub struct WoffHeader {
    pub signature: u32,        // 0x774F4646 ('wOFF')
    pub flavor: u32, // Original font signature (0x00010000 for TrueType, 0x4F54544F for CFF)
    pub length: u32, // Total size of WOFF file
    pub num_tables: u16, // Number of tables
    pub reserved: u16, // Reserved (set to 0)
    pub total_sfnt_size: u32, // Original SFNT size
    pub major_version: u16, // Major version
    pub minor_version: u16, // Minor version
    pub meta_offset: u32, // Metadata offset (0 if no metadata)
    pub meta_comp_length: u32, // Compressed metadata length
    pub meta_orig_length: u32, // Original metadata length
    pub priv_offset: u32, // Private data offset (0 if no private data)
    pub priv_length: u32, // Private data length
}

impl WoffHeader {
    pub fn validate(&self) -> Result<(), FontError> {
        if self.signature != 0x774F4646 {
            // "wOFF"
            return Err(FontError::InvalidMagicNumber {
                expected: 0x774F4646,
                actual: self.signature,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WoffTableDirectoryEntry {
    pub tag: Tag,
    pub offset: u32,
    pub comp_length: u32,
    pub orig_length: u32,
    pub checksum: u32,
}

impl<'a> ReadBytes<'a> for WoffTableDirectoryEntry {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let tag_bytes = [
            reader.read_u8()?,
            reader.read_u8()?,
            reader.read_u8()?,
            reader.read_u8()?,
        ];
        Ok(WoffTableDirectoryEntry {
            tag: Tag(tag_bytes),
            offset: reader.read_u32()?,
            comp_length: reader.read_u32()?,
            orig_length: reader.read_u32()?,
            checksum: reader.read_u32()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::io::Reader;

    #[test]
    fn test_woff_header_valid() {
        let data = vec![
            0x77, 0x4F, 0x46, 0x46, // signature = 'wOFF'
            0x00, 0x01, 0x00, 0x00, // flavor = TrueType
            0x00, 0x00, 0x10, 0x00, // length = 4096
            0x00, 0x05, // num_tables = 5
            0x00, 0x00, // reserved = 0
            0x00, 0x00, 0x08, 0x00, // total_sfnt_size = 2048
            0x00, 0x01, // major_version = 1
            0x00, 0x00, // minor_version = 0
            0x00, 0x00, 0x00, 0x00, // meta_offset = 0
            0x00, 0x00, 0x00, 0x00, // meta_comp_length = 0
            0x00, 0x00, 0x00, 0x00, // meta_orig_length = 0
            0x00, 0x00, 0x00, 0x00, // priv_offset = 0
            0x00, 0x00, 0x00, 0x00, // priv_length = 0
        ];
        let mut reader = Reader::new(&data);
        let header = WoffHeader::read_from(&mut reader).unwrap();

        assert_eq!(header.signature, 0x774F4646);
        assert_eq!(header.flavor, 0x00010000);
        assert_eq!(header.num_tables, 5);
        assert_eq!(header.major_version, 1);

        header.validate().unwrap();
    }

    #[test]
    fn test_woff_header_invalid_signature() {
        let data = vec![
            0x00, 0x00, 0x00, 0x00, // invalid signature
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x08, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut reader = Reader::new(&data);
        let header = WoffHeader::read_from(&mut reader).unwrap();

        assert!(header.validate().is_err());
    }

    #[test]
    fn test_woff_table_directory_entry() {
        let data = vec![
            b'h', b'e', b'a', b'd', // tag = 'head'
            0x00, 0x00, 0x00, 0x30, // offset = 48
            0x00, 0x00, 0x00, 0x36, // comp_length = 54
            0x00, 0x00, 0x00, 0x36, // orig_length = 54
            0x12, 0x34, 0x56, 0x78, // checksum
        ];
        let mut reader = Reader::new(&data);
        let entry = WoffTableDirectoryEntry::read_from(&mut reader).unwrap();

        assert_eq!(entry.tag.as_str(), "head");
        assert_eq!(entry.offset, 48);
        assert_eq!(entry.comp_length, 54);
        assert_eq!(entry.orig_length, 54);
        assert_eq!(entry.checksum, 0x12345678);
    }

    #[test]
    fn test_woff_cff_flavor() {
        let data = vec![
            0x77, 0x4F, 0x46, 0x46, // signature = 'wOFF'
            0x4F, 0x54, 0x54, 0x4F, // flavor = 'OTTO' (CFF)
            0x00, 0x00, 0x10, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut reader = Reader::new(&data);
        let header = WoffHeader::read_from(&mut reader).unwrap();

        assert_eq!(header.flavor, 0x4F54544F); // 'OTTO'
        header.validate().unwrap();
    }
}
