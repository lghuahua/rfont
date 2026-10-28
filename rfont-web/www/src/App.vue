<template>
  <div class="main-container flex w-full">
    <!-- Left Panel -->
    <div class="left-panel flex-col">
      <!-- Header -->
      <div class="flex-col gap-6 mb-10">
        <div class="app-title">字体子集化</div>
        <div class="app-subtitle">精简字体，优化性能</div>
      </div>

      <!-- Form Sections -->
      <div class="flex-col gap-20 flex-1">
        <!-- Upload Section -->
        <div class="flex-col">
          <div class="upload-zone flex-col flex-center" @click="selectFontFile">
            <div class="file-types flex-middle">
              <div class="type-text">选择字体文件，支持 TTF、TTC、WOFF、WOFF2</div>
            </div>
            <!-- 显示当前选择的文件名 -->
            <div v-if="fileName" class="selected-file">
              <div class="selected-file-name">已选择：{{ fileName }}</div>
            </div>
          </div>
          <!-- 浏览器文件选择（替代 Tauri 的 open 对话框） -->
          <input ref="fileInput" type="file" accept=".ttf,.otf,.ttc,.woff,.woff2"
            style="display: none" @change="onFileChange" />
        </div>

        <!-- Text Input Section -->
        <div class="flex-col flex-1">
          <Textarea v-model:value="textContent"
            placeholder="在此输入需要子集化的文字内容，例如：abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
            :auto-size="{ minRows: 8, maxRows: 15 }" show-count />
        </div>

        <!-- Output Section -->
        <div class="flex-col gap-10">
          <div class="output-row flex gap-10">
            <div class="filename-field flex-col gap-6">
              <Input v-model:value="filename" class="input-box w-full" />
            </div>
            <Select v-model:value="format" size="large">
              <SelectOption value="WOFF2">WOFF2</SelectOption>
              <SelectOption value="WOFF">WOFF</SelectOption>
              <SelectOption value="TTF">TTF</SelectOption>
            </Select>
          </div>
        </div>
      </div>

      <!-- Action Section -->
      <div class="action-section flex-col gap-8">
        <Button type="primary" :loading="loading" class="save-button flex-center gap-8 w-full" @click="handleGenerate">
          {{ loading ? '处理中...' : buttonText }}
        </Button>
      </div>
    </div>

    <!-- Right Panel -->
    <div class="right-panel flex-col">
      <!-- Font Info Card - Only show when font is loaded -->
      <div v-if="fontName" class="font-info-card">
        <svg viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" width="60" height="60">
          <path
            d="M635 888.5c-41.5-4.3-63-13-78.6-25.9l-1.1-1.1c-13.5-14.6-20.5-46.9-20.5-97.6V221.6c0-7.6 5.9-14 14-14h87.3c78.2 0 83 10.8 107.7 32.9 26.9 21.1 38.8 31.9 52.8 102 1.6 6.5 7 11.3 14 11.3h5.4c8.1 0 14.5-6.5 14-14.6l-1.6-232.6H78.5c-7.6 0-13.5 5.9-14 13.5l-1.1 218.5c-0.5 8.1 5.9 14.6 14 14.6h5.4c7 0 12.4-4.8 14-11.3 14-69.6 25.9-80.4 52.8-101.4l0.5-0.5c24.3-21.6 29.6-32.4 107.3-32.4h87.3c7.6 0 14 5.9 14 14v525.6c0 60.4-6.5 96.6-20 108.4-1.1 0.5-1.6 1.6-2.2 2.2-11.9 16.1-31.8 26.4-73.2 30.7-7.6 0.5-13 6.5-13 14 0 7.6 5.9 14 14 14h369.6c7.6 0 14-5.9 14-14 0.1-7-5.3-13.5-12.9-14z m324.3-217.6l-0.7-99H638.4c-3.2 0-5.7 2.6-6 5.7l-0.5 93c-0.3 3.5 2.6 6.2 6 6.2h2.3c3 0 5.3-2.1 6-4.8 6-29.6 11-34.2 22.5-43.1l0.3-0.3c10.4-9.2 12.7-13.8 45.8-13.8h37.3c3.2 0 6 2.6 6 5.9v223.7c0 25.7-2.7 41-8.5 46.1-0.5 0.3-0.7 0.7-0.9 0.9-5 6.9-13.6 11.2-31.3 13-3.2 0.3-5.6 2.7-5.6 5.9s2.6 5.9 6 5.9h157.8c3.2 0 6-2.6 6-5.9 0-3-2.3-5.7-5.6-5.9-17.7-1.8-26.9-5.5-33.6-11l-0.5-0.5c-5.7-6.2-8.8-20-8.8-41.6V620.8c0-3.2 2.6-5.9 6-5.9h37.3c33.4 0 35.4 4.6 46 14 11.5 8.9 16.6 13.6 22.5 43.4 0.7 2.7 3 4.8 6 4.8h2.3c3.6 0 6.4-2.8 6.1-6.2z"
            fill="#2197D8">
          </path>
        </svg>
        <div class="flex-col gap-6 flex-1">
          <div class="font-name">{{ fontName }}</div>
          <div class="font-meta-line flex-middle gap-6">
            <span class="font-meta">{{ fontType }}</span>
            <template v-if="fontInfo?.subfamily">
              <span class="divider">·</span>
              <span class="font-meta">{{ fontInfo.subfamily }}</span>
            </template>
            <span class="divider">·</span>
            <span class="font-meta">{{ glyphCount }} 字形</span>
            <span class="divider">·</span>
            <span class="font-size">{{ originalSize }}</span>
          </div>

          <!-- 字面选择：仅字体集合包含多个字面时出现 -->
          <div v-if="faces.length > 1" class="face-row flex-middle gap-8">
            <span class="face-label">字面</span>
            <Select v-model:value="selectedFace" size="small" class="face-select" @change="onFaceChange">
              <SelectOption v-for="face in faces" :key="face.index" :value="face.index">
                {{ faceLabel(face) }}
              </SelectOption>
            </Select>
          </div>

          <!-- 授权提示：字体自己声明的嵌入 / 子集化限制 -->
          <div v-if="restrictionWarnings.length" class="restriction-box">
            <div class="restriction-title">授权提示</div>
            <div v-for="(warning, index) in restrictionWarnings" :key="index" class="restriction-item">
              {{ warning }}
            </div>
          </div>

          <!-- 字体详情：默认折叠，避免挤占预览区 -->
          <details v-if="fontInfo" class="font-details">
            <summary class="details-summary">字体详情</summary>
            <div class="details-body">
              <div v-for="row in detailRowsList" :key="row.label" class="detail-row">
                <span class="detail-label">{{ row.label }}</span>
                <span class="detail-value">{{ row.value }}</span>
              </div>

              <div v-if="fontInfo.names.length" class="details-section">
                <div class="details-title">名称记录（{{ fontInfo.names.length }}）</div>
                <div v-for="(name, index) in fontInfo.names" :key="index" class="name-row">
                  <span class="name-meta">
                    {{ nameIdLabel(name.name_id) }} · {{ platformLabel(name.platform_id) }}/{{
                      languageLabel(name.platform_id, name.language_id)
                    }}
                  </span>
                  <span class="name-value">{{ name.value }}</span>
                </div>
              </div>

              <div v-if="fontInfo.tables.length" class="details-section">
                <div class="details-title">字体表（{{ fontInfo.tables.length }}）</div>
                <div v-for="table in fontInfo.tables" :key="table.tag" class="table-row">
                  <span class="table-tag">{{ table.tag }}</span>
                  <span class="table-length">{{ table.length }} B</span>
                </div>
              </div>
            </div>
          </details>

          <!-- 名称编辑：默认折叠，改动只作用于导出的文件 -->
          <details v-if="fontInfo" class="font-details">
            <summary class="details-summary">编辑名称</summary>
            <div class="details-body">
              <div class="edit-grid">
                <div class="edit-field">
                  <span class="edit-label">家族名</span>
                  <Input v-model:value="editForm.family" size="small" :placeholder="fontInfo.family || '未命名'" />
                </div>
                <div class="edit-field">
                  <span class="edit-label">样式名</span>
                  <Input v-model:value="editForm.subfamily" size="small" :placeholder="fontInfo.subfamily || 'Regular'" />
                </div>
                <div class="edit-field">
                  <span class="edit-label">作者</span>
                  <Input v-model:value="editForm.designer" size="small" :placeholder="fontInfo.designer || '未署名'" />
                </div>
                <div class="edit-field">
                  <span class="edit-label">版权</span>
                  <Input v-model:value="editForm.copyright" size="small" :placeholder="fontInfo.copyright || '无'" />
                </div>
              </div>
              <div class="edit-actions">
                <Button size="small" type="primary" :loading="editing" @click="handleEdit">应用并下载</Button>
                <span class="edit-hint">仅改名称信息，不影响字形；家族名或样式名变化时完整名称与 PostScript 名同步更新</span>
              </div>
            </div>
          </details>
        </div>
      </div>

      <!-- Preview Section -->
      <div class="flex-col gap-16 flex-1">
        <div class="preview-header flex-middle">
          <div class="preview-title">字体预览</div>
          <div class="preview-actions flex-middle gap-8">
            <div class="font-size-control flex-middle gap-8">
              <span class="size-label">A</span>
              <input type="range" v-model.number="fontSize" min="12" max="72" class="size-slider" />
              <span class="size-label">A</span>
              <span class="size-value">{{ fontSize }}px</span>
            </div>
            <Button size="small" :disabled="!fontBytes || !textContent.trim()" @click="handleExportSvg">
              导出 SVG
            </Button>
          </div>
        </div>
        <div class="preview-canvas flex-col gap-20 flex-1">
          <div class="preview-single" :style="{ fontSize: fontSize + 'px' }">
            {{ textContent || '请输入需要保留的文字' }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { Button, Select, SelectOption, Input, Textarea, message } from "ant-design-vue";
import {
  createFontUrl,
  cssFormat,
  detailRows,
  downloadBytes,
  editFontMeta,
  errText,
  faceLabel,
  formatLabel,
  generateFont,
  getFaceBytes,
  getFontInfo,
  languageLabel,
  listFaces,
  mimeFor,
  nameIdLabel,
  platformLabel,
  revokeFontUrl,
  svgText,
  type FaceSummary,
  type FontFormat,
  type FontInfo,
  type FontMetaEdit,
} from "./backend";

// State
const textContent = ref("");
const filename = ref("subset-font");
const format = ref("WOFF2");
const fileName = ref("");        // 已选择的原始文件名
const fontName = ref("");        // 字体家族名（来自 name 表）
const fontType = ref("");        // 容器格式（来自解析结果）
const fontInfo = ref<FontInfo | null>(null);
const fontFormat = ref<FontFormat | null>(null);
const detailRowsList = computed(() => (fontInfo.value ? detailRows(fontInfo.value) : []));
const restrictionWarnings = computed(() => {
  const restrictions = fontInfo.value?.restrictions;
  if (!restrictions) {
    return [];
  }
  return [...restrictions.subset_warnings, ...restrictions.rename_warnings];
});
const glyphCount = ref(0);
const originalSize = ref("");
const loading = ref(false);
const fontDataUrl = ref("");     // 预览用 Blob URL
const fontSize = ref(16);
const fontBytes = ref<Uint8Array | null>(null); // 原始字体字节
const fileInput = ref<HTMLInputElement | null>(null);
const faces = ref<FaceSummary[]>([]);  // 字体集合中的字面
const selectedFace = ref(0);            // 当前字面索引
const editing = ref(false);              // 名称编辑导出中
const editForm = ref<{ family: string; subfamily: string; designer: string; copyright: string }>({
  family: "",
  subfamily: "",
  designer: "",
  copyright: "",
});

// 计算按钮文字
const buttonText = computed(() =>
  textContent.value.trim() !== "" ? "生成子集化字体" : "转换字体格式",
);

// 应用 @font-face 用于预览
watch([fontDataUrl, fontFormat], ([newUrl, currentFormat]) => {
  const styleId = "dynamic-font-style";
  let styleElement = document.getElementById(styleId) as HTMLStyleElement | null;

  if (!newUrl) {
    styleElement?.remove();
    return;
  }

  if (!styleElement) {
    styleElement = document.createElement("style");
    styleElement.id = styleId;
    document.head.appendChild(styleElement);
  }

  const formatType = currentFormat ? cssFormat(currentFormat) : "truetype";

  styleElement.textContent = `
    @font-face {
      font-family: 'CustomFont';
      src: url('${newUrl}') format('${formatType}');
      font-display: swap;
    }
  `;
});

// 监听 format 变化，自动更新文件名扩展
watch(format, (newFormat) => {
  if (filename.value) {
    const baseName = filename.value.replace(/\.(ttf|otf|woff|woff2)$/i, "");
    filename.value = `${baseName}.${newFormat.toLowerCase()}`;
  }
});

function selectFontFile() {
  fileInput.value?.click();
}

async function onFileChange(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = ""; // 允许再次选择同一文件
  if (file) {
    await loadFile(file);
  }
}

async function loadFile(file: File) {
  loading.value = true;
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    fontBytes.value = bytes;
    fileName.value = file.name;

    // 字体集合：列出所有字面，默认选中第一个
    faces.value = listFaces(bytes);
    selectedFace.value = 0;

    applyFace();
  } catch (err) {
    message.error(`加载字体失败：${errText(err)}`);
    resetFontState();
  } finally {
    loading.value = false;
  }
}

/// 按当前字面刷新信息与预览。
function applyFace() {
  const bytes = fontBytes.value;
  if (!bytes) {
    return;
  }

  const info = getFontInfo(bytes, selectedFace.value);
  fontInfo.value = info;
  fontFormat.value = info.format;
  fontName.value = info.family || info.typographic_family || info.full_name || fileName.value;
  fontType.value = formatLabel(info.format);
  glyphCount.value = info.glyph_count;

  // 预览：使用当前字面的独立字体字节
  const faceBytes = getFaceBytes(bytes, selectedFace.value);
  revokeFontUrl(fontDataUrl.value);
  fontDataUrl.value = createFontUrl(faceBytes, info.format);
  originalSize.value = `${(faceBytes.byteLength / 1024).toFixed(2)} KB`;

  // 切换字体后清空上一次的编辑内容，避免误用到新字体
  resetEditForm();
}

/// 切换字面：刷新信息与预览。
function onFaceChange() {
  loading.value = true;
  try {
    applyFace();
  } catch (err) {
    message.error(`切换字面失败：${errText(err)}`);
  } finally {
    loading.value = false;
  }
}

function resetFontState() {
  revokeFontUrl(fontDataUrl.value);
  fontBytes.value = null;
  fileName.value = "";
  fontName.value = "";
  fontType.value = "";
  fontInfo.value = null;
  fontFormat.value = null;
  glyphCount.value = 0;
  originalSize.value = "";
  fontDataUrl.value = "";
  faces.value = [];
  selectedFace.value = 0;
  resetEditForm();
}

function resetEditForm() {
  editForm.value = { family: "", subfamily: "", designer: "", copyright: "" };
}

/// 应用名称修改并导出新的字体文件。
function handleEdit() {
  if (!fontBytes.value) {
    message.error("请先选择字体文件");
    return;
  }

  const edit: FontMetaEdit = {};
  const fields: Array<keyof FontMetaEdit> = ["family", "subfamily", "designer", "copyright"];
  for (const field of fields) {
    const value = editForm.value[field].trim();
    if (value) {
      edit[field] = value;
    }
  }

  if (Object.keys(edit).length === 0) {
    message.warning("请至少填写一项要修改的信息");
    return;
  }

  editing.value = true;
  try {
    const renameWarnings = fontInfo.value?.restrictions.rename_warnings ?? [];
    if (renameWarnings.length > 0) {
      message.warning(renameWarnings.join(" "));
    }

    const fmt = format.value.toLowerCase();
    const bytes = editFontMeta(fontBytes.value, edit, fmt, selectedFace.value);
    const base = (fileName.value || "font").replace(/\.(ttf|otf|woff|woff2)$/i, "");
    const outName = `${base}_edited.${fmt}`;
    downloadBytes(bytes, outName, mimeFor(fmt));

    message.success(`已生成 ${outName}（${(bytes.byteLength / 1024).toFixed(2)} KB）`);
  } catch (err) {
    message.error(`修改失败：${errText(err)}`);
  } finally {
    editing.value = false;
  }
}

function handleGenerate() {
  if (!fontBytes.value) {
    message.error("请先选择字体文件");
    return;
  }

  loading.value = true;
  try {
    const text = textContent.value.trim();
    if (text) {
      const subsetWarnings = fontInfo.value?.restrictions.subset_warnings ?? [];
      if (subsetWarnings.length > 0) {
        message.warning(subsetWarnings.join(" "));
      }
    }

    const fmt = format.value.toLowerCase();
    const base = (filename.value || "subset-font").replace(
      /\.(ttf|otf|woff|woff2)$/i,
      "",
    );
    const outName = `${base}.${fmt}`;

    const bytes = generateFont(fontBytes.value, textContent.value.trim(), fmt, selectedFace.value);
    downloadBytes(bytes, outName, mimeFor(fmt));

    message.success(`已生成 ${outName}（${(bytes.byteLength / 1024).toFixed(2)} KB）`);
  } catch (err) {
    message.error(`生成失败：${errText(err)}`);
  } finally {
    loading.value = false;
  }
}

/// 按当前预览字号把输入文本导出为 SVG 轮廓。
function handleExportSvg() {
  const bytes = fontBytes.value;
  const text = textContent.value.trim();
  if (!bytes || !text) {
    return;
  }

  try {
    const svg = svgText(bytes, text, fontSize.value, "#000000", selectedFace.value);
    const base = (fileName.value || "font").replace(/\.(ttf|otf|woff|woff2)$/i, "");
    downloadBytes(new TextEncoder().encode(svg), `${base}.svg`, "image/svg+xml");
    message.success(`已导出 ${base}.svg`);
  } catch (err) {
    message.error(`导出失败：${errText(err)}`);
  }
}
</script>

<style>
/* Global styles - prevent all scrolling */
html,
body,
#app {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  overflow-x: hidden;
  overflow-y: hidden;
}
</style>

<style scoped>
/* Main Container */
.main-container {
  height: 100vh;
  background-color: #f9fafb;
}

/* Left Panel */
.left-panel {
  width: 320px;
  min-width: 320px;
  flex-shrink: 0;
  border-right: 1px solid #e5e7eb;
  background-color: #ffffff;
  padding: 0 15px 15px;
}

/* ===== 公共原子类 ===== */
/* 布局 */
.flex,
.flex-col,
.flex-middle,
.flex-center {
  display: flex;
}

.flex-1 {
  flex: 1;
}

.flex-col {
  flex-direction: column;
}

.flex-middle,
.flex-center {
  align-items: center;
}

.flex-center {
  justify-content: center;
}

.mb-10 {
  margin-bottom: 10px;
}

/* 间距 */

.gap-6 {
  gap: 6px;
}

.gap-8 {
  gap: 8px;
}

.gap-10 {
  gap: 10px;
}

.gap-12 {
  gap: 12px;
}

.gap-16 {
  gap: 16px;
}

.gap-20 {
  gap: 20px;
}

/* 尺寸 */


.h-full,
.full {
  height: 100%;
}

.w-full,
.full {
  width: 100%;
}

/* ===== Header ===== */

.app-title {
  height: 32px;
  font-size: 22px;
  font-weight: 900;
  line-height: 32px;
  color: #111827;
}

.app-subtitle {
  height: 19px;
  font-size: 13px;
  font-weight: 400;
  line-height: 19px;
  color: #6b7280;
}

/* Form Sections */

.upload-zone {
  border-radius: 10px;
  border: 2px dashed #d1d5db;
  background-color: #f9fafb;
  padding: 10px;
  cursor: pointer;
  transition: all 0.3s ease;
}

.upload-zone:hover {
  border-color: #3b82f6;
  background-color: #eff6ff;
}

.selected-file {
  margin-top: 10px;
  padding: 8px 12px;
  background-color: #ecfdf5;
  border-radius: 6px;
  border: 1px solid #10b981;
}

.selected-file-name {
  font-size: 12px;
  color: #059669;
  font-weight: 500;
}

.file-types {
  width: 220px;
  border-radius: 6px;
  border: 1px solid #e5e7eb;
  background-color: #ffffff;
  padding: 6px 12px;
}

.type-text {
  height: 16px;
  width: 220px;
  font-size: 11px;
  font-weight: 400;
  line-height: 16px;
  color: #9ca3af;
}


.output-row {
  align-items: flex-start;
  height: 42px;
}

.filename-field {
  height: 40px;
  width: 254px;
  flex-grow: 1;
  align-items: start;
}

.input-box {
  height: 40px;
  border-radius: 8px;
  border: 1px solid #d1d5db;
  padding: 11px 14px;
  font-size: 14px;
  line-height: 20px;
  color: #1f2937;
}

.format-dropdown {
  height: 42px;
  width: 100px;
  border-radius: 8px;
  border: 1px solid #d1d5db;
  padding: 11px 14px;
}

/* Action Section */
.action-section {
  height: 48px;
  align-items: flex-start;
  padding-top: 4px;
}

.save-button {
  height: 44px;
  border-radius: 8px;
  background-color: #3b82f6;
  font-weight: 700;
}

.button-icon {
  height: 15px;
  width: 15px;
}

/* Right Panel */
.right-panel {
  flex: 1;
  min-width: 0;
  background-color: #f9fafb;
  padding: 15px;
}

/* ===== Main Layout ===== */

/* Font Info Card */
.font-info-card {
  display: grid;
  grid-template-columns: 60px minmax(0, 1fr);
  column-gap: 16px;
  row-gap: 12px;
  align-items: center;
  flex-shrink: 0;
  border-radius: 12px;
  border: 1px solid #e5e7eb;
  background-color: #ffffff;
  padding: 15px;
  box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
}

/* 折叠面板与授权提示横跨整行，避免挤在图标右侧的窄列里 */
.font-info-card > .font-details,
.font-info-card > .restriction-box {
  grid-column: 1 / -1;
}

.info-icon-wrapper {
  height: 36px;
  width: 36px;
  flex-shrink: 0;
}

.font-name {
  font-size: 16px;
  font-weight: 700;
  line-height: 23px;
  color: #1f2937;
  margin-bottom: 4px;
}

.font-meta {
  font-size: 13px;
  font-weight: 400;
  line-height: 19px;
  color: #6b7280;
}

.font-size {
  font-size: 13px;
  font-weight: 600;
  line-height: 19px;
  color: #3b82f6;
}

.divider {
  color: #d1d5db;
}

.font-meta-line {
  flex-wrap: wrap;
  row-gap: 2px;
}

/* Face selector (only shown for multi-face collections) */
.face-row {
  margin-top: 2px;
}

.face-label {
  font-size: 12px;
  color: #9ca3af;
  flex-shrink: 0;
}

.face-select {
  min-width: 220px;
  max-width: 100%;
}

/* Font details (collapsed by default) */
.font-details {
  margin-top: 6px;
  font-size: 12px;
  color: #4b5563;
}

.details-summary {
  cursor: pointer;
  color: #3b82f6;
  font-weight: 500;
  list-style: none;
}

.details-summary::-webkit-details-marker {
  display: none;
}

.details-summary::before {
  content: "▸ ";
}

.font-details[open] .details-summary::before {
  content: "▾ ";
}

.details-body {
  margin-top: 8px;
  max-height: 220px;
  overflow-y: auto;
  padding-right: 4px;
}

.detail-row,
.name-row,
.table-row {
  display: flex;
  gap: 8px;
  line-height: 1.6;
}

.detail-label {
  width: 68px;
  flex-shrink: 0;
  color: #9ca3af;
}

.detail-value {
  color: #4b5563;
  word-break: break-word;
}

.details-section {
  margin-top: 10px;
}

.details-title {
  font-weight: 600;
  color: #6b7280;
  margin-bottom: 2px;
}

.name-meta {
  width: 132px;
  flex-shrink: 0;
  color: #9ca3af;
}

.name-value {
  color: #4b5563;
  word-break: break-word;
}

.table-row {
  justify-content: space-between;
  max-width: 240px;
}

.table-tag {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  color: #6b7280;
}

.table-length {
  color: #9ca3af;
}

/* 授权提示 */
.restriction-box {
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid #fde68a;
  background-color: #fffbeb;
}

.restriction-title {
  font-weight: 600;
  color: #b45309;
}

.restriction-item {
  font-size: 11px;
  line-height: 1.6;
  color: #92400e;
}

/* 名称编辑 */
.edit-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px 10px;
}

.edit-field {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.edit-label {
  font-size: 11px;
  color: #9ca3af;
}

.edit-actions {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
}

.edit-hint {
  font-size: 11px;
  line-height: 1.6;
  color: #9ca3af;
}

/* Preview Section */
.preview-header {
  justify-content: space-between;
  height: 28px;
  width: 100%;
}

/* 右侧一组：字号滑杆 + 导出按钮 */
.preview-actions {
  gap: 12px;
}

.font-size-control {
  align-items: center;
  gap: 8px;
}

.size-label {
  font-size: 18px;
  font-weight: 700;
  color: #6b7280;
  line-height: 1;
}

.size-slider {
  width: 100px;
  height: 6px;
  border-radius: 3px;
  background: #e5e7eb;
  outline: none;
  -webkit-appearance: none;
  appearance: none;
}

.size-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #3b82f6;
  cursor: pointer;
  transition: background 0.2s;
}

.size-slider::-webkit-slider-thumb:hover {
  background: #2563eb;
}

.size-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #3b82f6;
  cursor: pointer;
  border: none;
  transition: background 0.2s;
}

.size-slider::-moz-range-thumb:hover {
  background: #2563eb;
}

.size-value {
  font-size: 12px;
  font-weight: 600;
  color: #6b7280;
  min-width: 36px;
  text-align: center;
}

.preview-title {
  height: 22px;
  width: 60px;
  font-size: 15px;
  font-weight: 700;
  line-height: 22px;
  color: #374151;
}

.preview-canvas {
  border-radius: 12px;
  border: 1px solid #e5e7eb;
  background-color: #ffffff;
  padding: 15px;
  box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  min-height: 0;
}

.preview-single {
  overflow-y: auto;
  overflow-x: hidden;
  font-weight: 400;
  line-height: 1.5;
  color: #1f2937;
  white-space: pre;
  text-wrap: auto;
  min-height: 100px;
  font-family: 'CustomFont';
}

:deep(.ant-input),
:deep(.ant-select-selector) {
  border-color: #d1d5db !important;
  box-shadow: none !important;
}

:deep(.ant-input:focus),
:deep(.ant-select-focused .ant-select-selector) {
  border-color: #3b82f6 !important;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.1) !important;
}
</style>
