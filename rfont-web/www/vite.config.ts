import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// https://vitejs.dev/config/
// GitHub Pages 项目站点部署在 /rfont/ 子路径下，因此 base 必须带上仓库名。
export default defineConfig({
  plugins: [vue()],
  base: "/rfont/",
  server: {
    port: 5173,
  },
  build: {
    target: "esnext",
    minify: "esbuild",
    sourcemap: false,
    cssCodeSplit: true,
  },
});
