use crate::Font;
use rfont_core::Head;
use rfont_types::{FontError, HEAD_TABLE_SIZE, ReadBytes, Reader, Tag, WriteBytes, Writer};

/// 更新 head 表（校验和调整、loca 格式与修改时间）
pub fn update_head(font: &Font, index_to_loc_format: u16) -> Result<Vec<u8>, FontError> {
    let head_data = font
        .font_data
        .get_table_bytes(Tag(*b"head"))
        .ok_or(FontError::TableNotFound {
            tag: "head".to_string(),
        })?
        .to_vec();
    let mut reader = Reader::new(&head_data);
    let mut head = Head::read_from(&mut reader)?;

    head.check_sum_adjustment = 0;
    head.index_to_loc_format = index_to_loc_format as i16;
    head.modified = crate::clock::now();

    let mut writer = Writer::with_capacity(HEAD_TABLE_SIZE);
    head.write_to(&mut writer)?;

    Ok(writer.data)
}
