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

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::Reader;

    #[test]
    fn test_hmtx_read_basic() {
        // 测试基本的 hmtx 读取
        // 假设有 3 个字形，2 个 long metrics
        let data = vec![
            // LongHorMetric 1: advance_width=500, lsb=50
            0x01, 0xF4, 0x00, 0x32,
            // LongHorMetric 2: advance_width=600, lsb=60
            0x02, 0x58, 0x00, 0x3C,
            // LeftSideBearing for glyph 3: lsb=70
            0x00, 0x46,
        ];
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 2, 3).unwrap();
        
        assert_eq!(hmtx.metrics.len(), 3);
        assert_eq!(hmtx.metrics[0].advance_width, 500);
        assert_eq!(hmtx.metrics[0].lsb, 50);
        assert_eq!(hmtx.metrics[1].advance_width, 600);
        assert_eq!(hmtx.metrics[1].lsb, 60);
        assert_eq!(hmtx.metrics[2].advance_width, 600); // 继承自最后一个 long metric
        assert_eq!(hmtx.metrics[2].lsb, 70);
    }

    #[test]
    fn test_hmtx_read_all_long_metrics() {
        // 测试所有字形都有 long metrics 的情况
        let data = vec![
            // Glyph 0
            0x01, 0xF4, 0x00, 0x32,
            // Glyph 1
            0x02, 0x58, 0x00, 0x3C,
            // Glyph 2
            0x03, 0xE8, 0x00, 0x46,
        ];
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 3, 3).unwrap();
        
        assert_eq!(hmtx.metrics.len(), 3);
        assert_eq!(hmtx.metrics[0].advance_width, 500);
        assert_eq!(hmtx.metrics[1].advance_width, 600);
        assert_eq!(hmtx.metrics[2].advance_width, 1000);
    }

    #[test]
    fn test_hmtx_read_single_glyph() {
        // 测试只有一个字形的情况
        let data = vec![
            0x01, 0xF4, 0x00, 0x32,
        ];
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 1, 1).unwrap();
        
        assert_eq!(hmtx.metrics.len(), 1);
        assert_eq!(hmtx.metrics[0].advance_width, 500);
        assert_eq!(hmtx.metrics[0].lsb, 50);
    }

    #[test]
    fn test_hmtx_get_metric() {
        let data = vec![
            0x01, 0xF4, 0x00, 0x32,
            0x02, 0x58, 0x00, 0x3C,
        ];
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 2, 2).unwrap();
        
        // 正常查询
        let metric = hmtx.get_metric(0).unwrap();
        assert_eq!(metric.advance_width, 500);
        assert_eq!(metric.lsb, 50);
        
        let metric = hmtx.get_metric(1).unwrap();
        assert_eq!(metric.advance_width, 600);
        assert_eq!(metric.lsb, 60);
        
        // 越界查询
        assert!(hmtx.get_metric(2).is_none());
        assert!(hmtx.get_metric(100).is_none());
    }

    #[test]
    fn test_hmtx_negative_lsb() {
        // 测试负的 lsb 值（字形可能超出左边界）
        let data = vec![
            // advance_width=500, lsb=-10 (0xFFF6)
            0x01, 0xF4, 0xFF, 0xF6,
        ];
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 1, 1).unwrap();
        
        assert_eq!(hmtx.metrics[0].advance_width, 500);
        assert_eq!(hmtx.metrics[0].lsb, -10);
    }

    #[test]
    fn test_hmtx_zero_advance_width() {
        // 测试 advance_width 为 0 的情况（如空格或控制字符）
        let data = vec![
            0x00, 0x00, 0x00, 0x00,
        ];
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 1, 1).unwrap();
        
        assert_eq!(hmtx.metrics[0].advance_width, 0);
        assert_eq!(hmtx.metrics[0].lsb, 0);
    }

    #[test]
    fn test_hmtx_many_remaining_lsbs() {
        // 测试大量剩余 lsb 的情况
        // 2 long metrics + 8 remaining glyphs
        let mut data = vec![
            // LongHorMetric 1
            0x01, 0xF4, 0x00, 0x32,
            // LongHorMetric 2
            0x02, 0x58, 0x00, 0x3C,
        ];
        
        // 添加 8 个剩余的 lsb
        for i in 0..8 {
            data.extend_from_slice(&[(i * 10) as u8, 0x00]);
        }
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 2, 10).unwrap();
        
        assert_eq!(hmtx.metrics.len(), 10);
        // 前两个有完整的 metrics
        assert_eq!(hmtx.metrics[0].advance_width, 500);
        assert_eq!(hmtx.metrics[1].advance_width, 600);
        // 后面的都继承 advance_width = 600
        for i in 2..10 {
            assert_eq!(hmtx.metrics[i].advance_width, 600);
        }
    }

    #[test]
    fn test_hmtx_number_of_metrics_less_than_glyphs() {
        // 测试 numberOfHMetrics < numGlyphs 的情况
        // 这是 OpenType 规范中的常见情况：多个字形共享相同的 advanceWidth
        let mut data = vec![
            // LongHorMetric 1: advanceWidth=500, lsb=50
            0x01, 0xF4, 0x00, 0x32,
            // LongHorMetric 2: advanceWidth=600, lsb=60
            0x02, 0x58, 0x00, 0x3C,
            // LongHorMetric 3: advanceWidth=700, lsb=70
            0x02, 0xBC, 0x00, 0x46,
        ];
        
        // 添加 7 个剩余的 lsb（总共 10 个字形，但只有 3 个 long metrics）
        for i in 0..7 {
            let lsb_value = ((i + 1) * 10) as i16;
            data.extend_from_slice(&lsb_value.to_be_bytes());
        }
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 3, 10).unwrap();
        
        assert_eq!(hmtx.metrics.len(), 10);
        
        // 前 3 个字形有各自的 advanceWidth
        assert_eq!(hmtx.metrics[0].advance_width, 500);
        assert_eq!(hmtx.metrics[1].advance_width, 600);
        assert_eq!(hmtx.metrics[2].advance_width, 700);
        
        // 后 7 个字形都继承最后一个 advanceWidth (700)
        for i in 3..10 {
            assert_eq!(hmtx.metrics[i].advance_width, 700);
            // remaining lsb 索引从 0 开始，对应 metrics[3] 是 i=0
            assert_eq!(hmtx.metrics[i].lsb, ((i - 3 + 1) * 10) as i16);
        }
    }

    #[test]
    fn test_hmtx_all_same_advance_width() {
        // 测试所有字形共享相同 advanceWidth 的情况
        // numberOfHMetrics = 1，所有字形使用同一个 advanceWidth
        let mut data = vec![
            // LongHorMetric: advanceWidth=1000, lsb=0
            0x03, 0xE8, 0x00, 0x00,
        ];
        
        // 添加 9 个剩余的 lsb（总共 10 个字形）
        for i in 0..9 {
            let lsb_value = (i * 5) as i16;
            data.extend_from_slice(&lsb_value.to_be_bytes());
        }
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 1, 10).unwrap();
        
        assert_eq!(hmtx.metrics.len(), 10);
        
        // 所有字形都有相同的 advanceWidth
        for i in 0..10 {
            assert_eq!(hmtx.metrics[i].advance_width, 1000);
        }
        
        // 但有不同的 lsb
        // metrics[0] 是 long metric，lsb=0
        assert_eq!(hmtx.metrics[0].lsb, 0);
        // metrics[1] 是第一个 remaining glyph，lsb=0 (i=0)
        assert_eq!(hmtx.metrics[1].lsb, 0);
        // metrics[2] 是第二个 remaining glyph，lsb=5 (i=1)
        assert_eq!(hmtx.metrics[2].lsb, 5);
        // metrics[9] 是第九个 remaining glyph，lsb=40 (i=8)
        assert_eq!(hmtx.metrics[9].lsb, 40);
    }

    #[test]
    fn test_hmtx_zero_width_glyphs() {
        // 测试零宽度字形（如空格、控制字符）
        let data = vec![
            // Space character: advanceWidth=0, lsb=0
            0x00, 0x00, 0x00, 0x00,
            // Control char: advanceWidth=0, lsb=0
            0x00, 0x00, 0x00, 0x00,
            // Normal char: advanceWidth=500, lsb=50
            0x01, 0xF4, 0x00, 0x32,
        ];
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 3, 3).unwrap();
        
        assert_eq!(hmtx.metrics.len(), 3);
        
        // 零宽度字形
        assert_eq!(hmtx.metrics[0].advance_width, 0);
        assert_eq!(hmtx.metrics[0].lsb, 0);
        assert_eq!(hmtx.metrics[1].advance_width, 0);
        assert_eq!(hmtx.metrics[1].lsb, 0);
        
        // 正常字形
        assert_eq!(hmtx.metrics[2].advance_width, 500);
        assert_eq!(hmtx.metrics[2].lsb, 50);
    }

    #[test]
    fn test_hmtx_extreme_values() {
        // 测试极端值（最大/最小 advanceWidth 和 lsb）
        let data = vec![
            // Max advanceWidth: 65535, Max lsb: 32767
            0xFF, 0xFF, 0x7F, 0xFF,
            // Min advanceWidth: 0, Min lsb: -32768
            0x00, 0x00, 0x80, 0x00,
        ];
        
        let mut reader = Reader::new(&data);
        let hmtx = Hmtx::read_from(&mut reader, 2, 2).unwrap();
        
        assert_eq!(hmtx.metrics.len(), 2);
        
        // 最大值
        assert_eq!(hmtx.metrics[0].advance_width, 65535);
        assert_eq!(hmtx.metrics[0].lsb, 32767);
        
        // 最小值
        assert_eq!(hmtx.metrics[1].advance_width, 0);
        assert_eq!(hmtx.metrics[1].lsb, -32768);
    }
}
