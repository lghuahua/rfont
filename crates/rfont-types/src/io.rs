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

// Placeholder for error type and IO structs to be implemented in rfont-core
use std::fmt;

#[derive(Debug)]
pub struct FontError(pub String);

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FontError: {}", self.0)
    }
}

impl std::error::Error for FontError {}

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
            return Err(FontError("Unexpected end of data".to_string()));
        }
        let v = self.data[self.offset];
        self.offset += 1;
        Ok(v)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], FontError> {
        if self.offset + len > self.data.len() {
            return Err(FontError("Unexpected end of data".to_string()));
        }
        let bytes = &self.data[self.offset..self.offset + len];
        self.offset += len;
        Ok(bytes)
    }

    pub fn read_u16(&mut self) -> Result<u16, FontError> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u16_at(&self, offset: usize) -> Result<u16, FontError> {
        if offset + 2 > self.data.len() {
            return Err(FontError("Unexpected end of data".to_string()));
        }
        let bytes = &self.data[offset..offset + 2];
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u32(&mut self) -> Result<u32, FontError> {
        if self.offset + 4 > self.data.len() {
            return Err(FontError("Unexpected end of data".to_string()));
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
            return Err(FontError("Slice out of bounds".to_string()));
        }
        Ok(Reader {
            data: &self.data[offset..offset + len],
            offset: 0,
        })
    }
}

pub struct Writer {
    pub data: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self { data: Vec::new() }
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

    pub fn pad_to_4_bytes(&mut self) {
        while self.data.len() % 4 != 0 {
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
                    val = val << 8; // 不足 4 字节补 0
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
