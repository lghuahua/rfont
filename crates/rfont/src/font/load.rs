use rfont_core::{Head, Maxp, Hhea, Loca, Cmap, Hmtx};
use rfont_core::tables::woff::{WoffHeader, WoffTableDirectoryEntry};
use rfont_types::{FontError, Reader, Tag, ReadBytes};
use flate2::read::ZlibDecoder;
use std::io::Read;
use tracing::{debug, info, span, Level};

use crate::font_data::FontData;
use crate::constants::TABLE_DIR_ENTRY_SIZE;

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
}
