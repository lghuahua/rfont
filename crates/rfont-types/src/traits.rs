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
#[derive(Debug)]
pub struct FontError(pub String);

pub struct Reader<'a> {
    pub data: &'a [u8],
    pub offset: usize,
}

pub struct Writer {
    pub data: Vec<u8>,
}
