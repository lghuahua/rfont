use rfont_core::{Head, Maxp, Hhea, Loca, Cmap, Hmtx};
use rfont_core::tables::woff::{WoffHeader, WoffTableDirectoryEntry};
use rfont_core::tables::woff2::{Woff2Header, Woff2TableDirectoryEntry, WOFF2_KNOWN_TAGS};
use rfont_types::{FontError, Reader, Tag, ReadBytes};
use flate2::read::ZlibDecoder;
use brotli::Decompressor;
use std::io::Read;
use tracing::{debug, info, span, Level};

use crate::font_data::FontData;
use crate::constants::TABLE_DIR_ENTRY_SIZE;

// 导出 WOFF2_KNOWN_TAGS 供 detect_format 使用
pub use rfont_core::tables::woff2::WOFF2_KNOWN_TAGS as CORE_WOFF2_KNOWN_TAGS;

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
        
        // 检查是否为 WOFF2 格式
        if data.len() >= 4 && &data[0..4] == b"wOF2" {
            info!("检测到 WOFF2 格式");
            Self::load_woff2(&data)
        }
        // 检查是否为 WOFF 格式
        else if data.len() >= 4 && &data[0..4] == b"wOFF" {
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

    /// 加载 WOFF2 格式字体
    fn load_woff2(data: &[u8]) -> Result<Self, FontError> {
        let span = span!(Level::DEBUG, "load_woff2");
        let _enter = span.enter();
        
        let mut reader = Reader::new(data);
        
        // 解析 WOFF2 Header
        let woff2_header = Woff2Header::read_from(&mut reader)?;
        debug!(flavor = format!("0x{:08X}", woff2_header.flavor), num_tables = woff2_header.num_tables, "WOFF2 Header 解析完成");
        
        // 解析表目录
        let mut table_entries = Vec::new();
        for _ in 0..woff2_header.num_tables {
            table_entries.push(Woff2TableDirectoryEntry::read_from(&mut reader, &WOFF2_KNOWN_TAGS)?);
        }
        
        debug!(table_count = table_entries.len(), "WOFF2 表目录解析完成");

        // 解压缩并重组为 SFNT 数据
        let mut sfnt_data = Vec::with_capacity(woff2_header.total_sfnt_size as usize);
        
        // 写入 Offset Table
        sfnt_data.extend_from_slice(&woff2_header.flavor.to_be_bytes());
        sfnt_data.extend_from_slice(&woff2_header.num_tables.to_be_bytes());
        
        // 计算 searchRange, entrySelector, rangeShift
        let num_tables = woff2_header.num_tables as u32;
        let max_pow2: u32 = if num_tables > 0 { 1 << (31 - num_tables.leading_zeros()) } else { 1 };
        let search_range = max_pow2 * 16;
        let entry_selector = max_pow2.trailing_zeros() as u16;
        let range_shift = ((num_tables as u32) * 16).saturating_sub(search_range) as u16;

        sfnt_data.extend_from_slice(&(search_range as u16).to_be_bytes());
        sfnt_data.extend_from_slice(&entry_selector.to_be_bytes());
        sfnt_data.extend_from_slice(&range_shift.to_be_bytes());
        
        // 先写入 Table Directory（占位，稍后回填偏移量）
        let table_dir_start = sfnt_data.len();
        for _ in &table_entries {
            sfnt_data.extend_from_slice(&[0u8; TABLE_DIR_ENTRY_SIZE]); // 每个表目录项 16 字节
        }
        
        // 读取并解压缩所有表数据
        let compressed_data_offset = reader.offset;
        let compressed_data = &data[compressed_data_offset..];
        
        // 使用 Brotli 解压缩整个数据块
        let mut decompressor = Decompressor::new(compressed_data, 4096);
        let mut decompressed_buffer = Vec::with_capacity(woff2_header.total_sfnt_size as usize);
        decompressor.read_to_end(&mut decompressed_buffer).map_err(|e| FontError::Generic(
            format!("WOFF2: Failed to decompress with Brotli: {}", e)
        ))?;
        
        debug!(decompressed_size = decompressed_buffer.len(), expected_size = woff2_header.total_sfnt_size, "Brotli 解压缩完成");
        
        // 验证解压缩大小
        if decompressed_buffer.len() != woff2_header.total_sfnt_size as usize {
            return Err(FontError::Generic(format!(
                "WOFF2: Decompressed size mismatch. Expected {}, got {}",
                woff2_header.total_sfnt_size,
                decompressed_buffer.len()
            )));
        }
        
        // 将解压缩的数据复制到 sfnt_data（跳过已写入的 header 和 directory）
        sfnt_data.extend_from_slice(&decompressed_buffer);
        
        // 回填 Table Directory
        let mut current_offset = (table_dir_start + table_entries.len() * TABLE_DIR_ENTRY_SIZE) as u32;
        for (i, entry) in table_entries.iter().enumerate() {
            let dir_offset = table_dir_start + i * TABLE_DIR_ENTRY_SIZE;
            
            if let Some(tag) = entry.tag {
                sfnt_data[dir_offset..dir_offset+4].copy_from_slice(&tag.0);
            } else {
                // 如果没有标签，使用默认值
                sfnt_data[dir_offset..dir_offset+4].copy_from_slice(b"????");
            }
            
            sfnt_data[dir_offset+4..dir_offset+8].copy_from_slice(&0u32.to_be_bytes()); // checksum
            sfnt_data[dir_offset+8..dir_offset+12].copy_from_slice(&current_offset.to_be_bytes());
            sfnt_data[dir_offset+12..dir_offset+16].copy_from_slice(&entry.orig_length.to_be_bytes());
            
            // 更新下一个表的偏移量（对齐到 4 字节边界）
            let padding = (4 - (entry.orig_length % 4)) % 4;
            current_offset += entry.orig_length + padding;
        }
        
        println!("WOFF2 Table directory written at offset {}, {} entries", table_dir_start, table_entries.len());
        
        // 使用重组后的 SFNT 数据创建 Font
        Self::load_ttf(&sfnt_data)
    }

    /// 获取原始字体数据
    pub fn font_data(&self) -> &FontData {
        &self.font_data
    }
    
    /// 检测字体格式并返回详细信息
    pub fn detect_format(data: &[u8]) -> Result<rfont_types::FontFormatInfo, FontError> {
        if data.len() < 4 {
            return Err(FontError::Generic("数据太短，无法检测格式".to_string()));
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
            let tag_bytes = [reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?];
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
        } else if flavor == 0x4F54544F { // 'OTTO'
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
        let optional_tables: Vec<String> = table_tags.iter()
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
        use rfont_types::{FontFormat, CompressionType};
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
            let tag_bytes = [reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?];
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
        let optional_tables: Vec<String> = table_tags.iter()
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
        use rfont_types::{FontFormat, CompressionType};
        let mut reader = Reader::new(data);
        
        // 读取 WOFF2 Header
        let _signature = reader.read_u32()?;
        let flavor = reader.read_u32()?;
        let _length = reader.read_u32()?;
        let num_tables = reader.read_u16()?;
        let _reserved = reader.read_u16()?;
        let _total_sfnt_size = reader.read_u32()?;
        
        // 解析 WOFF2 表目录（简化版本，只收集标签）
        let mut table_tags = Vec::new();
        for _ in 0..num_tables {
            // WOFF2 使用变长编码，这里简化处理
            // 实际应该按照 WOFF2 规范解析 flag 和 tag
            let flag = reader.read_u8()?;
            
            // 根据 flag 确定是否有显式 tag
            if (flag & 0x3F) == 0x3F {
                // 需要读取完整的 4 字节 tag
                let tag_bytes = [reader.read_u8()?, reader.read_u8()?, reader.read_u8()?, reader.read_u8()?];
                let tag_str = String::from_utf8_lossy(&tag_bytes).to_string();
                table_tags.push(tag_str);
            } else {
                // 从预定义列表中获取 tag
                let known_index = (flag & 0x3F) as usize;
                if known_index < CORE_WOFF2_KNOWN_TAGS.len() {
                    let tag = CORE_WOFF2_KNOWN_TAGS[known_index];
                    let tag_str = String::from_utf8_lossy(&tag.0).to_string();
                    table_tags.push(tag_str);
                }
            }
            
            // 跳过剩余字段（简化处理）
            // 实际应该正确解析变长整数
            // 这里假设每个条目最多 20 字节
            for _ in 0..20 {
                reader.read_u8().ok();
            }
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
        let optional_tables: Vec<String> = table_tags.iter()
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
