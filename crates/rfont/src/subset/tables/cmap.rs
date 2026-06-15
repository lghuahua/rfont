use crate::Font;
use rfont_core::Cmap;
use rfont_types::{FontError, WriteBytes, Writer};
use std::collections::{HashMap};


fn rebuild_cmap_subtable(
    old_map: &HashMap<u32, u16>,
    old_to_new: &HashMap<u16, u16>,
) -> HashMap<u32, u16> {
    let mut new_map = HashMap::new();
    for (char_code, &old_glyph_id) in old_map {
        if let Some(&new_glyph_id) = old_to_new.get(&old_glyph_id) {
            new_map.insert(*char_code, new_glyph_id);
        }
    }
    new_map
}

/// 重建 cmap 表（智能选择最佳格式）
pub fn rebuild_cmap(font: &Font, subset_glyphs: &[u16]) -> Result<Vec<u8>, FontError> {
    // let subset_set: HashSet<u16> = subset_glyphs.iter().filter(|&&gid| gid != 0).copied().collect();

    // 创建原始 glyph ID 到新 glyph ID 的映射
    // subset_glyphs 是按顺序排列的，索引就是新的 glyph ID
    let mut old_to_new_gid = HashMap::new();
    for (new_gid, &old_gid) in subset_glyphs.iter().enumerate() {
        if old_gid != 0  { 
            old_to_new_gid.insert(old_gid, new_gid as u16) ;
        }
    }

    let mut subtables = HashMap::new();

    font.cmap.subtables.iter().for_each(|(key, subtable)| { 
        let new_map = rebuild_cmap_subtable(subtable, &old_to_new_gid);
        if !new_map.is_empty() {
            subtables.insert(*key, new_map);
        }
    });

    let new_cmap = Cmap {
        subtables,
    };

    let mut writer = Writer::new();
    new_cmap.write_to(&mut writer)?;
    Ok(writer.data)

}
