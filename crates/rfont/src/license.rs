//! 字体授权与嵌入限制的识别
//!
//! 字体文件里能读到的授权线索只有两处：`OS/2.fsType` 声明嵌入权限，
//! name 表的 nameID 13/14 记录许可证文本与网址。
//! 二者只反映字体作者的填写，实际授权以随字体分发的协议文件为准，
//! 因此本模块只做识别与提示，不做拦截。

/// `OS/2.fsType` bit 1：禁止嵌入
pub const FS_TYPE_NO_EMBEDDING: u16 = 0x0002;
/// `OS/2.fsType` bit 2：仅允许预览与打印
pub const FS_TYPE_PREVIEW_AND_PRINT: u16 = 0x0004;
/// `OS/2.fsType` bit 3：允许可编辑嵌入
pub const FS_TYPE_EDITABLE_EMBEDDING: u16 = 0x0008;
/// `OS/2.fsType` bit 8：禁止子集化
pub const FS_TYPE_NO_SUBSETTING: u16 = 0x0100;
/// `OS/2.fsType` bit 9：仅允许位图嵌入（禁止嵌入轮廓）
pub const FS_TYPE_BITMAP_ONLY: u16 = 0x0200;

/// 从字体元信息中识别出的许可证类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum FontLicense {
    /// 未识别到许可证声明
    #[default]
    Unknown,
    /// SIL Open Font License
    SilOpenFontLicense,
    /// Apache License
    ApacheLicense,
    /// GNU General Public License
    GnuGeneralPublicLicense,
    /// MIT License
    MitLicense,
    /// 仅声明保留所有权利的专有字体
    Proprietary,
}

impl FontLicense {
    /// 展示名称
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "未声明",
            Self::SilOpenFontLicense => "SIL Open Font License",
            Self::ApacheLicense => "Apache License",
            Self::GnuGeneralPublicLicense => "GNU GPL",
            Self::MitLicense => "MIT License",
            Self::Proprietary => "专有授权",
        }
    }
}

/// 字体声明的授权与嵌入限制
///
/// 由 `OS/2.fsType` 与 name 表的许可证记录推导，字段含义见 OpenType 规范。
/// 提示文案由本模块统一生成，命令行与图形界面共用同一套措辞。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FontRestrictions {
    /// 原始 `OS/2.fsType` 值
    pub fs_type: u16,
    /// 识别出的许可证类型
    pub license: FontLicense,
    /// 禁止嵌入（fsType bit 1）
    pub no_embedding: bool,
    /// 仅允许预览与打印（fsType bit 2）
    pub preview_and_print_only: bool,
    /// 允许可编辑嵌入（fsType bit 3）
    pub editable_embedding: bool,
    /// 禁止子集化（fsType bit 8）
    pub no_subsetting: bool,
    /// 仅允许位图嵌入（fsType bit 9）
    pub bitmap_only: bool,
    /// 许可证声明了保留字体名，改名可能违反授权
    pub reserved_font_name: bool,
    /// 嵌入权限的中文说明
    pub embedding_label: String,
    /// 子集化前的风险提示
    pub subset_warnings: Vec<String>,
    /// 改名前的风险提示
    pub rename_warnings: Vec<String>,
}

impl FontRestrictions {
    /// 从 `fsType` 与许可证记录推导限制
    ///
    /// `license_text` 为 nameID 13，`license_url` 为 nameID 14。
    pub fn new(fs_type: u16, license_text: Option<&str>, license_url: Option<&str>) -> Self {
        let license_text = license_text.unwrap_or_default();
        let license_url = license_url.unwrap_or_default();

        let no_embedding = fs_type & FS_TYPE_NO_EMBEDDING != 0;
        let preview_and_print_only = fs_type & FS_TYPE_PREVIEW_AND_PRINT != 0;
        let editable_embedding = fs_type & FS_TYPE_EDITABLE_EMBEDDING != 0;
        let no_subsetting = fs_type & FS_TYPE_NO_SUBSETTING != 0;
        let bitmap_only = fs_type & FS_TYPE_BITMAP_ONLY != 0;
        let reserved_font_name = has_reserved_font_name(license_text);

        let embedding = if no_embedding {
            "禁止嵌入"
        } else if preview_and_print_only {
            "仅可预览与打印"
        } else if bitmap_only {
            "仅可位图嵌入"
        } else if editable_embedding {
            "可编辑嵌入"
        } else if fs_type == 0 {
            "可安装嵌入"
        } else {
            "未声明限制"
        };
        // 禁止子集化与嵌入权限是独立的位，但同属一条声明，合并展示避免误读
        let embedding_label = if no_subsetting {
            format!("{embedding} · 禁止子集化")
        } else {
            embedding.to_string()
        };

        let mut subset_warnings = Vec::new();
        if no_subsetting {
            subset_warnings.push(
                "字体声明禁止子集化（fsType bit 8），子集化后再分发可能违反授权。".to_string(),
            );
        }
        if no_embedding {
            subset_warnings
                .push("字体声明禁止嵌入（fsType bit 1），分发子集字体可能违反授权。".to_string());
        }
        if bitmap_only {
            subset_warnings
                .push("字体声明仅允许位图嵌入（fsType bit 9），子集化会改变字形轮廓。".to_string());
        }
        if preview_and_print_only {
            subset_warnings.push(
                "字体仅允许预览与打印（fsType bit 2），将子集字体嵌入网页或文档后再分发可能受限。"
                    .to_string(),
            );
        }

        let mut rename_warnings = Vec::new();
        if reserved_font_name {
            rename_warnings.push(
                "许可证声明了保留字体名（Reserved Font Name），改名后再分发可能违反授权。"
                    .to_string(),
            );
        }
        if no_embedding {
            rename_warnings
                .push("字体声明禁止嵌入（fsType bit 1），改名后再分发可能违反授权。".to_string());
        }
        if preview_and_print_only {
            rename_warnings.push(
                "字体仅允许预览与打印（fsType bit 2），改名后的字体嵌入到文档可能受限。"
                    .to_string(),
            );
        }

        Self {
            fs_type,
            license: detect_license(license_text, license_url),
            no_embedding,
            preview_and_print_only,
            editable_embedding,
            no_subsetting,
            bitmap_only,
            reserved_font_name,
            embedding_label: embedding_label.to_string(),
            subset_warnings,
            rename_warnings,
        }
    }

    /// 是否存在需要提示的限制
    pub fn is_restricted(&self) -> bool {
        self.no_embedding
            || self.preview_and_print_only
            || self.no_subsetting
            || self.bitmap_only
            || self.reserved_font_name
    }
}

/// 从许可证文本与网址中识别许可证类型
fn detect_license(license_text: &str, license_url: &str) -> FontLicense {
    let haystack = format!("{license_text}\n{license_url}").to_lowercase();

    if haystack.contains("open font license") {
        FontLicense::SilOpenFontLicense
    } else if haystack.contains("apache license") || haystack.contains("apache.org/licenses") {
        FontLicense::ApacheLicense
    } else if haystack.contains("gnu general public license") || haystack.contains("gnu gpl") {
        FontLicense::GnuGeneralPublicLicense
    } else if haystack.contains("mit license") || haystack.contains("mit-license") {
        FontLicense::MitLicense
    } else if haystack.contains("all rights reserved") || haystack.contains("版权所有") {
        FontLicense::Proprietary
    } else {
        FontLicense::Unknown
    }
}

/// 判断许可证文本是否声明了保留字体名
///
/// OFL 模板里 “"Reserved Font Name" refers to ...” 只是名词解释，
/// 只有以该词开头且没有解释从句的行才视为真正的保留声明。
fn has_reserved_font_name(license_text: &str) -> bool {
    license_text.lines().any(|line| {
        let line = line.trim().to_lowercase();
        line.starts_with("reserved font name") && !line.contains("refers to")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_type_bits_match_spec() {
        // 规范：bit 1 禁止嵌入、bit 2 预览打印、bit 3 可编辑、bit 8 禁止子集、bit 9 仅位图
        let r = FontRestrictions::new(0x0100, None, None);
        assert!(r.no_subsetting);
        assert!(!r.bitmap_only);

        let r = FontRestrictions::new(0x0200, None, None);
        assert!(r.bitmap_only);
        assert!(!r.no_subsetting);
    }

    #[test]
    fn test_embedding_permission_flags() {
        let r = FontRestrictions::new(0x0002, None, None);
        assert!(r.no_embedding);
        assert_eq!(r.embedding_label, "禁止嵌入");

        let r = FontRestrictions::new(0x0004, None, None);
        assert!(r.preview_and_print_only);
        assert_eq!(r.embedding_label, "仅可预览与打印");

        let r = FontRestrictions::new(0x0008, None, None);
        assert!(r.editable_embedding);
        assert_eq!(r.embedding_label, "可编辑嵌入");

        // 禁止子集化与嵌入权限并列展示
        let r = FontRestrictions::new(0x0108, None, None);
        assert_eq!(r.embedding_label, "可编辑嵌入 · 禁止子集化");

        let r = FontRestrictions::new(0, None, None);
        assert_eq!(r.embedding_label, "可安装嵌入");
        assert!(!r.is_restricted());
        assert!(r.subset_warnings.is_empty());
        assert!(r.rename_warnings.is_empty());
    }

    #[test]
    fn test_subset_warnings_cover_no_subsetting() {
        let r = FontRestrictions::new(0x0100, None, None);
        assert_eq!(r.subset_warnings.len(), 1);
        assert!(r.subset_warnings[0].contains("禁止子集化"));
        assert!(r.rename_warnings.is_empty());
    }

    #[test]
    fn test_rename_warnings_cover_reserved_font_name() {
        let license = "Copyright (c) 2024 Someone\nReserved Font Name: MyFont";
        let r = FontRestrictions::new(0, Some(license), None);
        assert!(r.reserved_font_name);
        assert_eq!(r.rename_warnings.len(), 1);
        assert!(r.rename_warnings[0].contains("保留字体名"));
    }

    #[test]
    fn test_ofl_template_definition_is_not_reserved_name() {
        let license = "\"Reserved Font Name\" refers to any names specified as such after the copyright statement(s).";
        let r = FontRestrictions::new(0, Some(license), None);
        assert!(!r.reserved_font_name);
    }

    #[test]
    fn test_detect_license_kinds() {
        assert_eq!(
            detect_license(
                "This Font Software is licensed under the SIL Open Font License, Version 1.1.",
                ""
            ),
            FontLicense::SilOpenFontLicense
        );
        assert_eq!(
            detect_license("", "https://www.apache.org/licenses/LICENSE-2.0"),
            FontLicense::ApacheLicense
        );
        assert_eq!(
            detect_license("Licensed under the GNU General Public License", ""),
            FontLicense::GnuGeneralPublicLicense
        );
        assert_eq!(
            detect_license("Released under the MIT License", ""),
            FontLicense::MitLicense
        );
        assert_eq!(
            detect_license("Copyright 2024. All rights reserved.", ""),
            FontLicense::Proprietary
        );
        assert_eq!(detect_license("", ""), FontLicense::Unknown);
    }

    #[test]
    fn test_license_label_is_not_empty() {
        assert_eq!(FontLicense::Unknown.label(), "未声明");
        assert_eq!(
            FontLicense::SilOpenFontLicense.label(),
            "SIL Open Font License"
        );
    }
}
