use rfont_types::{FontError, Reader};

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

    pub fn read_u16(&mut self) -> Result<u16, FontError> {
        if self.offset + 2 > self.data.len() {
            return Err(FontError("Unexpected end of data".to_string()));
        }
        let v = u16::from_be_bytes([self.data[self.offset], self.data[self.offset + 1]]);
        self.offset += 2;
        Ok(v)
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
