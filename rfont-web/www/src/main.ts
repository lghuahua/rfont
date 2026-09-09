import { createApp } from "vue";
import App from "./App.vue";
import { initWasm } from "./backend/wasm";

async function bootstrap() {
  try {
    await initWasm();
  } catch (e) {
    console.error("Failed to initialize WASM:", e);
    const el = document.getElementById("app");
    if (el) {
      el.innerHTML =
        '<div style="padding:24px;font-family:system-ui,sans-serif;color:#b91c1c">' +
        "WASM 初始化失败，请打开浏览器控制台查看详情。</div>";
    }
    return;
  }
  createApp(App).mount("#app");
}

bootstrap();
