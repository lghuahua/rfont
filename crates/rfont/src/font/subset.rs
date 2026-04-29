use crate::Font;
use rfont_types::Tag;
use tracing::span;
use tracing::Level;

use crate::info::{FontInfo, TableInfo};
use crate::subset::options::SubsetOptions;
use crate::subset::builder::FontSubsetBuilder;
use crate::subset::tables::{cmap, glyf_loca, hmtx, maxp, head, post};
use crate::checksum::calc_sfnt_checksum;
use crate::constants::SFNT_CHECKSUM_MAGIC;
use rfont_types::FontError;

impl Font {
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
    pub fn subset_builder(&self) -> FontSubsetBuilder<'_> {
        FontSubsetBuilder::new(self)
    }
    
    /// 使用配置选项进行子集化
    pub fn subset_with_options(&self, glyph_ids: &[u16], options: &SubsetOptions) -> Result<Vec<u8>, FontError> {
        use tracing::debug;
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
        
        use tracing::info;
        // 1. 构建 GlyphID 集合（确保包含 .notdef，即 ID 0）
        let mut subset_glyphs_vec: Vec<u16> = glyph_ids.to_vec();
        if !subset_glyphs_vec.contains(&0) {
            subset_glyphs_vec.push(0); // .notdef 必须存在
        }
        subset_glyphs_vec.sort();
        subset_glyphs_vec.dedup(); // 去重
        
        info!(glyph_count = subset_glyphs_vec.len(), "开始子集化处理");
        
        // 2. 提取 glyf 和 loca 数据
        let (new_loca_data, new_glyf_data) = glyf_loca::extract_glyf_and_loca(self, &subset_glyphs_vec)?;
        
        // 3. 构建新的 cmap
        let new_cmap_data = cmap::rebuild_cmap(self, &subset_glyphs_vec)?;
        
        // 4. 构建新的 hmtx
        let new_hmtx_data = hmtx::rebuild_hmtx(self, &subset_glyphs_vec)?;
        
        // 5. 更新 maxp
        let new_maxp_data = maxp::update_maxp(self, subset_glyphs_vec.len() as u16)?;
        
        // 6. 更新 head（校验和、修改时间等）- 暂时传入 0，稍后在 assemble_ttf 中更新
        let new_head_data = head::update_head(self, 0)?;
        
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
            let subset_post = post::subset_post_table(self, post_bytes)?;
            tables.push((Tag(*b"post"), subset_post));
        }
        
        Ok(tables)
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
        use crate::constants::{SFNT_VERSION_TTF, TABLE_DIR_ENTRY_SIZE};
        
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
                let head_with_zero_adj = head::update_head(self, 0)?;
                total_checksum += calc_sfnt_checksum(&head_with_zero_adj) as u64;
            } else {
                total_checksum += table_checksums[i] as u64;
            }
        }
        
        // 第四步：计算最终的 checkSumAdjustment
        let final_check_sum_adjustment = SFNT_CHECKSUM_MAGIC.saturating_sub((total_checksum & 0xFFFFFFFF) as u32);

        // 第五步：生成最终的 head 表数据
        let final_head_data = head::update_head(self, final_check_sum_adjustment)?;

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
