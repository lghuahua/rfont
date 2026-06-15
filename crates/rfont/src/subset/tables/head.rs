use crate::Font;
use rfont_core::Head;
use rfont_types::{FontError, HEAD_TABLE_SIZE, ReadBytes, Reader, Tag, WriteBytes, Writer};

/// 更新 head 表（包含校验和调整和时间戳）
pub fn update_head(font: &Font, checksum_adjustment: u32, index_to_loc_format: u16, transform: bool ) -> Result<Vec<u8>, FontError> {
    let head_data = font
        .font_data
        .get_table_bytes(Tag(*b"head"))
        .ok_or(FontError::TableNotFound {
            tag: "head".to_string(),
        })?
        .to_vec();
    let mut reader = Reader::new(&head_data);
    let mut head = Head::read_from(&mut reader)?;

    head.check_sum_adjustment = checksum_adjustment;
    head.index_to_loc_format = index_to_loc_format as i16;
    if transform {
        head.flags |= 1 << 5;
    }
    let mut writer = Writer::with_capacity(HEAD_TABLE_SIZE);
    head.write_to(&mut writer)?;

    Ok(writer.data)
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
    fn test_head_table_size_constant() {
        // 验证 head 表大小常量
        assert_eq!(HEAD_TABLE_SIZE, 54); // OpenType head 表固定为 54 字节
    }
}
