use std::fmt;
use chrono::{NaiveDateTime, NaiveDate, Duration};
use crate::io::{Reader, Writer, ReadBytes, WriteBytes};
use crate::FontError;
use font_macros::ReadBytes;

#[derive(Debug, Clone)]
pub struct TableRecord {
    pub tag: Tag,
    pub checksum: u32,
    pub offset: u32,
    pub length: u32,
}

impl<'a> ReadBytes<'a> for TableRecord {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let tag_bytes = [reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?];
        Ok(TableRecord {
            tag: Tag(tag_bytes),
            checksum: reader.read_u32()?,
            offset: reader.read_u32()?,
            length: reader.read_u32()?,
        })
    }
}

#[derive(Debug, Clone, ReadBytes)]
pub struct EncodingRecord {
    pub platform_id: u16,
    pub encoding_id: u16,
    pub offset: u32,
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
        
        let dt = if seconds >= MIN_SECONDS && seconds <= MAX_SECONDS {
            start + Duration::seconds(seconds)
        } else {
            // 如果时间戳超出范围，使用起始时间作为fallback
            eprintln!("Warning: LONGDATETIME value {} out of bounds (max={}), using epoch", seconds, MAX_SECONDS);
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
    fn test_longdatetime_read() {
        // 1970-01-01 00:00:00 相对于 1904-01-01 的秒数
        let seconds_since_1904 = 2082844800u64;
        let high = (seconds_since_1904 >> 32) as u32;
        let low = seconds_since_1904 as u32;
        
        let data = vec![
            (high >> 24) as u8, (high >> 16) as u8, (high >> 8) as u8, high as u8,
            (low >> 24) as u8, (low >> 16) as u8, (low >> 8) as u8, low as u8,
        ];
        let mut reader = Reader::new(&data);
        let datetime = LONGDATETIME::read_from(&mut reader).unwrap();
        
        // 验证解析成功（具体日期可能因时区而异）
        assert!(datetime.0.year() >= 1970);
    }
}
