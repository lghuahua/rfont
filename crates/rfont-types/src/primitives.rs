use crate::io::{ReadBytes, Reader, WriteBytes, Writer};
use crate::FontError;
use chrono::{Duration, NaiveDate, NaiveDateTime};
use font_macros::ReadBytes;
use std::fmt;

#[derive(Debug, Clone)]
pub struct TableRecord {
    pub tag: Tag,
    pub checksum: u32,
    pub offset: u32,
    pub length: u32,
}

impl<'a> ReadBytes<'a> for TableRecord {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let tag_bytes = [
            reader.read_u8()?,
            reader.read_u8()?,
            reader.read_u8()?,
            reader.read_u8()?,
        ];
        Ok(TableRecord {
            tag: Tag(tag_bytes),
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
pub struct LONGDATETIME(pub NaiveDateTime);

impl LONGDATETIME {
    fn get_start() -> Option<NaiveDateTime> {
        let date = NaiveDate::from_ymd_opt(1904, 1, 1)?;
        date.and_hms_opt(0, 0, 0)
    }
}

impl<'a> ReadBytes<'a> for LONGDATETIME {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let high = reader.read_u32()? as i64;
        let low = reader.read_u32()? as i64;
        let seconds = (high << 32) | low;

        let start = LONGDATETIME::get_start().ok_or(FontError::InvalidBaseDate)?;

        // chrono::Duration::seconds 的范围是 i64::MIN / 1_000_000_000 到 i64::MAX / 1_000_000_000
        // 大约 ±292 年。字体时间戳可能超出此范围，需要安全处理。
        const MAX_SECONDS: i64 = i64::MAX / 1_000_000_000;
        const MIN_SECONDS: i64 = i64::MIN / 1_000_000_000;

        let dt = if (MIN_SECONDS..=MAX_SECONDS).contains(&seconds) {
            start + Duration::seconds(seconds)
        } else {
            // 如果时间戳超出范围，使用起始时间作为fallback
            eprintln!(
                "Warning: LONGDATETIME value {} out of bounds (max={}), using epoch",
                seconds, MAX_SECONDS
            );
            start
        };

        Ok(LONGDATETIME(dt))
    }
}

impl WriteBytes for LONGDATETIME {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        let start = LONGDATETIME::get_start().ok_or(FontError::InvalidBaseDate)?;
        let duration = self.0.signed_duration_since(start);
        writer.write_u32(duration.num_seconds() as u32)?;
        writer.write_u32(0)?; // High 32 bits
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::Reader;
    use chrono::Datelike;

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
    fn test_longdatetime_read() {
        // 1970-01-01 00:00:00 相对于 1904-01-01 的秒数
        let seconds_since_1904 = 2082844800u64;
        let high = (seconds_since_1904 >> 32) as u32;
        let low = seconds_since_1904 as u32;

        let data = vec![
            (high >> 24) as u8,
            (high >> 16) as u8,
            (high >> 8) as u8,
            high as u8,
            (low >> 24) as u8,
            (low >> 16) as u8,
            (low >> 8) as u8,
            low as u8,
        ];
        let mut reader = Reader::new(&data);
        let datetime = LONGDATETIME::read_from(&mut reader).unwrap();

        // 验证解析成功（具体日期可能因时区而异）
        assert!(datetime.0.year() >= 1970);
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
    fn test_longdatetime_write() {
        use crate::io::Writer;
        use chrono::NaiveDate;

        let mut writer = Writer::new();
        let date = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let datetime = LONGDATETIME(date);

        datetime.write_to(&mut writer).unwrap();

        // 应该有 8 字节输出
        assert_eq!(writer.data.len(), 8);
    }

    #[test]
    fn test_longdatetime_write_basic() {
        use crate::io::Writer;
        use chrono::NaiveDate;

        let mut writer = Writer::new();
        let date = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let datetime = LONGDATETIME(date);

        datetime.write_to(&mut writer).unwrap();

        // 应该有 8 字节输出
        assert_eq!(writer.data.len(), 8);

        // 验证后 4 字节（高 32 位，write_to 只写了低 32 位）
        assert_eq!(&writer.data[4..8], &[0, 0, 0, 0]);
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
}
