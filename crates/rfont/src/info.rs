use rfont_types::TableRecord;

/// 字体表信息
///
/// 包含字体中单个表的元数据，从 TableRecord 转换而来。
#[derive(Debug, Clone)]
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
    ///
    /// # 参数
    /// - `record`: 原始的 TableRecord
    ///
    /// # 返回值
    /// 转换后的 `TableInfo` 实例
    pub fn from_record(record: &TableRecord) -> Self {
        let tag_str = String::from_utf8_lossy(&record.tag.0).to_string();
        TableInfo {
            tag: tag_str,
            checksum: record.checksum,
            offset: record.offset,
            length: record.length,
        }
    }
}

/// 字体基本信息
///
/// 包含字体的关键元数据，如字形数量、度量信息、支持的字符等。
/// 通过 `Font::get_font_info()` 方法获取。
///
/// # 示例
/// ```no_run
/// use rfont::Font;
///
/// let font = Font::load("font.ttf").unwrap();
/// let info = font.get_font_info();
/// println!("字形数量: {}", info.glyph_count);
/// println!("每 EM 单位: {}", info.units_per_em);
/// println!("支持字符数: {}", info.supported_char_count);
/// ```
#[derive(Debug, Clone)]
pub struct FontInfo {
    /// 字体家族名称（从 name 表提取）
    pub family_name: Option<String>,
    /// 字体样式名称（从 name 表提取）
    pub style_name: Option<String>,
    /// 字体版本
    pub version: Option<String>,
    /// 字形总数
    pub glyph_count: u16,
    /// 每 EM 单位数
    pub units_per_em: u16,
    /// 最小 x 坐标
    pub x_min: i16,
    /// 最小 y 坐标
    pub y_min: i16,
    /// 最大 x 坐标
    pub x_max: i16,
    /// 最大 y 坐标
    pub y_max: i16,
    /// 水平度量数量
    pub number_of_h_metrics: u16,
    /// ascender（上升高度）
    pub ascender: i16,
    /// descender（下降高度）
    pub descender: i16,
    /// line gap（行间距）
    pub line_gap: i16,
    /// 所有表的列表
    pub tables: Vec<TableInfo>,
    /// 支持的 Unicode 字符数量
    pub supported_char_count: usize,
}

impl Default for FontInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl FontInfo {
    /// 创建空的 FontInfo
    ///
    /// 所有字段初始化为默认值（0 或 None）。
    ///
    /// # 返回值
    /// 空的 `FontInfo` 实例
    pub fn new() -> Self {
        FontInfo {
            family_name: None,
            style_name: None,
            version: None,
            glyph_count: 0,
            units_per_em: 0,
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
            number_of_h_metrics: 0,
            ascender: 0,
            descender: 0,
            line_gap: 0,
            tables: Vec::new(),
            supported_char_count: 0,
        }
    }
}
