use crate::Font;
use crate::font::load::assemble_ttf;
use rfont_types::Tag;
use tracing::{Level, debug, info, span};

use crate::subset::builder::FontSubsetBuilder;
use crate::subset::options::SubsetOptions;
use crate::subset::tables::{cmap, head, hmtx, maxp, post};
use rfont_types::FontError;

impl Font {
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

                if start < end
                    && let Some(glyf_bytes) = self.font_data.get_table_bytes(Tag(*b"glyf"))
                    && end as usize <= glyf_bytes.len()
                {
                    let glyph_data = &glyf_bytes[start as usize..end as usize];
                    chunk.push((glyph_id as u16, glyph_data));
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

        // 1.5. 解析复合字形的依赖关系并提取 glyf/loca 数据（优化版本）
        // 使用 resolve_and_extract 一次性完成依赖解析和数据提取，减少 I/O 操作
        let glyf_data = self
            .font_data
            .get_table_bytes(rfont_types::Tag(*b"glyf"))
            .ok_or(FontError::TableNotFound {
                tag: "glyf".to_string(),
            })?;

        use rfont_core::tables::glyf_lazy::GlyfLazyLoader;
        let loader = GlyfLazyLoader::new(glyf_data, &self.loca.offsets);
        let (resolved_glyphs, new_loca_data, new_glyf_data) = 
            loader.resolve_and_extract(&subset_glyphs_vec)?;

        subset_glyphs_vec = resolved_glyphs;

        info!(
            resolved_glyph_count = subset_glyphs_vec.len(),
            loca_size = new_loca_data.len(),
            glyf_size = new_glyf_data.len(),
            "复合字形依赖解析和数据提取完成（优化版本）"
        );

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

            if start < end
                && let Some(glyf_bytes) = self.font.font_data.get_table_bytes(Tag(*b"glyf"))
                && end as usize <= glyf_bytes.len()
            {
                glyph_data = Some(&glyf_bytes[start as usize..end as usize]);
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
