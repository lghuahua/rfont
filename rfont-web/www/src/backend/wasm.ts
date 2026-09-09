// WASM 模块加载与类型归一化封装。
//
// wasm-pack 以 `--target web` 生成 `pkg/rfont_web.js`：默认导出 `init()`，
// 命名导出各绑定函数。这里统一初始化并把函数签名固定为我们期望的类型，
// 使上层业务代码不受 wasm-bindgen 生成类型细节（如 Uint8Array 泛型）的影响。
import init, * as wasm from "../../pkg/rfont_web.js";

export const font_info = wasm.font_info as unknown as (data: Uint8Array) => string;

export const convert = wasm.convert as unknown as (
  data: Uint8Array,
  format: string,
  compression_level: number,
) => Uint8Array;

export const subset = wasm.subset as unknown as (
  data: Uint8Array,
  text: string,
  format: string,
  compression_level: number,
) => Uint8Array;

let initPromise: Promise<void> | null = null;

/// 初始化 WASM 模块（幂等，仅首次真正加载）。
export function initWasm(): Promise<void> {
  if (!initPromise) {
    initPromise = init();
  }
  return initPromise;
}
