//! 字体元信息编辑：修改 name 表中的字体名、作者、版权等字段。
//!
//! 编辑只改写 `name` 表与 `head.modified`，其余表原样保留，
//! 因此字形数据、度量与布局完全不变。

use rfont_core::tables::name::{NameEdit, NameTable};
use rfont_core::{SfntRepack, build_name_table};
use rfont_types::{FontError, SFNT_VERSION_TTF, Tag};

use crate::Font;

/// head 表中 modified 字段的字节偏移
const HEAD_MODIFIED_OFFSET: usize = 28;

/// PostScript 名称允许的最大长度
const POSTSCRIPT_NAME_MAX_LEN: usize = 63;

/// PostScript 名称中不允许出现的字符
const POSTSCRIPT_NAME_FORBIDDEN: &[char] = &['[', ']', '{', '}', '(', ')', '<', '>', '/', '%'];

/// 可编辑的字体元信息
///
/// 字段为 `None` 表示不修改该项。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FontMetaEdit {
    /// 家族名称（nameID 1）
    pub family: Option<String>,
    /// 样式名称（nameID 2）
    pub subfamily: Option<String>,
    /// 完整名称（nameID 4），未指定时由家族名与样式名组合
    pub full_name: Option<String>,
    /// 版本字符串（nameID 5）
    pub version: Option<String>,
    /// PostScript 名称（nameID 6）
    pub postscript_name: Option<String>,
    /// 制造商（nameID 8）
    pub manufacturer: Option<String>,
    /// 设计者 / 作者（nameID 9）
    pub designer: Option<String>,
    /// 版权声明（nameID 0）
    pub copyright: Option<String>,
    /// 许可证描述（nameID 13）
    pub license: Option<String>,
}

impl FontMetaEdit {
    /// 是否没有需要修改的字段
    pub fn is_empty(&self) -> bool {
        self.family.is_none()
            && self.subfamily.is_none()
            && self.full_name.is_none()
            && self.version.is_none()
            && self.postscript_name.is_none()
            && self.manufacturer.is_none()
            && self.designer.is_none()
            && self.copyright.is_none()
            && self.license.is_none()
    }

    /// 校验编辑项
    pub fn validate(&self) -> Result<(), FontError> {
        if let Some(name) = &self.postscript_name {
            validate_postscript_name(name)?;
        }
        Ok(())
    }
}

/// 校验 PostScript 名称
///
/// OpenType 规定 PostScript 名称必须是长度不超过 63 的 ASCII 可见字符，
/// 且不能包含空格与 `[] {} () < > / %`。
fn validate_postscript_name(value: &str) -> Result<String, FontError> {
    if value.is_empty() {
        return Err(FontError::InvalidPostScriptName {
            reason: "名称不能为空".to_string(),
        });
    }

    if value.chars().count() > POSTSCRIPT_NAME_MAX_LEN {
        return Err(FontError::InvalidPostScriptName {
            reason: format!("长度超过 {} 个字符", POSTSCRIPT_NAME_MAX_LEN),
        });
    }

    if let Some(c) = value.chars().find(|c| !c.is_ascii_graphic()) {
        return Err(FontError::InvalidPostScriptName {
            reason: format!("包含非 ASCII 可见字符 '{}'", c),
        });
    }

    if let Some(c) = value
        .chars()
        .find(|c| POSTSCRIPT_NAME_FORBIDDEN.contains(c))
    {
        return Err(FontError::InvalidPostScriptName {
            reason: format!("包含非法字符 '{}'", c),
        });
    }

    Ok(value.to_string())
}

/// 把名称转换成 PostScript 名称允许的字符
fn sanitize_postscript_name(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect()
}

/// 由家族名与样式名推导 PostScript 名称
///
/// 非 ASCII 字符会被丢弃，推导结果为空时表示无法推导。
fn derive_postscript_name(family: Option<&str>, subfamily: Option<&str>) -> String {
    let family = family.map(sanitize_postscript_name).unwrap_or_default();
    let subfamily = subfamily.map(sanitize_postscript_name).unwrap_or_default();

    match (family.is_empty(), subfamily.is_empty()) {
        (true, true) => String::new(),
        (true, false) => subfamily,
        (false, true) => family,
        (false, false) => format!("{}-{}", family, subfamily),
    }
}

/// 组合完整名称
fn join_full_name(family: Option<&str>, subfamily: Option<&str>) -> String {
    [family.unwrap_or_default(), subfamily.unwrap_or_default()]
        .iter()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

impl Font {
    /// 依据编辑项与当前名称表生成 name 表编辑列表
    fn meta_edits(&self, edit: &FontMetaEdit) -> Result<Vec<NameEdit>, FontError> {
        let current = self.font_data.parse_table::<NameTable>(Tag(*b"name")).ok();
        let current_family = current.as_ref().and_then(|table| table.get_name_string(1));
        let current_subfamily = current.as_ref().and_then(|table| table.get_name_string(2));

        let family = edit.family.clone().or(current_family);
        let subfamily = edit.subfamily.clone().or(current_subfamily);
        let renamed = edit.family.is_some() || edit.subfamily.is_some();

        let mut edits: Vec<NameEdit> = Vec::new();

        if let Some(value) = &edit.family {
            edits.push((1, value.clone()));
        }
        if let Some(value) = &edit.subfamily {
            edits.push((2, value.clone()));
        }

        // 完整名称：未显式指定时由家族名与样式名组合
        match &edit.full_name {
            Some(value) => edits.push((4, value.clone())),
            None if renamed => {
                let full_name = join_full_name(family.as_deref(), subfamily.as_deref());
                if !full_name.is_empty() {
                    edits.push((4, full_name));
                }
            }
            None => {}
        }

        // PostScript 名称：显式指定时校验，未指定时在可推导的情况下同步更新
        match &edit.postscript_name {
            Some(value) => edits.push((6, validate_postscript_name(value)?)),
            None if renamed => {
                let derived = derive_postscript_name(family.as_deref(), subfamily.as_deref());
                if !derived.is_empty() {
                    edits.push((6, derived));
                }
            }
            None => {}
        }

        if let Some(value) = &edit.version {
            edits.push((5, value.clone()));
        }
        if let Some(value) = &edit.manufacturer {
            edits.push((8, value.clone()));
        }
        if let Some(value) = &edit.designer {
            edits.push((9, value.clone()));
        }
        if let Some(value) = &edit.copyright {
            edits.push((0, value.clone()));
        }
        if let Some(value) = &edit.license {
            edits.push((13, value.clone()));
        }

        Ok(edits)
    }

    /// 生成带当前修改时间的 head 表
    fn head_with_new_timestamp(&self) -> Result<Vec<u8>, FontError> {
        let mut head = self
            .font_data
            .get_table_bytes(Tag(*b"head"))
            .ok_or_else(|| FontError::TableNotFound {
                tag: "head".to_string(),
            })?
            .to_vec();

        if head.len() < HEAD_MODIFIED_OFFSET + 8 {
            return Err(FontError::TableTooShort {
                table: "head".to_string(),
                min_size: HEAD_MODIFIED_OFFSET + 8,
                actual_size: head.len(),
            });
        }

        let seconds = crate::clock::now().as_seconds();
        head[HEAD_MODIFIED_OFFSET..HEAD_MODIFIED_OFFSET + 8]
            .copy_from_slice(&seconds.to_be_bytes());

        Ok(head)
    }

    /// 应用元信息编辑，返回新的 SFNT 字体字节（TTF）
    ///
    /// 只改写 `name` 表并刷新 `head.modified`，其余表原样保留。
    ///
    /// # 参数
    /// - `edit`: 需要修改的元信息，`None` 字段保持原值
    ///
    /// # 示例
    /// ```no_run
    /// use rfont::{Font, FontMetaEdit};
    ///
    /// let font = Font::load("font.ttf").unwrap();
    /// let edit = FontMetaEdit {
    ///     family: Some("My Font".to_string()),
    ///     designer: Some("Alice".to_string()),
    ///     ..Default::default()
    /// };
    /// let data = font.edit_metadata(&edit).unwrap();
    /// ```
    pub fn edit_metadata(&self, edit: &FontMetaEdit) -> Result<Vec<u8>, FontError> {
        let edits = self.meta_edits(edit)?;
        if edits.is_empty() {
            return Ok(self.font_data.as_bytes().to_vec());
        }

        let name_table = self.font_data.parse_table::<NameTable>(Tag(*b"name")).ok();
        let name_bytes = build_name_table(name_table.as_ref(), &edits)?;

        let flavor = self.font_data.sfnt_flavor().unwrap_or(SFNT_VERSION_TTF);

        let mut repack = SfntRepack::new(flavor);
        for (tag, record) in self.font_data.get_table_records() {
            if *tag == Tag(*b"name") {
                continue;
            }

            let start = record.offset as usize;
            let end = start + record.length as usize;
            repack.insert(*tag, self.font_data.as_bytes()[start..end].to_vec());
        }

        repack.insert(Tag(*b"name"), name_bytes);
        repack.insert(Tag(*b"head"), self.head_with_new_timestamp()?);

        repack.build()
    }

    /// 应用元信息编辑并按指定格式输出
    ///
    /// # 参数
    /// - `edit`: 需要修改的元信息
    /// - `output_format`: 输出格式，"ttf" / "woff" / "woff2"
    /// - `compression_level`: 压缩级别（WOFF: 0-9，WOFF2: 0-11，TTF 忽略）
    pub fn edit_metadata_to(
        &self,
        edit: &FontMetaEdit,
        output_format: &str,
        compression_level: u8,
    ) -> Result<Vec<u8>, FontError> {
        let sfnt = self.edit_metadata(edit)?;

        match output_format.to_lowercase().as_str() {
            "ttf" => Ok(sfnt),
            "woff" => self.convert_to_woff(&sfnt, compression_level),
            "woff2" => self.convert_to_woff2(&sfnt, compression_level),
            _ => Err(FontError::UnsupportedOutputFormat {
                format: output_format.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_font() -> Font {
        Font::load("src/AlimamaDaoLiTi.ttf").unwrap()
    }

    fn family_of(data: &[u8]) -> Option<String> {
        let font = Font::from_bytes(data).unwrap();
        font.get_font_info().unwrap().family
    }

    #[test]
    fn edit_family_name_updates_name_table() {
        let font = test_font();
        let edit = FontMetaEdit {
            family: Some("Renamed Font".to_string()),
            ..Default::default()
        };

        let data = font.edit_metadata(&edit).unwrap();
        assert_eq!(family_of(&data).as_deref(), Some("Renamed Font"));
    }

    #[test]
    fn edit_preserves_glyph_data() {
        let font = test_font();
        let edit = FontMetaEdit {
            designer: Some("Alice".to_string()),
            ..Default::default()
        };

        let data = font.edit_metadata(&edit).unwrap();
        let edited = Font::from_bytes(&data).unwrap();
        let original_info = font.get_font_info().unwrap();
        let edited_info = edited.get_font_info().unwrap();

        assert_eq!(original_info.glyph_count, edited_info.glyph_count);
        assert_eq!(original_info.units_per_em, edited_info.units_per_em);
        assert_eq!(original_info.tables.len(), edited_info.tables.len());
    }

    #[test]
    fn edit_designer_and_copyright() {
        let font = test_font();
        let edit = FontMetaEdit {
            designer: Some("Alice".to_string()),
            copyright: Some("(c) 2026".to_string()),
            ..Default::default()
        };

        let data = font.edit_metadata(&edit).unwrap();
        let info = Font::from_bytes(&data).unwrap().get_font_info().unwrap();

        assert_eq!(info.designer.as_deref(), Some("Alice"));
        assert_eq!(info.copyright.as_deref(), Some("(c) 2026"));
    }

    #[test]
    fn edit_without_changes_returns_original_bytes() {
        let font = test_font();
        let data = font.edit_metadata(&FontMetaEdit::default()).unwrap();
        assert_eq!(data, font.font_data.as_bytes());
    }

    #[test]
    fn edit_derives_full_and_postscript_names() {
        let font = test_font();
        let edit = FontMetaEdit {
            family: Some("My Font".to_string()),
            subfamily: Some("Bold".to_string()),
            ..Default::default()
        };

        let data = font.edit_metadata(&edit).unwrap();
        let info = Font::from_bytes(&data).unwrap().get_font_info().unwrap();

        assert_eq!(info.family.as_deref(), Some("My Font"));
        assert_eq!(info.subfamily.as_deref(), Some("Bold"));
        assert_eq!(info.full_name.as_deref(), Some("My Font Bold"));
        assert_eq!(info.postscript_name.as_deref(), Some("MyFont-Bold"));
    }

    #[test]
    fn edit_keeps_postscript_name_when_not_derivable() {
        let font = test_font();
        let edit = FontMetaEdit {
            family: Some("中文名称".to_string()),
            ..Default::default()
        };

        let data = font.edit_metadata(&edit).unwrap();
        let info = Font::from_bytes(&data).unwrap().get_font_info().unwrap();

        assert_eq!(info.family.as_deref(), Some("中文名称"));
        assert!(info.postscript_name.is_some());
    }

    #[test]
    fn edit_rejects_invalid_postscript_name() {
        let font = test_font();
        let edit = FontMetaEdit {
            postscript_name: Some("Bad Name".to_string()),
            ..Default::default()
        };

        assert!(matches!(
            font.edit_metadata(&edit),
            Err(FontError::InvalidPostScriptName { .. })
        ));
    }

    #[test]
    fn edit_accepts_valid_postscript_name() {
        let font = test_font();
        let edit = FontMetaEdit {
            postscript_name: Some("Custom-Font_1.0".to_string()),
            ..Default::default()
        };

        let data = font.edit_metadata(&edit).unwrap();
        let info = Font::from_bytes(&data).unwrap().get_font_info().unwrap();
        assert_eq!(info.postscript_name.as_deref(), Some("Custom-Font_1.0"));
    }

    #[test]
    fn edit_updates_head_modified() {
        let font = test_font();
        let original = font.head.modified.clone().as_seconds();

        let data = font
            .edit_metadata(&FontMetaEdit {
                family: Some("Renamed".to_string()),
                ..Default::default()
            })
            .unwrap();

        let edited = Font::from_bytes(&data).unwrap();
        assert!(edited.head.modified.as_seconds() >= original);
    }

    #[test]
    fn edit_outputs_requested_format() {
        let font = test_font();
        let edit = FontMetaEdit {
            family: Some("Renamed Font".to_string()),
            ..Default::default()
        };

        let woff = font.edit_metadata_to(&edit, "woff", 9).unwrap();
        assert_eq!(&woff[0..4], b"wOFF");

        let woff2 = font.edit_metadata_to(&edit, "woff2", 11).unwrap();
        assert_eq!(&woff2[0..4], b"wOF2");

        let ttf = font.edit_metadata_to(&edit, "ttf", 0).unwrap();
        assert_eq!(family_of(&ttf).as_deref(), Some("Renamed Font"));
    }

    #[test]
    fn edit_works_on_compressed_source() {
        let font = Font::load("src/AlimamaDaoLiTi.woff").unwrap();
        let edit = FontMetaEdit {
            family: Some("Woff Renamed".to_string()),
            ..Default::default()
        };

        let ttf = font.edit_metadata_to(&edit, "ttf", 0).unwrap();
        assert_eq!(&ttf[0..4], &[0x00, 0x01, 0x00, 0x00]);
        assert_eq!(family_of(&ttf).as_deref(), Some("Woff Renamed"));
    }

    #[test]
    fn edit_rejects_unknown_output_format() {
        let font = test_font();
        let edit = FontMetaEdit {
            family: Some("Renamed".to_string()),
            ..Default::default()
        };

        assert!(matches!(
            font.edit_metadata_to(&edit, "svg", 0),
            Err(FontError::UnsupportedOutputFormat { .. })
        ));
    }

    #[test]
    fn validate_reports_invalid_postscript_name() {
        let edit = FontMetaEdit {
            postscript_name: Some("bad/name".to_string()),
            ..Default::default()
        };
        assert!(edit.validate().is_err());

        let edit = FontMetaEdit {
            postscript_name: Some("Good-Name".to_string()),
            ..Default::default()
        };
        assert!(edit.validate().is_ok());
    }
}
