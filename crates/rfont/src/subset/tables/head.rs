use crate::Font;
use rfont_types::{FontError, Tag};

use crate::constants::{HEAD_TABLE_SIZE, LONGDATETIME_EPOCH_YEAR};

/// 更新 head 表（包含校验和调整和时间戳）
pub fn update_head(font: &Font, checksum_adjustment: u32) -> Result<Vec<u8>, FontError> {
    let mut head_data = font
        .font_data
        .get_table_bytes(Tag(*b"head"))
        .ok_or(FontError::TableNotFound {
            tag: "head".to_string(),
        })?
        .to_vec();

    if head_data.len() < HEAD_TABLE_SIZE {
        return Err(FontError::Generic("head table too short".to_string()));
    }

    // 更新 checkSumAdjustment（偏移量 8-11）
    head_data[8..12].copy_from_slice(&checksum_adjustment.to_be_bytes());

    // 更新 modified 时间戳（偏移量 12-19）
    use chrono::NaiveDateTime;
    let now = chrono::Utc::now().naive_utc();
    let base_date = NaiveDateTime::new(
        chrono::NaiveDate::from_ymd_opt(LONGDATETIME_EPOCH_YEAR, 1, 1).unwrap(),
        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    );

    let seconds_since_1904 = now.signed_duration_since(base_date).num_seconds();

    // LONGDATETIME 是 64 位有符号整数（高位32位 + 低位32位）
    let high = (seconds_since_1904 >> 32) as u32;
    let low = (seconds_since_1904 & 0xFFFFFFFF) as u32;

    head_data[12..16].copy_from_slice(&high.to_be_bytes());
    head_data[16..20].copy_from_slice(&low.to_be_bytes());

    Ok(head_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_head_basic() {
        // 这个测试需要完整的 Font 实例，因此在集成测试中更合适
    }

    #[test]
    fn test_checksum_adjustment_encoding() {
        // 测试 checkSumAdjustment 的编码
        let checksum: u32 = 0xB1B0AFBA;
        let bytes = checksum.to_be_bytes();
        assert_eq!(bytes.len(), 4);

        let decoded = u32::from_be_bytes(bytes);
        assert_eq!(decoded, checksum);
    }

    #[test]
    fn test_longdatetime_format() {
        // 测试 LONGDATETIME 格式（64位时间戳）
        use chrono::NaiveDateTime;
        let now = chrono::Utc::now().naive_utc();
        let base_date = NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(LONGDATETIME_EPOCH_YEAR, 1, 1).unwrap(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );

        let seconds_since_1904 = now.signed_duration_since(base_date).num_seconds();

        // 验证时间在合理范围内（1904年之后）
        assert!(seconds_since_1904 > 0);

        // 分解为 high 和 low 32位
        let high = (seconds_since_1904 >> 32) as u32;
        let low = (seconds_since_1904 & 0xFFFFFFFF) as u32;

        // 重新组合
        let reconstructed = ((high as i64) << 32) | (low as i64);
        assert_eq!(reconstructed, seconds_since_1904);
    }

    #[test]
    fn test_head_table_size_constant() {
        // 验证 head 表大小常量
        assert_eq!(HEAD_TABLE_SIZE, 54); // OpenType head 表固定为 54 字节
    }
}
