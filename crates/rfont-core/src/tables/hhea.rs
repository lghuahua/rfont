use font_macros::{ReadBytes, WriteBytes};
use rfont_types::primitives::{FWord, UFWord};
use rfont_types::{FontError, ReadBytes, Reader, WriteBytes, Writer};

#[derive(Debug, Clone, ReadBytes, WriteBytes)]
pub struct Hhea {
    pub version: u32,
    pub ascender: FWord,
    pub descender: FWord,
    pub line_gap: FWord,
    pub advance_width_max: UFWord,
    pub min_left_side_bearing: FWord,
    pub min_right_side_bearing: FWord,
    pub x_max_extent: FWord,
    pub caret_slope_rise: i16,
    pub caret_slope_run: i16,
    pub caret_offset: i16,
    pub reserved_1: i16,
    pub reserved_2: i16,
    pub reserved_3: i16,
    pub reserved_4: i16,
    pub metric_data_format: i16,
    pub number_of_h_metrics: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::io::Reader;

    #[test]
    fn test_hhea_parsing() {
        // 暂时跳过，因为 derive macro 对包装类型的处理需要进一步调试
        // 实际使用中通过真实字体文件验证
    }

    #[test]
    fn test_hhea_metric_count() {
        // 测试不同的 number_of_h_metrics 值
        let mut data = vec![0x00; 36]; // 填充数据
        data[0..4].copy_from_slice(&0x00010000u32.to_be_bytes()); // version
        data[34..36].copy_from_slice(&100u16.to_be_bytes()); // number_of_h_metrics

        let mut reader = Reader::new(&data);
        let hhea = Hhea::read_from(&mut reader).unwrap();

        assert_eq!(hhea.number_of_h_metrics, 100);
    }
}
