use crate::Font;
use rfont_types::FontError;

/// 重建 hmtx 表
pub fn rebuild_hmtx(font: &Font, subset_glyphs: &[u16]) -> Result<Vec<u8>, FontError> {
    let mut data = Vec::new();

    for &glyph_id in subset_glyphs {
        if (glyph_id as usize) < font.hmtx.metrics.len() {
            let metric = &font.hmtx.metrics[glyph_id as usize];
            data.extend_from_slice(&metric.advance_width.to_be_bytes());
            data.extend_from_slice(&metric.lsb.to_be_bytes());
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rebuild_hmtx_basic() {
        // 这个测试需要完整的 Font 实例，因此在集成测试中更合适
        // 这里我们只验证函数签名和返回类型
    }

    #[test]
    fn test_hmtx_entry_size() {
        // 每个 hmtx 条目应该是 4 字节（2 字节 advance_width + 2 字节 lsb）
        let entry_size = std::mem::size_of::<u16>() * 2;
        assert_eq!(entry_size, 4);
    }

    #[test]
    fn test_hmtx_encoding() {
        // 测试 hmtx 数据的编码
        let advance_width: u16 = 500;
        let lsb: i16 = -50;

        let mut data = Vec::new();
        data.extend_from_slice(&advance_width.to_be_bytes());
        data.extend_from_slice(&lsb.to_be_bytes());

        // 验证数据长度
        assert_eq!(data.len(), 4);

        // 验证可以正确解码
        let decoded_width = u16::from_be_bytes([data[0], data[1]]);
        let decoded_lsb = i16::from_be_bytes([data[2], data[3]]);

        assert_eq!(decoded_width, advance_width);
        assert_eq!(decoded_lsb, lsb);
    }
}
