//! Name table parsing for OpenType fonts.
//!
//! The name table stores metadata about the font, such as:
//! - Copyright notice
//! - Font family name
//! - Font subfamily name
//! - Unique font identifier
//! - Full font name
//! - Version string
//! - PostScript name
//! - Trademark notice
//! - More...

use rfont_types::{FontError, ReadBytes, Reader, WriteBytes, Writer};

// Re-export name table types for external use
pub use rfont_types::name::{Name, NameId, NameRecord, NameVersion, PlatformId};

/// The name table containing font metadata.
///
/// This is a wrapper around rfont_types::name::Name that provides
/// additional functionality for working with name table entries.
#[derive(Debug, Clone)]
pub struct NameTable {
    inner: Name,
}

impl NameTable {
    /// Create a new NameTable from an inner Name
    pub fn new(name: Name) -> Self {
        NameTable { inner: name }
    }

    /// Get the name table version.
    pub fn version(&self) -> NameVersion {
        self.inner.version
    }

    /// Get the number of name records.
    pub fn count(&self) -> u16 {
        self.inner.count
    }

    /// Get all name records.
    pub fn records(&self) -> &[NameRecord] {
        &self.inner.records
    }

    /// Get the string data for a specific name record.
    pub fn get_string(&self, record: &NameRecord) -> Option<&[u8]> {
        self.inner.get_string(record)
    }

    /// Find all records with a specific name ID.
    pub fn find_by_name_id(&self, name_id: u16) -> Vec<&NameRecord> {
        self.inner.find_by_name_id(name_id)
    }

    /// Find the first record with a specific name ID and platform.
    pub fn find_first(&self, name_id: u16, platform_id: u16) -> Option<&NameRecord> {
        self.inner.find_first(name_id, platform_id)
    }

    /// Get the font family name (name ID 1) from Windows platform.
    pub fn get_family_name(&self) -> Option<&[u8]> {
        self.inner.get_family_name()
    }

    /// Get the PostScript name (name ID 6) from Windows platform.
    pub fn get_postscript_name(&self) -> Option<&[u8]> {
        self.inner.get_postscript_name()
    }

    /// Get the font subfamily name (name ID 2).
    pub fn get_subfamily_name(&self) -> Option<&[u8]> {
        self.find_first(2, 3).and_then(|r| self.get_string(r))
    }

    /// Get the full font name (name ID 4).
    pub fn get_full_name(&self) -> Option<&[u8]> {
        self.find_first(4, 3).and_then(|r| self.get_string(r))
    }

    /// Get the version string (name ID 5).
    pub fn get_version(&self) -> Option<&[u8]> {
        self.find_first(5, 3).and_then(|r| self.get_string(r))
    }

    /// Get the trademark notice (name ID 7).
    pub fn get_trademark(&self) -> Option<&[u8]> {
        self.find_first(7, 3).and_then(|r| self.get_string(r))
    }

    /// Get the manufacturer name (name ID 8).
    pub fn get_manufacturer(&self) -> Option<&[u8]> {
        self.find_first(8, 3).and_then(|r| self.get_string(r))
    }

    /// Get the designer name (name ID 9).
    pub fn get_designer(&self) -> Option<&[u8]> {
        self.find_first(9, 3).and_then(|r| self.get_string(r))
    }

    /// Get the description (name ID 10).
    pub fn get_description(&self) -> Option<&[u8]> {
        self.find_first(10, 3).and_then(|r| self.get_string(r))
    }

    /// Get the preferred family name (name ID 16).
    pub fn get_preferred_family(&self) -> Option<&[u8]> {
        self.find_first(16, 3).and_then(|r| self.get_string(r))
    }

    /// Get the preferred subfamily name (name ID 17).
    pub fn get_preferred_subfamily(&self) -> Option<&[u8]> {
        self.find_first(17, 3).and_then(|r| self.get_string(r))
    }

    /// Get the compatible full name (name ID 18).
    pub fn get_compatible_full(&self) -> Option<&[u8]> {
        self.find_first(18, 3).and_then(|r| self.get_string(r))
    }

    /// Try to decode a string as UTF-16 BE (Windows Unicode).
    pub fn decode_utf16_be(data: &[u8]) -> Option<String> {
        if !data.len().is_multiple_of(2) {
            return None;
        }

        let mut chars = Vec::new();
        let mut i = 0;
        while i + 1 < data.len() {
            let high = data[i];
            let low = data[i + 1];
            let code_point = ((high as u16) << 8) | (low as u16);
            if code_point == 0 {
                break;
            }
            if let Some(c) = char::from_u32(code_point as u32) {
                chars.push(c);
            }
            i += 2;
        }

        if chars.is_empty() {
            None
        } else {
            Some(chars.into_iter().collect())
        }
    }

    /// Try to decode a string as UTF-8.
    pub fn decode_utf8(data: &[u8]) -> Option<String> {
        std::str::from_utf8(data).ok().map(|s| s.to_string())
    }

    /// Try to decode a string using the appropriate encoding based on platform and encoding ID.
    pub fn decode_string(&self, record: &NameRecord) -> Option<String> {
        let data = self.get_string(record)?;

        match PlatformId::from(record.platform_id) {
            PlatformId::Unicode => {
                // Unicode platform uses UTF-16 BE
                Self::decode_utf16_be(data)
            }
            PlatformId::Windows => {
                match record.encoding_id {
                    0 => {
                        // Symbol - not commonly used
                        Self::decode_utf16_be(data)
                    }
                    1 | 10 => {
                        // Unicode BMP or UCS-4
                        Self::decode_utf16_be(data)
                    }
                    _ => {
                        // Other Windows encodings - try UTF-8 as fallback
                        Self::decode_utf8(data)
                    }
                }
            }
            PlatformId::Macintosh => {
                // Macintosh uses legacy encodings - return raw data for now
                Self::decode_utf8(data)
            }
            _ => {
                // Unknown platform - try UTF-8
                Self::decode_utf8(data)
            }
        }
    }

    /// Get the family name as a decoded string.
    pub fn get_family_name_str(&self) -> Option<String> {
        self.get_family_name()
            .and_then(|data| Self::decode_utf16_be(data).or_else(|| Self::decode_utf8(data)))
    }

    /// Get the PostScript name as a decoded string.
    pub fn get_postscript_name_str(&self) -> Option<String> {
        self.get_postscript_name().and_then(Self::decode_utf8)
    }

    /// Print all name table entries in a readable format.
    pub fn print_info(&self) {
        println!("Name Table Information:");
        println!("======================");
        println!("Version: {:?}", self.version());
        println!("Record Count: {}", self.count());
        println!();

        // Define name ID to label mapping
        let name_id_labels = [
            (1, "copyright"),
            (2, "family"),
            (3, "subfamily"),
            (4, "unique_id"),
            (5, "full_name"),
            (6, "version"),
            (7, "postscript_name"),
            (8, "trademark"),
            (9, "manufacturer"),
            (10, "designer"),
            (11, "manufacturer_url"),
            (12, "designer_url"),
            (13, "license_description"),
            (14, "license_url"),
            (16, "preferred_family"),
            (17, "preferred_subfamily"),
            (18, "compatible_full"),
            (19, "sample_text"),
            (20, "postscript_cid"),
            (21, "wws_family"),
            (22, "wws_subfamily"),
        ];

        // Print each unique name ID
        for (name_id, label) in name_id_labels.iter() {
            let records = self.find_by_name_id(*name_id);
            if !records.is_empty() {
                // Use the first record (preferably Windows platform)
                for record in &records {
                    if let Some(decoded) = self.decode_string(record) {
                        println!("{}: {}", label, decoded);
                    }
                }
            }
        }
    }
}

impl<'a> ReadBytes<'a> for NameTable {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let name = Name::read_from(reader)?;
        Ok(NameTable::new(name))
    }
}

impl WriteBytes for NameTable {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        self.inner.write_to(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::io::Reader;

    #[test]
    fn test_name_table_read() {
        // Create a minimal name table with one record
        // Header: 6 bytes (version + count + stringOffset)
        // Record: 12 bytes
        // String offset from start of string data = 0
        // Total string_offset = 6 + 12 = 18
        let mut data = vec![
            0x00, 0x00, // version = 0
            0x00, 0x01, // count = 1
            0x00, 0x12, // string_offset = 18 (6 + 12)
        ];

        // Name record (12 bytes)
        data.extend_from_slice(&[
            0x00, 0x03, // platform_id = 3 (Windows)
            0x00, 0x01, // encoding_id = 1 (Unicode BMP)
            0x04, 0x09, // language_id = 0x0409 (US English)
            0x00, 0x01, // name_id = 1 (Family name)
            0x00, 0x08, // length = 8
            0x00, 0x00, // offset = 0
        ]);

        // String data: "TestFont" in UTF-16 BE (16 bytes)
        data.extend_from_slice(&[
            0x00, 0x54, // 'T'
            0x00, 0x65, // 'e'
            0x00, 0x73, // 's'
            0x00, 0x74, // 't'
            0x00, 0x46, // 'F'
            0x00, 0x6F, // 'o'
            0x00, 0x6E, // 'n'
            0x00, 0x74, // 't'
        ]);

        let mut reader = Reader::new(&data);
        let name_table = NameTable::read_from(&mut reader).unwrap();

        assert_eq!(name_table.version(), NameVersion::V0);
        assert_eq!(name_table.count(), 1);
        assert_eq!(name_table.records().len(), 1);
    }

    #[test]
    fn test_name_table_get_family_name() {
        let mut data = vec![
            0x00, 0x00, // version = 0
            0x00, 0x01, // count = 1
            0x00, 0x12, // string_offset = 18 (6 + 12)
        ];

        // length = 16 (8 characters * 2 bytes each for UTF-16)
        data.extend_from_slice(&[
            0x00, 0x03, 0x00, 0x01, 0x04, 0x09, 0x00, 0x01, 0x00, 0x10, 0x00, 0x00,
        ]);

        // "TestFont" in UTF-16 BE (16 bytes)
        data.extend_from_slice(&[
            0x00, 0x54, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74, 0x00, 0x46, 0x00, 0x6F, 0x00, 0x6E,
            0x00, 0x74,
        ]);

        let mut reader = Reader::new(&data);
        let name_table = NameTable::read_from(&mut reader).unwrap();

        let family_bytes = name_table.get_family_name().unwrap();
        assert_eq!(
            family_bytes,
            &[
                0x00, 0x54, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74, 0x00, 0x46, 0x00, 0x6F, 0x00, 0x6E,
                0x00, 0x74
            ]
        );
    }

    #[test]
    fn test_name_table_decode_utf16_be() {
        let data = &[
            0x00, 0x54, // 'T'
            0x00, 0x65, // 'e'
            0x00, 0x73, // 's'
            0x00, 0x74, // 't'
        ];

        let result = NameTable::decode_utf16_be(data).unwrap();
        assert_eq!(result, "Test");
    }

    #[test]
    fn test_name_table_decode_utf8() {
        let data = b"TestFont";
        let result = NameTable::decode_utf8(data).unwrap();
        assert_eq!(result, "TestFont");
    }

    #[test]
    fn test_name_table_find_by_name_id() {
        let mut data = vec![
            0x00, 0x00, // version = 0
            0x00, 0x02, // count = 2
            0x00, 0x1E, // string_offset = 30 (6 + 2*12)
        ];

        // Record 1: name_id = 1 (Family) - 12 bytes
        data.extend_from_slice(&[
            0x00, 0x03, 0x00, 0x01, 0x04, 0x09, 0x00, 0x01, 0x00, 0x04, 0x00, 0x00,
        ]);

        // Record 2: name_id = 6 (PostScript) - 12 bytes
        data.extend_from_slice(&[
            0x00, 0x03, 0x00, 0x01, 0x04, 0x09, 0x00, 0x06, 0x00, 0x08, 0x00, 0x04,
        ]);

        // String data: "Test" (4 bytes) + "PostSC" (8 bytes) = 12 bytes
        data.extend_from_slice(b"Test");
        data.extend_from_slice(b"PostSC");

        let mut reader = Reader::new(&data);
        let name_table = NameTable::read_from(&mut reader).unwrap();

        let family_records = name_table.find_by_name_id(1);
        assert_eq!(family_records.len(), 1);
        assert_eq!(family_records[0].name_id, 1);

        let ps_records = name_table.find_by_name_id(6);
        assert_eq!(ps_records.len(), 1);
        assert_eq!(ps_records[0].name_id, 6);
    }

    #[test]
    fn test_name_table_multiple_platforms() {
        let mut data = vec![
            0x00, 0x00, // version = 0
            0x00, 0x02, // count = 2
            0x00, 0x1E, // string_offset = 30 (6 + 2*12)
        ];

        // Record 1: Windows platform, name_id = 1 - 12 bytes
        data.extend_from_slice(&[
            0x00, 0x03, // platform_id = 3 (Windows)
            0x00, 0x01, // encoding_id = 1
            0x04, 0x09, // language_id = US English
            0x00, 0x01, // name_id = 1
            0x00, 0x04, // length = 4
            0x00, 0x00, // offset = 0
        ]);

        // Record 2: Unicode platform, name_id = 1 - 12 bytes
        data.extend_from_slice(&[
            0x00, 0x00, // platform_id = 0 (Unicode)
            0x00, 0x00, // encoding_id = 0
            0x00, 0x00, // language_id = 0
            0x00, 0x01, // name_id = 1
            0x00, 0x04, // length = 4
            0x00, 0x04, // offset = 4
        ]);

        // String data: "Win " (4 bytes) + "Uni " (4 bytes) = 8 bytes
        data.extend_from_slice(b"Win ");
        data.extend_from_slice(b"Uni ");

        let mut reader = Reader::new(&data);
        let name_table = NameTable::read_from(&mut reader).unwrap();

        // Find Windows family name
        let win_record = name_table.find_first(1, 3);
        assert!(win_record.is_some());
        assert_eq!(win_record.unwrap().platform_id, 3);

        // Find Unicode family name
        let uni_record = name_table.find_first(1, 0);
        assert!(uni_record.is_some());
        assert_eq!(uni_record.unwrap().platform_id, 0);
    }

    #[test]
    fn test_name_table_write() {
        use rfont_types::io::Writer;

        let name = rfont_types::name::Name {
            version: NameVersion::V0,
            count: 1,
            string_offset: 12,
            records: vec![NameRecord {
                platform_id: 3,
                encoding_id: 1,
                language_id: 0x0409,
                name_id: 1,
                length: 4,
                offset: 0,
            }],
            strings: b"Test".to_vec(),
        };

        let name_table = NameTable::new(name);

        let mut writer = Writer::new();
        name_table.write_to(&mut writer).unwrap();

        // Should be 16 bytes: 6 (header) + 12 (record) + 4 (string)
        assert_eq!(writer.data.len(), 22);
    }

    #[test]
    fn test_name_table_roundtrip() {
        use rfont_types::io::Writer;

        let name = rfont_types::name::Name {
            version: NameVersion::V0,
            count: 2,
            string_offset: 30,
            records: vec![
                NameRecord {
                    platform_id: 3,
                    encoding_id: 1,
                    language_id: 0x0409,
                    name_id: 1,
                    length: 5,
                    offset: 0,
                },
                NameRecord {
                    platform_id: 3,
                    encoding_id: 1,
                    language_id: 0x0409,
                    name_id: 6,
                    length: 8,
                    offset: 5,
                },
            ],
            strings: b"FamilyPostSC".to_vec(),
        };

        let original = NameTable::new(name);

        let mut writer = Writer::new();
        original.write_to(&mut writer).unwrap();

        let mut reader = Reader::new(&writer.data);
        let restored = NameTable::read_from(&mut reader).unwrap();

        assert_eq!(restored.version(), original.version());
        assert_eq!(restored.count(), original.count());
        assert_eq!(restored.records().len(), original.records().len());
    }
}
