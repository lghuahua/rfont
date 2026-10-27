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

/// Macintosh Roman (MacRoman) assignments for bytes 0x80..=0xFF.
const MAC_ROMAN_HIGH: [char; 128] = [
    'Ä', 'Å', 'Ç', 'É', 'Ñ', 'Ö', 'Ü', 'á', 'à', 'â', 'ä', 'ã', 'å', 'ç', 'é', 'è', 'ê', 'ë', 'í',
    'ì', 'î', 'ï', 'ñ', 'ó', 'ò', 'ô', 'ö', 'õ', 'ú', 'ù', 'û', 'ü', '†', '°', '¢', '£', '§', '•',
    '¶', 'ß', '®', '©', '™', '´', '¨', '≠', 'Æ', 'Ø', '∞', '±', '≤', '≥', '¥', 'µ', '∂', '∑', '∏',
    'π', '∫', 'ª', 'º', 'Ω', 'æ', 'ø', '¿', '¡', '¬', '√', 'ƒ', '≈', '∆', '«', '»', '…',
    '\u{00A0}', 'À', 'Ã', 'Õ', 'Œ', 'œ', '–', '—', '“', '”', '‘', '’', '÷', '◊', 'ÿ', 'Ÿ', '⁄',
    '€', '‹', '›', 'ﬁ', 'ﬂ', '‡', '·', '‚', '„', '‰', 'Â', 'Ê', 'Á', 'Ë', 'È', 'Í', 'Î', 'Ï', 'Ì',
    'Ó', 'Ô', '\u{F8FF}', 'Ò', 'Ú', 'Û', 'Ù', 'ı', 'ˆ', '˜', '¯', '˘', '˙', '˚', '¸', '˝', '˛',
    'ˇ',
];

fn platform_rank(platform_id: u16) -> u8 {
    match platform_id {
        3 => 0,
        0 => 1,
        1 => 2,
        _ => 3,
    }
}

fn language_rank(platform_id: u16, language_id: u16) -> u8 {
    match platform_id {
        3 if language_id == 0x0409 => 0,
        1 if language_id == 0 => 0,
        _ => 1,
    }
}

/// Human-readable label for a name ID.
pub fn name_id_label(name_id: u16) -> String {
    match name_id {
        0 => "copyright".to_string(),
        1 => "family".to_string(),
        2 => "subfamily".to_string(),
        3 => "unique_id".to_string(),
        4 => "full_name".to_string(),
        5 => "version".to_string(),
        6 => "postscript_name".to_string(),
        7 => "trademark".to_string(),
        8 => "manufacturer".to_string(),
        9 => "designer".to_string(),
        10 => "description".to_string(),
        11 => "vendor_url".to_string(),
        12 => "designer_url".to_string(),
        13 => "license_description".to_string(),
        14 => "license_url".to_string(),
        16 => "typographic_family".to_string(),
        17 => "typographic_subfamily".to_string(),
        18 => "compatible_full".to_string(),
        19 => "sample_text".to_string(),
        20 => "postscript_cid".to_string(),
        21 => "wws_family".to_string(),
        22 => "wws_subfamily".to_string(),
        other => format!("name_{}", other),
    }
}

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

    /// Find the best record for a name ID.
    ///
    /// Prefers Windows Unicode (en-US first), then Unicode, then Macintosh.
    pub fn preferred_record(&self, name_id: u16) -> Option<&NameRecord> {
        self.inner
            .records
            .iter()
            .filter(|r| r.name_id == name_id && r.length > 0)
            .min_by_key(|r| {
                (
                    platform_rank(r.platform_id),
                    language_rank(r.platform_id, r.language_id),
                )
            })
    }

    fn name_bytes(&self, name_id: u16) -> Option<&[u8]> {
        self.preferred_record(name_id)
            .and_then(|r| self.get_string(r))
    }

    /// Get a decoded name string by name ID, using the preferred record.
    pub fn get_name_string(&self, name_id: u16) -> Option<String> {
        self.preferred_record(name_id)
            .and_then(|r| self.decode_string(r))
    }

    /// Get the raw font family name (name ID 1).
    pub fn get_family_name(&self) -> Option<&[u8]> {
        self.name_bytes(1)
    }

    /// Get the raw PostScript name (name ID 6).
    pub fn get_postscript_name(&self) -> Option<&[u8]> {
        self.name_bytes(6)
    }

    /// Get the raw font subfamily name (name ID 2).
    pub fn get_subfamily_name(&self) -> Option<&[u8]> {
        self.name_bytes(2)
    }

    /// Get the raw full font name (name ID 4).
    pub fn get_full_name(&self) -> Option<&[u8]> {
        self.name_bytes(4)
    }

    /// Get the raw version string (name ID 5).
    pub fn get_version(&self) -> Option<&[u8]> {
        self.name_bytes(5)
    }

    /// Get the raw trademark notice (name ID 7).
    pub fn get_trademark(&self) -> Option<&[u8]> {
        self.name_bytes(7)
    }

    /// Get the raw manufacturer name (name ID 8).
    pub fn get_manufacturer(&self) -> Option<&[u8]> {
        self.name_bytes(8)
    }

    /// Get the raw designer name (name ID 9).
    pub fn get_designer(&self) -> Option<&[u8]> {
        self.name_bytes(9)
    }

    /// Get the raw description (name ID 10).
    pub fn get_description(&self) -> Option<&[u8]> {
        self.name_bytes(10)
    }

    /// Get the raw preferred family name (name ID 16).
    pub fn get_preferred_family(&self) -> Option<&[u8]> {
        self.name_bytes(16)
    }

    /// Get the raw preferred subfamily name (name ID 17).
    pub fn get_preferred_subfamily(&self) -> Option<&[u8]> {
        self.name_bytes(17)
    }

    /// Get the raw compatible full name (name ID 18).
    pub fn get_compatible_full(&self) -> Option<&[u8]> {
        self.name_bytes(18)
    }

    /// Decode UTF-16 BE string data, stopping at the first NUL and
    /// handling surrogate pairs.
    pub fn decode_utf16_be(data: &[u8]) -> Option<String> {
        let (pairs, _) = data.as_chunks::<2>();
        let units: Vec<u16> = pairs
            .iter()
            .map(|pair| u16::from_be_bytes(*pair))
            .collect();
        let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());

        let text: String = char::decode_utf16(units[..end].iter().copied())
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();

        if text.is_empty() { None } else { Some(text) }
    }

    /// Decode Macintosh Roman string data.
    pub fn decode_mac_roman(data: &[u8]) -> Option<String> {
        let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        let data = &data[..end];
        if data.is_empty() {
            return None;
        }

        let text: String = data
            .iter()
            .map(|&b| {
                if b < 0x80 {
                    b as char
                } else {
                    MAC_ROMAN_HIGH[(b - 0x80) as usize]
                }
            })
            .collect();
        Some(text)
    }

    /// Try to decode a string as UTF-8.
    pub fn decode_utf8(data: &[u8]) -> Option<String> {
        let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        let data = &data[..end];
        if data.is_empty() {
            return None;
        }
        std::str::from_utf8(data).ok().map(|s| s.to_string())
    }

    /// Try to decode a string using the appropriate encoding based on platform and encoding ID.
    pub fn decode_string(&self, record: &NameRecord) -> Option<String> {
        let data = self.get_string(record)?;

        match PlatformId::from(record.platform_id) {
            PlatformId::Unicode => Self::decode_utf16_be(data),
            PlatformId::Windows => match record.encoding_id {
                0 | 1 | 10 => Self::decode_utf16_be(data).or_else(|| Self::decode_utf8(data)),
                _ => Self::decode_utf8(data).or_else(|| Self::decode_utf16_be(data)),
            },
            PlatformId::Macintosh => match record.encoding_id {
                0 => Self::decode_mac_roman(data),
                _ => Self::decode_utf8(data).or_else(|| Self::decode_mac_roman(data)),
            },
            _ => Self::decode_utf8(data).or_else(|| Self::decode_utf16_be(data)),
        }
    }

    /// Get the family name as a decoded string, preferring the typographic family.
    pub fn get_family_name_str(&self) -> Option<String> {
        self.get_name_string(16).or_else(|| self.get_name_string(1))
    }

    /// Get the subfamily (style) name as a decoded string, preferring the typographic subfamily.
    pub fn get_subfamily_name_str(&self) -> Option<String> {
        self.get_name_string(17).or_else(|| self.get_name_string(2))
    }

    /// Get the full name as a decoded string.
    pub fn get_full_name_str(&self) -> Option<String> {
        self.get_name_string(4)
    }

    /// Get the PostScript name as a decoded string.
    pub fn get_postscript_name_str(&self) -> Option<String> {
        self.get_name_string(6)
    }

    /// Get the version string as a decoded string.
    pub fn get_version_str(&self) -> Option<String> {
        self.get_name_string(5)
    }

    /// Get the trademark notice as a decoded string.
    pub fn get_trademark_str(&self) -> Option<String> {
        self.get_name_string(7)
    }

    /// Get the manufacturer name as a decoded string.
    pub fn get_manufacturer_str(&self) -> Option<String> {
        self.get_name_string(8)
    }

    /// Get the designer name as a decoded string.
    pub fn get_designer_str(&self) -> Option<String> {
        self.get_name_string(9)
    }

    /// Get the description as a decoded string.
    pub fn get_description_str(&self) -> Option<String> {
        self.get_name_string(10)
    }

    /// Get the license description as a decoded string.
    pub fn get_license_str(&self) -> Option<String> {
        self.get_name_string(13)
    }

    /// Print all name table entries in a readable format.
    pub fn print_info(&self) {
        println!("Name Table Information:");
        println!("======================");
        println!("Version: {:?}", self.version());
        println!("Record Count: {}", self.count());
        println!();

        for record in &self.inner.records {
            if let Some(decoded) = self.decode_string(record) {
                println!(
                    "{} [{}/{}/0x{:04X}]: {}",
                    name_id_label(record.name_id),
                    record.platform_id,
                    record.encoding_id,
                    record.language_id,
                    decoded
                );
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
    fn test_name_table_decode_utf16_be_surrogate_pair() {
        // U+1F600 GRINNING FACE = D83D DE00
        let data = &[0xD8, 0x3D, 0xDE, 0x00];
        assert_eq!(NameTable::decode_utf16_be(data).unwrap(), "😀");
    }

    #[test]
    fn test_name_table_decode_utf16_be_stops_at_nul() {
        let data = &[0x00, 0x41, 0x00, 0x00, 0x00, 0x42];
        assert_eq!(NameTable::decode_utf16_be(data).unwrap(), "A");
    }

    #[test]
    fn test_name_table_decode_mac_roman() {
        let data = &[0x8E, 0x80, 0xA5];
        assert_eq!(NameTable::decode_mac_roman(data).unwrap(), "éÄ•");
    }

    #[test]
    fn test_name_table_prefers_windows_english() {
        fn record(platform_id: u16, encoding_id: u16, language_id: u16, offset: u16) -> NameRecord {
            NameRecord {
                platform_id,
                encoding_id,
                language_id,
                name_id: 1,
                length: 4,
                offset,
            }
        }

        let name = Name {
            version: NameVersion::V0,
            count: 4,
            string_offset: 54,
            records: vec![
                record(1, 0, 0, 0),
                record(0, 3, 0, 4),
                record(3, 1, 0x0804, 8),
                record(3, 1, 0x0409, 12),
            ],
            lang_tag_records: Vec::new(),
            strings: b"MAC UNI ZH  EN  ".to_vec(),
        };

        let table = NameTable::new(name);
        let record = table.preferred_record(1).unwrap();
        assert_eq!(record.language_id, 0x0409);
        assert_eq!(table.get_string(record).unwrap(), b"EN  ");
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
            lang_tag_records: Vec::new(),
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
            lang_tag_records: Vec::new(),
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
