use rfont_types::{FontError, Reader, ReadBytes, Tag};
use font_macros::ReadBytes;

#[derive(Debug, Clone, ReadBytes)]
pub struct WoffHeader {
    pub signature: u32,      // 0x774F4646 ('wOFF')
    pub flavor: u32,         // Original font signature (0x00010000 for TrueType, 0x4F54544F for CFF)
    pub length: u32,         // Total size of WOFF file
    pub num_tables: u16,     // Number of tables
    pub reserved: u16,       // Reserved (set to 0)
    pub total_sfnt_size: u32, // Original SFNT size
    pub major_version: u16,   // Major version
    pub minor_version: u16,   // Minor version
    pub meta_offset: u32,    // Metadata offset (0 if no metadata)
    pub meta_comp_length: u32, // Compressed metadata length
    pub meta_orig_length: u32, // Original metadata length
    pub priv_offset: u32,    // Private data offset (0 if no private data)
    pub priv_length: u32,    // Private data length
}

impl WoffHeader {
    pub fn validate(&self) -> Result<(), FontError> {
        if self.signature != 0x774F4646 { // "wOFF"
            return Err(FontError::InvalidMagicNumber { 
                expected: 0x774F4646, 
                actual: self.signature 
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
        let tag_bytes = [reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?];
        Ok(WoffTableDirectoryEntry {
            tag: Tag(tag_bytes),
            offset: reader.read_u32()?,
            comp_length: reader.read_u32()?,
            orig_length: reader.read_u32()?,
            checksum: reader.read_u32()?,
        })
    }
}
