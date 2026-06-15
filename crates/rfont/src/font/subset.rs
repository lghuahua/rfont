use crate::font::load::assemble_ttf;
use crate::Font;
use rfont_types::{Reader, Tag, WriteBytes};
use tracing::{debug, error, info, span, warn, Level};

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

        info.supported_char_count = self.cmap.supported_chars_count();

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
            .map(|ch| self.cmap.get_glyph_id(ch).map_or(0, |v| v))
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

        // 1.5. 解析复合字形的依赖关系（使用懒加载）
        // 只在需要时才解析字形，避免不必要的解析工作
        let glyf_data = self
            .font_data
            .get_table_bytes(rfont_types::Tag(*b"glyf"))
            .ok_or(FontError::TableNotFound {
                tag: "glyf".to_string(),
            })?;

        use rfont_core::tables::glyf_lazy::GlyfLazyLoader;
        let loader = GlyfLazyLoader::new(glyf_data, &self.loca.offsets);
        let resolved_glyphs = loader.resolve_dependencies(&subset_glyphs_vec)?;

        subset_glyphs_vec = resolved_glyphs.into_iter().collect();
        subset_glyphs_vec.sort();

        info!(
            resolved_glyph_count = subset_glyphs_vec.len(),
            "复合字形依赖解析完成"
        );

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
        let index_to_loc_format: u16 = if new_glyf_data.len() < 65536 { 0 } else { 1 };
        let new_head_data = head::update_head(self, 0, index_to_loc_format, true)?;

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

        let mut all_tables = vec![
            (Tag(*b"head"), new_head_data),
            (Tag(*b"maxp"), new_maxp_data),
            (Tag(*b"cmap"), new_cmap_data),
            (Tag(*b"hhea"), hhea_data),
            (Tag(*b"hmtx"), new_hmtx_data),
            (Tag(*b"loca"), new_loca_data),
            (Tag(*b"glyf"), new_glyf_data),
        ];
        for (tag, data) in other_tables {
            debug!(table = ?tag, size = data.len(), "其他表");
            all_tables.push((tag, data));
        }

        let final_font = assemble_ttf(&mut all_tables)?;

        info!(size = final_font.len(), "子集字体生成完成");

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

        // 计算 total_sfnt_size: SFNT Offset Table (12) + Table Directory (num_tables * 16) + 所有表数据（4字节对齐）
        let mut total_sfnt_size = 12u32 + (num_tables as u32) * 16;

        for (tag, _checksum, offset, length) in &table_records {
            // 读取原始表数据
            let mut table_data = ttf_data[*offset as usize..(*offset + *length) as usize].to_vec();

            // 特殊处理 head 表：将 checkSumAdjustment 设置为 0
            if tag.as_str() == "head" && table_data.len() >= 12 {
                // checkSumAdjustment 位于 head 表的第 8-11 字节
                table_data[8] = 0;
                table_data[9] = 0;
                table_data[10] = 0;
                table_data[11] = 0;
            }

            // 每个表数据需要 4 字节对齐
            let padded_length = (*length + 3) & !3;
            total_sfnt_size += padded_length;

            // 准备 4 字节对齐的数据（用于计算 checksum）
            let mut aligned_data = table_data.to_vec();
            if aligned_data.len() < padded_length as usize {
                aligned_data.resize(padded_length as usize, 0);
            }

            // 使用 zlib 压缩
            let compressed_data = if compression_level == 0 {
                // 不压缩，直接使用原始数据
                table_data.clone()
            } else {
                let compression = match compression_level {
                    1..=3 => Compression::fast(),
                    4..=6 => Compression::new(compression_level as u32),
                    _ => Compression::best(),
                };

                let mut encoder = ZlibEncoder::new(Vec::new(), compression);
                encoder
                    .write_all(&table_data)
                    .map_err(|e| FontError::Generic(format!("WOFF compression failed: {}", e)))?;
                let result = encoder.finish().map_err(|e| {
                    FontError::Generic(format!("WOFF compression finish failed: {}", e))
                })?;

                // 如果压缩后反而变大，使用原始数据（不压缩）
                if result.len() >= table_data.len() {
                    info!(
                        tag = tag.as_str(),
                        original_size = table_data.len(),
                        compressed_size = result.len(),
                        "表数据压缩后变大，使用原始数据"
                    );
                    table_data.clone()
                } else {
                    debug!(
                        tag = tag.as_str(),
                        original_size = table_data.len(),
                        compressed_size = result.len(),
                        "表数据压缩成功"
                    );
                    result
                }
            };

            let comp_length = compressed_data.len() as u32;
            let padded_comp_length = (comp_length + 3) & !3; // 对齐到 4 字节

            // 记录表条目信息
            table_entries.push((
                *tag,
                current_offset,
                comp_length,
                padded_comp_length,
                *length,
                padded_length,
            ));

            debug!(
                tag = tag.as_str(),
                orig_length = length,
                padded_length = padded_length,
                comp_length = comp_length,
                offset = current_offset,
                "WOFF 表压缩完成"
            );

            woff_writer.data.extend_from_slice(&compressed_data);
            // 填充到 4 字节边界
            for _ in 0..(padded_comp_length - comp_length) {
                woff_writer.data.push(0);
            }

            current_offset = woff_writer.data.len() as u32;
        }

        // 回填表目录
        for (i, (tag, woff_offset, comp_length, _padded_comp_length, orig_length, padded_length)) in
            table_entries.iter().enumerate()
        {
            let dir_offset = table_dir_start + i * 20;

            // 从原始 TTF 数据中读取表数据（使用原始的 ttf_offset）
            // 注意：table_records 中存储的是 (Tag, checksum, ttf_offset, length)
            // 我们需要找到对应的 ttf_offset
            let ttf_offset = table_records
                .iter()
                .find(|(t, _, _, _)| t == tag)
                .map(|(_, _, o, _)| *o)
                .unwrap();

            let table_data = &ttf_data[ttf_offset as usize..(ttf_offset + *orig_length) as usize];
            let mut aligned_data = table_data.to_vec();
            if aligned_data.len() < *padded_length as usize {
                aligned_data.resize(*padded_length as usize, 0);
            }

            // 计算校验和（基于 4 字节对齐的数据）
            let mut checksum: u32 = 0;
            for chunk in aligned_data.chunks(4) {
                let mut value: u32 = 0;
                for (i, &byte) in chunk.iter().enumerate() {
                    value |= (byte as u32) << (24 - i * 8);
                }
                checksum = checksum.wrapping_add(value);
            }

            woff_writer.data[dir_offset..dir_offset + 4].copy_from_slice(&tag.0);
            woff_writer.data[dir_offset + 4..dir_offset + 8]
                .copy_from_slice(&woff_offset.to_be_bytes());
            woff_writer.data[dir_offset + 8..dir_offset + 12]
                .copy_from_slice(&comp_length.to_be_bytes());
            woff_writer.data[dir_offset + 12..dir_offset + 16]
                .copy_from_slice(&orig_length.to_be_bytes());
            woff_writer.data[dir_offset + 16..dir_offset + 20]
                .copy_from_slice(&checksum.to_be_bytes());

            debug!(
                tag = tag.as_str(),
                dir_offset = dir_offset,
                data_offset = woff_offset,
                checksum = format!("0x{:08X}", checksum),
                "WOFF 表目录回填"
            );
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
        use brotli::BrotliCompress;
        use rfont_types::{Reader, Writer};

        debug!("=== 开始 WOFF2 转换 ===");
        debug!(
            ttf_size = ttf_data.len(),
            compression_level = compression_level
        );

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

        // 按照 WOFF2 规范的顺序对表进行排序
        let predefined_tags = rfont_core::tables::woff2::WOFF2_KNOWN_TAGS;
        let mut sorted_tables: Vec<(Tag, u32, u32)> = table_records.clone();

        debug!(num_tables = num_tables, "解析表目录完成");
        sorted_tables.sort_by(|a, b| {
            let a_idx = predefined_tags.iter().position(|t| t == &a.0);
            let b_idx = predefined_tags.iter().position(|t| t == &b.0);
            match (a_idx, b_idx) {
                (Some(ai), Some(bi)) => ai.cmp(&bi),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.0.as_str().cmp(b.0.as_str()),
            }
        });

        debug!("表排序完成:");
        for (tag, offset, length) in &sorted_tables {
            debug!(table = ?tag, offset = offset, size = length, table_name = tag.as_str());
        }

        // ⭐ 关键改进：对 glyf/loca 表执行转换以提高压缩率
        let mut transformed_tables: std::collections::HashMap<Tag, Vec<u8>> =
            std::collections::HashMap::new();

        // 查找 glyf、loca 和 head 表
        let glyf_record = sorted_tables
            .iter()
            .find(|(tag, _, _)| tag.as_str() == "glyf");
        let loca_record = sorted_tables
            .iter()
            .find(|(tag, _, _)| tag.as_str() == "loca");
        let head_record = sorted_tables
            .iter()
            .find(|(tag, _, _)| tag.as_str() == "head");

        if let (Some((_, glyf_offset, glyf_length)), Some((_, loca_offset, loca_length))) =
            (glyf_record, loca_record)
        {
            use rfont_core::tables::glyf_lazy::GlyfLazyLoader;
            use rfont_core::tables::glyf_transform::transform_glyf_and_loca;

            // 提取 glyf 和 loca 数据
            let glyf_data =
                &ttf_data[*glyf_offset as usize..(*glyf_offset + *glyf_length) as usize];
            let loca_data =
                &ttf_data[*loca_offset as usize..(*loca_offset + *loca_length) as usize];

            // 从 head 表获取 loca 格式（offset 51-52）
            let index_to_loc_format = if let Some((_, head_offset, _)) = head_record {
                if *head_offset as usize + 51 < ttf_data.len() {
                    ttf_data[*head_offset as usize + 51]
                } else {
                    1 // 默认为 long 格式
                }
            } else {
                1 // 默认为 long 格式
            };

            debug!(
                index_to_loc_format = index_to_loc_format,
                loca_table_size = loca_length,
                "开始解析 loca 表"
            );

            // 解析 loca 表获取偏移量
            let mut loca_offsets = Vec::new();
            if index_to_loc_format == 0 {
                // Short format: 2 bytes per offset, values are divided by 2
                let mut i = 0;
                while i + 2 <= loca_data.len() {
                    let offset = u16::from_be_bytes([loca_data[i], loca_data[i + 1]]) as u32 * 2;
                    loca_offsets.push(offset);
                    i += 2;
                }
            } else {
                // Long format: 4 bytes per offset
                let mut i = 0;
                while i + 4 <= loca_data.len() {
                    let offset = u32::from_be_bytes([
                        loca_data[i],
                        loca_data[i + 1],
                        loca_data[i + 2],
                        loca_data[i + 3],
                    ]);
                    loca_offsets.push(offset);
                    i += 4;
                }
            }

            debug!(glyph_count = loca_offsets.len() - 1, "loca 表解析完成");

            // 使用懒加载器解析所有字形
            let loader = GlyfLazyLoader::new(glyf_data, &loca_offsets);
            let num_glyphs = (loca_offsets.len() - 1) as u16;

            let mut all_glyphs = Vec::with_capacity(num_glyphs as usize);
            for glyph_id in 0..num_glyphs {
                match loader.load_glyph(glyph_id) {
                    Ok(Some(record)) => all_glyphs.push(record),
                    Ok(None) => {} // 空字形
                    Err(e) => {
                        warn!(glyph_id = glyph_id, error = ?e, "加载字形失败");
                    }
                }
            }

            debug!(loaded_glyphs = all_glyphs.len(), "字形加载完成");

            // 执行 glyf/loca 转换
            debug!("开始 glyf/loca 转换，字形数量 = {}", all_glyphs.len());
            match transform_glyf_and_loca(&all_glyphs, index_to_loc_format) {
                Ok(transformed_glyf) => {
                    debug!("glyf/loca 转换成功");
                    let original_size = *glyf_length as f64;
                    let transformed_size = transformed_glyf.len() as f64;
                    let ratio = (1.0 - transformed_size / original_size) * 100.0;

                    info!(
                        original_glyf_size = *glyf_length,
                        transformed_glyf_size = transformed_glyf.len(),
                        original_loca_size = *loca_length,
                        compression_ratio = format!("{:.1}%", ratio),
                        "glyf 表转换成功"
                    );

                    transformed_tables.insert(Tag(*b"glyf"), transformed_glyf);
                    transformed_tables.insert(Tag(*b"loca"), Vec::new());
                }
                Err(e) => {
                    error!(error = ?e, "glyf/loca 转换失败，将使用原始数据");
                    // 转换失败时使用原始数据
                }
            }
        }

        // ⭐ 先拼接表数据流，然后使用实际大小作为 total_sfnt_size
        // 注意：total_sfnt_size 应该是解压后的完整 SFNT 大小（包括 SFNT 头、表目录和所有表数据）

        // 将所有表数据按 WOFF2 规范顺序拼接成一个流,无需进行4字对齐
        let mut uncompressed_table_stream = Vec::new();
        for (tag, offset, length) in &sorted_tables {
            // 如果该表已转换，使用转换后的数据
            if let Some(transformed_data) = transformed_tables.get(tag) {
                uncompressed_table_stream.extend_from_slice(transformed_data);
            } else {
                let table_data = &ttf_data[*offset as usize..(*offset as usize + *length as usize)];
                uncompressed_table_stream.extend_from_slice(table_data);
            }
        }

        info!(
            stream_size = uncompressed_table_stream.len(),
            "表数据流拼接完成"
        );

        // Brotli 压缩表数据流
        let quality = compression_level.min(11) as i32;
        let lgwin = 22;
        let mut compressed_table_stream = Vec::new();
        BrotliCompress(
            &mut &uncompressed_table_stream[..],
            &mut compressed_table_stream,
            &brotli::enc::BrotliEncoderParams {
                quality,
                lgwin,
                mode: brotli::enc::backward_references::BrotliEncoderMode::BROTLI_MODE_FONT,
                ..Default::default()
            },
        )
        .map_err(|e| FontError::Generic(format!("WOFF2 stream compression failed: {:?}", e)))?;

        let total_compressed_size = compressed_table_stream.len() as u32;
        info!(compressed_size = total_compressed_size, "Brotli 压缩完成");

        // 构建 WOFF2 文件
        let mut woff2_writer = Writer::new();

        // WOFF2 Header (48 bytes)
        let header_offset = woff2_writer.data.len();
        woff2_writer.write_u32(0x774F4632)?; // signature 'wOF2'
        woff2_writer.write_u32(sfnt_version)?; // flavor
        woff2_writer.write_u32(0)?; // length (稍后回填)
        woff2_writer.write_u16(num_tables)?;
        woff2_writer.write_u16(0)?; // reserved
        woff2_writer.write_u32(ttf_data.len() as u32)?; // total_sfnt_size
        woff2_writer.write_u32(total_compressed_size)?; // total_compressed_size
        woff2_writer.write_u16(1)?; // major_version
        woff2_writer.write_u16(0)?; // minor_version
        woff2_writer.write_u32(0)?; // meta_offset (no metadata)
        woff2_writer.write_u32(0)?; // meta_length
        woff2_writer.write_u32(0)?; // meta_orig_length
        woff2_writer.write_u32(0)?; // priv_offset (no private data)
        woff2_writer.write_u32(0)?; // priv_length

        // 写入表目录（使用 Base128 编码）
        for (tag, _offset, length) in &sorted_tables {
            let tag_index = predefined_tags.iter().position(|t| t == tag);
            let is_glyf_loca = tag.as_str() == "glyf" || tag.as_str() == "loca";

            // 确定原始长度和转换后的长度
            let orig_length = *length;
            let transform_length = if let Some(transformed_data) = transformed_tables.get(tag) {
                transformed_data.len() as u32
            } else {
                orig_length
            };

            // 确定 table_type 和是否需要写入自定义标签
            let (table_type, needs_custom_tag) = if let Some(idx) = tag_index {
                (idx as u8, false)
            } else {
                (63, true)
            };

            // 确定 transform_version
            // glyf/loca: 已转换=0, 未转换=3 (null transform)
            // 其他表: 0
            let transform_version = if is_glyf_loca && transformed_tables.contains_key(tag) {
                0u8 // 已转换
            } else if is_glyf_loca {
                3u8 // null transform
            } else {
                0u8
            };

            // 计算并写入 flags: [transform_version(2 bits) | table_type(6 bits)]
            let flags = (transform_version << 6) | table_type;
            woff2_writer.write_u8(flags)?;

            // 如果是自定义标签，写入 4 字节 tag
            if needs_custom_tag {
                tag.write_to(&mut woff2_writer)?;
            }

            // 写入 origLength (Base128 编码)
            woff2_writer.write_base128(orig_length)?;

            // 条件性写入 transformLength
            // 根据 WOFF2 规范：glyf/loca 表且 transform_version != 3 时必须写入
            if is_glyf_loca && transform_version != 3 {
                woff2_writer.write_base128(transform_length)?;
            }
        }

        // 写入压缩后的表数据流
        woff2_writer
            .data
            .extend_from_slice(&compressed_table_stream);

        // 回填总长度
        woff2_writer.pad_to_4_bytes();
        let total_length = woff2_writer.data.len() as u32;

        woff2_writer.data[header_offset + 8..header_offset + 12]
            .copy_from_slice(&total_length.to_be_bytes());

        debug!(
            total_sfnt_size = ttf_data.len() as u32,
            total_compressed_size = total_compressed_size,
            woff2_size = total_length,
            "WOFF2 文件生成完成"
        );

        Ok(woff2_writer.data)
    }
}
