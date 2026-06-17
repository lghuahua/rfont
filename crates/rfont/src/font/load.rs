use anyhow::Context;
use brotli::Decompressor;
use flate2::read::ZlibDecoder;
use rfont_core::tables::glyf::GlyfTable;
use rfont_core::tables::woff::{WoffHeader, WoffTableDirectoryEntry};
use rfont_core::tables::woff2::{WOFF2_KNOWN_TAGS, Woff2Header, Woff2TableDirectoryEntry};
use rfont_core::tables::woff2_transform::GlyfDecoder;
use rfont_core::{Cmap, Head, Hhea, Hmtx, Loca, Maxp, calc_sfnt_checksum, pad4};
use rfont_types::{
    FontError, ReadBytes, Reader, SFNT_CHECKSUM_MAGIC, TableRecord, Tag, WriteBytes, Writer,
};
use std::io::Read;
use tracing::{Level, debug, info, span};

use rfont_types::TABLE_DIR_ENTRY_SIZE;

use crate::font_data::FontData;

/// 高层字体对象（预加载核心表）
///
/// `Font` 是 rfont 库的核心结构，代表一个已加载的字体文件。
/// 它预加载了所有必要的字体表（head, maxp, hhea, loca, cmap, hmtx），
/// 提供高效的字形查询和子集化功能。
///
/// # 示例
/// ```no_run
/// use rfont::Font;
///
/// // 从文件加载字体
/// let font = Font::load("font.ttf").unwrap();
///
/// // 获取字体信息
/// let info = font.get_font_info();
/// println!("字形数量: {}", info.glyph_count);
///
/// // 创建子集
/// let subset_data = font.subset_builder()
///     .text("Hello")
///     .build()
///     .unwrap();
/// ```
pub struct Font {
    pub(crate) font_data: FontData,

    // 核心表（预加载）
    pub(crate) head: Head,
    pub(crate) maxp: Maxp,
    pub(crate) hhea: Hhea,
    pub(crate) loca: Loca,
    pub(crate) cmap: Cmap,
    pub(crate) hmtx: Hmtx,
    pub(crate) glyf: GlyfTable,
}

impl Font {
    /// 从文件路径加载字体（支持 TTF、WOFF 和 WOFF2）
    ///
    /// 自动检测文件格式并解析相应的结构。
    /// 对于压缩格式（WOFF/WOFF2），会先解压缩再解析。
    ///
    /// # 参数
    /// - `path`: 字体文件的路径
    ///
    /// # 返回值
    /// - `Ok(Font)`: 成功加载的字体对象
    /// - `Err(anyhow::Error)`: 加载失败时的错误信息，包含详细上下文
    ///
    /// # 错误
    /// - 文件读取失败
    /// - 必需的字体表缺失
    /// - 其他解析错误
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("path/to/font.ttf").expect("无法加载字体");
    /// ```
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let span = span!(Level::INFO, "load_font", path = path);
        let _enter = span.enter();

        info!("开始加载字体文件");
        let data = std::fs::read(path)
            .map_err(|e| anyhow::anyhow!("无法读取字体文件 '{}': {}", path, e))?;

        debug!(size = data.len(), "字体文件读取完成");

        // 检查是否为 WOFF2 格式
        if data.len() >= 4 && &data[0..4] == b"wOF2" {
            info!("检测到 WOFF2 格式");
            Self::load_woff2(&data)
                .context(format!("解析 WOFF2 字体文件 '{}' 失败", path))
        }
        // 检查是否为 WOFF 格式
        else if data.len() >= 4 && &data[0..4] == b"wOFF" {
            info!("检测到 WOFF 格式");
            Self::load_woff(&data)
                .context(format!("解析 WOFF 字体文件 '{}' 失败", path))
        } else {
            info!("检测到 TTF/OTF 格式");
            Self::load_ttf(&data)
                .context(format!("解析 TTF/OTF 字体文件 '{}' 失败", path))
        }
    }

    /// 加载 TTF 格式字体（预加载核心表）
    fn load_ttf(data: &[u8]) -> Result<Self, FontError> {
        let span = span!(Level::DEBUG, "load_ttf");
        let _enter = span.enter();

        let font_data = FontData::new(data.to_vec())?;

        // 解析核心表（按依赖顺序）
        let head: Head = font_data.parse_table(Tag(*b"head"))?;
        debug!(
            index_to_loc_format = head.index_to_loc_format,
            "Head 表解析完成"
        );

        let maxp: Maxp = font_data.parse_table(Tag(*b"maxp"))?;
        debug!(num_glyphs = maxp.num_glyphs, "Maxp 表解析完成");

        let hhea: Hhea = font_data.parse_table(Tag(*b"hhea"))?;
        debug!(
            number_of_h_metrics = hhea.number_of_h_metrics,
            "Hhea 表解析完成"
        );

        // loca 需要 head 的参数
        let loca_bytes =
            font_data
                .get_table_bytes(Tag(*b"loca"))
                .ok_or(FontError::TableNotFound {
                    tag: "loca".to_string(),
                })?;
        let loca = Loca::read_from(
            &mut Reader::new(loca_bytes),
            head.index_to_loc_format,
            maxp.num_glyphs,
        )?;
        debug!(offsets_count = loca.offsets.len(), "Loca 表解析完成");

        let cmap_bytes =
            font_data
                .get_table_bytes(Tag(*b"cmap"))
                .ok_or(FontError::TableNotFound {
                    tag: "cmap".to_string(),
                })?;
        let cmap = Cmap::read_from(&mut Reader::new(cmap_bytes))?;
        debug!("Cmap 表解析完成");

        // hmtx 需要 hhea 和 maxp 的参数
        let hmtx_bytes =
            font_data
                .get_table_bytes(Tag(*b"hmtx"))
                .ok_or(FontError::TableNotFound {
                    tag: "hmtx".to_string(),
                })?;
        let hmtx = Hmtx::read_from(
            &mut Reader::new(hmtx_bytes),
            hhea.number_of_h_metrics,
            maxp.num_glyphs,
        )?;
        debug!(metrics_count = hmtx.metrics.len(), "Hmtx 表解析完成");
        let glyf_bytes =
            font_data
                .get_table_bytes(Tag(*b"glyf"))
                .ok_or(FontError::TableNotFound {
                    tag: "glyf".to_string(),
                })?;
        // 使用 from_vec 转换为 Rc，避免额外拷贝
        let glyf = GlyfTable::from_vec(glyf_bytes.to_vec());

        Ok(Font {
            font_data,
            head,
            maxp,
            hhea,
            loca,
            cmap,
            hmtx,
            glyf,
        })
    }

    /// 加载 WOFF 格式字体
    fn load_woff(data: &[u8]) -> Result<Self, FontError> {
        let span = span!(Level::DEBUG, "load_woff");
        let _enter = span.enter();

        let mut reader = Reader::new(data);

        // 解析 WOFF Header
        let woff_header = WoffHeader::read_from(&mut reader)?;
        debug!(
            flavor = format!("0x{:08X}", woff_header.flavor),
            num_tables = woff_header.num_tables,
            "WOFF Header 解析完成"
        );

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
        let max_pow2: u32 = if num_tables > 0 {
            1 << (31 - num_tables.leading_zeros())
        } else {
            1
        };
        let search_range = max_pow2 * 16;
        let entry_selector = max_pow2.trailing_zeros() as u16;
        let range_shift = (num_tables * 16).saturating_sub(search_range) as u16;

        sfnt_data.extend_from_slice(&(search_range as u16).to_be_bytes());
        sfnt_data.extend_from_slice(&entry_selector.to_be_bytes());
        sfnt_data.extend_from_slice(&range_shift.to_be_bytes());

        // 先收集所有解压后的表数据
        let mut decompressed_tables = Vec::new();

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
                decoder
                    .read_to_end(&mut buf)
                    .map_err(|e| FontError::WoffDecompressionError {
                        message: format!("Failed to decompress table {:?}: {}", entry.tag, e),
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
            sfnt_data.extend(std::iter::repeat_n(0, *padding));
        }

        // 回填 Table Directory
        for (i, (tag, _, orig_length, _)) in decompressed_tables.iter().enumerate() {
            let dir_offset = table_dir_start + i * TABLE_DIR_ENTRY_SIZE;
            sfnt_data[dir_offset..dir_offset + 4].copy_from_slice(&tag.0);
            sfnt_data[dir_offset + 4..dir_offset + 8].copy_from_slice(&0u32.to_be_bytes()); // checksum
            sfnt_data[dir_offset + 8..dir_offset + 12]
                .copy_from_slice(&table_offsets[i].to_be_bytes());
            sfnt_data[dir_offset + 12..dir_offset + 16]
                .copy_from_slice(&(*orig_length as u32).to_be_bytes());
        }

        debug!(
            offset = table_dir_start,
            entries = decompressed_tables.len(),
            "Table directory written"
        );

        // 使用重组后的 SFNT 数据创建 Font
        Self::load_ttf(&sfnt_data)
    }

    /// 加载 WOFF2 格式字体
    fn load_woff2(data: &[u8]) -> Result<Self, FontError> {
        let span = span!(Level::DEBUG, "load_woff2");
        let _enter = span.enter();

        let mut reader = Reader::new(data);

        // 解析 WOFF2 Header
        let mut woff2_header = Woff2Header::read_from(&mut reader)?;
        debug!(
            flavor = format!("0x{:08X}", woff2_header.flavor),
            num_tables = woff2_header.num_tables,
            "WOFF2 Header 解析完成"
        );

        // 解析表目录
        let mut table_entries = Vec::new();
        let mut transform_size = 0;
        for _ in 0..woff2_header.num_tables {
            let entry = Woff2TableDirectoryEntry::read_from(&mut reader, &WOFF2_KNOWN_TAGS)?;
            debug!(tag = ?entry.tag, orig_length = entry.orig_length, transform_length = entry.transform_length);

            transform_size += entry.transform_length.map_or(entry.orig_length, |x| x);
            table_entries.push(entry);
        }

        debug!(table_count = table_entries.len(), "WOFF2 表目录解析完成");
        // 解压缩并重组为 SFNT 数据
        woff2_header.total_sfnt_size = transform_size;

        // 写入
        let sfnt_data = write_ttf_data(&mut reader, &woff2_header, &table_entries)?;

        tracing::debug!(
            final_size = sfnt_data.len(),
            // expected = woff2_header.total_sfnt_size as usize,
            "WOFF2 重组完成"
        );

        // 使用重组后的 SFNT 数据创建 Font
        let f = Self::load_ttf(&sfnt_data)?;

        Ok(f)
    }

    /// 获取原始字体数据
    ///
    /// 返回包含完整字体二进制数据的 `FontData` 对象。
    /// 可用于直接访问字体表或进行底层操作。
    pub fn font_data(&self) -> &FontData {
        &self.font_data
    }

    /// 获取 head 表的只读引用
    pub fn head(&self) -> &Head {
        &self.head
    }

    /// 获取 maxp 表的只读引用
    pub fn maxp(&self) -> &Maxp {
        &self.maxp
    }

    /// 获取 hhea 表的只读引用
    pub fn hhea(&self) -> &Hhea {
        &self.hhea
    }

    /// 获取 loca 表的只读引用
    pub fn loca(&self) -> &Loca {
        &self.loca
    }

    /// 获取 cmap 表的只读引用
    pub fn cmap(&self) -> &Cmap {
        &self.cmap
    }

    /// 获取 hmtx 表的只读引用
    pub fn hmtx(&self) -> &Hmtx {
        &self.hmtx
    }

    /// 检测字体格式并返回详细信息
    ///
    /// 分析字体数据的头部信息，识别字体格式（TTF、OTF、WOFF、WOFF2），
    /// 并返回包含版本、压缩类型、表列表等详细信息的 `FontFormatInfo`。
    ///
    /// # 参数
    /// - `data`: 字体文件的原始字节数据
    ///
    /// # 返回值
    /// - `Ok(FontFormatInfo)`: 字体格式详细信息
    /// - `Err(FontError)`: 解析失败时的错误信息
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let data = std::fs::read("font.ttf").unwrap();
    /// let format_info = Font::detect_format(&data).unwrap();
    /// println!("格式: {:?}", format_info.format);
    /// println!("是否可变字体: {}", format_info.is_variable);
    /// ```
    pub fn detect_format(data: &[u8]) -> Result<rfont_types::FontFormatInfo, FontError> {
        if data.len() < 4 {
            return Err(FontError::InvalidFileFormat { 
                reason: "数据太短，无法检测格式".to_string(),
                actual_length: data.len()
            });
        }

        // 检测 WOFF2
        if &data[0..4] == b"wOF2" {
            Self::detect_woff2_format(data)
        }
        // 检测 WOFF
        else if &data[0..4] == b"wOFF" {
            Self::detect_woff_format(data)
        }
        // 检测 TTF/OTF
        else {
            Self::detect_sfnt_format(data)
        }
    }

    /// 获取 name 表
    pub fn get_name_table(&self) -> Option<rfont_core::NameTable> {
        use rfont_core::NameTable;
        use rfont_types::ReadBytes;

        let name_bytes = self.font_data.get_table_bytes(Tag(*b"name"))?;
        let mut reader = Reader::new(name_bytes);
        NameTable::read_from(&mut reader).ok()
    }

    /// 检测 SFNT 格式（TTF/OTF）
    fn detect_sfnt_format(data: &[u8]) -> Result<rfont_types::FontFormatInfo, FontError> {
        use rfont_types::FontFormat;

        let mut reader = Reader::new(data);

        // 读取 SFNT version (flavor)
        let flavor = reader.read_u32()?;
        let num_tables = reader.read_u16()?;

        // 跳过 search_range, entry_selector, range_shift
        reader.read_u16()?;
        reader.read_u16()?;
        reader.read_u16()?;

        // 收集所有表标签
        let mut table_tags = Vec::new();
        for _ in 0..num_tables {
            let tag_bytes = [
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
            ];
            let tag_str = String::from_utf8_lossy(&tag_bytes).to_string();
            table_tags.push(tag_str);

            // 跳过 checksum, offset, length
            reader.read_u32()?;
            reader.read_u32()?;
            reader.read_u32()?;
        }

        // 判断是 TTF 还是 OTF
        let format = if flavor == 0x00010000 {
            FontFormat::Ttf
        } else if flavor == 0x4F54544F {
            // 'OTTO'
            FontFormat::Otf
        } else {
            FontFormat::Ttf // 默认为 TTF
        };

        // 检查是否为可变字体（包含 fvar 表）
        let is_variable = table_tags.iter().any(|t| t == "fvar");

        // 必需表列表
        let required_tables = vec![
            "cmap".to_string(),
            "head".to_string(),
            "hhea".to_string(),
            "maxp".to_string(),
        ];

        // 可选表列表
        let optional_tables: Vec<String> = table_tags
            .iter()
            .filter(|t| !required_tables.contains(t))
            .cloned()
            .collect();

        let version = format!("0x{:08X}", flavor);

        Ok(rfont_types::FontFormatInfo::new(
            format,
            version,
            is_variable,
            None, // SFNT 无压缩
            required_tables,
            optional_tables,
            num_tables,
        ))
    }

    /// 检测 WOFF 格式
    fn detect_woff_format(data: &[u8]) -> Result<rfont_types::FontFormatInfo, FontError> {
        use rfont_types::{CompressionType, FontFormat};
        let mut reader = Reader::new(data);

        // 读取 WOFF Header
        let _signature = reader.read_u32()?;
        let flavor = reader.read_u32()?;
        let _length = reader.read_u32()?;
        let num_tables = reader.read_u16()?;
        let _reserved = reader.read_u16()?;
        let _total_sfnt_size = reader.read_u32()?;
        let major_version = reader.read_u16()?;
        let minor_version = reader.read_u16()?;
        let _meta_offset = reader.read_u32()?;
        let _meta_comp_length = reader.read_u32()?;
        let _meta_orig_length = reader.read_u32()?;
        let _priv_offset = reader.read_u32()?;
        let _priv_length = reader.read_u32()?;

        // 收集所有表标签
        let mut table_tags = Vec::new();
        for _ in 0..num_tables {
            let tag_bytes = [
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
            ];
            let tag_str = String::from_utf8_lossy(&tag_bytes).to_string();
            table_tags.push(tag_str);

            // 跳过 offset, comp_length, orig_length, checksum
            reader.read_u32()?;
            reader.read_u32()?;
            reader.read_u32()?;
            reader.read_u32()?;
        }

        // 判断内部格式
        let format = if flavor == 0x00010000 {
            FontFormat::Ttf
        } else if flavor == 0x4F54544F {
            FontFormat::Otf
        } else {
            FontFormat::Ttf
        };

        // 检查是否为可变字体
        let is_variable = table_tags.iter().any(|t| t == "fvar");

        // 必需表
        let required_tables = vec![
            "cmap".to_string(),
            "head".to_string(),
            "hhea".to_string(),
            "maxp".to_string(),
        ];

        // 可选表
        let optional_tables: Vec<String> = table_tags
            .iter()
            .filter(|t| !required_tables.contains(t))
            .cloned()
            .collect();

        let version = format!("{}.{}", major_version, minor_version);

        Ok(rfont_types::FontFormatInfo::new(
            format,
            version,
            is_variable,
            Some(CompressionType::Zlib),
            required_tables,
            optional_tables,
            num_tables,
        ))
    }

    /// 检测 WOFF2 格式
    fn detect_woff2_format(data: &[u8]) -> Result<rfont_types::FontFormatInfo, FontError> {
        use rfont_core::tables::woff2::read_table_directory;
        use rfont_types::{CompressionType, FontFormat};
        let mut reader = Reader::new(data);

        // 读取 WOFF2 Header
        let _signature = reader.read_u32()?;
        let flavor = reader.read_u32()?;
        let _length = reader.read_u32()?;
        let num_tables = reader.read_u16()?;
        let _reserved = reader.read_u16()?;
        let _total_sfnt_size = reader.read_u32()?;

        // 跳过 WOFF2 Header 剩余字段（total_compressed_size + version + meta + priv = 24 bytes）
        for _ in 0..6 {
            reader.read_u32()?;
        }

        // 使用规范的表目录解析，正确处理 Base128 变长编码
        let table_entries = read_table_directory(&mut reader, num_tables)?;

        let table_tags: Vec<String> = table_entries
            .iter()
            .map(|e| e.tag.as_str().to_string())
            .collect();

        // 判断内部格式
        let format = if flavor == 0x00010000 {
            FontFormat::Ttf
        } else if flavor == 0x4F54544F {
            FontFormat::Otf
        } else {
            FontFormat::Ttf
        };

        // 检查是否为可变字体
        let is_variable = table_tags.iter().any(|t| t == "fvar");

        // 必需表
        let required_tables = vec![
            "cmap".to_string(),
            "head".to_string(),
            "hhea".to_string(),
            "maxp".to_string(),
        ];

        // 可选表
        let optional_tables: Vec<String> = table_tags
            .iter()
            .filter(|t| !required_tables.contains(t))
            .cloned()
            .collect();

        let version = format!("0x{:08X}", flavor);

        Ok(rfont_types::FontFormatInfo::new(
            format,
            version,
            is_variable,
            Some(CompressionType::Brotli),
            required_tables,
            optional_tables,
            num_tables,
        ))
    }
}

fn write_ttf_data(
    reader: &mut Reader,
    hdr: &Woff2Header,
    table_entries: &[Woff2TableDirectoryEntry],
) -> Result<Vec<u8>, FontError> {
    // write_ttf_header(writer, hdr)?;

    // 解压
    let decompressed_data = woff2_uncomprss(reader, hdr)?;
    let mut table_reader = Reader::new(&decompressed_data);
    let mut all_tables = reconstruct_transformed_tables(&mut table_reader, table_entries)?;
    assemble_ttf(&mut all_tables)
}

fn reconstruct_transformed_tables(
    reader: &mut Reader,
    table_entries: &[Woff2TableDirectoryEntry],
) -> Result<Vec<(Tag, Vec<u8>)>, FontError> {
    tracing::debug!(
        entries_len = table_entries.len(),
        "reconstruct transformed tables"
    );
    let mut loca_data_ = Vec::new();
    let mut all_tables = Vec::new();

    for entry in table_entries.iter() {
        let mut data_vec = Vec::new();
        if let Some(transform_length) = entry.transform_length {
            if entry.tag.as_str() == "glyf" {
                let transform_data = reader.read_bytes(transform_length as usize)?;
                let mut glyf_decoder = GlyfDecoder::new(transform_data)?;
                match glyf_decoder.decode() {
                    Ok((glyf_data, loca_data)) => {
                        data_vec = glyf_data;
                        loca_data_ = loca_data;
                    }
                    Err(e) => {
                        tracing::error!(
                            error = ?e,
                            "Failed to reconstruct glyf/loca"
                        );
                        return Err(e);
                    }
                }
            } else if entry.tag.as_str() == "loca" {
                tracing::debug!(tag = entry.tag.as_str(), "loca reconstruct");
                data_vec.extend_from_slice(&loca_data_);
            // } else if entry.tag.as_str() == "hmtx" {
            //     tracing::debug!(tag = entry.tag.as_str(), "hmtx reconstruct");
            } else {
                tracing::warn!(tag = entry.tag.as_str(), "Unknow transform");
            }
        } else {
            let data = reader.read_bytes(entry.orig_length as usize)?;
            data_vec = if entry.tag.as_str() == "head" {
                // 设置 checkSumAdjustment 为 0
                if data.len() >= 12 {
                    let mut vec = data.to_vec();
                    vec[8..12].copy_from_slice(&[0u8; 4]);
                    vec
                } else {
                    tracing::warn!(
                        tag = entry.tag.as_str(),
                        length = data.len(),
                        "head table length is too short"
                    );
                    return Err(FontError::Generic(
                        "head 表数据太短，无法设置 checkSumAdjustment".to_string(),
                    ));
                }
            } else {
                data.to_vec()
            };
        }

        all_tables.push((entry.tag, data_vec));
    }

    Ok(all_tables)
}

fn woff2_uncomprss(reader: &mut Reader, hdr: &Woff2Header) -> Result<Vec<u8>, FontError> {
    // 读取并解压缩所有表数据
    let compressed_data = reader.read_bytes(hdr.total_compressed_size as usize)?;

    // 使用 Brotli 解压缩整个数据块
    let mut decompressor = Decompressor::new(compressed_data, 4096);
    let mut decompressed_buffer = Vec::with_capacity(hdr.total_sfnt_size as usize);
    decompressor
        .read_to_end(&mut decompressed_buffer)
        .map_err(|e| {
            FontError::Woff2DecompressionError { 
                message: format!("Brotli 解压缩失败: {}", e) 
            }
        })?;

    debug!(
        decompressed_size = decompressed_buffer.len(),
        expected_size = hdr.total_sfnt_size,
        "Brotli 解压缩完成"
    );

    // 验证解压缩大小
    let actual_decompressed_size = decompressed_buffer.len() as u32;
    if actual_decompressed_size != hdr.total_sfnt_size {
        tracing::warn!(
            "WOFF2: Table data size mismatch. Expected table data {}, got {}",
            hdr.total_sfnt_size,
            actual_decompressed_size
        );
    } else {
        tracing::debug!(
            "WOFF2: Table data size verified. Table data: {}, Total SFNT: {}",
            actual_decompressed_size,
            hdr.total_sfnt_size
        );
    };
    Ok(decompressed_buffer)
}

/// 组装最终的 TTF 文件（包含校验和计算）
pub fn assemble_ttf(all_tables: &mut [(Tag, Vec<u8>)]) -> Result<Vec<u8>, FontError> {
    use rfont_types::{SFNT_VERSION_TTF, TABLE_DIR_ENTRY_SIZE};
    debug!("Assembling TTF");
    // 按标签排序（TTF 规范要求）
    // all_tables.sort_by_key(|(tag, _)| tag.0);
    all_tables.sort_by(|a, b| {
        let a_idx = WOFF2_KNOWN_TAGS.iter().position(|t| t == &a.0);
        let b_idx = WOFF2_KNOWN_TAGS.iter().position(|t| t == &b.0);
        match (a_idx, b_idx) {
            (Some(ai), Some(bi)) => ai.cmp(&bi),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.0.as_str().cmp(b.0.as_str()),
        }
    });

    let num_tables = all_tables.len() as u16;

    let mut font_writer = Writer::new();

    font_writer.write_u32(SFNT_VERSION_TTF)?;
    font_writer.write_u16(num_tables)?;
    let max_pow2: u32 = if num_tables > 0 {
        // 找到小于等于 num_tables 的最大 2 的幂
        1 << (31 - num_tables.leading_zeros())
    } else {
        1
    };

    let search_range = max_pow2 * 16;
    let entry_selector = max_pow2.trailing_zeros() as u16;
    let range_shift = ((num_tables as u32) * 16).saturating_sub(search_range) as u16;
    font_writer.write_u16(search_range as u16)?;
    font_writer.write_u16(entry_selector)?;
    font_writer.write_u16(range_shift)?;

    let mut font_checksum: u64 = 0;
    let mut head_offset = 0;

    let start_offset = (12 + num_tables as usize * TABLE_DIR_ENTRY_SIZE) as u32;
    let mut table_data = Vec::new();

    for (tag, data) in all_tables {
        let checksum = calc_sfnt_checksum(data);
        font_checksum += checksum as u64;
        let offset = start_offset + table_data.len() as u32;

        if tag.as_str() == "head" {
            head_offset = table_data.len();
        }
        table_data.extend_from_slice(data);

        let table_record = TableRecord {
            tag: *tag,
            checksum,
            offset,
            length: data.len() as u32,
        };
        table_record.write_to(&mut font_writer)?;

        pad4(&mut table_data);
    }
    // snft header + table records
    font_checksum += calc_sfnt_checksum(&font_writer.data) as u64;

    let font_checksum_u32 = (font_checksum & 0xFFFFFFFF) as u32;
    let checksum_adjustment = SFNT_CHECKSUM_MAGIC.wrapping_sub(font_checksum_u32);

    table_data[head_offset + 8..head_offset + 12]
        .copy_from_slice(&checksum_adjustment.to_be_bytes());

    font_writer.write_bytes(&table_data)?;

    Ok(font_writer.data)
}

impl Font {
    /// 获取字体信息
    ///
    /// 从各个字体表中提取基本信息，包括字形数量、度量信息、支持的字符数等。
    ///
    /// # 返回值
    /// `FontInfo` 结构体，包含字体的基本信息
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// let info = font.get_font_info().unwrap();
    /// println!("字形数量: {}", info.glyph_count);
    /// println!("支持的字符数: {}", info.supported_char_count);
    /// ```
    pub fn get_font_info(&self) -> anyhow::Result<crate::info::FontInfo> {
        use crate::info::{FontInfo, TableInfo};

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
        info.ascender = self.hhea.ascender.0;
        info.descender = self.hhea.descender.0;
        info.line_gap = self.hhea.line_gap.0;

        // 提取表列表
        info.tables = self
            .font_data
            .get_table_records()
            .values()
            .map(TableInfo::from_record)
            .collect();

        // 统计支持的字符数量
        info.supported_char_count = self.cmap.supported_chars_count();

        // 从 name 表提取 family_name, style_name, version
        if let Some(name_table) = self.get_name_table() {
            info.family_name = name_table.get_family_name_str();
            info.style_name = name_table.get_subfamily_name().and_then(|data| {
                rfont_core::NameTable::decode_utf16_be(data)
                    .or_else(|| rfont_core::NameTable::decode_utf8(data))
            });
            info.version = name_table.get_version().and_then(|data| {
                rfont_core::NameTable::decode_utf16_be(data)
                    .or_else(|| rfont_core::NameTable::decode_utf8(data))
            });
        }

        Ok(info)
    }

    /// 获取所有表的列表
    ///
    /// 返回字体中所有表的详细信息（标签、校验和、偏移量、长度）。
    ///
    /// # 返回值
    /// `Vec<TableInfo>` 包含所有表的信息
    pub fn get_table_list(&self) -> Vec<crate::info::TableInfo> {
        use crate::info::TableInfo;

        self.font_data
            .get_table_records()
            .values()
            .map(TableInfo::from_record)
            .collect()
    }

    /// 检查字体是否支持特定字符
    ///
    /// # 参数
    /// - `unicode`: Unicode 码点
    ///
    /// # 返回值
    /// - `true`: 字体支持该字符
    /// - `false`: 字体不支持该字符
    pub fn supports_character(&self, unicode: char) -> bool {
        self.cmap.get_glyph_id(unicode).is_some()
    }

    /// 将文本转换为字形 ID 列表
    ///
    /// 使用 cmap 表将文本中的每个字符映射到对应的字形 ID。
    /// 如果字符在字体中不存在，会被跳过。
    ///
    /// # 参数
    /// - `text`: 要转换的文本
    ///
    /// # 返回值
    /// 字形 ID 列表（可能为空，如果文本中没有支持的字符）
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// let glyph_ids = font.text_to_glyph_ids("Hello");
    /// println!("字形 ID: {:?}", glyph_ids);
    /// ```
    pub fn text_to_glyph_ids(&self, text: &str) -> Vec<u16> {
        text.chars()
            .filter_map(|ch| self.cmap.get_glyph_id(ch))
            .collect()
    }

    /// 根据文本获取 GlyphID 列表
    ///
    /// 将文本中的每个字符映射到字形 ID。如果字符不存在，返回 0（.notdef）。
    ///
    /// # 参数
    /// - `text`: 要转换的文本
    ///
    /// # 返回值
    /// 字形 ID 列表，长度与文本中的字符数相同
    pub fn get_glyph_ids_for_text(&self, text: &str) -> Vec<u16> {
        use tracing::{Level, span};

        let span = span!(
            Level::TRACE,
            "get_glyph_ids_for_text",
            text_len = text.len()
        );
        let _enter = span.enter();

        text.chars()
            .map(|ch| self.cmap.get_glyph_id(ch).map_or(0, |v| v))
            .collect()
    }
}
