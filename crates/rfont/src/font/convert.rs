use crate::Font;
use rfont_types::FontError;
use rfont_types::{Tag, WriteBytes, Writer};
use std::io::Write;
use tracing::{debug, error, info, warn};

impl Font {
    /// 将 TTF 数据转换为 WOFF 格式
    ///
    /// 使用 zlib 压缩将 TTF 字体数据转换为 WOFF 格式。
    /// WOFF 格式在保持与 TTF 相同结构的同时，对每个表数据进行压缩以减小文件大小。
    ///
    /// # 参数
    /// - `ttf_data`: TTF 格式的字体数据
    /// - `compression_level`: zlib 压缩级别（0-9），0 表示不压缩，1 表示最快，9 表示最高压缩
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
        use flate2::Compression;
        use flate2::write::ZlibEncoder;

        debug!("=== 开始 WOFF 转换 ===");
        debug!(
            ttf_size = ttf_data.len(),
            compression_level = compression_level
        );

        // 解析 TTF 数据结构（复用 FontData 的目录解析）
        let table_map = crate::font_data::FontData::parse_directory(ttf_data)?;
        let sfnt_version = u32::from_be_bytes([ttf_data[0], ttf_data[1], ttf_data[2], ttf_data[3]]);
        let num_tables = table_map.len() as u16;
        let table_records: Vec<(Tag, u32, u32, u32)> = table_map
            .into_values()
            .map(|r| (r.tag, r.checksum, r.offset, r.length))
            .collect();

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
                    .map_err(|e| FontError::WoffCompressionError {
                        message: format!("压缩表数据失败: {}", e),
                    })?;
                let result = encoder
                    .finish()
                    .map_err(|e| FontError::WoffCompressionError {
                        message: format!("完成压缩失败: {}", e),
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
        use rfont_types::Writer;

        debug!("=== 开始 WOFF2 转换 ===");
        debug!(
            ttf_size = ttf_data.len(),
            compression_level = compression_level
        );

        // 解析 TTF 数据结构（复用 FontData 的目录解析）
        let table_map = crate::font_data::FontData::parse_directory(ttf_data)?;
        let sfnt_version = u32::from_be_bytes([ttf_data[0], ttf_data[1], ttf_data[2], ttf_data[3]]);
        let num_tables = table_map.len() as u16;

        // 按照 WOFF2 规范的顺序对表进行排序
        let predefined_tags = rfont_core::tables::woff2::WOFF2_KNOWN_TAGS;
        let mut sorted_tables: Vec<(Tag, u32, u32)> = table_map
            .values()
            .map(|r| (r.tag, r.offset, r.length))
            .collect();

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
        .map_err(|e| FontError::Woff2DecompressionError {
            message: format!("Brotli 压缩流失败: {:?}", e),
        })?;

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
