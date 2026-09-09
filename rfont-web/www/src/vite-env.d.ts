/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

// wasm-pack 生成的 JS 胶水模块（--target web）。
// pkg 目录在构建时才生成，这里提供环境声明以便编辑器/类型检查在生成前也不报错。
declare module "*rfont_web.js" {
  const init: (path?: string | URL | Request) => Promise<void>;
  export default init;
  export function font_info(data: Uint8Array): string;
  export function convert(
    data: Uint8Array,
    format: string,
    compression_level: number,
  ): Uint8Array;
  export function subset(
    data: Uint8Array,
    text: string,
    format: string,
    compression_level: number,
  ): Uint8Array;
}
