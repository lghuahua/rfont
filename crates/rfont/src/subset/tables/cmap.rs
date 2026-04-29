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

use crate::Font;
use rfont_types::FontError;
use std::collections::HashSet;

use crate::constants::{CMAP_HEADER_SIZE, ENCODING_RECORD_SIZE};

/// 重建 cmap 表（智能选择最佳格式）
pub fn rebuild_cmap(font: &Font, subset_glyphs: &[u16]) -> Result<Vec<u8>, FontError> {
    let subset_set: HashSet<u16> = subset_glyphs.iter().copied().collect();
    
    // 过滤出子集中存在的映射
    let mut new_unicode_map: Vec<(u32, u16)> = font.cmap.unicode_map.iter()
        .filter(|(_, &gid)| subset_set.contains(&gid))
        .map(|(&unicode, &gid)| (unicode, gid))
        .collect();
    
    new_unicode_map.sort_by_key(|&(unicode, _)| unicode);
    
    if new_unicode_map.is_empty() {
        return Err(FontError::Generic("No glyphs in cmap".to_string()));
    }

    println!("  cmap 字符统计: {} 个字符", new_unicode_map.len());
    
    // 检查是否有非 BMP 字符（> 0xFFFF）
    let has_non_bmp = new_unicode_map.iter().any(|&(unicode, _)| unicode > 0xFFFF);
    
    // 检查是否所有字符都在 0-255 范围内且数量较少
    let all_in_byte_range = new_unicode_map.iter().all(|&(unicode, _)| unicode <= 255);
    
    // 智能选择格式
    if has_non_bmp {
        // 有非 BMP 字符，使用 Format 12
        println!("  cmap 格式: Format 12 (支持 Unicode 补充平面)");
        build_cmap_format12(&new_unicode_map)
    } else if all_in_byte_range && new_unicode_map.len() <= 256 {
        // 所有字符在字节范围内，使用 Format 0
        println!("  cmap 格式: Format 0 (简单字节映射)");
        build_cmap_format0(&new_unicode_map)
    } else {
        // 默认使用 Format 4
        let segments = build_cmap_segments(&new_unicode_map);
        println!("  cmap 格式: Format 4 ({} 个段)", segments.len());
        build_cmap_format4(&segments)
    }
}

/// 构建 cmap 段（合并连续的码点）
fn build_cmap_segments(unicode_map: &[(u32, u16)]) -> Vec<CmapSegment> {
    if unicode_map.is_empty() {
        return vec![];
    }
    
    let mut segments = Vec::new();
    let mut i = 0;
    
    while i < unicode_map.len() {
        let start_code = unicode_map[i].0 as u16;
        let start_gid = unicode_map[i].1;
        let mut end_code = start_code;
        let mut j = i + 1;
        
        // 向后查找连续的码点
        while j < unicode_map.len() {
            let next_code = unicode_map[j].0 as u16;
            let next_gid = unicode_map[j].1;
            
            // 检查是否连续：码点连续 且 glyph ID 差值相同
            if next_code == end_code + 1 && 
               (next_gid as i32 - start_gid as i32) == (end_code as i32 - start_code as i32) {
                end_code = next_code;
                j += 1;
            } else {
                break;
            }
        }
        
        // 计算 id_delta
        let id_delta = (start_gid as i32 - start_code as i32) as i16;
        
        segments.push(CmapSegment {
            start_code,
            end_code,
            id_delta,
            id_range_offset: 0, // 不使用 glyph_id_array
        });
        
        i = j;
    }
    
    segments
}

/// 构建 Format 4 cmap 子表
fn build_cmap_format4(segments: &[CmapSegment]) -> Result<Vec<u8>, FontError> {
    let seg_count = segments.len();
    
    // 计算 search_range, entry_selector, range_shift
    let max_pow2 = if seg_count > 0 { 
        (seg_count as f32).log2().floor().exp2() as usize 
    } else { 
        1 
    };
    let search_range = 2 * max_pow2;
    let entry_selector = max_pow2.trailing_zeros() as u16;
    let range_shift = (2 * seg_count).saturating_sub(search_range) as u16;

    let mut data = Vec::new();
    
    // cmap header
    data.extend_from_slice(&0u16.to_be_bytes()); // version
    data.extend_from_slice(&1u16.to_be_bytes()); // num_tables
    
    // Encoding Record
    data.extend_from_slice(&3u16.to_be_bytes()); // platform_id (Unicode)
    data.extend_from_slice(&1u16.to_be_bytes()); // encoding_id (Unicode BMP)
    let subtable_offset = (CMAP_HEADER_SIZE + ENCODING_RECORD_SIZE) as u32;
    data.extend_from_slice(&subtable_offset.to_be_bytes());
    
    // Format 4 subtable
    let subtable_start = data.len();
    data.extend_from_slice(&4u16.to_be_bytes()); // format
    data.extend_from_slice(&0u16.to_be_bytes()); // reserved
    // length 和 language 稍后回填
    
    let seg_count_x2 = (seg_count * 2) as u16;
    data.extend_from_slice(&seg_count_x2.to_be_bytes()); // segCountX2
    data.extend_from_slice(&(search_range as u16).to_be_bytes());
    data.extend_from_slice(&entry_selector.to_be_bytes());
    data.extend_from_slice(&range_shift.to_be_bytes());
    
    // 写入 end_code 数组
    for seg in segments {
        data.extend_from_slice(&seg.end_code.to_be_bytes());
    }
    data.extend_from_slice(&0xFFFFu16.to_be_bytes()); // sentinel
    
    // 写入 reservedPad
    data.extend_from_slice(&0u16.to_be_bytes());
    
    // 写入 start_code 数组
    for seg in segments {
        data.extend_from_slice(&seg.start_code.to_be_bytes());
    }
    data.extend_from_slice(&0xFFFFu16.to_be_bytes()); // sentinel
    
    // 写入 id_delta 数组
    for seg in segments {
        data.extend_from_slice(&seg.id_delta.to_be_bytes());
    }
    data.extend_from_slice(&0u16.to_be_bytes()); // sentinel
    
    // 写入 id_range_offset 数组
    for seg in segments {
        data.extend_from_slice(&seg.id_range_offset.to_be_bytes());
    }
    data.extend_from_slice(&0u16.to_be_bytes()); // sentinel
    
    // glyph_id_array（为空，因为使用 id_delta）
    
    // 回填 length 和 language
    let subtable_length = (data.len() - subtable_start) as u16;
    data[subtable_start+2..subtable_start+4].copy_from_slice(&subtable_length.to_be_bytes());
    data[subtable_start+4..subtable_start+6].copy_from_slice(&0u16.to_be_bytes()); // language
    
    Ok(data)
}

/// 构建 Format 0 cmap 子表（简单字节映射）
fn build_cmap_format0(unicode_map: &[(u32, u16)]) -> Result<Vec<u8>, FontError> {
    let mut data = Vec::new();
    
    // cmap header
    data.extend_from_slice(&0u16.to_be_bytes()); // version
    data.extend_from_slice(&1u16.to_be_bytes()); // num_tables
    
    // Encoding Record (Mac Roman)
    data.extend_from_slice(&1u16.to_be_bytes()); // platform_id (Macintosh)
    data.extend_from_slice(&0u16.to_be_bytes()); // encoding_id (Roman)
    let subtable_offset = (CMAP_HEADER_SIZE + ENCODING_RECORD_SIZE) as u32;
    data.extend_from_slice(&subtable_offset.to_be_bytes());
    
    // Format 0 subtable
    data.extend_from_slice(&0u16.to_be_bytes()); // format
    data.extend_from_slice(&262u16.to_be_bytes()); // length (6 + 256)
    data.extend_from_slice(&0u16.to_be_bytes()); // language
    
    // glyphIdArray (256 bytes)
    let mut glyph_array = [0u8; 256];
    for &(unicode, gid) in unicode_map {
        if unicode < 256 {
            glyph_array[unicode as usize] = gid as u8;
        }
    }
    data.extend_from_slice(&glyph_array);
    
    Ok(data)
}

/// 构建 Format 12 cmap 子表（支持 Unicode 补充平面）
fn build_cmap_format12(unicode_map: &[(u32, u16)]) -> Result<Vec<u8>, FontError> {
    // 构建连续的组（groups）
    let groups = build_cmap_groups(unicode_map);
    
    println!("  cmap Format 12: {} 个字符 -> {} 个组", unicode_map.len(), groups.len());
    
    let mut data = Vec::new();
    
    // cmap header
    data.extend_from_slice(&0u16.to_be_bytes()); // version
    data.extend_from_slice(&1u16.to_be_bytes()); // num_tables
    
    // Encoding Record (Unicode Full Repertoire)
    data.extend_from_slice(&3u16.to_be_bytes()); // platform_id (Microsoft)
    data.extend_from_slice(&10u16.to_be_bytes()); // encoding_id (Unicode UCS-4)
    let subtable_offset = (CMAP_HEADER_SIZE + ENCODING_RECORD_SIZE) as u32;
    data.extend_from_slice(&subtable_offset.to_be_bytes());
    
    // Format 12 subtable
    let subtable_start = data.len();
    data.extend_from_slice(&12u16.to_be_bytes()); // format
    data.extend_from_slice(&0u16.to_be_bytes()); // reserved
    
    // length 和 nGroups 稍后回填
    let length_pos = data.len();
    data.extend_from_slice(&0u32.to_be_bytes()); // length placeholder
    data.extend_from_slice(&0u32.to_be_bytes()); // language
    data.extend_from_slice(&(groups.len() as u32).to_be_bytes()); // nGroups
    
    // 写入 SequentialMapGroup 数组
    for group in &groups {
        data.extend_from_slice(&group.start_char_code.to_be_bytes());
        data.extend_from_slice(&group.end_char_code.to_be_bytes());
        data.extend_from_slice(&group.start_glyph_id.to_be_bytes());
    }
    
    // 回填 length
    let subtable_length = (data.len() - subtable_start) as u32;
    data[length_pos..length_pos+4].copy_from_slice(&subtable_length.to_be_bytes());
    
    Ok(data)
}

/// 构建 Format 12 的连续组
fn build_cmap_groups(unicode_map: &[(u32, u16)]) -> Vec<CmapGroup> {
    if unicode_map.is_empty() {
        return vec![];
    }
    
    let mut groups = Vec::new();
    let mut i = 0;
    
    while i < unicode_map.len() {
        let start_char_code = unicode_map[i].0;
        let start_glyph_id = unicode_map[i].1 as u32;
        let mut end_char_code = start_char_code;
        let mut end_glyph_id = start_glyph_id;
        let mut j = i + 1;
        
        // 向后查找连续的码点和 glyph ID
        while j < unicode_map.len() {
            let next_char = unicode_map[j].0;
            let next_gid = unicode_map[j].1 as u32;
            
            if next_char == end_char_code + 1 && next_gid == end_glyph_id + 1 {
                end_char_code = next_char;
                end_glyph_id = next_gid;
                j += 1;
            } else {
                break;
            }
        }
        
        groups.push(CmapGroup {
            start_char_code,
            end_char_code,
            start_glyph_id,
        });
        
        i = j;
    }
    
    groups
}
