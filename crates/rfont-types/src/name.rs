//! Name table types for OpenType fonts.
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
//! - Manufacturer name
//! - Designer name
//! - Description
//! - Vendor URL
//! - Designer URL
//! - License description
//! - License info URL
//! - Preferred family name
//! - Preferred subfamily name
//! - Compatible full name
//! - Sample text
//! - PostScript CID findfont name
//! - WOFF2 metadata
//! - Variations PS name prefix

use crate::io::{ReadBytes, Reader, WriteBytes, Writer};
use crate::FontError;

/// Name table version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameVersion {
    /// Version 0 (original)
    V0,
    /// Version 1 (with UTF-8 support)
    V1,
    /// Unknown version
    Unknown(u16),
}

impl From<u16> for NameVersion {
    fn from(v: u16) -> Self {
        match v {
            0 => NameVersion::V0,
            1 => NameVersion::V1,
            _ => NameVersion::Unknown(v),
        }
    }
}

/// Platform ID for name records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformId {
    /// Unicode
    Unicode,
    /// Macintosh
    Macintosh,
    /// Reserved (formerly ASCII)
    Reserved,
    /// Windows
    Windows,
    /// Custom platform
    Custom(u16),
}

impl From<u16> for PlatformId {
    fn from(v: u16) -> Self {
        match v {
            0 => PlatformId::Unicode,
            1 => PlatformId::Macintosh,
            2 => PlatformId::Reserved,
            3 => PlatformId::Windows,
            _ => PlatformId::Custom(v),
        }
    }
}

/// Unicode encoding ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeEncodingId {
    /// Unicode BMP (UECS-2)
    BMP,
    /// Unicode Full Range
    FullRange,
    /// Unicode Variable Fonts
    VariableFonts,
    /// Custom encoding
    Custom(u16),
}

impl From<u16> for UnicodeEncodingId {
    fn from(v: u16) -> Self {
        match v {
            0 => UnicodeEncodingId::BMP,
            1 => UnicodeEncodingId::FullRange,
            2 => UnicodeEncodingId::VariableFonts,
            _ => UnicodeEncodingId::Custom(v),
        }
    }
}

/// Windows encoding ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsEncodingId {
    /// Symbol (legacy)
    Symbol,
    /// Unicode BMP (UTF-16)
    UnicodeBMP,
    /// Shift JIS
    ShiftJIS,
    /// PRC
    PRC,
    /// Big5
    Big5,
    /// Wansung (Korean)
    Wansung,
    /// Johab
    Johab,
    /// Unicode UCS-4
    UnicodeUCS4,
    /// Custom encoding
    Custom(u16),
}

impl From<u16> for WindowsEncodingId {
    fn from(v: u16) -> Self {
        match v {
            0 => WindowsEncodingId::Symbol,
            1 => WindowsEncodingId::UnicodeBMP,
            2 => WindowsEncodingId::ShiftJIS,
            3 => WindowsEncodingId::PRC,
            4 => WindowsEncodingId::Big5,
            5 => WindowsEncodingId::Wansung,
            6 => WindowsEncodingId::Johab,
            10 => WindowsEncodingId::UnicodeUCS4,
            _ => WindowsEncodingId::Custom(v),
        }
    }
}

/// Macintosh encoding ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacEncodingId {
    /// Roman
    Roman,
    /// Japanese
    Japanese,
    /// Traditional Chinese
    TraditionalChinese,
    /// Korean
    Korean,
    /// Arabic
    Arabic,
    /// Hebrew
    Hebrew,
    /// Greek
    Greek,
    /// Cyrillic
    Cyrillic,
    /// Custom encoding
    Custom(u16),
}

impl From<u16> for MacEncodingId {
    fn from(v: u16) -> Self {
        match v {
            0 => MacEncodingId::Roman,
            1 => MacEncodingId::Japanese,
            2 => MacEncodingId::TraditionalChinese,
            3 => MacEncodingId::Korean,
            4 => MacEncodingId::Arabic,
            5 => MacEncodingId::Hebrew,
            6 => MacEncodingId::Greek,
            7 => MacEncodingId::Cyrillic,
            _ => MacEncodingId::Custom(v),
        }
    }
}

/// Name ID - identifies the type of name string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameId {
    /// Copyright notice
    Copyright,
    /// Font family name
    Family,
    /// Font subfamily name
    Subfamily,
    /// Unique font identifier
    UniqueId,
    /// Full font name
    FullName,
    /// Version string
    Version,
    /// PostScript name
    PostScriptName,
    /// Trademark notice
    Trademark,
    /// Manufacturer name
    Manufacturer,
    /// Designer name
    Designer,
    /// Description
    Description,
    /// Vendor URL
    VendorUrl,
    /// Designer URL
    DesignerUrl,
    /// License description
    LicenseDescription,
    /// License info URL
    LicenseUrl,
    /// Reserved (must be 0)
    Reserved1,
    /// Preferred family name
    PreferredFamily,
    /// Preferred subfamily name
    PreferredSubfamily,
    /// Compatible full name
    CompatibleFull,
    /// Sample text
    SampleText,
    /// PostScript CID findfont name
    PostScriptCID,
    /// WOFF2 metadata
    Woff2Metadata,
    /// Variations PS name prefix
    VariationsPSNamePrefix,
    /// Custom name ID
    Custom(u16),
}

impl From<u16> for NameId {
    fn from(v: u16) -> Self {
        match v {
            0 => NameId::Copyright,
            1 => NameId::Family,
            2 => NameId::Subfamily,
            3 => NameId::UniqueId,
            4 => NameId::FullName,
            5 => NameId::Version,
            6 => NameId::PostScriptName,
            7 => NameId::Trademark,
            8 => NameId::Manufacturer,
            9 => NameId::Designer,
            10 => NameId::Description,
            11 => NameId::VendorUrl,
            12 => NameId::DesignerUrl,
            13 => NameId::LicenseDescription,
            14 => NameId::LicenseUrl,
            15 => NameId::Reserved1,
            16 => NameId::PreferredFamily,
            17 => NameId::PreferredSubfamily,
            18 => NameId::CompatibleFull,
            19 => NameId::SampleText,
            20 => NameId::PostScriptCID,
            21 => NameId::Woff2Metadata,
            22 => NameId::VariationsPSNamePrefix,
            _ => NameId::Custom(v),
        }
    }
}

impl From<NameId> for u16 {
    fn from(v: NameId) -> Self {
        match v {
            NameId::Copyright => 0,
            NameId::Family => 1,
            NameId::Subfamily => 2,
            NameId::UniqueId => 3,
            NameId::FullName => 4,
            NameId::Version => 5,
            NameId::PostScriptName => 6,
            NameId::Trademark => 7,
            NameId::Manufacturer => 8,
            NameId::Designer => 9,
            NameId::Description => 10,
            NameId::VendorUrl => 11,
            NameId::DesignerUrl => 12,
            NameId::LicenseDescription => 13,
            NameId::LicenseUrl => 14,
            NameId::Reserved1 => 15,
            NameId::PreferredFamily => 16,
            NameId::PreferredSubfamily => 17,
            NameId::CompatibleFull => 18,
            NameId::SampleText => 19,
            NameId::PostScriptCID => 20,
            NameId::Woff2Metadata => 21,
            NameId::VariationsPSNamePrefix => 22,
            NameId::Custom(v) => v,
        }
    }
}

/// A single name record in the name table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRecord {
    pub platform_id: u16,
    pub encoding_id: u16,
    pub language_id: u16,
    pub name_id: u16,
    pub length: u16,
    pub offset: u16,
}

impl ReadBytes<'_> for NameRecord {
    fn read_from(reader: &mut Reader<'_>) -> Result<Self, FontError> {
        Ok(NameRecord {
            platform_id: reader.read_u16()?,
            encoding_id: reader.read_u16()?,
            language_id: reader.read_u16()?,
            name_id: reader.read_u16()?,
            length: reader.read_u16()?,
            offset: reader.read_u16()?,
        })
    }
}

impl WriteBytes for NameRecord {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_u16(self.platform_id)?;
        writer.write_u16(self.encoding_id)?;
        writer.write_u16(self.language_id)?;
        writer.write_u16(self.name_id)?;
        writer.write_u16(self.length)?;
        writer.write_u16(self.offset)
    }
}

/// The name table containing font metadata.
#[derive(Debug, Clone)]
pub struct Name {
    pub version: NameVersion,
    pub count: u16,
    pub string_offset: u16,
    pub records: Vec<NameRecord>,
    pub strings: Vec<u8>,
}

impl Name {
    /// Get the string data for a specific name record.
    pub fn get_string(&self, record: &NameRecord) -> Option<&[u8]> {
        let start = record.offset as usize;
        let end = start + record.length as usize;
        if end <= self.strings.len() {
            Some(&self.strings[start..end])
        } else {
            None
        }
    }

    /// Find all records with a specific name ID.
    pub fn find_by_name_id(&self, name_id: u16) -> Vec<&NameRecord> {
        self.records
            .iter()
            .filter(|r| r.name_id == name_id)
            .collect()
    }

    /// Find the first record with a specific name ID and platform.
    pub fn find_first(&self, name_id: u16, platform_id: u16) -> Option<&NameRecord> {
        self.records
            .iter()
            .find(|r| r.name_id == name_id && r.platform_id == platform_id)
    }

    /// Get the font family name (name ID 1).
    pub fn get_family_name(&self) -> Option<&[u8]> {
        self.find_first(1, 3).and_then(|r| self.get_string(r))
    }

    /// Get the PostScript name (name ID 6).
    pub fn get_postscript_name(&self) -> Option<&[u8]> {
        self.find_first(6, 3).and_then(|r| self.get_string(r))
    }
}

impl<'a> ReadBytes<'a> for Name {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let version = NameVersion::from(reader.read_u16()?);
        let count = reader.read_u16()?;
        let string_offset = reader.read_u16()?;

        // Read all name records
        let mut records = Vec::with_capacity(count as usize);
        for _ in 0..count {
            records.push(NameRecord::read_from(reader)?);
        }

        // Read string data
        let string_start = string_offset as usize;
        if string_start > reader.len() {
            return Err(FontError::InvalidOffset {
                table: "name".to_string(),
                offset: string_offset as u32,
                max: reader.len() as u32,
            });
        }

        let strings = reader.read_bytes(reader.len() - string_start)?.to_vec();

        Ok(Name {
            version,
            count,
            string_offset,
            records,
            strings,
        })
    }
}

impl WriteBytes for Name {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        // Write version
        let version: u16 = match self.version {
            NameVersion::V0 => 0,
            NameVersion::V1 => 1,
            NameVersion::Unknown(v) => v,
        };
        writer.write_u16(version)?;

        // Write count
        writer.write_u16(self.count)?;

        // Calculate string offset (header + records)
        let string_offset = 6 + self.count as usize * 12;
        writer.write_u16(string_offset as u16)?;

        // Write records
        for record in &self.records {
            record.write_to(writer)?;
        }

        // Write string data
        writer.write_bytes(&self.strings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::Reader;

    #[test]
    fn test_name_version_conversion() {
        assert_eq!(NameVersion::from(0), NameVersion::V0);
        assert_eq!(NameVersion::from(1), NameVersion::V1);
        assert_eq!(NameVersion::from(2), NameVersion::Unknown(2));
    }

    #[test]
    fn test_platform_id_conversion() {
        assert_eq!(PlatformId::from(0), PlatformId::Unicode);
        assert_eq!(PlatformId::from(1), PlatformId::Macintosh);
        assert_eq!(PlatformId::from(3), PlatformId::Windows);
        assert_eq!(PlatformId::from(99), PlatformId::Custom(99));
    }

    #[test]
    fn test_name_id_conversion() {
        assert_eq!(NameId::from(1), NameId::Family);
        assert_eq!(NameId::from(6), NameId::PostScriptName);
        assert_eq!(NameId::from(99), NameId::Custom(99));
    }

    #[test]
    fn test_name_id_to_u16() {
        assert_eq!(u16::from(NameId::Family), 1);
        assert_eq!(u16::from(NameId::PostScriptName), 6);
        assert_eq!(u16::from(NameId::Custom(99)), 99);
    }

    #[test]
    fn test_name_record_read() {
        let data = vec![
            0x00, 0x03, // platform_id = 3 (Windows)
            0x00, 0x01, // encoding_id = 1 (Unicode BMP)
            0x04, 0x09, // language_id = 0x0409 (US English)
            0x00, 0x01, // name_id = 1 (Family name)
            0x00, 0x0A, // length = 10
            0x00, 0x20, // offset = 32
        ];

        let mut reader = Reader::new(&data);
        let record = NameRecord::read_from(&mut reader).unwrap();

        assert_eq!(record.platform_id, 3);
        assert_eq!(record.encoding_id, 1);
        assert_eq!(record.language_id, 0x0409);
        assert_eq!(record.name_id, 1);
        assert_eq!(record.length, 10);
        assert_eq!(record.offset, 32);
    }

    #[test]
    fn test_name_record_write() {
        use crate::io::Writer;

        let record = NameRecord {
            platform_id: 3,
            encoding_id: 1,
            language_id: 0x0409,
            name_id: 1,
            length: 10,
            offset: 32,
        };

        let mut writer = Writer::new();
        record.write_to(&mut writer).unwrap();

        assert_eq!(writer.data.len(), 12);
        assert_eq!(&writer.data, &[0x00, 0x03, 0x00, 0x01, 0x04, 0x09, 0x00, 0x01, 0x00, 0x0A, 0x00, 0x20]);
    }

    #[test]
    fn test_name_record_roundtrip() {
        use crate::io::Writer;

        let original = NameRecord {
            platform_id: 3,
            encoding_id: 1,
            language_id: 0x0409,
            name_id: 6,
            length: 20,
            offset: 100,
        };

        let mut writer = Writer::new();
        original.write_to(&mut writer).unwrap();

        let mut reader = Reader::new(&writer.data);
        let restored = NameRecord::read_from(&mut reader).unwrap();

        assert_eq!(original, restored);
    }

    #[test]
    fn test_name_table_read() {
        // Header: 6 bytes, Record: 12 bytes, string_offset = 18
        let mut data = vec![
            0x00, 0x00, // version = 0
            0x00, 0x01, // count = 1
            0x00, 0x12, // string_offset = 18 (6 + 12)
            ];
    
            // Name record (12 bytes)
            data.extend_from_slice(&[
                0x00, 0x03, // platform_id = 3
                0x00, 0x01, // encoding_id = 1
                0x04, 0x09, // language_id = 0x0409
                0x00, 0x01, // name_id = 1
                0x00, 0x04, // length = 4
                0x00, 0x00, // offset = 0
            ]);
    
            // String data: "Test" (4 bytes)
            data.extend_from_slice(b"Test");
    
            let mut reader = Reader::new(&data);
            let name = Name::read_from(&mut reader).unwrap();
    
            assert_eq!(name.version, NameVersion::V0);
            assert_eq!(name.count, 1);
            assert_eq!(name.string_offset, 18);
            assert_eq!(name.records.len(), 1);
            assert_eq!(name.records[0].name_id, 1);
            assert_eq!(name.records[0].length, 4);
        }
    #[test]
    fn test_name_get_string() {
        // Header: 6 bytes, Record: 12 bytes, string_offset = 18
        let mut data = vec![
            0x00, 0x00, // version = 0
            0x00, 0x01, // count = 1
            0x00, 0x12, // string_offset = 18 (6 + 12)
        ];

        data.extend_from_slice(&[
            0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
            0x00, 0x01, 0x00, 0x04, 0x00, 0x00,
        ]);

        data.extend_from_slice(b"Test");

        let mut reader = Reader::new(&data);
        let name = Name::read_from(&mut reader).unwrap();

        let string = name.get_string(&name.records[0]).unwrap();
        assert_eq!(string, b"Test");
    }

    #[test]
    fn test_name_find_by_name_id() {
        // Header: 6 bytes, 2 Records: 24 bytes, string_offset = 30
        let mut data = vec![
            0x00, 0x00, // version = 0
            0x00, 0x02, // count = 2
            0x00, 0x1E, // string_offset = 30 (6 + 2*12)
        ];

        // Record 1: name_id = 1 (Family) - 12 bytes
        data.extend_from_slice(&[
            0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
            0x00, 0x01, 0x00, 0x04, 0x00, 0x00,
        ]);

        // Record 2: name_id = 6 (PostScript) - 12 bytes
        data.extend_from_slice(&[
            0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
            0x00, 0x06, 0x00, 0x08, 0x00, 0x04,
        ]);

        // String data: "Family" (6 bytes) + "PostSC" (6 bytes) = 12 bytes
        data.extend_from_slice(b"Family");
        data.extend_from_slice(b"PostSC");

        let mut reader = Reader::new(&data);
        let name = Name::read_from(&mut reader).unwrap();

        let family_records = name.find_by_name_id(1);
        assert_eq!(family_records.len(), 1);
        assert_eq!(family_records[0].name_id, 1);

        let ps_records = name.find_by_name_id(6);
        assert_eq!(ps_records.len(), 1);
        assert_eq!(ps_records[0].name_id, 6);
    }

    #[test]
    fn test_name_write() {
        use crate::io::Writer;

        let name = Name {
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

        let mut writer = Writer::new();
        name.write_to(&mut writer).unwrap();

        // Should be 16 bytes: 6 (header) + 12 (record) + 4 (string)
        assert_eq!(writer.data.len(), 22);
    }

    #[test]
    fn test_name_roundtrip() {
        use crate::io::Writer;

        let original = Name {
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

        let mut writer = Writer::new();
        original.write_to(&mut writer).unwrap();

        let mut reader = Reader::new(&writer.data);
        let restored = Name::read_from(&mut reader).unwrap();

        assert_eq!(restored.version, original.version);
        assert_eq!(restored.count, original.count);
        assert_eq!(restored.records.len(), original.records.len());
        assert_eq!(restored.strings, original.strings);
    }
}
