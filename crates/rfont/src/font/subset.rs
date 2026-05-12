use crate::Font;
use rfont_types::{Reader, Tag};
use tracing::{debug, span, Level};

use crate::checksum::calc_sfnt_checksum;
use crate::constants::SFNT_CHECKSUM_MAGIC;
use crate::info::{FontInfo, TableInfo};
use crate::subset::builder::FontSubsetBuilder;
use crate::subset::options::SubsetOptions;
use crate::subset::tables::{cmap, glyf_loca, head, hmtx, maxp, post};
use rfont_types::FontError;

impl Font {
    /// 获取字体元数据信息
    ///
    /// 从已加载的字体表中提取关键元数据，包括字形数量、度量信息、表列表等。
    ///
    /// # 返回值
    /// `FontInfo` 结构体，包含字体的基本信息
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// let info = font.get_font_info();
    /// println!("字形数量: {}", info.glyph_count);
    /// println!("支持的字符数: {}", info.supported_char_count);
    /// ```
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
        info.ascender = self.hhea.ascender.0; // FWord 包装类型，需要解包
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
        info.supported_char_count = self.cmap.unicode_map.len();

        // TODO: 从 name 表提取 family_name, style_name, version
        // 这需要解析 name 表，暂时留为 None

        info
    }

    /// 获取所有表的列表
    ///
    /// 返回字体中所有表的详细信息（标签、校验和、偏移量、长度）。
    ///
    /// # 返回值
    /// `Vec<TableInfo>` 包含所有表的信息
    pub fn get_table_list(&self) -> Vec<TableInfo> {
        self.font_data
            .get_table_records()
            .values()
            .map(TableInfo::from_record)
            .collect()
    }

    /// 获取字体支持的所有 Unicode 字符
    ///
    /// 从 cmap 表中提取所有映射的 Unicode 码点，并排序返回。
    ///
    /// # 返回值
    /// 排序后的 Unicode 码点列表
    pub fn get_supported_characters(&self) -> Vec<u32> {
        let mut chars: Vec<u32> = self.cmap.unicode_map.keys().cloned().collect();
        chars.sort();
        chars
    }

    /// 检查字体是否支持特定字符
    ///
    /// # 参数
    /// - `unicode`: Unicode 码点
    ///
    /// # 返回值
    /// - `true`: 字体支持该字符
    /// - `false`: 字体不支持该字符
    pub fn supports_character(&self, unicode: u32) -> bool {
        self.cmap.unicode_map.contains_key(&unicode)
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
            .filter_map(|ch| {
                let unicode = ch as u32;
                self.cmap.unicode_map.get(&unicode).copied()
            })
            .collect()
    }

    /// 流式字形迭代器：逐字形处理，减少内存峰值
    ///
    /// 返回一个 `GlyphIterator`，可以逐个遍历字体中的所有字形。
    /// 适合处理大型字体，避免一次性加载所有字形数据到内存。
    ///
    /// # 返回值
    /// `GlyphIterator` 迭代器，每次返回 `(glyph_id, Option<&[u8]>)`
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// for (glyph_id, glyph_data) in font.glyph_iter() {
    ///     if let Some(data) = glyph_data {
    ///         println!("字形 {}: {} bytes", glyph_id, data.len());
    ///     }
    /// }
    /// ```
    pub fn glyph_iter(&self) -> GlyphIterator<'_> {
        GlyphIterator {
            font: self,
            current_index: 0,
            total_glyphs: self.maxp.num_glyphs as usize,
        }
    }

    /// 批量获取字形数据（分块处理）
    ///
    /// 将字形分成多个块进行处理，每处理完一个块就调用处理器函数。
    /// 这种方式可以在处理大型字体时控制内存使用。
    ///
    /// # 参数
    /// - `chunk_size`: 每个块的字形数量
    /// - `processor`: 处理函数，接收 `(glyph_id, glyph_data)` 元组的向量
    ///
    /// # 错误
    /// 如果处理器函数返回错误，会立即停止处理并传播错误
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// font.get_glyphs_chunked(100, |chunk| {
    ///     println!("处理块，包含 {} 个字形", chunk.len());
    ///     Ok(())
    /// }).unwrap();
    /// ```
    pub fn get_glyphs_chunked<F>(
        &self,
        chunk_size: usize,
        mut processor: F,
    ) -> Result<(), FontError>
    where
        F: FnMut(Vec<(u16, &[u8])>) -> Result<(), FontError>,
    {
        let total_glyphs = self.maxp.num_glyphs as usize;
        let mut chunk = Vec::with_capacity(chunk_size);

        for glyph_id in 0..total_glyphs {
            if glyph_id < self.loca.offsets.len() - 1 {
                let start = self.loca.offsets[glyph_id];
                let end = self.loca.offsets[glyph_id + 1];

                if start < end {
                    if let Some(glyf_bytes) = self.font_data.get_table_bytes(Tag(*b"glyf")) {
                        if end as usize <= glyf_bytes.len() {
                            let glyph_data = &glyf_bytes[start as usize..end as usize];
                            chunk.push((glyph_id as u16, glyph_data));
                        }
                    }
                }
            }

            // 当块达到指定大小或处理完所有字形时，调用处理器
            if (chunk.len() >= chunk_size || glyph_id == total_glyphs - 1) && !chunk.is_empty() {
                processor(std::mem::take(&mut chunk))?;
            }
        }

        Ok(())
    }

    /// 创建子集化 Builder（Builder 模式）
    ///
    /// 返回一个 `FontSubsetBuilder`，支持链式调用配置子集化参数。
    /// 这是创建字体子集的推荐方式，提供灵活的配置选项。
    ///
    /// # 返回值
    /// `FontSubsetBuilder` 构建器
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    ///
    /// // 基于文本创建子集
    /// let subset = font.subset_builder()
    ///     .text("Hello World")
    ///     .build()
    ///     .unwrap();
    ///
    /// // 基于 Unicode 范围创建子集
    /// let subset = font.subset_builder()
    ///     .unicode_range(0x0041, 0x005A) // A-Z
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn subset_builder(&self) -> FontSubsetBuilder<'_> {
        FontSubsetBuilder::new(self)
    }

    /// 使用配置选项进行子集化
    ///
    /// 根据指定的字形 ID 列表和配置选项执行子集化操作。
    /// 支持输出为 TTF、WOFF 或 WOFF2 格式。
    ///
    /// # 参数
    /// - `glyph_ids`: 要包含在子集中的字形 ID 列表
    /// - `options`: 子集化配置选项（格式、压缩级别等）
    ///
    /// # 返回值
    /// - `Ok(Vec<u8>)`: 子集化后的字体数据
    /// - `Err(FontError)`: 子集化失败时的错误信息
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::{Font, SubsetOptions};
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// let glyph_ids = vec![0, 1, 2, 3]; // 包含 .notdef
    /// let options = SubsetOptions::web_optimized();
    ///
    /// let subset_data = font.subset_with_options(&glyph_ids, &options).unwrap();
    /// ```
    pub fn subset_with_options(
        &self,
        glyph_ids: &[u16],
        options: &SubsetOptions,
    ) -> Result<Vec<u8>, FontError> {
        use tracing::debug;
        debug!(
            glyph_count = glyph_ids.len(),
            format = options.output_format,
            "开始子集化"
        );

        // 执行核心子集化逻辑
        let subset_data = self.subset_and_serialize(glyph_ids)?;

        // 根据输出格式处理
        match options.output_format.as_str() {
            "woff" => self.convert_to_woff(&subset_data, options.compression_level),
            "woff2" => self.convert_to_woff2(&subset_data, options.compression_level),
            _ => {
                // 默认返回 TTF
                Ok(subset_data)
            }
        }
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
        let span = span!(
            Level::TRACE,
            "get_glyph_ids_for_text",
            text_len = text.len()
        );
        let _enter = span.enter();

        text.chars()
            .map(|ch| {
                self.cmap
                    .unicode_map
                    .get(&(ch as u32))
                    .copied()
                    .unwrap_or(0)
            })
            .collect()
    }

    /// 子集化并序列化字体
    ///
    /// 执行完整的子集化流程：提取字形、重建表、计算校验和、组装最终字体。
    /// 这是子集化的核心方法，返回标准的 TTF 格式数据。
    ///
    /// # 参数
    /// - `glyph_ids`: 要包含在子集中的字形 ID 列表
    ///
    /// # 返回值
    /// - `Ok(Vec<u8>)`: 子集化后的 TTF 字体数据
    /// - `Err(FontError)`: 子集化失败时的错误信息
    ///
    /// # 注意
    /// 此方法会自动包含 .notdef (glyph 0)，并对字形 ID 进行排序和去重。
    pub fn subset_and_serialize(&self, glyph_ids: &[u16]) -> Result<Vec<u8>, FontError> {
        let span = span!(
            Level::INFO,
            "subset_and_serialize",
            input_glyphs = glyph_ids.len()
        );
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
        let (new_loca_data, new_glyf_data) =
            glyf_loca::extract_glyf_and_loca(self, &subset_glyphs_vec)?;

        // 3. 构建新的 cmap
        let new_cmap_data = cmap::rebuild_cmap(self, &subset_glyphs_vec)?;

        // 4. 构建新的 hmtx
        let new_hmtx_data = hmtx::rebuild_hmtx(self, &subset_glyphs_vec)?;

        // 5. 更新 maxp
        let new_maxp_data = maxp::update_maxp(self, subset_glyphs_vec.len() as u16)?;

        // 6. 更新 head（校验和、修改时间等）- 暂时传入 0，稍后在 assemble_ttf 中更新
        let new_head_data = head::update_head(self, 0)?;

        // 7. 复制其他不变的表（name, os2, post 等）
        let num_glyphs_subset = subset_glyphs_vec.len() as u16;
        let mut other_tables = self.copy_unchanged_tables(num_glyphs_subset)?;

        // 从 other_tables 中提取 hhea
        let hhea_data = other_tables
            .iter()
            .position(|(tag, _)| *tag == Tag(*b"hhea"))
            .map(|idx| other_tables.remove(idx).1)
            .ok_or(FontError::TableNotFound {
                tag: "hhea".to_string(),
            })?;

        // 打印各表大小统计
        debug!("各表大小统计:");
        debug!(table = "head", size = new_head_data.len());
        debug!(table = "maxp", size = new_maxp_data.len());
        debug!(table = "cmap", size = new_cmap_data.len());
        debug!(table = "hhea", size = hhea_data.len());
        debug!(table = "hmtx", size = new_hmtx_data.len());
        debug!(table = "loca", size = new_loca_data.len());
        debug!(table = "glyf", size = new_glyf_data.len());
        for (tag, data) in &other_tables {
            debug!(table = ?tag, size = data.len(), "其他表");
        }

        // 8. 组装最终的 TTF 文件
        let final_font = self.assemble_ttf(
            &new_head_data,
            &new_maxp_data,
            &new_cmap_data,
            &hhea_data,
            &new_hmtx_data,
            &new_loca_data,
            &new_glyf_data,
            &other_tables,
        )?;

        info!(size = final_font.len(), "子集字体生成完成");

        // 验证校验和
        if let Some(_head_bytes) = self.font_data.get_table_bytes(Tag(*b"head")) {
            let head_offset = final_font.iter().position(|&b| b == b'h').and_then(|i| {
                if i + 16 <= final_font.len() && &final_font[i..i + 4] == b"head" {
                    Some(u32::from_be_bytes([
                        final_font[i + 8],
                        final_font[i + 9],
                        final_font[i + 10],
                        final_font[i + 11],
                    ]) as usize)
                } else {
                    None
                }
            });

            if let Some(offset) = head_offset {
                if offset + 12 <= final_font.len() {
                    let checksum_adj = u32::from_be_bytes([
                        final_font[offset + 8],
                        final_font[offset + 9],
                        final_font[offset + 10],
                        final_font[offset + 11],
                    ]);
                    debug!(
                        checksum = format!("0x{:08X}", checksum_adj),
                        "head.checkSumAdjustment"
                    );
                }
            }
        }

        Ok(final_font)
    }

    /// 复制其他不变的表
    fn copy_unchanged_tables(
        &self,
        num_glyphs_subset: u16,
    ) -> Result<Vec<(Tag, Vec<u8>)>, FontError> {
        let mut tables = Vec::new();

        // 复制并更新 hhea 表
        if let Some(hhea_bytes) = self.font_data.get_table_bytes(Tag(*b"hhea")) {
            let mut hhea_data = hhea_bytes.to_vec();

            // 更新 number_of_h_metrics 为子集后的字形数量
            // number_of_h_metrics 位于 offset 34-35
            hhea_data[34] = (num_glyphs_subset >> 8) as u8;
            hhea_data[35] = (num_glyphs_subset & 0xFF) as u8;

            tables.push((Tag(*b"hhea"), hhea_data));
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
                all_tables.push((*tag, data.clone()));
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

        let max_pow2: u32 = if num_tables > 0 {
            // 找到小于等于 num_tables 的最大 2 的幂
            (1u32 << (16 - num_tables.leading_zeros() - 1)).min(num_tables as u32)
        } else {
            1
        };
        let search_range = max_pow2 * 16;
        let entry_selector = max_pow2.trailing_zeros() as u16;
        let range_shift = ((num_tables as u32) * 16).saturating_sub(search_range) as u16;

        offset_table.extend_from_slice(&(search_range as u16).to_be_bytes());
        offset_table.extend_from_slice(&entry_selector.to_be_bytes());
        offset_table.extend_from_slice(&range_shift.to_be_bytes());

        // 构建临时 Table Directory（checksum 和 offset 暂为 0）
        let mut table_dir = Vec::new();
        for (tag, data) in all_tables.iter() {
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
        let final_check_sum_adjustment =
            SFNT_CHECKSUM_MAGIC.saturating_sub((total_checksum & 0xFFFFFFFF) as u32);

        // 第五步：生成最终的 head 表数据
        let final_head_data = head::update_head(self, final_check_sum_adjustment)?;

        // 第六步：组装最终字体文件
        let mut font_data = Vec::new();
        font_data.extend_from_slice(&SFNT_VERSION_TTF.to_be_bytes()); // sfnt version (TrueType)
        font_data.extend_from_slice(&num_tables.to_be_bytes());

        let max_pow2: u32 = if num_tables > 0 {
            // 找到小于等于 num_tables 的最大 2 的幂
            (1u32 << (16 - num_tables.leading_zeros() - 1)).min(num_tables as u32)
        } else {
            1
        };
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
        for (tag, data) in all_tables.iter() {
            let offset = font_data.len() as u32;
            table_offsets.push(offset);

            if tag.0 == *b"head" {
                // 使用更新后的 head 数据
                font_data.extend_from_slice(&final_head_data);

                // 4 字节对齐
                let padding = (4 - (final_head_data.len() % 4)) % 4;
                font_data.extend(std::iter::repeat_n(0, padding));
            } else {
                font_data.extend_from_slice(data);

                // 4 字节对齐
                let padding = (4 - (data.len() % 4)) % 4;
                font_data.extend(std::iter::repeat_n(0, padding));
            }
        }

        // 第七步：回填 Table Directory（包含正确的校验和和偏移量）
        for (i, (tag, data)) in all_tables.iter().enumerate() {
            let dir_offset = table_dir_start + i * TABLE_DIR_ENTRY_SIZE;
            font_data[dir_offset..dir_offset + 4].copy_from_slice(&tag.0);

            let checksum = if tag.0 == *b"head" {
                calc_sfnt_checksum(&final_head_data)
            } else {
                table_checksums[i]
            };

            font_data[dir_offset + 4..dir_offset + 8].copy_from_slice(&checksum.to_be_bytes());
            font_data[dir_offset + 8..dir_offset + 12]
                .copy_from_slice(&table_offsets[i].to_be_bytes());
            font_data[dir_offset + 12..dir_offset + 16]
                .copy_from_slice(&(data.len() as u32).to_be_bytes());
        }

        Ok(font_data)
    }
}

/// 流式字形迭代器
///
/// 实现 `Iterator` trait，可以逐个遍历字体中的所有字形。
/// 每次迭代返回 `(glyph_id, Option<&[u8]>)`，其中第二个元素是字形的原始数据（如果存在）。
///
/// # 示例
/// ```no_run
/// use rfont::Font;
///
/// let font = Font::load("font.ttf").unwrap();
/// for (glyph_id, glyph_data) in font.glyph_iter() {
///     match glyph_data {
///         Some(data) => println!("字形 {}: {} bytes", glyph_id, data.len()),
///         None => println!("字形 {}: 空", glyph_id),
///     }
/// }
/// ```
pub struct GlyphIterator<'a> {
    font: &'a Font,
    current_index: usize,
    total_glyphs: usize,
}

impl<'a> Iterator for GlyphIterator<'a> {
    type Item = (u16, Option<&'a [u8]>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.total_glyphs {
            return None;
        }

        let glyph_id = self.current_index as u16;
        let mut glyph_data = None;

        // 获取字形数据
        if self.current_index < self.font.loca.offsets.len() - 1 {
            let start = self.font.loca.offsets[self.current_index];
            let end = self.font.loca.offsets[self.current_index + 1];

            if start < end {
                if let Some(glyf_bytes) = self.font.font_data.get_table_bytes(Tag(*b"glyf")) {
                    if end as usize <= glyf_bytes.len() {
                        glyph_data = Some(&glyf_bytes[start as usize..end as usize]);
                    }
                }
            }
        }

        self.current_index += 1;
        Some((glyph_id, glyph_data))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total_glyphs - self.current_index;
        (remaining, Some(remaining))
    }
}

impl Font {
    /// 将 TTF 数据转换为 WOFF 格式
    ///
    /// 使用 zlib 压缩将 TTF 字体数据转换为 WOFF 格式。
    /// WOFF (Web Open Font Format) 是专为 Web 设计的字体格式，具有更好的压缩率。
    ///
    /// # 参数
    /// - `ttf_data`: TTF 格式的字体数据
    /// - `compression_level`: 压缩级别（0-9），0 表示不压缩，9 表示最大压缩
    ///
    /// # 返回值
    /// - `Ok(Vec<u8>)`: WOFF 格式的字体数据
    /// - `Err(FontError)`: 转换失败时的错误信息
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// let ttf_data = font.subset_and_serialize(&[0, 1, 2]).unwrap();
    /// let woff_data = font.convert_to_woff(&ttf_data, 6).unwrap();
    /// ```
    pub fn convert_to_woff(
        &self,
        ttf_data: &[u8],
        compression_level: u8,
    ) -> Result<Vec<u8>, FontError> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use rfont_types::Writer;
        use std::io::Write;

        // 解析 TTF 数据结构
        let mut reader = Reader::new(ttf_data);
        let sfnt_version = reader.read_u32()?;
        let num_tables = reader.read_u16()?;
        let _search_range = reader.read_u16()?;
        let _entry_selector = reader.read_u16()?;
        let _range_shift = reader.read_u16()?;

        // 读取表目录
        let mut table_records = Vec::new();
        for _ in 0..num_tables {
            let tag_bytes = [
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
            ];
            let checksum = reader.read_u32()?;
            let offset = reader.read_u32()?;
            let length = reader.read_u32()?;
            table_records.push((Tag(tag_bytes), checksum, offset, length));
        }

        // 构建 WOFF 数据
        let mut woff_writer = Writer::new();

        // WOFF Header
        woff_writer.write_u32(0x774F4646)?; // signature 'wOFF'
        woff_writer.write_u32(sfnt_version)?; // flavor
        woff_writer.write_u32(0)?; // length (稍后回填)
        woff_writer.write_u16(num_tables)?;
        woff_writer.write_u16(0)?; // reserved
        woff_writer.write_u32(0)?; // total_sfnt_size (稍后计算)
        woff_writer.write_u16(0)?; // major_version
        woff_writer.write_u16(0)?; // minor_version
        woff_writer.write_u32(0)?; // meta_offset
        woff_writer.write_u32(0)?; // meta_length
        woff_writer.write_u32(0)?; // meta_orig_length
        woff_writer.write_u32(0)?; // priv_offset
        woff_writer.write_u32(0)?; // priv_length

        // 写入表目录（占位）
        let table_dir_start = woff_writer.data.len();
        for _ in &table_records {
            woff_writer.write_u32(0)?; // tag
            woff_writer.write_u32(0)?; // offset
            woff_writer.write_u32(0)?; // comp_length
            woff_writer.write_u32(0)?; // orig_length
            woff_writer.write_u32(0)?; // checksum
        }

        // 压缩并写入表数据
        let mut table_entries = Vec::new();
        let mut current_offset = woff_writer.data.len() as u32;
        let mut total_sfnt_size = 0u32;

        for (tag, checksum, offset, length) in &table_records {
            let table_data = &ttf_data[*offset as usize..(*offset + *length) as usize];
            total_sfnt_size += length + (4 - (length % 4)) % 4; // 对齐到 4 字节

            // 使用 zlib 压缩
            let compression = match compression_level {
                0 => Compression::none(),
                1..=3 => Compression::fast(),
                4..=6 => Compression::new(compression_level as u32),
                _ => Compression::best(),
            };

            let mut encoder = ZlibEncoder::new(Vec::new(), compression);
            encoder
                .write_all(table_data)
                .map_err(|e| FontError::Generic(format!("WOFF compression failed: {}", e)))?;
            let compressed_data = encoder.finish().map_err(|e| {
                FontError::Generic(format!("WOFF compression finish failed: {}", e))
            })?;

            let comp_length = compressed_data.len() as u32;
            let padded_comp_length = (comp_length + 3) & !3; // 对齐到 4 字节

            table_entries.push((*tag, current_offset, comp_length, *length, *checksum));

            woff_writer.data.extend_from_slice(&compressed_data);
            // 填充到 4 字节边界
            for _ in 0..(padded_comp_length - comp_length) {
                woff_writer.data.push(0);
            }

            current_offset = woff_writer.data.len() as u32;
        }

        // 回填表目录
        for (i, (tag, offset, comp_length, orig_length, checksum)) in
            table_entries.iter().enumerate()
        {
            let dir_offset = table_dir_start + i * 20;
            woff_writer.data[dir_offset..dir_offset + 4].copy_from_slice(&tag.0);
            woff_writer.data[dir_offset + 4..dir_offset + 8].copy_from_slice(&offset.to_be_bytes());
            woff_writer.data[dir_offset + 8..dir_offset + 12]
                .copy_from_slice(&comp_length.to_be_bytes());
            woff_writer.data[dir_offset + 12..dir_offset + 16]
                .copy_from_slice(&orig_length.to_be_bytes());
            woff_writer.data[dir_offset + 16..dir_offset + 20]
                .copy_from_slice(&checksum.to_be_bytes());
        }

        // 回填 header 中的长度字段
        let total_length = woff_writer.data.len() as u32;
        woff_writer.data[8..12].copy_from_slice(&total_length.to_be_bytes());
        woff_writer.data[16..20].copy_from_slice(&total_sfnt_size.to_be_bytes());

        Ok(woff_writer.data)
    }

    /// 将 TTF 数据转换为 WOFF2 格式
    ///
    /// 使用 Brotli 压缩将 TTF 字体数据转换为 WOFF2 格式。
    /// WOFF2 是 WOFF 的下一代格式，提供更高的压缩率（通常比 WOFF 小 30%）。
    ///
    /// # 参数
    /// - `ttf_data`: TTF 格式的字体数据
    /// - `compression_level`: Brotli 压缩质量（0-11），0 表示最快，11 表示最高压缩
    ///
    /// # 返回值
    /// - `Ok(Vec<u8>)`: WOFF2 格式的字体数据
    /// - `Err(FontError)`: 转换失败时的错误信息
    ///
    /// # 注意
    /// 当前实现使用简化的 WOFF2 编码，完整的 WOFF2 规范需要更复杂的表转换。
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::Font;
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// let ttf_data = font.subset_and_serialize(&[0, 1, 2]).unwrap();
    /// let woff2_data = font.convert_to_woff2(&ttf_data, 4).unwrap();
    /// ```
    pub fn convert_to_woff2(
        &self,
        ttf_data: &[u8],
        compression_level: u8,
    ) -> Result<Vec<u8>, FontError> {
        use brotli::CompressorWriter;
        use rfont_types::Writer;
        use std::io::Write;

        // 解析 TTF 数据结构
        let mut reader = Reader::new(ttf_data);
        let sfnt_version = reader.read_u32()?;
        let num_tables = reader.read_u16()?;
        let _search_range = reader.read_u16()?;
        let _entry_selector = reader.read_u16()?;
        let _range_shift = reader.read_u16()?;

        // 读取表目录
        let mut table_records = Vec::new();
        for _ in 0..num_tables {
            let tag_bytes = [
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
                reader.read_u8()?,
            ];
            let _checksum = reader.read_u32()?;
            let offset = reader.read_u32()?;
            let length = reader.read_u32()?;
            table_records.push((Tag(tag_bytes), offset, length));
        }

        // 构建要压缩的数据块（所有表数据按顺序拼接）
        let mut uncompressed_data = Vec::new();
        let mut total_sfnt_size = 0u32;

        for (_tag, offset, length) in &table_records {
            let table_data = &ttf_data[*offset as usize..(*offset + *length) as usize];
            uncompressed_data.extend_from_slice(table_data);
            total_sfnt_size += length + (4 - (length % 4)) % 4; // 对齐到 4 字节
        }

        // 使用 Brotli 压缩
        let quality = compression_level.min(11) as u32; // Brotli quality 0-11
        let lgwin = 22u32; // Window size

        let mut compressor = CompressorWriter::new(Vec::new(), 4096, quality, lgwin);
        compressor
            .write_all(&uncompressed_data)
            .map_err(|e| FontError::Generic(format!("WOFF2 compression failed: {}", e)))?;
        let compressed_data = compressor.into_inner();

        // 构建 WOFF2 文件
        let mut woff2_writer = Writer::new();

        // WOFF2 Header
        woff2_writer.write_u32(0x774F4632)?; // signature 'wOF2'
        woff2_writer.write_u32(sfnt_version)?; // flavor
        woff2_writer.write_u32(0)?; // length (稍后回填)
        woff2_writer.write_u16(num_tables)?;
        woff2_writer.write_u16(0)?; // reserved
        woff2_writer.write_u32(total_sfnt_size)?; // total_sfnt_size

        // 简化的 WOFF2 表目录（这里使用简单格式，实际 WOFF2 有更复杂的编码）
        // 注意：完整的 WOFF2 实现需要更复杂的表转换和编码
        // 这里提供一个基本实现

        // 写入压缩数据
        woff2_writer.data.extend_from_slice(&compressed_data);

        // 回填总长度
        let total_length = woff2_writer.data.len() as u32;
        woff2_writer.data[8..12].copy_from_slice(&total_length.to_be_bytes());

        debug!(
            original_size = ttf_data.len(),
            compressed_size = compressed_data.len(),
            "WOFF2 转换完成"
        );

        Ok(woff2_writer.data)
    }
}
