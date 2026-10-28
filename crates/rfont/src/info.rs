use rfont_core::{Cmap, Head, Hhea, Maxp, NameTable};
use rfont_types::{FontError, FontFormat, TableRecord, Tag};

use crate::font_data::FontData;
use crate::license::FontRestrictions;

/// 字体表信息
///
/// 包含字体中单个表的元数据，从 TableRecord 转换而来。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TableInfo {
    /// 表标签（如 "head", "cmap"）
    pub tag: String,
    /// 校验和
    pub checksum: u32,
    /// 偏移量
    pub offset: u32,
    /// 长度（字节）
    pub length: u32,
}

impl TableInfo {
    /// 从 TableRecord 创建 TableInfo
    pub fn from_record(record: &TableRecord) -> Self {
        TableInfo {
            tag: String::from_utf8_lossy(&record.tag.0).to_string(),
            checksum: record.checksum,
            offset: record.offset,
            length: record.length,
        }
    }
}

/// 单条 name 表记录（已解码）
///
/// 同一名称可能以多个平台/语言记录存在，前端可按需筛选展示。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct NameEntry {
    /// 平台 ID（0=Unicode，1=Macintosh，3=Windows）
    pub platform_id: u16,
    /// 编码 ID
    pub encoding_id: u16,
    /// 语言 ID（Windows 下 0x0409 为 en-US）
    pub language_id: u16,
    /// 名称 ID（1=family，2=subfamily，9=designer 等）
    pub name_id: u16,
    /// 解码后的文本
    pub value: String,
}

/// 字体基本信息
///
/// 包含字体的关键元数据，如格式、字形数量、度量信息、名称记录等。
/// 通过 `Font::get_font_info()` 方法获取。
///
/// # 示例
/// ```no_run
/// use rfont::Font;
///
/// let font = Font::load("font.ttf").unwrap();
/// let info = font.get_font_info().unwrap();
/// println!("格式: {}", info.format);
/// println!("字形数量: {}", info.glyph_count);
/// println!("字体家族: {:?}", info.family);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FontInfo {
    /// 字体容器格式（TTF / OTF / WOFF / WOFF2）
    pub format: FontFormat,
    /// 当前字面在集合中的索引（单字体恒为 0）
    pub face_index: usize,
    /// 集合中的字面总数（单字体恒为 1）
    pub face_count: usize,
    /// 全部 name 表记录
    pub names: Vec<NameEntry>,
    /// 字体家族名称（nameID 1）
    pub family: Option<String>,
    /// 排版字体家族名称（nameID 16）
    pub typographic_family: Option<String>,
    /// 样式名称（nameID 2）
    pub subfamily: Option<String>,
    /// 完整名称（nameID 4）
    pub full_name: Option<String>,
    /// PostScript 名称（nameID 6）
    pub postscript_name: Option<String>,
    /// 版本字符串（nameID 5）
    pub version: Option<String>,
    /// 设计者/作者（nameID 9）
    pub designer: Option<String>,
    /// 制造商（nameID 8）
    pub manufacturer: Option<String>,
    /// 版权声明（nameID 0）
    pub copyright: Option<String>,
    /// 许可证描述（nameID 13）
    pub license: Option<String>,
    /// 许可证网址（nameID 14）
    pub license_url: Option<String>,
    /// 字形总数
    pub glyph_count: u16,
    /// 每 EM 单位数
    pub units_per_em: u16,
    /// 边界框 (x_min, y_min, x_max, y_max)
    pub bbox: (i16, i16, i16, i16),
    /// ascender（上升高度）
    pub ascender: i16,
    /// descender（下降高度）
    pub descender: i16,
    /// line gap（行间距）
    pub line_gap: i16,
    /// 水平度量数量
    pub number_of_h_metrics: u16,
    /// 支持的 Unicode 字符数量
    pub supported_char_count: usize,
    /// 字重（OS/2.usWeightClass）
    pub weight_class: u16,
    /// 字宽（OS/2.usWidthClass）
    pub width_class: u16,
    /// 嵌入权限位（OS/2.fsType）
    pub fs_type: u16,
    /// 授权与嵌入限制（由 fsType 与许可证记录推导）
    pub restrictions: FontRestrictions,
    /// 所有表的列表（按 tag 排序）
    pub tables: Vec<TableInfo>,
}

impl Default for FontInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl FontInfo {
    /// 创建空的 FontInfo
    pub fn new() -> Self {
        FontInfo {
            format: FontFormat::Ttf,
            face_index: 0,
            face_count: 1,
            names: Vec::new(),
            family: None,
            typographic_family: None,
            subfamily: None,
            full_name: None,
            postscript_name: None,
            version: None,
            designer: None,
            manufacturer: None,
            copyright: None,
            license: None,
            license_url: None,
            glyph_count: 0,
            units_per_em: 0,
            bbox: (0, 0, 0, 0),
            ascender: 0,
            descender: 0,
            line_gap: 0,
            number_of_h_metrics: 0,
            supported_char_count: 0,
            weight_class: 0,
            width_class: 0,
            fs_type: 0,
            restrictions: FontRestrictions::default(),
            tables: Vec::new(),
        }
    }
}

/// 从字体目录提取信息，不依赖 glyf/loca。
///
/// CFF 字体同样适用，只要 head/maxp/hhea/cmap 存在。
pub(crate) fn extract(
    font_data: &FontData,
    format: FontFormat,
    face_index: usize,
    face_count: usize,
) -> Result<FontInfo, FontError> {
    let head: Head = font_data.parse_table(Tag(*b"head"))?;
    let maxp: Maxp = font_data.parse_table(Tag(*b"maxp"))?;
    let hhea: Hhea = font_data.parse_table(Tag(*b"hhea"))?;
    let cmap: Cmap = font_data.parse_table(Tag(*b"cmap"))?;

    let mut info = FontInfo {
        format,
        face_index,
        face_count,
        glyph_count: maxp.num_glyphs,
        units_per_em: head.units_per_em,
        bbox: (head.x_min, head.y_min, head.x_max, head.y_max),
        ascender: hhea.ascender.0,
        descender: hhea.descender.0,
        line_gap: hhea.line_gap.0,
        number_of_h_metrics: hhea.number_of_h_metrics,
        supported_char_count: cmap.supported_chars_count(),
        tables: collect_tables(font_data),
        ..FontInfo::new()
    };

    if let Ok(name_table) = font_data.parse_table::<NameTable>(Tag(*b"name")) {
        info.names = name_entries(&name_table);
        info.family = name_table.get_name_string(1);
        info.typographic_family = name_table.get_name_string(16);
        info.subfamily = name_table.get_name_string(2);
        info.full_name = name_table.get_name_string(4);
        info.postscript_name = name_table.get_name_string(6);
        info.version = name_table.get_name_string(5);
        info.designer = name_table.get_name_string(9);
        info.manufacturer = name_table.get_name_string(8);
        info.copyright = name_table.get_name_string(0);
        info.license = name_table.get_name_string(13);
        info.license_url = name_table.get_name_string(14);
    }

    if let Some(bytes) = font_data.get_table_bytes(Tag(*b"OS/2"))
        && let Some((weight, width, fs_type)) = parse_os2_metrics(bytes)
    {
        info.weight_class = weight;
        info.width_class = width;
        info.fs_type = fs_type;
    }

    info.restrictions = FontRestrictions::new(
        info.fs_type,
        info.license.as_deref(),
        info.license_url.as_deref(),
    );

    Ok(info)
}

/// 解码所有 name 记录并按 nameID、平台、编码、语言排序。
pub(crate) fn name_entries(name_table: &NameTable) -> Vec<NameEntry> {
    let mut entries: Vec<NameEntry> = name_table
        .records()
        .iter()
        .filter_map(|record| {
            name_table.decode_string(record).map(|value| NameEntry {
                platform_id: record.platform_id,
                encoding_id: record.encoding_id,
                language_id: record.language_id,
                name_id: record.name_id,
                value,
            })
        })
        .collect();

    entries.sort_by_key(|e| (e.name_id, e.platform_id, e.encoding_id, e.language_id));
    entries
}

pub(crate) fn collect_tables(font_data: &FontData) -> Vec<TableInfo> {
    let mut tables: Vec<TableInfo> = font_data
        .get_table_records()
        .values()
        .map(TableInfo::from_record)
        .collect();
    tables.sort_by(|a, b| a.tag.cmp(&b.tag));
    tables
}

/// 读取 OS/2 中与展示相关的最小字段：字重、字宽、嵌入权限。
fn parse_os2_metrics(bytes: &[u8]) -> Option<(u16, u16, u16)> {
    let weight = u16::from_be_bytes([*bytes.get(4)?, *bytes.get(5)?]);
    let width = u16::from_be_bytes([*bytes.get(6)?, *bytes.get(7)?]);
    let fs_type = u16::from_be_bytes([*bytes.get(8)?, *bytes.get(9)?]);
    Some((weight, width, fs_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os2_metrics_rejects_short_table() {
        assert!(parse_os2_metrics(&[0; 9]).is_none());
    }

    #[test]
    fn test_os2_metrics_reads_big_endian_fields() {
        let mut data = vec![0u8; 10];
        data[4..6].copy_from_slice(&700u16.to_be_bytes());
        data[6..8].copy_from_slice(&5u16.to_be_bytes());
        data[8..10].copy_from_slice(&0x0004u16.to_be_bytes());

        assert_eq!(parse_os2_metrics(&data), Some((700, 5, 4)));
    }
}
