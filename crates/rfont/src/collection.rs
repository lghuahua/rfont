use std::path::Path;

use rfont_core::{NameTable, SfntRepack};
use rfont_types::{FontError, FontFormat, ReadBytes, Reader, TableRecord};

use crate::Font;

/// TTC 文件标识
const TTC_TAG: &[u8; 4] = b"ttcf";

/// TTC 头部固定长度（tag + major/minor version + numFonts）
const TTC_HEADER_SIZE: usize = 12;

/// TTC v2 头部在字面偏移数组之后的 DSIG 字段长度
const TTC_V2_DSIG_SIZE: usize = 12;

/// SFNT Offset Table 头部长度
const SFNT_HEADER_SIZE: usize = 12;

/// SFNT 目录项长度
const TABLE_RECORD_SIZE: usize = 16;

/// 字体集合中单个字面的摘要信息
///
/// 只包含列表展示所需的字段，避免为每个字面重建完整 [`Font`]。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FaceSummary {
    /// 字面索引
    pub index: usize,
    /// 字面轮廓格式（TrueType 或 CFF）
    pub format: FontFormat,
    /// 字体家族名称（nameID 1）
    pub family: Option<String>,
    /// 样式名称（nameID 2）
    pub subfamily: Option<String>,
    /// 完整名称（nameID 4）
    pub full_name: Option<String>,
    /// PostScript 名称（nameID 6）
    pub postscript_name: Option<String>,
    /// 字形数量
    pub glyph_count: u16,
}

/// TTC/OTC 字体集合
///
/// 解析 `ttcf` 头与各字面的偏移表，可按索引提取独立的 SFNT 字体。
/// 集合内各字面的表偏移是相对整个集合文件的，因此提取时会重排表数据并
/// 重算校验和，得到可直接使用的 TTF/OTF 字节。
///
/// # 示例
/// ```no_run
/// use rfont::FontCollection;
///
/// let data = std::fs::read("collection.ttc").unwrap();
/// let collection = FontCollection::from_bytes(data).unwrap();
/// for face in collection.faces().unwrap() {
///     println!("{}: {:?}", face.index, face.family);
/// }
/// let bytes = collection.face_bytes(0).unwrap();
/// ```
pub struct FontCollection {
    data: Vec<u8>,
    version: (u16, u16),
    offsets: Vec<u32>,
}

impl FontCollection {
    /// 判断字节数据是否为 TTC/OTC 字体集合
    pub fn is_collection(data: &[u8]) -> bool {
        data.len() >= TTC_TAG.len() && &data[..TTC_TAG.len()] == TTC_TAG
    }

    /// 从字符串路径加载字体集合
    pub fn load(path: impl AsRef<Path>) -> Result<Self, FontError> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|e| FontError::FontLoad {
            path: path.to_path_buf(),
            source: Box::new(FontError::Io(e)),
        })?;

        Self::from_bytes(data).map_err(|e| FontError::FontLoad {
            path: path.to_path_buf(),
            source: Box::new(e),
        })
    }

    /// 从字节数据解析字体集合
    ///
    /// 会校验所有字面的偏移表范围，确保后续提取不会越界。
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, FontError> {
        if data.len() < TTC_HEADER_SIZE {
            return Err(invalid("文件长度不足 12 字节，缺少完整 TTC 头"));
        }
        if &data[..4] != TTC_TAG {
            return Err(invalid("缺少 ttcf 标识"));
        }

        let major = u16::from_be_bytes([data[4], data[5]]);
        let minor = u16::from_be_bytes([data[6], data[7]]);
        if major != 1 && major != 2 {
            return Err(invalid(format!(
                "不支持的 TTC 版本 {}.{}，仅支持 1.x 与 2.x",
                major, minor
            )));
        }

        let num_fonts = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        if num_fonts == 0 {
            return Err(invalid("集合中没有字面"));
        }
        let num_fonts = num_fonts as usize;

        let offsets_start = TTC_HEADER_SIZE;
        let offsets_len = num_fonts
            .checked_mul(4)
            .ok_or_else(|| invalid("字面偏移数组长度溢出"))?;
        let offsets_end = offsets_start
            .checked_add(offsets_len)
            .ok_or_else(|| invalid("字面偏移数组长度溢出"))?;
        if offsets_end > data.len() {
            return Err(invalid("字面偏移数组被截断"));
        }
        if major == 2
            && offsets_end
                .checked_add(TTC_V2_DSIG_SIZE)
                .is_none_or(|end| end > data.len())
        {
            return Err(invalid("TTC 2.x 的 DSIG 字段被截断"));
        }

        let mut offsets = Vec::with_capacity(num_fonts);
        for i in 0..num_fonts {
            let start = offsets_start + i * 4;
            let offset = u32::from_be_bytes([
                data[start],
                data[start + 1],
                data[start + 2],
                data[start + 3],
            ]);
            read_directory(&data, offset)?;
            offsets.push(offset);
        }

        Ok(Self {
            data,
            version: (major, minor),
            offsets,
        })
    }

    /// TTC 版本号（major, minor）
    pub fn version(&self) -> (u16, u16) {
        self.version
    }

    /// 字面数量
    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    /// 是否为空集合
    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    /// 提取指定字面为独立 SFNT 字体字节
    ///
    /// 提取结果按标签排序、重算目录校验和并回填 `head.checkSumAdjustment`。
    pub fn face_bytes(&self, index: usize) -> Result<Vec<u8>, FontError> {
        let offset = *self.offsets.get(index).ok_or({
            FontError::FaceIndexOutOfRange {
                index,
                count: self.offsets.len(),
            }
        })?;

        let (flavor, records) = read_directory(&self.data, offset)?;
        let mut repack = SfntRepack::new(flavor);
        for record in records {
            let start = record.offset as usize;
            let end = start + record.length as usize;
            repack.insert(record.tag, self.data[start..end].to_vec());
        }
        repack.build()
    }

    /// 加载指定字面为 [`Font`]
    ///
    /// 返回的字体保留集合信息：`face_index` 为字面索引，`face_count` 为字面总数。
    pub fn face(&self, index: usize) -> Result<Font, FontError> {
        let mut font = Font::from_bytes(&self.face_bytes(index)?)?;
        font.face_index = index;
        font.face_count = self.offsets.len();
        Ok(font)
    }

    /// 列出所有字面的摘要信息
    pub fn faces(&self) -> Result<Vec<FaceSummary>, FontError> {
        (0..self.len()).map(|index| self.summary(index)).collect()
    }

    fn summary(&self, index: usize) -> Result<FaceSummary, FontError> {
        let offset = self.offsets[index];
        let (flavor, records) = read_directory(&self.data, offset)?;

        let table_bytes = |tag: &[u8; 4]| {
            records
                .iter()
                .find(|record| &record.tag.0 == tag)
                .map(|record| {
                    let start = record.offset as usize;
                    &self.data[start..start + record.length as usize]
                })
        };

        let maxp = table_bytes(b"maxp").ok_or_else(|| FontError::TableNotFound {
            tag: "maxp".to_string(),
        })?;
        if maxp.len() < 6 {
            return Err(FontError::TableTooShort {
                table: "maxp".to_string(),
                min_size: 6,
                actual_size: maxp.len(),
            });
        }
        let glyph_count = u16::from_be_bytes([maxp[4], maxp[5]]);

        let name = table_bytes(b"name")
            .and_then(|bytes| NameTable::read_from(&mut Reader::new(bytes)).ok());

        Ok(FaceSummary {
            index,
            format: face_format(flavor),
            family: name.as_ref().and_then(|n| n.get_name_string(1)),
            subfamily: name.as_ref().and_then(|n| n.get_name_string(2)),
            full_name: name.as_ref().and_then(|n| n.get_name_string(4)),
            postscript_name: name.as_ref().and_then(|n| n.get_name_string(6)),
            glyph_count,
        })
    }
}

fn invalid(reason: impl Into<String>) -> FontError {
    FontError::TtcInvalid {
        reason: reason.into(),
    }
}

fn face_format(flavor: u32) -> FontFormat {
    if flavor == u32::from_be_bytes(*b"OTTO") {
        FontFormat::Otf
    } else {
        FontFormat::Ttf
    }
}

/// 读取字面偏移处的 SFNT 目录，并校验目录与表数据均位于集合范围内。
fn read_directory(data: &[u8], face_offset: u32) -> Result<(u32, Vec<TableRecord>), FontError> {
    let start = face_offset as usize;
    let header_end = start
        .checked_add(SFNT_HEADER_SIZE)
        .ok_or_else(|| invalid("字面偏移溢出"))?;
    if header_end > data.len() {
        return Err(invalid(format!("字面偏移 {face_offset} 超出文件范围")));
    }

    let mut reader = Reader::new(&data[start..]);
    let flavor = reader.read_u32()?;
    let num_tables = reader.read_u16()?;
    reader.skip(6)?;

    let directory_end = SFNT_HEADER_SIZE
        .checked_add(num_tables as usize * TABLE_RECORD_SIZE)
        .ok_or_else(|| invalid("表目录长度溢出"))?;
    if directory_end > data.len() - start {
        return Err(invalid("表目录被截断"));
    }

    let mut records = Vec::with_capacity(num_tables as usize);
    for _ in 0..num_tables {
        let record = TableRecord::read_from(&mut reader)?;
        let table_start = record.offset as usize;
        let table_end = table_start
            .checked_add(record.length as usize)
            .ok_or_else(|| invalid(format!("表 {} 的范围溢出", record.tag)))?;
        if table_end > data.len() {
            return Err(invalid(format!("表 {} 的范围超出集合文件", record.tag)));
        }
        records.push(record);
    }

    Ok((flavor, records))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::Tag;

    fn source_tables() -> Vec<(Tag, Vec<u8>)> {
        let font = Font::load("src/AlimamaDaoLiTi.ttf").unwrap();
        font.font_data()
            .get_table_records()
            .values()
            .map(|record| {
                (
                    record.tag,
                    font.font_data()
                        .get_table_bytes(record.tag)
                        .unwrap()
                        .to_vec(),
                )
            })
            .collect()
    }

    /// 将同一份表集重复写成两个字面，构造最小的合法 TTC。
    fn build_ttc(tables: &[(Tag, Vec<u8>)], num_faces: usize) -> Vec<u8> {
        let header_len = TTC_HEADER_SIZE + 4 * num_faces;
        let mut data = vec![0u8; header_len];
        let mut offsets = Vec::with_capacity(num_faces);

        for _ in 0..num_faces {
            offsets.push(data.len() as u32);

            data.extend_from_slice(&0x0001_0000u32.to_be_bytes());
            let num_tables = tables.len() as u16;
            let (search_range, entry_selector, range_shift) =
                rfont_core::table_header_fields(num_tables);
            data.extend_from_slice(&num_tables.to_be_bytes());
            data.extend_from_slice(&search_range.to_be_bytes());
            data.extend_from_slice(&entry_selector.to_be_bytes());
            data.extend_from_slice(&range_shift.to_be_bytes());

            let directory_start = data.len();
            data.resize(directory_start + tables.len() * TABLE_RECORD_SIZE, 0);

            for (i, (tag, table)) in tables.iter().enumerate() {
                let offset = data.len() as u32;
                data.extend_from_slice(table);
                while !data.len().is_multiple_of(4) {
                    data.push(0);
                }

                let entry = directory_start + i * TABLE_RECORD_SIZE;
                data[entry..entry + 4].copy_from_slice(&tag.0);
                data[entry + 8..entry + 12].copy_from_slice(&offset.to_be_bytes());
                data[entry + 12..entry + 16].copy_from_slice(&(table.len() as u32).to_be_bytes());
            }
        }

        data[..4].copy_from_slice(TTC_TAG);
        data[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        data[8..12].copy_from_slice(&(num_faces as u32).to_be_bytes());
        for (i, offset) in offsets.iter().enumerate() {
            let start = TTC_HEADER_SIZE + i * 4;
            data[start..start + 4].copy_from_slice(&offset.to_be_bytes());
        }

        data
    }

    #[test]
    fn test_collection_lists_faces_and_extracts() {
        let ttc = build_ttc(&source_tables(), 2);
        assert!(FontCollection::is_collection(&ttc));

        let collection = FontCollection::from_bytes(ttc.clone()).unwrap();
        assert_eq!(collection.version(), (1, 0));
        assert_eq!(collection.len(), 2);

        let faces = collection.faces().unwrap();
        assert_eq!(faces.len(), 2);
        assert_eq!(faces[0].family.as_deref(), Some("Alimama DaoLiTi"));
        assert_eq!(faces[1].glyph_count, 7044);
        assert_eq!(faces[0].format, FontFormat::Ttf);
        assert_eq!(faces[0].index, 0);

        let extracted = collection.face_bytes(1).unwrap();
        let font = Font::from_bytes(&extracted).unwrap();
        let info = font.get_font_info().unwrap();
        assert_eq!(info.family.as_deref(), Some("Alimama DaoLiTi"));
        assert_eq!(info.format, FontFormat::Ttf);

        let face = collection.face(1).unwrap();
        let info = face.get_font_info().unwrap();
        assert_eq!((info.face_index, info.face_count), (1, 2));

        let subset = face.subset_builder().text("Hello").build().unwrap();
        assert!(!subset.is_empty());

        let via_font = Font::from_bytes_with_face(&ttc, 1).unwrap();
        assert_eq!(via_font.get_font_info().unwrap().face_index, 1);
    }

    #[test]
    fn test_collection_rejects_invalid_input() {
        assert!(!FontCollection::is_collection(b"ttf"));
        assert!(FontCollection::from_bytes(b"not a ttc".to_vec()).is_err());

        let mut truncated = vec![0u8; 8];
        truncated[..4].copy_from_slice(TTC_TAG);
        assert!(matches!(
            FontCollection::from_bytes(truncated),
            Err(FontError::TtcInvalid { .. })
        ));

        let mut empty = vec![0u8; TTC_HEADER_SIZE];
        empty[..4].copy_from_slice(TTC_TAG);
        empty[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        assert!(matches!(
            FontCollection::from_bytes(empty),
            Err(FontError::TtcInvalid { .. })
        ));

        let mut bad_offset = vec![0u8; TTC_HEADER_SIZE + 4];
        bad_offset[..4].copy_from_slice(TTC_TAG);
        bad_offset[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        bad_offset[8..12].copy_from_slice(&1u32.to_be_bytes());
        bad_offset[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(matches!(
            FontCollection::from_bytes(bad_offset),
            Err(FontError::TtcInvalid { .. })
        ));
    }

    #[test]
    fn test_collection_rejects_out_of_range_face() {
        let ttc = build_ttc(&source_tables(), 1);
        let collection = FontCollection::from_bytes(ttc).unwrap();
        match collection.face(1) {
            Err(FontError::FaceIndexOutOfRange { index, count }) => {
                assert_eq!((index, count), (1, 1));
            }
            Err(other) => panic!("expected FaceIndexOutOfRange, got {other:?}"),
            Ok(_) => panic!("expected FaceIndexOutOfRange, got Ok"),
        }
    }

    #[test]
    fn test_font_from_bytes_with_face_rejects_face_for_single_font() {
        let data = std::fs::read("src/AlimamaDaoLiTi.ttf").unwrap();
        assert!(matches!(
            Font::from_bytes_with_face(&data, 1),
            Err(FontError::FaceIndexOutOfRange { .. })
        ));

        let font = Font::from_bytes_with_face(&data, 0).unwrap();
        assert_eq!(font.get_font_info().unwrap().face_count, 1);
    }
}
