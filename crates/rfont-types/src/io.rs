use crate::error::FontError;

/// Trait for reading bytes from a stream.
pub trait ReadBytes<'a> {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError>
    where
        Self: Sized;
}

/// Trait for writing bytes to a stream.
pub trait WriteBytes {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError>;
}

pub struct Reader<'a> {
    pub data: &'a [u8],
    pub offset: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn read_u8(&mut self) -> Result<u8, FontError> {
        if self.offset + 1 > self.data.len() {
            return Err(FontError::UnexpectedEndOfData {
                offset: self.offset,
                needed: 1,
            });
        }
        let v = self.data[self.offset];
        self.offset += 1;
        Ok(v)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], FontError> {
        if self.offset + len > self.data.len() {
            return Err(FontError::UnexpectedEndOfData {
                offset: self.offset,
                needed: len,
            });
        }
        let bytes = &self.data[self.offset..self.offset + len];
        self.offset += len;
        Ok(bytes)
    }

    pub fn skip(&mut self, len: usize) -> Result<(), FontError> {
        if self.offset + len > self.data.len() {
            return Err(FontError::UnexpectedEndOfData {
                offset: self.offset,
                needed: len,
            });
        }
        self.offset += len;
        Ok(())
    }

    pub fn read_u16(&mut self) -> Result<u16, FontError> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u16_at(&self, offset: usize) -> Result<u16, FontError> {
        if offset + 2 > self.data.len() {
            return Err(FontError::UnexpectedEndOfData { offset, needed: 2 });
        }
        let bytes = &self.data[offset..offset + 2];
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u32(&mut self) -> Result<u32, FontError> {
        if self.offset + 4 > self.data.len() {
            return Err(FontError::UnexpectedEndOfData {
                offset: self.offset,
                needed: 4,
            });
        }
        let v = u32::from_be_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(v)
    }

    pub fn read_i16(&mut self) -> Result<i16, FontError> {
        let v = self.read_u16()?;
        Ok(v as i16)
    }

    pub fn read_i32(&mut self) -> Result<i32, FontError> {
        let v = self.read_u32()?;
        Ok(v as i32)
    }

    pub fn read_array<T: ReadBytes<'a>>(&mut self, count: usize) -> Result<Vec<T>, FontError> {
        let mut vec = Vec::with_capacity(count);
        for _ in 0..count {
            vec.push(T::read_from(self)?);
        }
        Ok(vec)
    }

    pub fn slice(&self, offset: usize, len: usize) -> Result<Reader<'a>, FontError> {
        if offset + len > self.data.len() {
            return Err(FontError::InvalidOffset {
                table: "slice".to_string(),
                offset: offset as u32,
                max: self.data.len() as u32,
            });
        }
        Ok(Reader {
            data: &self.data[offset..offset + len],
            offset: 0,
        })
    }

    /// 读取 Base128 编码的无符号整数
    ///
    /// Base128 是一种变长整数编码，每 7 位使用一个字节，最高位作为继续位。
    /// - 如果最高位为 1，表示还有后续字节
    /// - 如果最高位为 0，表示这是最后一个字节
    ///
    /// # 错误
    /// - 如果数值超过 2^28-1，返回 `FontError::Generic`
    ///
    /// # 示例
    /// ```
    /// use rfont_types::{Reader, Writer};
    /// let mut writer = Writer::new();
    /// writer.write_base128(300).unwrap();
    ///
    /// let mut reader = Reader::new(&writer.data);
    /// let value = reader.read_base128().unwrap();
    /// assert_eq!(value, 300);
    /// ```
    pub fn read_base128(&mut self) -> Result<u32, FontError> {
        let mut result: u32 = 0;

        loop {
            // 检查溢出（WOFF2 规范限制）
            if result > 0x0FFFFFFF {
                return Err(FontError::Generic("Base128 overflow".to_string()));
            }

            let byte = self.read_u8()?;
            result = (result << 7) | ((byte & 0x7F) as u32);

            // 如果最高位为 0，表示结束
            if byte & 0x80 == 0 {
                break;
            }
        }

        Ok(result)
    }
}

pub struct Writer {
    pub data: Vec<u8>,
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl Writer {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn write_u8(&mut self, v: u8) -> Result<(), FontError> {
        self.data.push(v);
        Ok(())
    }

    pub fn write_u16(&mut self, v: u16) -> Result<(), FontError> {
        self.data.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    pub fn write_u32(&mut self, v: u32) -> Result<(), FontError> {
        self.data.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    pub fn write_i16(&mut self, v: i16) -> Result<(), FontError> {
        self.write_u16(v as u16)
    }

    pub fn write_i32(&mut self, v: i32) -> Result<(), FontError> {
        self.write_u32(v as u32)
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), FontError> {
        self.data.extend_from_slice(bytes);
        Ok(())
    }

    /// 写入 Base128 编码的无符号整数
    ///
    /// Base128 是一种变长整数编码，每 7 位使用一个字节，最高位作为继续位。
    /// - 值 0-127: 1 字节
    /// - 值 128-16383: 2 字节
    /// - 值 16384-2097151: 3 字节
    /// - 以此类推，最多 5 字节（支持到 2^28-1）
    ///
    /// # 示例
    /// ```
    /// use rfont_types::Writer;
    /// let mut writer = Writer::new();
    /// writer.write_base128(127).unwrap();  // [0x7F]
    /// writer.write_base128(128).unwrap();  // [0x81, 0x00]
    /// writer.write_base128(300).unwrap();  // [0x82, 0x2C]
    /// ```
    pub fn write_base128(&mut self, value: u32) -> Result<(), FontError> {
        if value == 0 {
            return self.write_u8(0);
        }

        // 计算需要多少字节
        let mut temp = value;
        let mut num_bytes = 0;
        while temp > 0 {
            num_bytes += 1;
            temp >>= 7;
        }

        // 从最高位开始写入
        for i in (0..num_bytes).rev() {
            let shift = i * 7;
            let byte = ((value >> shift) & 0x7F) as u8;

            // 如果不是最后一个字节，设置继续位（最高位=1）
            if i > 0 {
                self.write_u8(byte | 0x80)?;
            } else {
                self.write_u8(byte)?;
            }
        }

        Ok(())
    }

    pub fn pad_to_4_bytes(&mut self) {
        while !self.data.len().is_multiple_of(4) {
            self.data.push(0);
        }
    }

    /// 计算 OpenType 规范的 32 位校验和
    pub fn calculate_checksum(data: &[u8]) -> u32 {
        let mut sum: u32 = 0;
        let mut i = 0;
        while i < data.len() {
            let mut val: u32 = 0;
            // 每次读取 4 个字节
            for j in 0..4 {
                if i + j < data.len() {
                    val = (val << 8) | data[i + j] as u32;
                } else {
                    val <<= 8; // 不足 4 字节补 0
                }
            }
            sum = sum.wrapping_add(val);
            i += 4;
        }
        sum
    }
}

impl<'a> ReadBytes<'a> for u8 {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        reader.read_u8()
    }
}

impl WriteBytes for u8 {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_u8(*self)
    }
}

impl<'a> ReadBytes<'a> for u16 {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        reader.read_u16()
    }
}

impl WriteBytes for u16 {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_u16(*self)
    }
}

impl<'a> ReadBytes<'a> for u32 {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        reader.read_u32()
    }
}

impl WriteBytes for u32 {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_u32(*self)
    }
}

impl<'a> ReadBytes<'a> for i16 {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        reader.read_i16()
    }
}

impl WriteBytes for i16 {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_i16(*self)
    }
}

impl<'a> ReadBytes<'a> for i32 {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        reader.read_i32()
    }
}

impl WriteBytes for i32 {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_i32(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_new() {
        let data = vec![0x00, 0x01, 0x02, 0x03];
        let reader = Reader::new(&data);
        assert_eq!(reader.offset, 0);
        assert_eq!(reader.data.len(), 4);
    }

    #[test]
    fn test_reader_read_u8() {
        let data = vec![0x42];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_u8().unwrap(), 0x42);
    }

    #[test]
    fn test_reader_read_u8_eof() {
        let data = vec![];
        let mut reader = Reader::new(&data);
        assert!(reader.read_u8().is_err());
    }

    #[test]
    fn test_reader_read_u16() {
        let data = vec![0x00, 0x42];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_u16().unwrap(), 0x0042);
    }

    #[test]
    fn test_reader_read_u16_big_endian() {
        let data = vec![0x12, 0x34];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_u16().unwrap(), 0x1234);
    }

    #[test]
    fn test_reader_read_u16_eof() {
        let data = vec![0x00];
        let mut reader = Reader::new(&data);
        assert!(reader.read_u16().is_err());
    }

    #[test]
    fn test_reader_read_u32() {
        let data = vec![0x00, 0x00, 0x00, 0x42];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_u32().unwrap(), 0x00000042);
    }

    #[test]
    fn test_reader_read_u32_big_endian() {
        let data = vec![0x12, 0x34, 0x56, 0x78];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_u32().unwrap(), 0x12345678);
    }

    #[test]
    fn test_reader_read_u32_eof() {
        let data = vec![0x00, 0x00, 0x00];
        let mut reader = Reader::new(&data);
        assert!(reader.read_u32().is_err());
    }

    #[test]
    fn test_reader_read_i16() {
        let data = vec![0xFF, 0xFE]; // -2 in big-endian
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_i16().unwrap(), -2);
    }

    #[test]
    fn test_reader_read_i32() {
        let data = vec![0xFF, 0xFF, 0xFF, 0xFE]; // -2 in big-endian
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_i32().unwrap(), -2);
    }

    #[test]
    fn test_reader_read_bytes() {
        let data = vec![0x01, 0x02, 0x03];
        let mut reader = Reader::new(&data);
        let bytes = reader.read_bytes(2).unwrap();
        assert_eq!(bytes, &[0x01, 0x02]);
        assert_eq!(reader.offset, 2);
    }

    #[test]
    fn test_reader_read_bytes_eof() {
        let data = vec![0x01];
        let mut reader = Reader::new(&data);
        assert!(reader.read_bytes(2).is_err());
    }

    #[test]
    fn test_reader_slice() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let reader = Reader::new(&data);
        let sliced = reader.slice(1, 2).unwrap();
        assert_eq!(sliced.data, &[0x02, 0x03]);
        assert_eq!(sliced.offset, 0);
    }

    #[test]
    fn test_reader_slice_out_of_bounds() {
        let data = vec![0x01, 0x02];
        let reader = Reader::new(&data);
        assert!(reader.slice(0, 3).is_err());
    }

    #[test]
    fn test_reader_read_u16_at() {
        let data = vec![0x00, 0x01, 0x02, 0x03];
        let reader = Reader::new(&data);
        assert_eq!(reader.read_u16_at(1).unwrap(), 0x0102);
    }

    #[test]
    fn test_reader_read_u16_at_eof() {
        let data = vec![0x00];
        let reader = Reader::new(&data);
        assert!(reader.read_u16_at(0).is_err());
    }

    #[test]
    fn test_writer_new() {
        let writer = Writer::new();
        assert_eq!(writer.data.len(), 0);
    }

    #[test]
    fn test_writer_write_u8() {
        let mut writer = Writer::new();
        writer.write_u8(0x42).unwrap();
        assert_eq!(writer.data, vec![0x42]);
    }

    #[test]
    fn test_writer_write_u16() {
        let mut writer = Writer::new();
        writer.write_u16(0x1234).unwrap();
        assert_eq!(writer.data, vec![0x12, 0x34]);
    }

    #[test]
    fn test_writer_write_u32() {
        let mut writer = Writer::new();
        writer.write_u32(0x12345678).unwrap();
        assert_eq!(writer.data, vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_writer_write_i16() {
        let mut writer = Writer::new();
        writer.write_i16(-2).unwrap();
        assert_eq!(writer.data, vec![0xFF, 0xFE]);
    }

    #[test]
    fn test_writer_write_i32() {
        let mut writer = Writer::new();
        writer.write_i32(-2).unwrap();
        assert_eq!(writer.data, vec![0xFF, 0xFF, 0xFF, 0xFE]);
    }

    #[test]
    fn test_writer_write_bytes() {
        let mut writer = Writer::new();
        writer.write_bytes(&[0x01, 0x02, 0x03]).unwrap();
        assert_eq!(writer.data, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_writer_pad_to_4_bytes() {
        let mut writer = Writer::new();
        writer.write_u8(0x01).unwrap();
        writer.pad_to_4_bytes();
        assert_eq!(writer.data.len(), 4);
        assert_eq!(writer.data, vec![0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_writer_pad_already_aligned() {
        let mut writer = Writer::new();
        writer.write_u16(0x1234).unwrap();
        writer.write_u16(0x5678).unwrap();
        let len_before = writer.data.len();
        writer.pad_to_4_bytes();
        assert_eq!(writer.data.len(), len_before); // 不需要填充
    }

    #[test]
    fn test_calculate_checksum_empty() {
        assert_eq!(Writer::calculate_checksum(&[]), 0);
    }

    #[test]
    fn test_calculate_checksum_basic() {
        let data = vec![0x00, 0x00, 0x00, 0x01];
        let checksum = Writer::calculate_checksum(&data);
        assert_eq!(checksum, 1);
    }

    #[test]
    fn test_calculate_checksum_multiple_words() {
        let data = vec![0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02];
        let checksum = Writer::calculate_checksum(&data);
        assert_eq!(checksum, 3);
    }

    #[test]
    fn test_calculate_checksum_padding() {
        // [0x00, 0x00, 0x00, 0x01, 0x02] 会被填充为 [0x00, 0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00]
        // 第一个 word: 0x00000001
        // 第二个 word: 0x02000000
        // checksum = 0x00000001 + 0x02000000 = 0x02000001 = 33554433
        let data = vec![0x00, 0x00, 0x00, 0x01, 0x02];
        let checksum = Writer::calculate_checksum(&data);
        assert_eq!(checksum, 0x02000001); // 33554433
    }

    #[test]
    fn test_read_bytes_trait_u8() {
        let data = vec![0x42];
        let mut reader = Reader::new(&data);
        let value: u8 = u8::read_from(&mut reader).unwrap();
        assert_eq!(value, 0x42);
    }

    #[test]
    fn test_write_bytes_trait_u8() {
        let mut writer = Writer::new();
        let value: u8 = 0x42;
        value.write_to(&mut writer).unwrap();
        assert_eq!(writer.data, vec![0x42]);
    }

    #[test]
    fn test_read_bytes_trait_u16() {
        let data = vec![0x12, 0x34];
        let mut reader = Reader::new(&data);
        let value: u16 = u16::read_from(&mut reader).unwrap();
        assert_eq!(value, 0x1234);
    }

    #[test]
    fn test_write_bytes_trait_u16() {
        let mut writer = Writer::new();
        let value: u16 = 0x1234;
        value.write_to(&mut writer).unwrap();
        assert_eq!(writer.data, vec![0x12, 0x34]);
    }

    #[test]
    fn test_read_bytes_trait_u32() {
        let data = vec![0x12, 0x34, 0x56, 0x78];
        let mut reader = Reader::new(&data);
        let value: u32 = u32::read_from(&mut reader).unwrap();
        assert_eq!(value, 0x12345678);
    }

    #[test]
    fn test_write_bytes_trait_u32() {
        let mut writer = Writer::new();
        let value: u32 = 0x12345678;
        value.write_to(&mut writer).unwrap();
        assert_eq!(writer.data, vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_read_bytes_trait_i16() {
        let data = vec![0xFF, 0xFE];
        let mut reader = Reader::new(&data);
        let value: i16 = i16::read_from(&mut reader).unwrap();
        assert_eq!(value, -2);
    }

    #[test]
    fn test_write_bytes_trait_i16() {
        let mut writer = Writer::new();
        let value: i16 = -2;
        value.write_to(&mut writer).unwrap();
        assert_eq!(writer.data, vec![0xFF, 0xFE]);
    }

    #[test]
    fn test_read_bytes_trait_i32() {
        let data = vec![0xFF, 0xFF, 0xFF, 0xFE];
        let mut reader = Reader::new(&data);
        let value: i32 = i32::read_from(&mut reader).unwrap();
        assert_eq!(value, -2);
    }

    #[test]
    fn test_write_bytes_trait_i32() {
        let mut writer = Writer::new();
        let value: i32 = -2;
        value.write_to(&mut writer).unwrap();
        assert_eq!(writer.data, vec![0xFF, 0xFF, 0xFF, 0xFE]);
    }

    // ==================== Reader 边界条件测试 ====================

    #[test]
    fn test_reader_read_u8_boundary_exact() {
        // 刚好读取到最后一个字节
        let data = vec![0xFF];
        let mut reader = Reader::new(&data);

        assert_eq!(reader.read_u8().unwrap(), 0xFF);
        // 再次读取应该失败
        assert!(reader.read_u8().is_err());
    }

    #[test]
    fn test_reader_read_u8_offset_tracking() {
        let data = vec![0x10, 0x20, 0x30];
        let mut reader = Reader::new(&data);

        assert_eq!(reader.offset, 0);
        reader.read_u8().unwrap();
        assert_eq!(reader.offset, 1);
        reader.read_u8().unwrap();
        assert_eq!(reader.offset, 2);
        reader.read_u8().unwrap();
        assert_eq!(reader.offset, 3);
    }

    #[test]
    fn test_reader_read_u16_insufficient_data_one_byte() {
        // 只有 1 个字节，但需要 2 个
        let data = vec![0x42];
        let mut reader = Reader::new(&data);

        assert!(reader.read_u16().is_err());
    }

    #[test]
    fn test_reader_read_u16_boundary_exact() {
        // 刚好读取到最后两个字节
        let data = vec![0x12, 0x34];
        let mut reader = Reader::new(&data);

        assert_eq!(reader.read_u16().unwrap(), 0x1234);
        // 再次读取应该失败
        assert!(reader.read_u16().is_err());
    }

    #[test]
    fn test_reader_read_u16_offset_tracking() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let mut reader = Reader::new(&data);

        assert_eq!(reader.offset, 0);
        reader.read_u16().unwrap();
        assert_eq!(reader.offset, 2);
        reader.read_u16().unwrap();
        assert_eq!(reader.offset, 4);
    }

    #[test]
    fn test_reader_read_u32_insufficient_data_three_bytes() {
        // 只有 3 个字节，但需要 4 个
        let data = vec![0x01, 0x02, 0x03];
        let mut reader = Reader::new(&data);

        assert!(reader.read_u32().is_err());
    }

    #[test]
    fn test_reader_read_u32_insufficient_data_two_bytes() {
        let data = vec![0x01, 0x02];
        let mut reader = Reader::new(&data);

        assert!(reader.read_u32().is_err());
    }

    #[test]
    fn test_reader_read_u32_insufficient_data_one_byte() {
        let data = vec![0x01];
        let mut reader = Reader::new(&data);

        assert!(reader.read_u32().is_err());
    }

    #[test]
    fn test_reader_read_u32_boundary_exact() {
        // 刚好读取到最后四个字节
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let mut reader = Reader::new(&data);

        assert_eq!(reader.read_u32().unwrap(), 0xDEADBEEF);
        // 再次读取应该失败
        assert!(reader.read_u32().is_err());
    }

    #[test]
    fn test_reader_read_u32_offset_tracking() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut reader = Reader::new(&data);

        assert_eq!(reader.offset, 0);
        reader.read_u32().unwrap();
        assert_eq!(reader.offset, 4);
        reader.read_u32().unwrap();
        assert_eq!(reader.offset, 8);
    }

    #[test]
    fn test_reader_mixed_reads() {
        let data = vec![
            0x01, // u8
            0x02, 0x03, // u16
            0x04, 0x05, 0x06, 0x07, // u32
        ];
        let mut reader = Reader::new(&data);

        assert_eq!(reader.read_u8().unwrap(), 0x01);
        assert_eq!(reader.read_u16().unwrap(), 0x0203);
        assert_eq!(reader.read_u32().unwrap(), 0x04050607);
        assert_eq!(reader.offset, 7);
    }

    #[test]
    fn test_reader_read_after_error() {
        let data = vec![0x01];
        let mut reader = Reader::new(&data);

        // 成功读取一个字节
        assert_eq!(reader.read_u8().unwrap(), 0x01);

        // 尝试读取 u16 应该失败
        assert!(reader.read_u16().is_err());

        // offset 不应该改变（错误时不回滚）
        assert_eq!(reader.offset, 1);
    }

    #[test]
    fn test_reader_slice_zero_length() {
        let data = vec![0x01, 0x02, 0x03];
        let reader = Reader::new(&data);

        let sub_reader = reader.slice(1, 0).unwrap();
        assert_eq!(sub_reader.data, &[]);
    }

    #[test]
    fn test_reader_slice_out_of_bounds_offset() {
        let data = vec![0x01, 0x02, 0x03];
        let reader = Reader::new(&data);

        // offset 超出范围
        assert!(reader.slice(5, 1).is_err());
    }

    #[test]
    fn test_reader_slice_out_of_bounds_length() {
        let data = vec![0x01, 0x02, 0x03];
        let reader = Reader::new(&data);

        // length 超出范围
        assert!(reader.slice(1, 5).is_err());
    }

    #[test]
    fn test_reader_slice_out_of_bounds_combined() {
        let data = vec![0x01, 0x02, 0x03];
        let reader = Reader::new(&data);

        // offset + length 超出范围
        assert!(reader.slice(2, 2).is_err());
    }

    #[test]
    fn test_reader_slice_empty_data() {
        let data: Vec<u8> = vec![];
        let reader = Reader::new(&data);

        // 空数据上创建零长度切片应该成功
        let sub_reader = reader.slice(0, 0).unwrap();
        assert_eq!(sub_reader.data, &[]);

        // 任何非零长度都应该失败
        assert!(reader.slice(0, 1).is_err());
    }

    #[test]
    fn test_reader_slice_independence() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let reader = Reader::new(&data);

        // 创建子切片
        let mut sub_reader = reader.slice(1, 2).unwrap();

        // 在子切片上读取不应该影响父 reader
        assert_eq!(sub_reader.read_u8().unwrap(), 0x02);
        assert_eq!(sub_reader.read_u8().unwrap(), 0x03);

        // 父 reader 的 offset 应该仍然是 0
        assert_eq!(reader.offset, 0);
    }

    #[test]
    fn test_reader_slice_chained() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let reader = Reader::new(&data);

        // 创建第一个切片
        let sub1 = reader.slice(0, 4).unwrap();
        assert_eq!(sub1.data, &[0x01, 0x02, 0x03, 0x04]);

        // 在第一个切片上创建第二个切片
        let sub2 = sub1.slice(1, 2).unwrap();
        assert_eq!(sub2.data, &[0x02, 0x03]);
    }

    #[test]
    fn test_reader_large_offset() {
        let data = vec![0x00; 1000];
        let mut reader = Reader::new(&data);

        // 手动设置 offset 到接近末尾
        reader.offset = 999;
        assert_eq!(reader.read_u8().unwrap(), 0x00);

        // 再读取应该失败
        assert!(reader.read_u8().is_err());
    }

    #[test]
    fn test_reader_all_zeros() {
        let data = vec![0x00; 10];
        let mut reader = Reader::new(&data);

        assert_eq!(reader.read_u8().unwrap(), 0x00);
        assert_eq!(reader.read_u16().unwrap(), 0x0000);
        assert_eq!(reader.read_u32().unwrap(), 0x00000000);
    }

    #[test]
    fn test_reader_all_ones() {
        let data = vec![0xFF; 10];
        let mut reader = Reader::new(&data);

        assert_eq!(reader.read_u8().unwrap(), 0xFF);
        assert_eq!(reader.read_u16().unwrap(), 0xFFFF);
        assert_eq!(reader.read_u32().unwrap(), 0xFFFFFFFF);
    }

    // ==================== Writer 边界条件测试 ====================

    #[test]
    fn test_writer_multiple_writes() {
        let mut writer = Writer::new();
        writer.write_u8(0x01).unwrap();
        writer.write_u16(0x0203).unwrap();
        writer.write_u32(0x04050607).unwrap();

        assert_eq!(writer.data.len(), 7);
        assert_eq!(writer.data, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
    }

    #[test]
    fn test_writer_pad_various_sizes() {
        for size in 1..=10 {
            let mut writer = Writer::new();
            for _ in 0..size {
                writer.write_u8(0xFF).unwrap();
            }
            writer.pad_to_4_bytes();

            // 验证对齐后是 4 的倍数
            assert_eq!(writer.data.len() % 4, 0);

            // 验证填充的是 0
            for i in size..writer.data.len() {
                assert_eq!(writer.data[i], 0);
            }
        }
    }

    #[test]
    fn test_writer_calculate_checksum_single_byte() {
        // [0x01] 会被填充为 [0x01, 0x00, 0x00, 0x00]
        let data = vec![0x01];
        let checksum = Writer::calculate_checksum(&data);
        assert_eq!(checksum, 0x01000000);
    }

    #[test]
    fn test_writer_calculate_checksum_two_bytes() {
        // [0x01, 0x02] 会被填充为 [0x01, 0x02, 0x00, 0x00]
        let data = vec![0x01, 0x02];
        let checksum = Writer::calculate_checksum(&data);
        assert_eq!(checksum, 0x01020000);
    }

    #[test]
    fn test_writer_calculate_checksum_three_bytes() {
        // [0x01, 0x02, 0x03] 会被填充为 [0x01, 0x02, 0x03, 0x00]
        let data = vec![0x01, 0x02, 0x03];
        let checksum = Writer::calculate_checksum(&data);
        assert_eq!(checksum, 0x01020300);
    }

    #[test]
    fn test_writer_calculate_checksum_overflow() {
        // 测试校验和溢出时的回绕行为
        let data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01];
        let checksum = Writer::calculate_checksum(&data);
        // 0xFFFFFFFF + 0x00000001 = 0x100000000 -> 回绕为 0x00000000
        assert_eq!(checksum, 0x00000000);
    }

    #[test]
    fn test_reader_read_array_u8() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let mut reader = Reader::new(&data);

        let values: Vec<u8> = reader.read_array(3).unwrap();
        assert_eq!(values, vec![0x01, 0x02, 0x03]);
        assert_eq!(reader.offset, 3);
    }

    #[test]
    fn test_reader_read_array_u16() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let mut reader = Reader::new(&data);

        let values: Vec<u16> = reader.read_array(2).unwrap();
        assert_eq!(values, vec![0x0102, 0x0304]);
        assert_eq!(reader.offset, 4);
    }

    #[test]
    fn test_reader_read_array_empty() {
        let data = vec![0x01, 0x02];
        let mut reader = Reader::new(&data);

        let values: Vec<u8> = reader.read_array(0).unwrap();
        assert_eq!(values.len(), 0);
        assert_eq!(reader.offset, 0); // 没有读取任何数据
    }

    #[test]
    fn test_reader_read_array_eof() {
        let data = vec![0x01, 0x02];
        let mut reader = Reader::new(&data);

        // 尝试读取 3 个字节，但只有 2 个
        let result: Result<Vec<u8>, FontError> = reader.read_array(3);
        assert!(result.is_err());
    }

    #[test]
    fn test_base128_write_single_byte() {
        // 测试单字节 Base128 编码（0-127）
        for value in [0, 1, 64, 127] {
            let mut writer = Writer::new();
            writer.write_base128(value).unwrap();
            assert_eq!(writer.data.len(), 1);
            assert_eq!(writer.data[0], value as u8);
        }
    }

    #[test]
    fn test_base128_write_multi_byte() {
        // 值 128: [0x81, 0x00]
        let mut writer = Writer::new();
        writer.write_base128(128).unwrap();
        assert_eq!(writer.data, vec![0x81, 0x00]);

        // 值 300: [0x82, 0x2C]
        let mut writer = Writer::new();
        writer.write_base128(300).unwrap();
        assert_eq!(writer.data, vec![0x82, 0x2C]);

        // 值 16383: [0xFF, 0x7F]
        let mut writer = Writer::new();
        writer.write_base128(16383).unwrap();
        assert_eq!(writer.data, vec![0xFF, 0x7F]);
    }

    #[test]
    fn test_base128_read_single_byte() {
        // 测试单字节 Base128 解码
        for value in [0, 1, 64, 127] {
            let data = vec![value as u8];
            let mut reader = Reader::new(&data);
            let result = reader.read_base128().unwrap();
            assert_eq!(result, value as u32);
        }
    }

    #[test]
    fn test_base128_read_multi_byte() {
        // 值 128: [0x81, 0x00]
        let data = vec![0x81, 0x00];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_base128().unwrap(), 128);

        // 值 300: [0x82, 0x2C]
        let data = vec![0x82, 0x2C];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_base128().unwrap(), 300);
    }

    #[test]
    fn test_base128_roundtrip() {
        // 测试读写往返
        let test_values = vec![
            0, 1, 64, 127, // 单字节
            128, 255, 256, 300, // 双字节
            1000, 16383, 16384, // 双/三字节边界
            100000, 1000000, // 大数值
        ];

        for value in test_values {
            // 写入
            let mut writer = Writer::new();
            writer.write_base128(value).unwrap();

            // 读取
            let mut reader = Reader::new(&writer.data);
            let read_value = reader.read_base128().unwrap();

            assert_eq!(read_value, value, "Failed for value {}", value);
        }
    }
}
