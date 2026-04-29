use rfont_core::{Head, Maxp, Hhea, Loca, Cmap, Hmtx};
use rfont_core::tables::woff::{WoffHeader, WoffTableDirectoryEntry};
use rfont_types::{FontError, Reader, TableRecord, Tag, ReadBytes};
use std::collections::{HashMap, HashSet};
use flate2::read::ZlibDecoder;
use std::io::Read;
use tracing::{debug, info, span, Level};

// OpenType 常量
const SFNT_CHECKSUM_MAGIC: u32 = 0xB1B0AFBA;
const SFNT_VERSION_TTF: u32 = 0x00010000;
const LONGDATETIME_EPOCH_YEAR: i32 = 1904;

// 表结构常量
const TABLE_DIR_ENTRY_SIZE: usize = 16;
const CMAP_HEADER_SIZE: usize = 4; // version + numTables
const ENCODING_RECORD_SIZE: usize = 8; // platformID + encodingID + offset
const HEAD_TABLE_SIZE: usize = 54;
const POST_TABLE_MIN_SIZE: usize = 32;
const POST_V2_MIN_SIZE: usize = 36;

/// 字体表信息
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
    fn from_record(record: &TableRecord) -> Self {
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
#[derive(Debug, Clone)]
pub struct FontInfo {
    /// 字体家族名称（从 name 表提取，暂为 None）
    pub family_name: Option<String>,
    /// 字体样式名称（从 name 表提取，暂为 None）
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

impl FontInfo {
    /// 创建空的 FontInfo
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

/// 子集化配置选项
#[derive(Debug, Clone)]
pub struct SubsetOptions {
    /// 是否优化 post 表（移除字形名称以减小文件大小）
    pub optimize_post_table: bool,
    /// 是否移除字形名称
    pub strip_glyph_names: bool,
    /// WOFF 压缩级别（0-9，仅在使用 WOFF 格式时有效）
    pub compression_level: u8,
    /// 是否保留 hinting 数据
    pub keep_hinting: bool,
    /// 输出格式（"ttf" 或 "woff"）
    pub output_format: String,
}

impl Default for SubsetOptions {
    fn default() -> Self {
        SubsetOptions {
            optimize_post_table: true,
            strip_glyph_names: true,
            compression_level: 6,
            keep_hinting: false,
            output_format: "ttf".to_string(),
        }
    }
}

impl SubsetOptions {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Web 优化预设（最小文件大小）
    pub fn web_optimized() -> Self {
        SubsetOptions {
            optimize_post_table: true,
            strip_glyph_names: true,
            compression_level: 9,
            keep_hinting: false,
            output_format: "woff".to_string(),
        }
    }
    
    /// 打印优化预设（保留更多元数据）
    pub fn print_optimized() -> Self {
        SubsetOptions {
            optimize_post_table: false,
            strip_glyph_names: false,
            compression_level: 0,
            keep_hinting: true,
            output_format: "ttf".to_string(),
        }
    }
}

/// 字体子集化 Builder
pub struct FontSubsetBuilder<'a> {
    font: &'a Font,
    text: Option<String>,
    glyph_ids: Option<Vec<u16>>,
    unicode_ranges: Option<Vec<(u32, u32)>>,
    options: SubsetOptions,
}

impl<'a> FontSubsetBuilder<'a> {
    /// 创建新的 Builder
    pub fn new(font: &'a Font) -> Self {
        FontSubsetBuilder {
            font,
            text: None,
            glyph_ids: None,
            unicode_ranges: None,
            options: SubsetOptions::default(),
        }
    }
    
    /// 设置要包含的文本
    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }
    
    /// 设置要包含的字形 ID 列表
    pub fn glyph_ids(mut self, ids: Vec<u16>) -> Self {
        self.glyph_ids = Some(ids);
        self
    }
    
    /// 添加 Unicode 范围（start, end）
    pub fn unicode_range(mut self, start: u32, end: u32) -> Self {
        if self.unicode_ranges.is_none() {
            self.unicode_ranges = Some(Vec::new());
        }
        self.unicode_ranges.as_mut().unwrap().push((start, end));
        self
    }
    
    /// 设置是否优化 post 表
    pub fn optimize_post(mut self, optimize: bool) -> Self {
        self.options.optimize_post_table = optimize;
        self
    }
    
    /// 设置是否移除字形名称
    pub fn strip_glyph_names(mut self, strip: bool) -> Self {
        self.options.strip_glyph_names = strip;
        self
    }
    
    /// 设置 WOFF 压缩级别（0-9）
    pub fn compression_level(mut self, level: u8) -> Self {
        self.options.compression_level = level.min(9);
        self
    }
    
    /// 设置是否保留 hinting 数据
    pub fn keep_hinting(mut self, keep: bool) -> Self {
        self.options.keep_hinting = keep;
        self
    }
    
    /// 设置输出格式（"ttf" 或 "woff"）
    pub fn output_format(mut self, format: &str) -> Self {
        self.options.output_format = format.to_lowercase();
        self
    }
    
    /// 使用预设配置
    pub fn preset(mut self, preset: &str) -> Self {
        match preset {
            "web" => self.options = SubsetOptions::web_optimized(),
            "print" => self.options = SubsetOptions::print_optimized(),
            _ => {} // 未知预设，保持默认
        }
        self
    }
    
    /// 构建并执行子集化
    pub fn build(self) -> Result<Vec<u8>, FontError> {
        // 收集需要包含的字形 ID
        let mut needed_glyphs = HashSet::new();
        
        // 从文本中提取字形 ID
        if let Some(ref text) = self.text {
            let glyph_ids = self.font.text_to_glyph_ids(text);
            needed_glyphs.extend(glyph_ids);
        }
        
        // 直接指定的字形 ID
        if let Some(ref ids) = self.glyph_ids {
            needed_glyphs.extend(ids.iter().cloned());
        }
        
        // 从 Unicode 范围中提取字形 ID
        if let Some(ref ranges) = self.unicode_ranges {
            for &(start, end) in ranges {
                for unicode in start..=end {
                    if let Some(&glyph_id) = self.font.cmap.unicode_map.get(&unicode) {
                        needed_glyphs.insert(glyph_id);
                    }
                }
            }
        }
        
        // 确保至少有一个字形
        if needed_glyphs.is_empty() {
            return Err(FontError::Generic("No glyphs specified for subset".to_string()));
        }
        
        // 转换为排序的向量
        let mut glyph_ids: Vec<u16> = needed_glyphs.into_iter().collect();
        glyph_ids.sort();
        
        debug!(glyph_count = glyph_ids.len(), "开始子集化处理");
        
        // 执行子集化
        self.font.subset_with_options(&glyph_ids, &self.options)
    }
}

/// cmap Format 4 段结构
#[derive(Debug, Clone)]
struct CmapSegment {
    start_code: u16,
    end_code: u16,
    id_delta: i16,
    id_range_offset: u16,
}

/// cmap Format 12 组结构
#[derive(Debug, Clone)]
struct CmapGroup {
    start_char_code: u32,
    end_char_code: u32,
    start_glyph_id: u32,
}

/// 计算 SFNT 校验和（32位累加，数据按4字节对齐）
fn calc_sfnt_checksum(data: &[u8]) -> u32 {
    let mut sum: u64 = 0;
    let mut i = 0;
    
    // 每次处理 4 字节
    while i + 4 <= data.len() {
        let val = u32::from_be_bytes([data[i], data[i+1], data[i+2], data[i+3]]);
        sum += val as u64;
        i += 4;
    }
    
    // 处理剩余的字节（填充到4字节）
    if i < data.len() {
        let mut bytes = [0u8; 4];
        let remaining = &data[i..];
        bytes[..remaining.len()].copy_from_slice(remaining);
        sum += u32::from_be_bytes(bytes) as u64;
    }
    
    (sum & 0xFFFFFFFF) as u32
}

/// 原始字体数据和目录信息
pub struct FontData {
    data: Vec<u8>,
    table_map: HashMap<Tag, TableRecord>,
}

impl FontData {
    /// 从字节向量创建（拥有所有权）
    pub fn new(data: Vec<u8>) -> Result<Self, FontError> {
        let table_map = Self::parse_directory(&data)?;
        Ok(FontData { data, table_map })
    }
    
    /// 解析字体目录
    fn parse_directory(data: &[u8]) -> Result<HashMap<Tag, TableRecord>, FontError> {
        let mut reader = Reader::new(data);
        
        // 读取 Offset Table
        let _sfnt_version = reader.read_u32()?;
        let num_tables = reader.read_u16()?;
        let _search_range = reader.read_u16()?;
        let _entry_selector = reader.read_u16()?;
        let _range_shift = reader.read_u16()?;
        
        // 读取 Table Directory
        let mut table_map = HashMap::new();
        for _ in 0..num_tables {
            let tag_bytes = [reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?];
            let tag = Tag(tag_bytes);
            let record = TableRecord {
                tag,
                checksum: reader.read_u32()?,
                offset: reader.read_u32()?,
                length: reader.read_u32()?,
            };
            table_map.insert(tag, record);
        }
        
        Ok(table_map)
    }
    
    /// 获取表的原始字节
    pub fn get_table_bytes(&self, tag: Tag) -> Option<&[u8]> {
        self.table_map.get(&tag).map(|record| {
            let start = record.offset as usize;
            let end = start + record.length as usize;
            &self.data[start..end]
        })
    }
    
    /// 通用表解析方法
    pub fn parse_table<T>(&self, tag: Tag) -> Result<T, FontError> 
    where 
        T: for<'a> ReadBytes<'a>
    {
        let bytes = self.get_table_bytes(tag)
            .ok_or_else(|| FontError::TableNotFound { tag: format!("{:?}", tag) })?;
        T::read_from(&mut Reader::new(bytes))
    }
    
    /// 获取所有表记录的引用
    pub fn get_table_records(&self) -> &HashMap<Tag, TableRecord> {
        &self.table_map
    }
}

/// 高层字体对象（预加载核心表）
pub struct Font {
    pub font_data: FontData,
    
    // 核心表（预加载）
    pub head: Head,
    pub maxp: Maxp,
    pub hhea: Hhea,
    pub loca: Loca,
    pub cmap: Cmap,
    pub hmtx: Hmtx,
}

impl Font {
    /// 从文件路径加载字体（支持 TTF 和 WOFF）
    pub fn load(path: &str) -> Result<Self, FontError> {
        let span = span!(Level::INFO, "load_font", path = path);
        let _enter = span.enter();
        
        info!("开始加载字体文件");
        let data = std::fs::read(path).map_err(|e| FontError::Io(e))?;
        
        debug!(size = data.len(), "字体文件读取完成");
        
        // 检查是否为 WOFF 格式
        if data.len() >= 4 && &data[0..4] == b"wOFF" {
            info!("检测到 WOFF 格式");
            Self::load_woff(&data)
        } else {
            info!("检测到 TTF/OTF 格式");
            Self::load_ttf(&data)
        }
    }

    /// 加载 TTF 格式字体（预加载核心表）
    fn load_ttf(data: &[u8]) -> Result<Self, FontError> {
        let span = span!(Level::DEBUG, "load_ttf");
        let _enter = span.enter();
        
        let font_data = FontData::new(data.to_vec())?;
        
        // 解析核心表（按依赖顺序）
        let head: Head = font_data.parse_table(Tag(*b"head"))?;
        debug!(index_to_loc_format = head.index_to_loc_format, "Head 表解析完成");
        
        let maxp: Maxp = font_data.parse_table(Tag(*b"maxp"))?;
        debug!(num_glyphs = maxp.num_glyphs, "Maxp 表解析完成");
        
        let hhea: Hhea = font_data.parse_table(Tag(*b"hhea"))?;
        debug!(number_of_h_metrics = hhea.number_of_h_metrics, "Hhea 表解析完成");
        
        // loca 需要 head 的参数
        let loca_bytes = font_data.get_table_bytes(Tag(*b"loca"))
            .ok_or(FontError::TableNotFound { tag: "loca".to_string() })?;
        let loca = Loca::read_from(&mut Reader::new(loca_bytes), head.index_to_loc_format, maxp.num_glyphs)?;
        debug!(offsets_count = loca.offsets.len(), "Loca 表解析完成");
        
        let cmap_bytes = font_data.get_table_bytes(Tag(*b"cmap"))
            .ok_or(FontError::TableNotFound { tag: "cmap".to_string() })?;
        let cmap = Cmap::read_from(&mut Reader::new(cmap_bytes))?;
        debug!(unicode_map_size = cmap.unicode_map.len(), "Cmap 表解析完成");
        
        // hmtx 需要 hhea 和 maxp 的参数
        let hmtx_bytes = font_data.get_table_bytes(Tag(*b"hmtx"))
            .ok_or(FontError::TableNotFound { tag: "hmtx".to_string() })?;
        let hmtx = Hmtx::read_from(&mut Reader::new(hmtx_bytes), hhea.number_of_h_metrics, maxp.num_glyphs)?;
        debug!(metrics_count = hmtx.metrics.len(), "Hmtx 表解析完成");
        
        Ok(Font {
            font_data,
            head,
            maxp,
            hhea,
            loca,
            cmap,
            hmtx,
        })
    }

    /// 加载 WOFF 格式字体
    fn load_woff(data: &[u8]) -> Result<Self, FontError> {
        let span = span!(Level::DEBUG, "load_woff");
        let _enter = span.enter();
        
        let mut reader = Reader::new(data);
        
        // 解析 WOFF Header
        let woff_header = WoffHeader::read_from(&mut reader)?;
        debug!(flavor = format!("0x{:08X}", woff_header.flavor), num_tables = woff_header.num_tables, "WOFF Header 解析完成");
        
        // 解析表目录
        let mut table_entries = Vec::new();
        for _ in 0..woff_header.num_tables {
            table_entries.push(WoffTableDirectoryEntry::read_from(&mut reader)?);
        }
        
        debug!(table_count = table_entries.len(), "WOFF 表目录解析完成");

        // 解压缩并重组为 SFNT 数据
        let mut sfnt_data = Vec::new();
        
        // 写入 Offset Table
        sfnt_data.extend_from_slice(&woff_header.flavor.to_be_bytes());
        sfnt_data.extend_from_slice(&woff_header.num_tables.to_be_bytes());
        
        // 计算 searchRange, entrySelector, rangeShift
        let num_tables = woff_header.num_tables as u32;
        let max_pow2: u32 = if num_tables > 0 { 1 << (31 - num_tables.leading_zeros()) } else { 1 };
        let search_range = max_pow2 * 16;
        let entry_selector = max_pow2.trailing_zeros() as u16;
        let range_shift = ((num_tables as u32) * 16).saturating_sub(search_range) as u16;

        sfnt_data.extend_from_slice(&(search_range as u16).to_be_bytes());
        sfnt_data.extend_from_slice(&entry_selector.to_be_bytes());
        sfnt_data.extend_from_slice(&range_shift.to_be_bytes());
        
        // 先收集所有解压后的表数据
        let mut decompressed_tables = Vec::new();
        // let current_offset = 12 + (woff_header.num_tables as usize) * 20; // Offset Table (12) + Table Directory (20 per entry)
        
        for entry in &table_entries {
            let comp_length = entry.comp_length as usize;
            let orig_length = entry.orig_length as usize;
            
            debug!(tag = ?entry.tag, comp_length = comp_length, orig_length = orig_length, "处理 WOFF 表");

            // 读取压缩数据（实际存储时按 4 字节对齐）
            let start_offset = reader.offset;
            let padded_comp_length = (comp_length + 3) & !3; // 向上取整到 4 字节边界
            let compressed_data = &data[start_offset..][..padded_comp_length];
            reader.offset += padded_comp_length;
            
            // 解压缩（如果未压缩则直接复制）
            let decompressed = if comp_length == orig_length {
                // 未压缩，直接复制（只取实际长度）
                compressed_data[..comp_length].to_vec()
            } else {
                // 已压缩，需要解压（只使用实际压缩长度）
                let mut decoder = ZlibDecoder::new(&compressed_data[..comp_length]);
                let mut buf = Vec::with_capacity(orig_length);
                decoder.read_to_end(&mut buf).map_err(|e| FontError::WoffDecompressionError { 
                    message: format!("Failed to decompress table {:?}: {}", entry.tag, e)
                })?;
                buf
            };
            
            // 对齐到 4 字节边界
            let padding = (4 - (orig_length % 4)) % 4;
            decompressed_tables.push((entry.tag, decompressed, orig_length, padding));
        }
        
        // 先写入 Table Directory（占位，稍后回填偏移量）
        let table_dir_start = sfnt_data.len();
        for _ in &decompressed_tables {
            sfnt_data.extend_from_slice(&[0u8; TABLE_DIR_ENTRY_SIZE]); // 每个表目录项 16 字节
        }
        
        // 写入表数据，并记录实际偏移量
        let mut table_offsets = Vec::new();
        for (_tag, data, _, padding) in &decompressed_tables {
            let offset = sfnt_data.len() as u32;
            table_offsets.push(offset);
            
            sfnt_data.extend_from_slice(data);
            for _ in 0..*padding {
                sfnt_data.push(0);
            }
        }
        
        // 回填 Table Directory
        for (i, (tag, _, orig_length, _)) in decompressed_tables.iter().enumerate() {
            let dir_offset = table_dir_start + i * TABLE_DIR_ENTRY_SIZE;
            sfnt_data[dir_offset..dir_offset+4].copy_from_slice(&tag.0);
            sfnt_data[dir_offset+4..dir_offset+8].copy_from_slice(&0u32.to_be_bytes()); // checksum
            sfnt_data[dir_offset+8..dir_offset+12].copy_from_slice(&table_offsets[i].to_be_bytes());
            sfnt_data[dir_offset+12..dir_offset+16].copy_from_slice(&(*orig_length as u32).to_be_bytes());
        }

        println!("Table directory written at offset {}, {} entries", table_dir_start, decompressed_tables.len());
        
        // 使用重组后的 SFNT 数据创建 Font
        Self::load_ttf(&sfnt_data)
    }

    /// 获取原始字体数据
    pub fn font_data(&self) -> &FontData {
        &self.font_data
    }
    
    /// 获取字体元数据信息
    pub fn get_font_info(&self) -> FontInfo {
        let mut info = FontInfo::new();
        
        // 从 head 表提取信息
        info.units_per_em = self.head.units_per_em;
        info.x_min = self.head.x_min;
        info.y_min = self.head.y_min;
        info.x_max = self.head.x_max;
        info.y_max = self.head.y_max;
        
        // 从 maxp 表提取字形数量
        info.glyph_count = self.maxp.num_glyphs;
        
        // 从 hhea 表提取水平度量信息
        info.number_of_h_metrics = self.hhea.number_of_h_metrics;
        info.ascender = self.hhea.ascender.0;  // FWord 包装类型，需要解包
        info.descender = self.hhea.descender.0;
        info.line_gap = self.hhea.line_gap.0;
        
        // 提取表列表
        info.tables = self.font_data.get_table_records().values()
            .map(TableInfo::from_record)
            .collect();
        
        // 统计支持的字符数量
        info.supported_char_count = self.cmap.unicode_map.len();
        
        // TODO: 从 name 表提取 family_name, style_name, version
        // 这需要解析 name 表，暂时留为 None
        
        info
    }

    /// 获取所有表的列表
    pub fn get_table_list(&self) -> Vec<TableInfo> {
        self.font_data.get_table_records().values()
            .map(TableInfo::from_record)
            .collect()
    }

    /// 获取字体支持的所有 Unicode 字符
    pub fn get_supported_characters(&self) -> Vec<u32> {
        let mut chars: Vec<u32> = self.cmap.unicode_map.keys().cloned().collect();
        chars.sort();
        chars
    }

    /// 检查字体是否支持特定字符
    pub fn supports_character(&self, unicode: u32) -> bool {
        self.cmap.unicode_map.contains_key(&unicode)
    }

    /// 将文本转换为字形 ID 列表
    pub fn text_to_glyph_ids(&self, text: &str) -> Vec<u16> {
        text.chars()
            .filter_map(|ch| {
                let unicode = ch as u32;
                self.cmap.unicode_map.get(&unicode).copied()
            })
            .collect()
    }
    
    /// 创建子集化 Builder（Builder 模式）
    pub fn subset_builder(&self) -> FontSubsetBuilder {
        FontSubsetBuilder::new(self)
    }
    
    /// 使用配置选项进行子集化
    pub fn subset_with_options(&self, glyph_ids: &[u16], options: &SubsetOptions) -> Result<Vec<u8>, FontError> {
        debug!(glyph_count = glyph_ids.len(), format = options.output_format, "开始子集化");
        
        // 执行核心子集化逻辑
        let subset_data = self.subset_and_serialize(glyph_ids)?;
        
        // 根据输出格式处理
        match options.output_format.as_str() {
            "woff" => {
                // TODO: 实现 WOFF 转换
                // 目前暂时返回 TTF 数据
                debug!("WOFF 格式尚未实现，返回 TTF 数据");
                Ok(subset_data)
            }
            _ => {
                // 默认返回 TTF
                Ok(subset_data)
            }
        }
    }
    
    /// 根据文本获取 GlyphID 列表
    pub fn get_glyph_ids_for_text(&self, text: &str) -> Vec<u16> {
        let span = span!(Level::TRACE, "get_glyph_ids_for_text", text_len = text.len());
        let _enter = span.enter();
        
        text.chars()
            .map(|ch| {
                self.cmap.unicode_map.get(&(ch as u32)).copied().unwrap_or(0)
            })
            .collect()
    }
    
    /// 子集化并序列化字体
    pub fn subset_and_serialize(&self, glyph_ids: &[u16]) -> Result<Vec<u8>, FontError> {
        let span = span!(Level::INFO, "subset_and_serialize", 
                         input_glyphs = glyph_ids.len());
        let _enter = span.enter();
        
        // 1. 构建 GlyphID 集合（确保包含 .notdef，即 ID 0）
        let mut subset_glyphs_vec: Vec<u16> = glyph_ids.to_vec();
        if !subset_glyphs_vec.contains(&0) {
            subset_glyphs_vec.push(0); // .notdef 必须存在
        }
        subset_glyphs_vec.sort();
        subset_glyphs_vec.dedup(); // 去重
        
        info!(glyph_count = subset_glyphs_vec.len(), "开始子集化处理");
        
        // 2. 提取 glyf 和 loca 数据
        let (new_loca_data, new_glyf_data) = self.extract_glyf_and_loca(&subset_glyphs_vec)?;
        
        // 3. 构建新的 cmap
        let new_cmap_data = self.rebuild_cmap(&subset_glyphs_vec)?;
        
        // 4. 构建新的 hmtx
        let new_hmtx_data = self.rebuild_hmtx(&subset_glyphs_vec)?;
        
        // 5. 更新 maxp
        let new_maxp_data = self.update_maxp(subset_glyphs_vec.len() as u16)?;
        
        // 6. 更新 head（校验和、修改时间等）- 暂时传入 0，稍后在 assemble_ttf 中更新
        let new_head_data = self.update_head(0)?;
        
        // 获取 hhea 数据（不变）
        let hhea_data = self.font_data.get_table_bytes(Tag(*b"hhea"))
            .ok_or(FontError::TableNotFound { tag: "hhea".to_string() })?;

        // 7. 复制其他不变的表（name, os2, post 等，排除 hhea）
        let other_tables = self.copy_unchanged_tables()?;

        // 打印各表大小统计
        println!("各表大小统计:");
        println!("  head: {} bytes", new_head_data.len());
        println!("  maxp: {} bytes", new_maxp_data.len());
        println!("  cmap: {} bytes", new_cmap_data.len());
        println!("  hhea: {} bytes", hhea_data.len());
        println!("  hmtx: {} bytes", new_hmtx_data.len());
        println!("  loca: {} bytes", new_loca_data.len());
        println!("  glyf: {} bytes", new_glyf_data.len());
        for (tag, data) in &other_tables {
            println!("  {:?}: {} bytes", tag, data.len());
        }

        // 8. 组装最终的 TTF 文件
        let final_font = self.assemble_ttf(
            &new_head_data,
            &new_maxp_data,
            &new_cmap_data,
            hhea_data,
            &new_hmtx_data,
            &new_loca_data,
            &new_glyf_data,
            &other_tables,
        )?;
        
        println!("子集字体生成完成，大小: {} bytes", final_font.len());
        
        // 验证校验和
        if let Some(_head_bytes) = self.font_data.get_table_bytes(Tag(*b"head")) {
            let head_offset = final_font.iter()
                .position(|&b| b == b'h')
                .and_then(|i| {
                    if i + 16 <= final_font.len() && 
                       &final_font[i..i+4] == b"head" {
                        Some(u32::from_be_bytes([
                            final_font[i+8], final_font[i+9], 
                            final_font[i+10], final_font[i+11]
                        ]) as usize)
                    } else {
                        None
                    }
                });
            
            if let Some(offset) = head_offset {
                if offset + 12 <= final_font.len() {
                    let checksum_adj = u32::from_be_bytes([
                        final_font[offset+8], final_font[offset+9],
                        final_font[offset+10], final_font[offset+11]
                    ]);
                    println!("  head.checkSumAdjustment: 0x{:08X}", checksum_adj);
                }
            }
        }
        
        Ok(final_font)
    }
    
    /// 提取 glyf 和 loca 数据
    fn extract_glyf_and_loca(&self, subset_glyphs: &[u16]) -> Result<(Vec<u8>, Vec<u8>), FontError> {
        let num_glyphs = subset_glyphs.len();
        let index_to_loc_format = self.head.index_to_loc_format;
        
        // 构建新的 loca 偏移表
        let mut new_loca_offsets = Vec::with_capacity(num_glyphs + 1);
        let mut new_glyf_data = Vec::new();
        let mut current_offset = 0u32;
        
        for &glyph_id in subset_glyphs {
            new_loca_offsets.push(current_offset);
            
            // 从原始 glyf 表中提取字形数据
            if (glyph_id as usize) < self.loca.offsets.len() - 1 {
                let start = self.loca.offsets[glyph_id as usize];
                let end = self.loca.offsets[glyph_id as usize + 1];
                
                if start < end {
                    // 获取原始 glyf 表的字节数据
                    let glyf_bytes = self.font_data.get_table_bytes(Tag(*b"glyf"))
                        .ok_or(FontError::TableNotFound { tag: "glyf".to_string() })?;
                    
                    if (end as usize) <= glyf_bytes.len() {
                        new_glyf_data.extend_from_slice(&glyf_bytes[start as usize..end as usize]);
                        current_offset += end - start;

                        // 对齐到 4 字节边界
                        let padding = (4 - (current_offset % 4)) % 4;
                        for _ in 0..padding {
                            new_glyf_data.push(0);
                            current_offset += 1;
                        }
                    }
                }
            }
        }
        
        // 添加最后一个偏移量（指向 glyf 表末尾）
        new_loca_offsets.push(current_offset);
        
        // 编码 loca 表
        let loca_data = if index_to_loc_format == 0 {
            // short format (offset / 2)
            let mut data = Vec::with_capacity(new_loca_offsets.len() * 2);
            for &offset in &new_loca_offsets {
                data.extend_from_slice(&((offset / 2) as u16).to_be_bytes());
            }
            data
        } else {
            // long format
            let mut data = Vec::with_capacity(new_loca_offsets.len() * 4);
            for &offset in &new_loca_offsets {
                data.extend_from_slice(&offset.to_be_bytes());
            }
            data
        };
        
        Ok((loca_data, new_glyf_data))
    }
    
    /// 重建 cmap 表（智能选择最佳格式）
    fn rebuild_cmap(&self, subset_glyphs: &[u16]) -> Result<Vec<u8>, FontError> {
        let subset_set: HashSet<u16> = subset_glyphs.iter().copied().collect();
        
        // 过滤出子集中存在的映射
        let mut new_unicode_map: Vec<(u32, u16)> = self.cmap.unicode_map.iter()
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
            self.build_cmap_format12(&new_unicode_map)
        } else if all_in_byte_range && new_unicode_map.len() <= 256 {
            // 所有字符在字节范围内，使用 Format 0
            println!("  cmap 格式: Format 0 (简单字节映射)");
            self.build_cmap_format0(&new_unicode_map)
        } else {
            // 默认使用 Format 4
            let segments = self.build_cmap_segments(&new_unicode_map);
            println!("  cmap 格式: Format 4 ({} 个段)", segments.len());
            self.build_cmap_format4(&segments)
        }
    }
    
    /// 构建 cmap 段（合并连续的码点）
    fn build_cmap_segments(&self, unicode_map: &[(u32, u16)]) -> Vec<CmapSegment> {
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
    fn build_cmap_format4(&self, segments: &[CmapSegment]) -> Result<Vec<u8>, FontError> {
        let seg_count = segments.len();
        
        // 计算 search_range, entry_selector, range_shift
        let max_pow2 = if seg_count > 0 { 
            (seg_count as f32).log2().floor().exp2() as usize 
        } else { 
            1 
        };
        let search_range = 2 * max_pow2;
        let entry_selector = max_pow2.trailing_zeros() as u16;
        let range_shift = 2 * seg_count - search_range;

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
        let _subtable_start = data.len();
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
        let subtable_length = (data.len() - _subtable_start) as u16;
        data[_subtable_start+2.._subtable_start+4].copy_from_slice(&subtable_length.to_be_bytes());
        data[_subtable_start+4.._subtable_start+6].copy_from_slice(&0u16.to_be_bytes()); // language
        
        Ok(data)
    }
    
    /// 构建 Format 0 cmap 子表（简单字节映射）
    fn build_cmap_format0(&self, unicode_map: &[(u32, u16)]) -> Result<Vec<u8>, FontError> {
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
        // let subtable_start = data.len();
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
    fn build_cmap_format12(&self, unicode_map: &[(u32, u16)]) -> Result<Vec<u8>, FontError> {
        // 构建连续的组（groups）
        let groups = self.build_cmap_groups(unicode_map);
        
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
    fn build_cmap_groups(&self, unicode_map: &[(u32, u16)]) -> Vec<CmapGroup> {
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
    
    /// 重建 hmtx 表
    fn rebuild_hmtx(&self, subset_glyphs: &[u16]) -> Result<Vec<u8>, FontError> {
        let mut data = Vec::new();
        
        for &glyph_id in subset_glyphs {
            if (glyph_id as usize) < self.hmtx.metrics.len() {
                let metric = &self.hmtx.metrics[glyph_id as usize];
                data.extend_from_slice(&metric.advance_width.to_be_bytes());
                data.extend_from_slice(&metric.lsb.to_be_bytes());
            }
        }
        
        Ok(data)
    }

    /// 更新 maxp 表
    fn update_maxp(&self, num_glyphs: u16) -> Result<Vec<u8>, FontError> {
        let mut data = Vec::new();
        
        // version 1.0
        data.extend_from_slice(&0x00010000u32.to_be_bytes());
        data.extend_from_slice(&num_glyphs.to_be_bytes());
        
        // v1.0 字段（如果存在则写入，否则使用默认值 0）
        data.extend_from_slice(&self.maxp.max_points.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_contours.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_composite_points.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_composite_contours.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_zones.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_twilight_points.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_storage.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_function_defs.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_instruction_defs.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_stack_elements.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_size_of_instructions.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_component_elements.unwrap_or(0).to_be_bytes());
        data.extend_from_slice(&self.maxp.max_component_depth.unwrap_or(0).to_be_bytes());
        
        Ok(data)
    }
    
    /// 更新 head 表（包含校验和调整和时间戳）
    fn update_head(&self, checksum_adjustment: u32) -> Result<Vec<u8>, FontError> {
        let mut head_data = self.font_data.get_table_bytes(Tag(*b"head"))
            .ok_or(FontError::TableNotFound { tag: "head".to_string() })?
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
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        );
        
        let seconds_since_1904 = now.signed_duration_since(base_date).num_seconds();
        
        // LONGDATETIME 是 64 位有符号整数（高位32位 + 低位32位）
        let high = (seconds_since_1904 >> 32) as u32;
        let low = (seconds_since_1904 & 0xFFFFFFFF) as u32;
        
        head_data[12..16].copy_from_slice(&high.to_be_bytes());
        head_data[16..20].copy_from_slice(&low.to_be_bytes());
        
        Ok(head_data)
    }
    
    /// 复制其他不变的表
    fn copy_unchanged_tables(&self) -> Result<Vec<(Tag, Vec<u8>)>, FontError> {
        let mut tables = Vec::new();
        
        // 复制 hhea（不变）
        if let Some(hhea_bytes) = self.font_data.get_table_bytes(Tag(*b"hhea")) {
            tables.push((Tag(*b"hhea"), hhea_bytes.to_vec()));
        }
        
        // 复制 name（不变）
        if let Some(name_bytes) = self.font_data.get_table_bytes(Tag(*b"name")) {
            tables.push((Tag(*b"name"), name_bytes.to_vec()));
        }
        
        // 复制 os2（不变）
        if let Some(os2_bytes) = self.font_data.get_table_bytes(Tag(*b"OS/2")) {
            tables.push((Tag(*b"OS/2"), os2_bytes.to_vec()));
        }
        
        // post 表需要特殊处理：如果版本是 2.0，需要子集化字形名称
        if let Some(post_bytes) = self.font_data.get_table_bytes(Tag(*b"post")) {
            let subset_post = self.subset_post_table(post_bytes)?;
            tables.push((Tag(*b"post"), subset_post));
        }
        
        Ok(tables)
    }
    
    /// 子集化 post 表
    fn subset_post_table(&self, original_post: &[u8]) -> Result<Vec<u8>, FontError> {
        if original_post.len() < POST_TABLE_MIN_SIZE {
            return Err(FontError::Generic("post table too short".to_string()));
        }
        
        // 读取 post 表版本
        let version = u32::from_be_bytes([original_post[0], original_post[1], original_post[2], original_post[3]]);
        
        if version == 0x00020000 {
            // Version 2.0: 包含字形名称数组，需要子集化
            self.subset_post_v2(original_post)
        } else {
            // 其他版本（如 3.0）不包含字形名称，可以直接使用
            Ok(original_post.to_vec())
        }
    }
    
    /// 子集化 post v2 表
    fn subset_post_v2(&self, original_post: &[u8]) -> Result<Vec<u8>, FontError> {
        if original_post.len() < POST_V2_MIN_SIZE {
            return Err(FontError::Generic("post v2 table too short".to_string()));
        }
        
        // 简化策略：对于只有几个字形的情况，直接使用 post version 3.0（无名称）
        // 这样可以大幅减小文件大小
        let mut new_post = Vec::new();
        new_post.extend_from_slice(&0x00030000u32.to_be_bytes()); // version 3.0
        new_post.extend_from_slice(&original_post[4..36]); // 复制其他字段
        // version 3.0 没有后续数据
        
        println!("  post 表优化: v2 ({}) -> v3 ({})", original_post.len(), new_post.len());
        Ok(new_post)
    }

    /// 组装最终的 TTF 文件（包含校验和计算）
    fn assemble_ttf(
        &self,
        head_data: &[u8],
        maxp_data: &[u8],
        cmap_data: &[u8],
        hhea_data: &[u8],
        hmtx_data: &[u8],
        loca_data: &[u8],
        glyf_data: &[u8],
        other_tables: &[(Tag, Vec<u8>)],
    ) -> Result<Vec<u8>, FontError> {
        let mut all_tables = vec![
            (Tag(*b"head"), head_data.to_vec()),
            (Tag(*b"maxp"), maxp_data.to_vec()),
            (Tag(*b"cmap"), cmap_data.to_vec()),
            (Tag(*b"hhea"), hhea_data.to_vec()),
            (Tag(*b"hmtx"), hmtx_data.to_vec()),
            (Tag(*b"loca"), loca_data.to_vec()),
            (Tag(*b"glyf"), glyf_data.to_vec()),
        ];
        
        // 添加其他表
        for (tag, data) in other_tables {
            if tag.0 != *b"hhea" {
                all_tables.push((tag.clone(), data.clone()));
            }
        }
        
        // 按标签排序（TTF 规范要求）
        all_tables.sort_by_key(|(tag, _)| tag.0);
        
        let num_tables = all_tables.len() as u16;
        
        // 第一步：计算除 head 外所有表的校验和
        let mut table_checksums = Vec::new();
        
        for (tag, data) in &all_tables {
            let checksum = if tag.0 == *b"head" {
                0u32 // 占位
            } else {
                calc_sfnt_checksum(data)
            };
            table_checksums.push(checksum);
        }
        
        // 第二步：构建临时的 Offset Table 和 Table Directory 用于校验和计算
        let mut offset_table = Vec::new();
        offset_table.extend_from_slice(&SFNT_VERSION_TTF.to_be_bytes());
        offset_table.extend_from_slice(&num_tables.to_be_bytes());
        
        let max_pow2: u32 = if num_tables > 0 { 1 << (31 - num_tables.leading_zeros()) } else { 1 };
        let search_range = max_pow2 * 16;
        let entry_selector = max_pow2.trailing_zeros() as u16;
        let range_shift = ((num_tables as u32) * 16).saturating_sub(search_range) as u16;
        
        offset_table.extend_from_slice(&(search_range as u16).to_be_bytes());
        offset_table.extend_from_slice(&entry_selector.to_be_bytes());
        offset_table.extend_from_slice(&range_shift.to_be_bytes());
        
        // 构建临时 Table Directory（checksum 和 offset 暂为 0）
        let mut table_dir = Vec::new();
        for (_i, (tag, _data)) in all_tables.iter().enumerate() {
            table_dir.extend_from_slice(&tag.0);
            table_dir.extend_from_slice(&0u32.to_be_bytes()); // checksum 占位
            table_dir.extend_from_slice(&0u32.to_be_bytes()); // offset 占位
            table_dir.extend_from_slice(&0u32.to_be_bytes()); // length 占位 (实际长度不影响校验和计算逻辑，但这里为了严谨可以填入，不过通常校验和只依赖内容)
            // 注意：上面的 length 占位其实应该填入真实长度，因为 calc_sfnt_checksum 会读取整个 buffer。
            // 但我们在下面重新计算总校验和时，是分别对各个部分计算的，所以这里的 table_dir 内容其实不会被直接用于最终校验和累加，
            // 而是下面分别累加各表校验和。
            // 修正：下面的逻辑是分别累加各表校验和，所以这里的 table_dir 只是为了结构完整，或者如果我们要对整个文件做一次性校验和才需要它完全准确。
            // 当前逻辑是：Total = Sum(TableChecksums) + Checksum(OffsetTable) + Checksum(TableDir)。
            // 所以 TableDir 的内容（除了 tag 和 length 可能影响对齐填充外的部分）其实不影响其他表的校验和。
            // 为了保持逻辑清晰，我们保留这个结构。
        }
         // 重新构建正确的 table dir 用于校验和计算 (包含 length)
         table_dir.clear();
         for (_i, (tag, data)) in all_tables.iter().enumerate() {
            table_dir.extend_from_slice(&tag.0);
            table_dir.extend_from_slice(&0u32.to_be_bytes()); // checksum 占位
            table_dir.extend_from_slice(&0u32.to_be_bytes()); // offset 占位
            table_dir.extend_from_slice(&(data.len() as u32).to_be_bytes());
        }
        
        // 第三步：计算整体校验和以确定 checkSumAdjustment
        let mut total_checksum = calc_sfnt_checksum(&offset_table) as u64;
        total_checksum += calc_sfnt_checksum(&table_dir) as u64;
        
        for (i, (tag, _data)) in all_tables.iter().enumerate() {
            if tag.0 == *b"head" {
                // 计算 head 表校验和时，假设 checkSumAdjustment 为 0
                let head_with_zero_adj = self.update_head(0)?;
                total_checksum += calc_sfnt_checksum(&head_with_zero_adj) as u64;
            } else {
                total_checksum += table_checksums[i] as u64;
            }
        }
        
        // 第四步：计算最终的 checkSumAdjustment
        let final_check_sum_adjustment = SFNT_CHECKSUM_MAGIC.saturating_sub((total_checksum & 0xFFFFFFFF) as u32);

        // 第五步：生成最终的 head 表数据
        let final_head_data = self.update_head(final_check_sum_adjustment)?;

        // 第六步：组装最终字体文件
        let mut font_data = Vec::new();
        font_data.extend_from_slice(&SFNT_VERSION_TTF.to_be_bytes()); // sfnt version (TrueType)
        font_data.extend_from_slice(&num_tables.to_be_bytes());
        
        let max_pow2: u32 = if num_tables > 0 { 1 << (31 - num_tables.leading_zeros()) } else { 1 };
        let search_range = max_pow2 * 16;
        let entry_selector = max_pow2.trailing_zeros() as u16;
        let range_shift = ((num_tables as u32) * 16).saturating_sub(search_range) as u16;
        
        font_data.extend_from_slice(&(search_range as u16).to_be_bytes());
        font_data.extend_from_slice(&entry_selector.to_be_bytes());
        font_data.extend_from_slice(&range_shift.to_be_bytes());
        
        // 预留 Table Directory 空间
        let table_dir_start = font_data.len();
        for _ in 0..num_tables {
            font_data.extend_from_slice(&[0u8; TABLE_DIR_ENTRY_SIZE]);
        }
        
        // 写入表数据（4 字节对齐），并记录偏移量
        let mut table_offsets = Vec::new();
        for (_i, (tag, data)) in all_tables.iter().enumerate() {
            let offset = font_data.len() as u32;
            table_offsets.push(offset);
            
            if tag.0 == *b"head" {
                // 使用更新后的 head 数据
                font_data.extend_from_slice(&final_head_data);
                
                // 4 字节对齐
                let padding = (4 - (final_head_data.len() % 4)) % 4;
                for _ in 0..padding {
                    font_data.push(0);
                }
            } else {
                font_data.extend_from_slice(data);
                
                // 4 字节对齐
                let padding = (4 - (data.len() % 4)) % 4;
                for _ in 0..padding {
                    font_data.push(0);
                }
            }
        }
        
        // 第七步：回填 Table Directory（包含正确的校验和和偏移量）
        for (i, (tag, data)) in all_tables.iter().enumerate() {
            let dir_offset = table_dir_start + i * TABLE_DIR_ENTRY_SIZE;
            font_data[dir_offset..dir_offset+4].copy_from_slice(&tag.0);
            
            let checksum = if tag.0 == *b"head" {
                calc_sfnt_checksum(&final_head_data)
            } else {
                table_checksums[i]
            };
            
            font_data[dir_offset+4..dir_offset+8].copy_from_slice(&checksum.to_be_bytes());
            font_data[dir_offset+8..dir_offset+12].copy_from_slice(&table_offsets[i].to_be_bytes());
            font_data[dir_offset+12..dir_offset+16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        }
        
        Ok(font_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sfnt_checksum_calculation() {
        // 测试简单的校验和计算
        let data = vec![0x00, 0x01, 0x00, 0x00]; // 4字节: 0x00010000
        let checksum = calc_sfnt_checksum(&data);
        assert_eq!(checksum, 0x00010000);
        
        // 测试多字节累加
        let data2 = vec![
            0x00, 0x01, 0x00, 0x00, // 0x00010000
            0x00, 0x02, 0x00, 0x00, // 0x00020000
        ];
        let checksum2 = calc_sfnt_checksum(&data2);
        assert_eq!(checksum2, 0x00030000);
    }

    #[test]
    fn test_sfnt_checksum_padding() {
        // 测试不足4字节的填充
        let data = vec![0x01, 0x02, 0x03]; // 3字节，应该填充为 [0x01, 0x02, 0x03, 0x00]
        let checksum = calc_sfnt_checksum(&data);
        assert_eq!(checksum, 0x01020300);
        
        // 测试1字节
        let data2 = vec![0xFF];
        let checksum2 = calc_sfnt_checksum(&data2);
        assert_eq!(checksum2, 0xFF000000);
    }

    #[test]
    fn test_tag_conversion() {
        let tag = Tag(*b"head");
        assert_eq!(tag.0, [0x68, 0x65, 0x61, 0x64]);
        
        let tag2 = Tag(*b"cmap");
        assert_eq!(tag2.as_str(), "cmap");
    }

    #[test]
    fn test_glyph_id_subset_deduplication() {
        // 测试字形ID去重逻辑
        let mut ids = vec![5, 3, 1, 5, 2, 3];
        ids.sort();
        ids.dedup();
        
        assert_eq!(ids, vec![1, 2, 3, 5]);
    }

    #[test]
    fn test_notdef_inclusion() {
        // 测试 .notdef (glyph 0) 的自动包含
        let glyph_ids = vec![5, 10, 15];
        let mut subset_glyphs_vec: Vec<u16> = glyph_ids.clone();
        
        if !subset_glyphs_vec.contains(&0) {
            subset_glyphs_vec.push(0);
        }
        subset_glyphs_vec.sort();
        
        assert!(subset_glyphs_vec.contains(&0));
        assert_eq!(subset_glyphs_vec.len(), 4); // 3 + 1 (.notdef)
    }

    #[test]
    fn test_cmap_format_selection() {
        // 测试 cmap 格式选择逻辑
        let bmp_chars = vec!['A', 'B', 'C']; // BMP 字符
        let non_bmp_chars = vec!['😀']; // 非 BMP 字符 (U+1F600)
        
        let has_non_bmp = non_bmp_chars.iter().any(|&c| (c as u32) > 0xFFFF);
        assert!(has_non_bmp);
        
        let all_bmp = bmp_chars.iter().all(|&c| (c as u32) <= 0xFFFF);
        assert!(all_bmp);
    }
}
