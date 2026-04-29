use rfont_types::{FontError, Reader};

#[derive(Debug, Clone, Copy)]
pub struct HmtxRecord {
    pub advance_width: u16,
    pub lsb: i16,
}

#[derive(Debug, Clone)]
pub struct Hmtx {
    pub metrics: Vec<HmtxRecord>,
}

impl Hmtx {
    pub fn read_from(reader: &mut Reader, num_h_metrics: u16, num_glyphs: u16) -> Result<Self, FontError> {
        let mut metrics = Vec::with_capacity(num_glyphs as usize);

        // 1. 读取 LongHorMetric
        for _ in 0..num_h_metrics {
            metrics.push(HmtxRecord {
                advance_width: reader.read_u16()?,
                lsb: reader.read_i16()?,
            });
        }

        // 2. 读取剩余的 LeftSideBearing
        let remaining = num_glyphs.saturating_sub(num_h_metrics);
        for _ in 0..remaining {
            if let Some(last) = metrics.last().copied() {
                metrics.push(HmtxRecord {
                    advance_width: last.advance_width,
                    lsb: reader.read_i16()?,
                });
            }
        }

        Ok(Self { metrics })
    }

    pub fn get_metric(&self, glyph_id: u16) -> Option<&HmtxRecord> {
        self.metrics.get(glyph_id as usize)
    }
}
