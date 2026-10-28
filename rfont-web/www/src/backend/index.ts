// 前端“后端适配层”。
//
// 桌面版通过 Tauri 的 invoke/dialog 与本地 Rust 通信；网页版没有 Tauri 运行时，
// 这里改为：浏览器 File API 读字节 -> WASM 计算 -> Blob 下载 / 预览。
// 对上层 App.vue 暴露与桌面版语义一致的高层函数。
import { convert, edit_metadata, face_bytes, font_info, list_faces, subset, svg_text } from "./wasm";
import type { FaceSummary, FontInfo, FontMetaEdit } from "./fontInfo";

export type {
  FaceSummary,
  FontFormat,
  FontInfo,
  FontLicenseKind,
  FontMetaEdit,
  FontRestrictions,
  NameEntry,
  TableInfo,
} from "./fontInfo";
export {
  cssFormat,
  detailRows,
  faceLabel,
  formatLabel,
  languageLabel,
  licenseLabel,
  nameIdLabel,
  platformLabel,
} from "./fontInfo";

/// 根据输出格式返回压缩级别（与桌面端一致：WOFF=9，WOFF2=11，TTF=0）。
function levelFor(format: string): number {
  switch (format.toLowerCase()) {
    case "woff":
      return 9;
    case "woff2":
      return 11;
    default:
      return 0;
  }
}

/// 根据扩展名或格式名推断 MIME 类型。
export function mimeFor(formatOrName: string): string {
  const ext = formatOrName.includes(".")
    ? (formatOrName.split(".").pop() ?? "")
    : formatOrName;
  switch (ext.toLowerCase()) {
    case "woff":
      return "font/woff";
    case "woff2":
      return "font/woff2";
    case "otf":
      return "font/otf";
    default:
      return "font/ttf";
  }
}

/// 读取字体信息（内部调用 WASM，返回已解析的对象）。
/// 字体集合内通过 `face` 选择字面。
export function getFontInfo(data: Uint8Array, face = 0): FontInfo {
  return JSON.parse(font_info(data, face)) as FontInfo;
}

/// 列出字体集合中的字面；非集合文件返回空数组。
export function listFaces(data: Uint8Array): FaceSummary[] {
  return JSON.parse(list_faces(data)) as FaceSummary[];
}

/// 提取指定字面的独立字体字节，用于预览。
export function getFaceBytes(data: Uint8Array, face = 0): Uint8Array {
  return face_bytes(data, face);
}

/// 生成字体：text 为空时做纯格式转换，否则按文本子集化。
/// 与桌面端 subset_font 命令的分支逻辑保持一致。
export function generateFont(
  data: Uint8Array,
  text: string,
  format: string,
  face = 0,
): Uint8Array {
  const fmt = format.toLowerCase();
  const level = levelFor(fmt);
  if (!text) {
    return convert(data, fmt, level, face);
  }
  return subset(data, text, fmt, level, face);
}

/// 修改字体元信息，返回新的字体字节。
/// 只改写 name 表并刷新 head.modified，字形与度量保持不变。
export function editFontMeta(
  data: Uint8Array,
  edit: FontMetaEdit,
  format: string,
  face = 0,
): Uint8Array {
  const fmt = format.toLowerCase();
  return edit_metadata(data, JSON.stringify(edit), fmt, levelFor(fmt), face);
}

/// 将单行文本导出为 SVG 文档（TrueType 轮廓，按 hmtx 步进排版）。
export function svgText(
  data: Uint8Array,
  text: string,
  fontSize: number,
  fill = "#000000",
  face = 0,
): string {
  return svg_text(data, text, fontSize, fill, face);
}

/// 用字体字节创建可用于 @font-face / 预览的 Blob URL。
export function createFontUrl(data: Uint8Array, name: string): string {
  const blob = new Blob([data as unknown as BlobPart], { type: mimeFor(name) });
  return URL.createObjectURL(blob);
}

/// 释放预览用的 Blob URL。
export function revokeFontUrl(url: string): void {
  if (url.startsWith("blob:")) {
    URL.revokeObjectURL(url);
  }
}

/// 触发浏览器下载。
export function downloadBytes(
  data: Uint8Array,
  filename: string,
  mime: string,
): void {
  const blob = new Blob([data as unknown as BlobPart], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/// 统一提取错误文本（WASM 抛出的是字符串或 Error）。
export function errText(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return String(err);
}
