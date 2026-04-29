use rfont_types::{FontError, Writer};

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

    pub fn pad_to_4_bytes(&mut self) {
        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }
    }
}
