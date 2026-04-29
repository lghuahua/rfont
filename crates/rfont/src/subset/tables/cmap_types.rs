/// cmap Format 4 段结构
#[derive(Debug, Clone)]
pub struct CmapSegment {
    pub start_code: u16,
    pub end_code: u16,
    pub id_delta: i16,
    pub id_range_offset: u16,
}

/// cmap Format 12 组结构
#[derive(Debug, Clone)]
pub struct CmapGroup {
    pub start_char_code: u32,
    pub end_char_code: u32,
    pub start_glyph_id: u32,
}
