// 前端“后端适配层”。
//
// 桌面版通过 Tauri 的 invoke/dialog 与本地 Rust 通信；网页版没有 Tauri 运行时，
// 这里改为：浏览器 File API 读字节 -> WASM 计算 -> Blob 下载 / 预览。
// 对上层 App.vue 暴露与桌面版语义一致的高层函数。
import { convert, font_info, subset } from "./wasm";

export interface TableInfo {
  tag: string;
  checksum: number;
  offset: number;
  length: number;
}

export interface FontInfo {
  family_name: string | null;
  style_name: string | null;
  version: string | null;
  glyph_count: number;
  units_per_em: number;
  supported_char_count: number;
  ascender: number;
  descender: number;
  line_gap: number;
  x_min: number;
  y_min: number;
  x_max: number;
  y_max: number;
  tables: TableInfo[];
}

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
export function getFontInfo(data: Uint8Array): FontInfo {
  return JSON.parse(font_info(data)) as FontInfo;
}

/// 生成字体：text 为空时做纯格式转换，否则按文本子集化。
/// 与桌面端 subset_font 命令的分支逻辑保持一致。
export function generateFont(
  data: Uint8Array,
  text: string,
  format: string,
): Uint8Array {
  const fmt = format.toLowerCase();
  const level = levelFor(fmt);
  if (!text) {
    return convert(data, fmt, level);
  }
  return subset(data, text, fmt, level);
}

/// 用字体字节创建可用于 @font-face / 预览的 Blob URL。
export function createFontUrl(data: Uint8Array, name: string): string {
  const blob = new Blob([data as unknown as BlobPart], { type: mimeFor(name) });
  return URL.createObjectURL(blob);
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
