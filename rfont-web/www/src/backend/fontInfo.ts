// 与 Rust 端 `rfont::FontInfo` 一一对应的前端类型。
//
// 桌面端通过 Tauri invoke、网页端通过 WASM JSON 拿到同一份序列化结构，
// 因此两端的类型与展示辅助函数保持一致（沿用桌面/Web 共用 UI 的 shim 做法）。

export type FontFormat = "ttf" | "otf" | "woff" | "woff2";

export interface TableInfo {
  tag: string;
  checksum: number;
  offset: number;
  length: number;
}

export interface NameEntry {
  platform_id: number;
  encoding_id: number;
  language_id: number;
  name_id: number;
  value: string;
}

/// 许可证类型（与 Rust 端 `rfont::FontLicense` 对应）。
export type FontLicenseKind =
  | "Unknown"
  | "SilOpenFontLicense"
  | "ApacheLicense"
  | "GnuGeneralPublicLicense"
  | "MitLicense"
  | "Proprietary";

/// 授权与嵌入限制（与 Rust 端 `rfont::FontRestrictions` 对应）。
/// 提示文案由 Rust 端统一生成，这里只做展示。
export interface FontRestrictions {
  fs_type: number;
  license: FontLicenseKind;
  no_embedding: boolean;
  preview_and_print_only: boolean;
  editable_embedding: boolean;
  no_subsetting: boolean;
  bitmap_only: boolean;
  reserved_font_name: boolean;
  embedding_label: string;
  subset_warnings: string[];
  rename_warnings: string[];
}

export interface FontInfo {
  format: FontFormat;
  face_index: number;
  face_count: number;
  names: NameEntry[];
  family: string | null;
  typographic_family: string | null;
  subfamily: string | null;
  full_name: string | null;
  postscript_name: string | null;
  version: string | null;
  designer: string | null;
  manufacturer: string | null;
  copyright: string | null;
  license: string | null;
  license_url: string | null;
  glyph_count: number;
  units_per_em: number;
  bbox: [number, number, number, number];
  ascender: number;
  descender: number;
  line_gap: number;
  number_of_h_metrics: number;
  supported_char_count: number;
  weight_class: number;
  width_class: number;
  fs_type: number;
  restrictions: FontRestrictions;
  tables: TableInfo[];
}

/// 字体集合中单个字面的摘要（与 Rust 端 `rfont::FaceSummary` 对应）。
export interface FaceSummary {
  index: number;
  format: FontFormat;
  family: string | null;
  subfamily: string | null;
  full_name: string | null;
  postscript_name: string | null;
  glyph_count: number;
}

/// 字体元信息编辑项（与 Rust 端 `rfont::FontMetaEdit` 对应）。
/// 未出现的字段保持原值。
export interface FontMetaEdit {
  family?: string;
  subfamily?: string;
  designer?: string;
  copyright?: string;
}

const FORMAT_LABELS: Record<FontFormat, string> = {
  ttf: "TrueType",
  otf: "OpenType",
  woff: "WOFF",
  woff2: "WOFF2",
};

/// 界面展示用的格式名。
export function formatLabel(format: FontFormat): string {
  return FORMAT_LABELS[format] ?? format.toUpperCase();
}

const CSS_FORMATS: Record<FontFormat, string> = {
  ttf: "truetype",
  otf: "opentype",
  woff: "woff",
  woff2: "woff2",
};

/// `@font-face` 的 format() 提示值。
export function cssFormat(format: FontFormat): string {
  return CSS_FORMATS[format] ?? "truetype";
}

/// 字面在选择器中的展示文案：序号 + 名称。
export function faceLabel(face: FaceSummary): string {
  const name =
    face.full_name ||
    [face.family, face.subfamily].filter(Boolean).join(" ") ||
    face.postscript_name ||
    "未命名字面";
  return `${face.index + 1}. ${name}`;
}

const NAME_ID_LABELS: Record<number, string> = {
  0: "版权",
  1: "家族名",
  2: "样式名",
  3: "唯一标识",
  4: "完整名称",
  5: "版本",
  6: "PostScript 名",
  7: "商标",
  8: "制造商",
  9: "设计者",
  10: "描述",
  11: "厂商网址",
  12: "设计者网址",
  13: "许可证",
  14: "许可证网址",
  16: "排版家族名",
  17: "排版样式名",
  18: "兼容完整名称",
  19: "示例文本",
  20: "PostScript CID",
  21: "WWS 家族名",
  22: "WWS 样式名",
  25: "可变字体 PostScript 前缀",
};

export function nameIdLabel(nameId: number): string {
  return NAME_ID_LABELS[nameId] ?? `nameID ${nameId}`;
}

export function platformLabel(platformId: number): string {
  switch (platformId) {
    case 0:
      return "Unicode";
    case 1:
      return "Mac";
    case 3:
      return "Windows";
    default:
      return `平台 ${platformId}`;
  }
}

const WINDOWS_LANGUAGES: Record<number, string> = {
  0x0404: "zh-TW",
  0x0409: "en-US",
  0x0411: "ja-JP",
  0x0412: "ko-KR",
  0x0804: "zh-CN",
};

export function languageLabel(platformId: number, languageId: number): string {
  if (platformId === 3) {
    return WINDOWS_LANGUAGES[languageId] ?? `0x${languageId.toString(16).padStart(4, "0")}`;
  }
  if (platformId === 1 && languageId === 0) {
    return "en";
  }
  return `0x${languageId.toString(16)}`;
}

const WEIGHT_LABELS: Record<number, string> = {
  100: "Thin",
  200: "ExtraLight",
  300: "Light",
  400: "Regular",
  500: "Medium",
  600: "SemiBold",
  700: "Bold",
  800: "ExtraBold",
  900: "Black",
};

function weightLabel(weight: number): string {
  const label = WEIGHT_LABELS[weight];
  return label ? `${weight} ${label}` : String(weight);
}

const WIDTH_LABELS: Record<number, string> = {
  1: "UltraCondensed",
  2: "ExtraCondensed",
  3: "Condensed",
  4: "SemiCondensed",
  5: "Normal",
  6: "SemiExpanded",
  7: "Expanded",
  8: "ExtraExpanded",
  9: "UltraExpanded",
};

function widthLabel(width: number): string {
  const label = WIDTH_LABELS[width];
  return label ? `${width} ${label}` : String(width);
}

const LICENSE_LABELS: Record<FontLicenseKind, string> = {
  Unknown: "未声明",
  SilOpenFontLicense: "SIL Open Font License",
  ApacheLicense: "Apache License",
  GnuGeneralPublicLicense: "GNU GPL",
  MitLicense: "MIT License",
  Proprietary: "专有授权",
};

/// 许可证类型的展示名称。
export function licenseLabel(kind: FontLicenseKind): string {
  return LICENSE_LABELS[kind] ?? kind;
}

export interface DetailRow {
  label: string;
  value: string;
}

/// “字体详情”折叠区展示的行，空值自动省略。
export function detailRows(info: FontInfo): DetailRow[] {
  const rows: DetailRow[] = [];
  const push = (label: string, value: string | null | undefined) => {
    if (value) {
      rows.push({ label, value });
    }
  };

  push("样式", info.subfamily);
  push("完整名称", info.full_name);
  push("PostScript", info.postscript_name);
  push("版本", info.version);
  push("设计者", info.designer);
  push("制造商", info.manufacturer);
  push("版权", info.copyright);
  push("许可证", info.license);
  push("许可证类型", licenseLabel(info.restrictions.license));
  push("许可证网址", info.license_url);
  push("字重", weightLabel(info.weight_class));
  push("字宽", widthLabel(info.width_class));
  push(
    "嵌入权限",
    `0x${info.fs_type.toString(16).padStart(4, "0").toUpperCase()} · ${info.restrictions.embedding_label}`,
  );
  push(
    "度量",
    `${info.units_per_em} UPM · 字符 ${info.supported_char_count} · 上下 ${info.ascender}/${info.descender} · 行距 ${info.line_gap}`,
  );
  push("边界框", info.bbox.join(", "));

  return rows;
}
