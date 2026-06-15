use crate::io::{ReadBytes, Reader, WriteBytes, Writer};
use crate::FontError;
use font_macros::ReadBytes;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

const MAC_TO_UNIX_OFFSET: i64 = 2082844800;
#[derive(Debug, Clone)]
pub struct TableRecord {
    pub tag: Tag,
    pub checksum: u32,
    pub offset: u32,
    pub length: u32,
}

impl TableRecord {
    pub fn to_be_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.tag.0);
        bytes[4..8].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.offset.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.length.to_be_bytes());
        bytes
    }
}

impl<'a> ReadBytes<'a> for TableRecord {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let tag = Tag::read_from(reader)?;
        Ok(TableRecord {
            tag,
            checksum: reader.read_u32()?,
            offset: reader.read_u32()?,
            length: reader.read_u32()?,
        })
    }
}

impl WriteBytes for TableRecord {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        self.tag.write_to(writer)?;
        writer.write_u32(self.checksum)?;
        writer.write_u32(self.offset)?;
        writer.write_u32(self.length)
    }
}

#[derive(Debug, Clone, ReadBytes)]
pub struct EncodingRecord {
    pub platform_id: u16,
    pub encoding_id: u16,
    pub offset: u32,
}

impl WriteBytes for EncodingRecord {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_u16(self.platform_id)?;
        writer.write_u16(self.encoding_id)?;
        writer.write_u32(self.offset)
    }
}

#[derive(Debug, Clone)]
pub struct LONGDATETIME(i64);

impl LONGDATETIME {
    /// 从 SystemTime 创建
    pub fn from_system_time(time: SystemTime) -> Self {
        let unix_seconds = match time.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(e) => -(e.duration().as_secs() as i64),
        };
        Self(unix_seconds + MAC_TO_UNIX_OFFSET)
    }    
    /// 获取原始秒数
    pub fn as_seconds(self) -> i64 {
        self.0
    }
}

impl<'a> ReadBytes<'a> for LONGDATETIME {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let buf = reader.read_bytes(8)?;
        let seconds = i64::from_be_bytes(buf.try_into().map_err(|_| FontError::InvalidBaseDate)?);

        Ok(LONGDATETIME(seconds))
    }
}

impl WriteBytes for LONGDATETIME {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        // 写入当前时间
        // let now = SystemTime::now();
        // println!("now: {:?}", now);
        // let seconds = LONGDATETIME::from_system_time(now).as_seconds();
  
        writer.write_bytes(&self.0.to_be_bytes())
    }
}

/// 4-byte OpenType tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tag(pub [u8; 4]);

impl Tag {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != 4 {
            return Err("Tag must be exactly 4 bytes");
        }
        let mut arr = [0u8; 4];
        arr.copy_from_slice(bytes);
        Ok(Tag(arr))
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("")
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<'a> ReadBytes<'a> for Tag {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let bytes = [
            reader.read_u8()?,
            reader.read_u8()?,
            reader.read_u8()?,
            reader.read_u8()?,
        ];
        Ok(Tag(bytes))
    }
}

impl WriteBytes for Tag {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_bytes(&self.0)
    }
}

/// 16-bit signed fixed-point number (2.14 format).
#[derive(Debug, Clone, Copy)]
pub struct F2Dot14(pub f32);

impl F2Dot14 {
    pub fn from_u16(v: u16) -> Self {
        F2Dot14((v as i16) as f32 / 16384.0)
    }
}

/// 32-bit signed fixed-point number (16.16 format).
#[derive(Debug, Clone, Copy)]
pub struct Fixed(pub f32);

impl Fixed {
    pub fn from_i32(v: i32) -> Self {
        Fixed(v as f32 / 65536.0)
    }
}

#[derive(Debug, Clone, Copy, ReadBytes)]
pub struct FWord(pub i16);

impl WriteBytes for FWord {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_i16(self.0)
    }
}

#[derive(Debug, Clone, Copy, ReadBytes)]
pub struct UFWord(pub u16);

impl WriteBytes for UFWord {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_u16(self.0)
    }
}

/// 255UInt16: A variable-length unsigned integer used in WOFF2.
/// Encoding (per WOFF2 spec):
/// - 0-252: stored as single byte (value itself)
/// - 253: followed by u8, actual value = 253 + u8 (range: 253-505)
/// - 254: followed by u8, actual value = 508 + u8 (range: 508-760)
/// - 255: followed by u16 (big-endian), actual value = u16 (range: 0-65535)
///
/// This provides efficient encoding for small values while supporting full u16 range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U255(pub u16);

impl<'a> ReadBytes<'a> for U255 {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let first_byte = reader.read_u8()?;
        
        let value = match first_byte {
            253 => {
                // 253 + next byte (0-252)
                let offset = reader.read_u8()?;
                253 + offset as u16
            }
            254 => {
                // 508 + next byte (0-252)
                let offset = reader.read_u8()?;
                508 + offset as u16
            }
            255 => {
                // Full u16 value
                reader.read_u16()? as u16
            }
            0..=252 => first_byte as u16,
        };
        
        Ok(U255(value))
    }
}

impl WriteBytes for U255 {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        let value = self.0;
        
        if value <= 252 {
            // Single byte encoding
            writer.write_u8(value as u8)
        } else if value <= 505 {
            // Two-byte encoding: 253 + offset
            let offset = (value - 253) as u8;
            writer.write_u8(253)?;
            writer.write_u8(offset)
        } else if value <= 760 {
            // Two-byte encoding: 254 + offset
            let offset = (value - 508) as u8;
            writer.write_u8(254)?;
            writer.write_u8(offset)
        } else {
            // Three-byte encoding: 255 + u16
            writer.write_u8(255)?;
            writer.write_u16(value)
        }
    }
}

impl U255 {
    /// Create a new U255 value
    pub fn new(value: u16) -> Self {
        U255(value)
    }
    
    /// Get the inner value
    pub fn value(&self) -> u16 {
        self.0
    }
    
    /// Get the encoded size in bytes (1, 2, or 3)
    pub fn encoded_size(&self) -> usize {
        let value = self.0;
        if value <= 252 {
            1
        } else if value <= 760 {
            2
        } else {
            3
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::Reader;

    #[test]
    fn test_tag_creation() {
        let tag = Tag(*b"head");
        assert_eq!(tag.as_str(), "head");

        let tag2 = Tag([0x68, 0x65, 0x61, 0x64]); // 'h', 'e', 'a', 'd'
        assert_eq!(tag2.as_str(), "head");
    }

    #[test]
    fn test_fixed_conversion() {
        // 测试 Fixed 类型的转换
        let fixed = Fixed(0x0001_8000 as f32 / 65536.0); // 1.5 in 16.16 format
        assert!((fixed.0 - 1.5).abs() < f32::EPSILON);

        let fixed2 = Fixed::from_i32(0x0002_8000);
        assert!((fixed2.0 - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fword_conversion() {
        let fword = FWord(1000);
        assert_eq!(fword.0, 1000);

        let fword2 = FWord(-500);
        assert_eq!(fword2.0, -500);
    }

    #[test]
    fn test_table_record_read() {
        let data = vec![
            0x68, 0x65, 0x61, 0x64, // 'head'
            0x00, 0x00, 0x00, 0x01, // checksum
            0x00, 0x00, 0x00, 0x0C, // offset (12)
            0x00, 0x00, 0x00, 0x36, // length (54)
        ];
        let mut reader = Reader::new(&data);
        let record = TableRecord::read_from(&mut reader).unwrap();

        assert_eq!(record.tag.as_str(), "head");
        assert_eq!(record.checksum, 1);
        assert_eq!(record.offset, 12);
        assert_eq!(record.length, 54);
    }

    #[test]
    fn test_table_record_write() {
        use crate::io::Writer;

        let mut writer = Writer::new();
        let record = TableRecord {
            tag: Tag(*b"cmap"),
            checksum: 0x12345678,
            offset: 100,
            length: 200,
        };
        record.write_to(&mut writer).unwrap();

        // 应该有 16 字节输出 (4 + 4 + 4 + 4)
        assert_eq!(writer.data.len(), 16);
    }

    #[test]
    fn test_table_record_roundtrip() {
        use crate::io::Writer;

        let original = TableRecord {
            tag: Tag(*b"maxp"),
            checksum: 0xDEADBEEF,
            offset: 1024,
            length: 512,
        };

        // 写入
        let mut writer = Writer::new();
        original.write_to(&mut writer).unwrap();

        // 读取
        let mut reader = Reader::new(&writer.data);
        let read_record = TableRecord::read_from(&mut reader).unwrap();

        assert_eq!(read_record.tag, original.tag);
        assert_eq!(read_record.checksum, original.checksum);
        assert_eq!(read_record.offset, original.offset);
        assert_eq!(read_record.length, original.length);
    }

    #[test]
    fn test_tag_from_bytes_valid() {
        let tag = Tag::from_bytes(b"cmap").unwrap();
        assert_eq!(tag.as_str(), "cmap");

        let tag2 = Tag::from_bytes(b"glyf").unwrap();
        assert_eq!(tag2.as_str(), "glyf");
    }

    #[test]
    fn test_tag_write() {
        use crate::io::Writer;

        let mut writer = Writer::new();
        let tag = Tag(*b"head");
        tag.write_to(&mut writer).unwrap();

        assert_eq!(writer.data, vec![0x68, 0x65, 0x61, 0x64]); // 'h', 'e', 'a', 'd'
    }

    #[test]
    fn test_tag_roundtrip() {
        use crate::io::Writer;

        let original = Tag(*b"glyf");

        // 写入
        let mut writer = Writer::new();
        original.write_to(&mut writer).unwrap();

        // 读取
        let mut reader = Reader::new(&writer.data);
        let read_tag = Tag::read_from(&mut reader).unwrap();

        assert_eq!(original, read_tag);
        assert_eq!(read_tag.as_str(), "glyf");
    }

    #[test]
    fn test_tag_from_bytes_invalid_length() {
        assert!(Tag::from_bytes(b"hea").is_err()); // 太短
        assert!(Tag::from_bytes(b"headx").is_err()); // 太长
        assert!(Tag::from_bytes(b"").is_err()); // 空
    }

    #[test]
    fn test_tag_display() {
        let tag = Tag(*b"head");
        assert_eq!(format!("{}", tag), "head");

        // 全零字节不是有效的 UTF-8，但 from_utf8 会返回包含这些字节的字符串
        let tag2 = Tag([0x00, 0x00, 0x00, 0x00]);
        let display_str = format!("{}", tag2);
        assert_eq!(display_str.len(), 4); // 应该包含 4 个字符（可能是空字符）
    }

    #[test]
    fn test_tag_equality() {
        let tag1 = Tag(*b"head");
        let tag2 = Tag(*b"head");
        let tag3 = Tag(*b"cmap");

        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[test]
    fn test_tag_clone_and_copy() {
        let tag1 = Tag(*b"maxp");
        let tag2 = tag1;
        let tag3 = tag1; // Tag implements Copy, no need for clone

        assert_eq!(tag1, tag2);
        assert_eq!(tag1, tag3);
    }

    #[test]
    fn test_tag_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Tag(*b"head"));
        set.insert(Tag(*b"cmap"));
        set.insert(Tag(*b"head")); // 重复，不应添加

        assert_eq!(set.len(), 2);
        assert!(set.contains(&Tag(*b"head")));
        assert!(set.contains(&Tag(*b"cmap")));
    }

    #[test]
    fn test_f2dot14_conversion() {
        // 1.0 in 2.14 format = 0x4000
        let f = F2Dot14::from_u16(0x4000);
        assert!((f.0 - 1.0).abs() < 0.0001);

        // 0.5 in 2.14 format = 0x2000
        let f2 = F2Dot14::from_u16(0x2000);
        assert!((f2.0 - 0.5).abs() < 0.0001);

        // -1.0 in 2.14 format = 0xC000 (signed)
        let f3 = F2Dot14::from_u16(0xC000);
        assert!((f3.0 - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_f2dot14_zero() {
        let f = F2Dot14::from_u16(0x0000);
        assert!((f.0 - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_fixed_from_i32_positive() {
        // 1.5 in 16.16 format = 0x00018000
        let fixed = Fixed::from_i32(0x00018000);
        assert!((fixed.0 - 1.5).abs() < 0.0001);
    }

    #[test]
    fn test_fixed_from_i32_negative() {
        // -1.5 in 16.16 format
        let fixed = Fixed::from_i32(-0x00018000);
        assert!((fixed.0 - (-1.5)).abs() < 0.0001);
    }

    #[test]
    fn test_fixed_from_i32_zero() {
        let fixed = Fixed::from_i32(0);
        assert!((fixed.0 - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_fixed_from_i32_fractional() {
        // 0.25 in 16.16 format = 0x00004000
        let fixed = Fixed::from_i32(0x00004000);
        assert!((fixed.0 - 0.25).abs() < 0.0001);
    }

    #[test]
    fn test_fword_write() {
        use crate::io::Writer;

        let mut writer = Writer::new();
        let fword = FWord(1000);
        fword.write_to(&mut writer).unwrap();

        assert_eq!(writer.data, vec![0x03, 0xE8]); // 1000 in big-endian
    }

    #[test]
    fn test_fword_write_negative() {
        use crate::io::Writer;

        let mut writer = Writer::new();
        let fword = FWord(-500);
        fword.write_to(&mut writer).unwrap();

        // -500 in i16 big-endian = 0xFE0C
        assert_eq!(writer.data, vec![0xFE, 0x0C]);
    }

    #[test]
    fn test_ufword_write() {
        use crate::io::Writer;

        let mut writer = Writer::new();
        let ufword = UFWord(1000);
        ufword.write_to(&mut writer).unwrap();

        assert_eq!(writer.data, vec![0x03, 0xE8]); // 1000 in big-endian
    }

    #[test]
    fn test_ufword_max_value() {
        use crate::io::Writer;

        let mut writer = Writer::new();
        let ufword = UFWord(u16::MAX);
        ufword.write_to(&mut writer).unwrap();

        assert_eq!(writer.data, vec![0xFF, 0xFF]);
    }

    #[test]
    fn test_encoding_record_read() {
        let data = vec![
            0x00, 0x03, // platform_id = 3 (Windows)
            0x00, 0x01, // encoding_id = 1 (Unicode BMP)
            0x00, 0x00, 0x00, 0x0C, // offset = 12
        ];
        let mut reader = Reader::new(&data);
        let record = EncodingRecord::read_from(&mut reader).unwrap();

        assert_eq!(record.platform_id, 3);
        assert_eq!(record.encoding_id, 1);
        assert_eq!(record.offset, 12);
    }

    #[test]
    fn test_encoding_record_write() {
        use crate::io::Writer;

        let mut writer = Writer::new();
        let record = EncodingRecord {
            platform_id: 3,
            encoding_id: 1,
            offset: 12,
        };
        record.write_to(&mut writer).unwrap();

        // 应该有 8 字节输出 (2 + 2 + 4)
        assert_eq!(writer.data.len(), 8);
    }

    #[test]
    fn test_encoding_record_roundtrip() {
        use crate::io::Writer;

        let original = EncodingRecord {
            platform_id: 1, // Mac
            encoding_id: 0, // Roman
            offset: 100,
        };

        // 写入
        let mut writer = Writer::new();
        original.write_to(&mut writer).unwrap();

        // 读取
        let mut reader = Reader::new(&writer.data);
        let read_record = EncodingRecord::read_from(&mut reader).unwrap();

        assert_eq!(read_record.platform_id, original.platform_id);
        assert_eq!(read_record.encoding_id, original.encoding_id);
        assert_eq!(read_record.offset, original.offset);
    }

    #[test]
    fn test_encoding_record_mac_platform() {
        let data = vec![
            0x00, 0x01, // platform_id = 1 (Mac)
            0x00, 0x00, // encoding_id = 0 (Roman)
            0x00, 0x00, 0x00, 0x08, // offset = 8
        ];
        let mut reader = Reader::new(&data);
        let record = EncodingRecord::read_from(&mut reader).unwrap();

        assert_eq!(record.platform_id, 1);
        assert_eq!(record.encoding_id, 0);
    }


    #[test]
    fn test_table_record_debug() {
        let tag = Tag(*b"head");
        let record = TableRecord {
            tag,
            checksum: 0x12345678,
            offset: 100,
            length: 54,
        };

        let debug_str = format!("{:?}", record);
        // Debug 输出包含字段名和值
        assert!(debug_str.contains("checksum") || debug_str.contains("100"));
    }

    #[test]
    fn test_table_record_clone() {
        let tag = Tag(*b"cmap");
        let record1 = TableRecord {
            tag,
            checksum: 0xABCD,
            offset: 200,
            length: 100,
        };

        let record2 = record1.clone();

        assert_eq!(record2.tag.as_str(), "cmap");
        assert_eq!(record2.checksum, 0xABCD);
        assert_eq!(record2.offset, 200);
        assert_eq!(record2.length, 100);
    }

    #[test]
    fn test_encoding_record_debug() {
        let record = EncodingRecord {
            platform_id: 3,
            encoding_id: 1,
            offset: 12,
        };

        let debug_str = format!("{:?}", record);
        assert!(debug_str.contains("3"));
        assert!(debug_str.contains("1"));
    }

    #[test]
    fn test_f2dot14_edge_cases() {
        // 最大值接近 2.0 (0x7FFF = 32767 / 16384 ≈ 1.9999)
        let max = F2Dot14::from_u16(0x7FFF);
        assert!(max.0 < 2.0);
        assert!(max.0 > 1.9);

        // 最小值 (0x8000 = -32768 / 16384 = -2.0)
        let min = F2Dot14::from_u16(0x8000);
        assert!((min.0 - (-2.0)).abs() < 0.0001); // 应该等于 -2.0
    }

    #[test]
    fn test_fixed_edge_cases() {
        // 大数值
        let large = Fixed::from_i32(0x00FF0000); // 255.0
        assert!((large.0 - 255.0).abs() < 0.0001);

        // 小数值
        let small = Fixed::from_i32(0x00000001); // 1/65536
        assert!(small.0 > 0.0);
        assert!(small.0 < 0.0001);
    }

    #[test]
    fn test_u255_single_byte_encoding() {
        // 测试单字节编码 (0-252)
        let u255 = U255::new(100);
        assert_eq!(u255.value(), 100);
        assert_eq!(u255.encoded_size(), 1);

        let mut writer = Writer::new();
        u255.write_to(&mut writer).unwrap();
        assert_eq!(writer.data, vec![100]);
    }

    #[test]
    fn test_u255_max_single_byte() {
        // 测试单字节编码最大值 252
        let u255 = U255::new(252);
        assert_eq!(u255.value(), 252);
        assert_eq!(u255.encoded_size(), 1);

        let mut writer = Writer::new();
        u255.write_to(&mut writer).unwrap();
        assert_eq!(writer.data, vec![252]);
    }

    #[test]
    fn test_u255_two_byte_encoding_range1() {
        // 测试两字节编码：253 + offset (范围 253-505)
        let u255 = U255::new(253);
        assert_eq!(u255.value(), 253);
        assert_eq!(u255.encoded_size(), 2);

        let mut writer = Writer::new();
        u255.write_to(&mut writer).unwrap();
        // 253 + 0 = 253
        assert_eq!(writer.data, vec![253, 0]);

        // 测试中间值
        let u255_mid = U255::new(300);
        assert_eq!(u255_mid.encoded_size(), 2);
        let mut writer_mid = Writer::new();
        u255_mid.write_to(&mut writer_mid).unwrap();
        // 253 + 47 = 300
        assert_eq!(writer_mid.data, vec![253, 47]);
    }

    #[test]
    fn test_u255_two_byte_encoding_range2() {
        // 测试两字节编码：254 + offset (范围 508-760)
        let u255 = U255::new(508);
        assert_eq!(u255.value(), 508);
        assert_eq!(u255.encoded_size(), 2);

        let mut writer = Writer::new();
        u255.write_to(&mut writer).unwrap();
        // 254 + 0 = 508
        assert_eq!(writer.data, vec![254, 0]);

        // 测试最大值
        let u255_max = U255::new(760);
        assert_eq!(u255_max.encoded_size(), 2);
        let mut writer_max = Writer::new();
        u255_max.write_to(&mut writer_max).unwrap();
        // 254 + 252 = 760
        assert_eq!(writer_max.data, vec![254, 252]);
    }

    #[test]
    fn test_u255_three_byte_encoding() {
        // 测试三字节编码：255 + u16 (范围 > 760)
        let u255 = U255::new(761);
        assert_eq!(u255.value(), 761);
        assert_eq!(u255.encoded_size(), 3);

        let mut writer = Writer::new();
        u255.write_to(&mut writer).unwrap();
        // 255 + u16(761) = 255 + 0x02F9
        assert_eq!(writer.data, vec![255, 0x02, 0xF9]);

        // 测试最大值 65535
        let u255_max = U255::new(65535);
        assert_eq!(u255_max.encoded_size(), 3);
        let mut writer_max = Writer::new();
        u255_max.write_to(&mut writer_max).unwrap();
        assert_eq!(writer_max.data, vec![255, 0xFF, 0xFF]);
    }

    #[test]
    fn test_u255_read_single_byte() {
        // 测试读取单字节值
        let data = vec![100];
        let mut reader = Reader::new(&data);
        let u255 = U255::read_from(&mut reader).unwrap();
        assert_eq!(u255.value(), 100);
    }

    #[test]
    fn test_u255_read_two_byte_range1() {
        // 测试读取两字节值 (253 + offset)
        let data = vec![253, 0]; // 253 + 0 = 253
        let mut reader = Reader::new(&data);
        let u255 = U255::read_from(&mut reader).unwrap();
        assert_eq!(u255.value(), 253);

        let data2 = vec![253, 47]; // 253 + 47 = 300
        let mut reader2 = Reader::new(&data2);
        let u255_2 = U255::read_from(&mut reader2).unwrap();
        assert_eq!(u255_2.value(), 300);
    }

    #[test]
    fn test_u255_read_two_byte_range2() {
        // 测试读取两字节值 (254 + offset)
        let data = vec![254, 0]; // 254 + 0 = 508
        let mut reader = Reader::new(&data);
        let u255 = U255::read_from(&mut reader).unwrap();
        assert_eq!(u255.value(), 508);

        let data2 = vec![254, 252]; // 254 + 252 = 760
        let mut reader2 = Reader::new(&data2);
        let u255_2 = U255::read_from(&mut reader2).unwrap();
        assert_eq!(u255_2.value(), 760);
    }

    #[test]
    fn test_u255_read_three_byte() {
        // 测试读取三字节值
        let data = vec![255, 0x02, 0xF9]; // 255 + u16(761)
        let mut reader = Reader::new(&data);
        let u255 = U255::read_from(&mut reader).unwrap();
        assert_eq!(u255.value(), 761);

        let data_max = vec![255, 0xFF, 0xFF]; // 65535
        let mut reader_max = Reader::new(&data_max);
        let u255_max = U255::read_from(&mut reader_max).unwrap();
        assert_eq!(u255_max.value(), 65535);
    }

    #[test]
    fn test_u255_roundtrip_all_ranges() {
        // 测试所有范围的读写往返
        let test_values = vec![0, 100, 252, 253, 300, 505, 508, 600, 760, 761, 1000, 10000, 65535];

        for value in test_values {
            let original = U255::new(value);

            let mut writer = Writer::new();
            original.write_to(&mut writer).unwrap();

            let mut reader = Reader::new(&writer.data);
            let read_u255 = U255::read_from(&mut reader).unwrap();

            assert_eq!(read_u255, original, "Roundtrip failed for value {}", value);
            assert_eq!(read_u255.value(), value, "Value mismatch for {}", value);
        }
    }

    #[test]
    fn test_u255_boundary_values() {
        // 测试关键边界值
        // 252: 单字节最大值
        let u252 = U255::new(252);
        let mut writer = Writer::new();
        u252.write_to(&mut writer).unwrap();
        assert_eq!(writer.data, vec![252]);

        // 253: 两字节范围 1 开始
        let u253 = U255::new(253);
        let mut writer2 = Writer::new();
        u253.write_to(&mut writer2).unwrap();
        assert_eq!(writer2.data, vec![253, 0]);

        // 505: 两字节范围 1 结束
        let u505 = U255::new(505);
        let mut writer3 = Writer::new();
        u505.write_to(&mut writer3).unwrap();
        assert_eq!(writer3.data, vec![253, 252]);

        // 508: 两字节范围 2 开始
        let u508 = U255::new(508);
        let mut writer4 = Writer::new();
        u508.write_to(&mut writer4).unwrap();
        assert_eq!(writer4.data, vec![254, 0]);

        // 760: 两字节范围 2 结束
        let u760 = U255::new(760);
        let mut writer5 = Writer::new();
        u760.write_to(&mut writer5).unwrap();
        assert_eq!(writer5.data, vec![254, 252]);

        // 761: 三字节开始
        let u761 = U255::new(761);
        let mut writer6 = Writer::new();
        u761.write_to(&mut writer6).unwrap();
        assert_eq!(writer6.data, vec![255, 0x02, 0xF9]);
    }
}
