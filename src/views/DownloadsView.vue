<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { NInput, NInputNumber, NModal, NSelect, NSwitch, useDialog, useMessage, useNotification } from "naive-ui";
import AppIcon from "../components/AppIcon.vue";
import TaskCard from "../components/TaskCard.vue";
import { bsd } from "../lib/bsd";
import { cookiePairCount, mergeHeaderLines, normalizeCookie, parseCurlCommand } from "../lib/curl";
import { clamp, looksLikeUrl } from "../lib/format";
import { addTask, clearFinished, coreStatus, removeTask, settings, tasks } from "../lib/store";
import type { DownloadJob, TaskRecord } from "../lib/types";

type FilterKey = "all" | "active" | "success" | "failed";

/** 一个下载条目：url 必填；文件名与校验各自可选（留空即交给核心自动处理）。 */
interface DraftRow {
  id: number;
  url: string;
  output: string;
  alg: string;
  checksum: string;
}

const message = useMessage();
const notification = useNotification();
const dialog = useDialog();

let sequence = 1;
function makeRow(url = ""): DraftRow {
  return { id: sequence++, url, output: "", alg: "", checksum: "" };
}

const rows = ref<DraftRow[]>([makeRow()]);
const advancedOpen = ref(false);
const busy = ref(false);
const filter = ref<FilterKey>("all");

const rateOptions = [
  { label: "不限速", value: 0 },
  { label: "1 MB/s", value: 1 << 20 },
  { label: "2 MB/s", value: 2 << 20 },
  { label: "5 MB/s", value: 5 << 20 },
  { label: "10 MB/s", value: 10 << 20 },
  { label: "20 MB/s", value: 20 << 20 },
];

const algOptions = [
  { label: "不校验", value: "" },
  { label: "SHA-256", value: "sha256" },
  { label: "SHA-512", value: "sha512" },
  { label: "SHA-1", value: "sha1" },
  { label: "MD5", value: "md5" },
];

/** 示例链接放在脚本里：vue-tsc 在 v-for 子树内遇到静态属性里的 `//` 会丢失循环变量作用域。 */
const urlPlaceholder = "https://example.com/file.zip";

const coreReady = computed(() => coreStatus.value === null || coreStatus.value.loaded);

// —— curl / Cookie 导入 ——
type ImportMode = "" | "curl" | "cookie";
const importMode = ref<ImportMode>("");
const importText = ref("");

const importOpen = computed({
  get: () => importMode.value !== "",
  set: (value: boolean) => {
    if (!value) {
      importMode.value = "";
      importText.value = "";
    }
  },
});

const importTitle = computed(() => (importMode.value === "cookie" ? "导入 Cookie" : "导入 cURL 命令"));

const importHint = computed(() =>
  importMode.value === "cookie"
    ? "粘贴浏览器 Cookie 字符串或 document.cookie 结果，会自动去掉重复的键。"
    : "从浏览器开发者工具里复制「Copy as cURL」，会自动识别链接、Referer、User-Agent 与 Cookie。",
);

const importSummary = computed(() => {
  if (importMode.value === "curl") {
    const parsed = parseCurlCommand(importText.value);
    const items: string[] = [];
    if (parsed.url) items.push("下载链接");
    if (parsed.userAgent) items.push("User-Agent");
    if (parsed.referer) items.push("Referer");
    if (parsed.cookie) items.push(`Cookie ${cookiePairCount(parsed.cookie)} 项`);
    if (parsed.extraHeaders.length > 0) items.push(`其他请求头 ${parsed.extraHeaders.length} 条`);
    return items;
  }
  const cookie = normalizeCookie(importText.value);
  return cookie ? [`Cookie ${cookiePairCount(cookie)} 项`] : [];
});

function openImport(mode: "curl" | "cookie"): void {
  importMode.value = mode;
  importText.value = "";
}

function applyImport(): void {
  if (importMode.value === "curl") {
    const parsed = parseCurlCommand(importText.value);
    const additions: string[] = [];
    if (parsed.referer) additions.push(`Referer: ${parsed.referer}`);
    if (parsed.cookie) additions.push(`Cookie: ${parsed.cookie}`);
    additions.push(...parsed.extraHeaders);

    if (!parsed.url && additions.length === 0 && !parsed.userAgent) {
      message.warning("没有从命令里识别到链接或请求头，确认复制的是完整的 cURL 命令");
      return;
    }
    if (parsed.url) {
      const empty = rows.value.find((row) => row.url.trim().length === 0);
      if (empty) empty.url = parsed.url;
      else rows.value.push(makeRow(parsed.url));
    }
    if (parsed.userAgent) settings.userAgent = parsed.userAgent;
    if (additions.length > 0) settings.headersText = mergeHeaderLines(settings.headersText, additions);
    message.success(`已导入：${importSummary.value.join("、")}`);
    importOpen.value = false;
    return;
  }

  const cookie = normalizeCookie(importText.value);
  if (!cookie) {
    message.warning("没有识别到有效的 Cookie（需要形如 key=value; key2=value2）");
    return;
  }
  settings.headersText = mergeHeaderLines(settings.headersText, [`Cookie: ${cookie}`]);
  message.success(`已写入 Cookie ${cookiePairCount(cookie)} 项`);
  importOpen.value = false;
}

function buildJob(row: DraftRow): DownloadJob {
  const digest = row.checksum.trim();
  return {
    url: row.url.trim(),
    dir: settings.dir,
    output: row.output.trim(),
    checksum: digest ? `${row.alg}:${digest}` : "",
    threads: settings.threads,
    retry: settings.retry,
    timeout: settings.timeout,
    resume: settings.resume,
    limitRate: settings.limitRate,
    proxy: settings.proxy,
    userAgent: settings.userAgent,
    headers: settings.headersText
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0),
  };
}

function addRow(): void {
  rows.value.push(makeRow());
}

function removeRow(id: number): void {
  if (rows.value.length === 1) {
    rows.value = [makeRow()];
    return;
  }
  rows.value = rows.value.filter((row) => row.id !== id);
}

function splitLines(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

/** 在链接输入框里粘贴多行时，自动拆成多条任务行。 */
function onRowPaste(event: Event, row: DraftRow): void {
  const clipboard = (event as ClipboardEvent).clipboardData?.getData("text") ?? "";
  const lines = splitLines(clipboard);
  if (lines.length <= 1) return;
  event.preventDefault();
  row.url = lines[0];
  const index = rows.value.findIndex((item) => item.id === row.id);
  rows.value.splice(index + 1, 0, ...lines.slice(1).map((line) => makeRow(line)));
}

async function pasteFromClipboard(): Promise<void> {
  let text = "";
  try {
    text = await navigator.clipboard.readText();
  } catch {
    try {
      text = await bsd.readClipboard();
    } catch (error) {
      message.error(`无法读取剪贴板：${String(error)}`);
      return;
    }
  }
  const lines = splitLines(text);
  if (lines.length === 0) {
    message.warning("剪贴板里没有链接");
    return;
  }
  const filled = rows.value.filter(
    (row) => row.url.trim() || row.output.trim() || row.checksum.trim(),
  );
  const added = lines.map((line) => makeRow(line));
  rows.value = filled.length === 0 ? added : [...rows.value, ...added];
}

async function startDownload(): Promise<void> {
  const drafts = rows.value.filter((row) => row.url.trim().length > 0);
  if (drafts.length === 0) {
    message.warning("请先粘贴下载链接");
    return;
  }
  if (!settings.dir.trim()) {
    message.warning("请先选择保存目录");
    return;
  }

  for (const [index, row] of drafts.entries()) {
    const label = `第 ${index + 1} 个链接`;
    if (!looksLikeUrl(row.url.trim())) {
      message.error(`${label}必须以 http:// 或 https:// 开头`);
      return;
    }
    if (row.checksum.trim() && !row.alg) {
      message.error(`${label}填了哈希值，请同时选择校验算法`);
      return;
    }
  }

  busy.value = true;
  const failures: string[] = [];
  let started = 0;
  for (const row of drafts) {
    try {
      const result = await bsd.start(buildJob(row));
      addTask(result);
      started += 1;
    } catch (error) {
      failures.push(`${row.url.trim()}\n${String(error)}`);
    }
  }
  busy.value = false;

  if (started > 0) {
    rows.value = [makeRow()];
    filter.value = "all";
    message.success(started === 1 ? "任务已开始下载" : `已开始 ${started} 个下载任务`);
  }
  if (failures.length > 0) {
    notification.error({
      title: `${failures.length} 个任务未能开始`,
      meta: "请检查链接、保存目录、校验值或请求头",
      content: failures.join("\n\n"),
      duration: 12000,
    });
  }
}

async function pickDir(): Promise<void> {
  try {
    const picked = await bsd.pickFolder(settings.dir);
    if (picked) settings.dir = picked;
  } catch (error) {
    notification.error({ title: "无法打开目录选择器", content: String(error) });
  }
}

function isFinished(task: TaskRecord): boolean {
  return ["success", "failed", "cancelled", "interrupted"].includes(task.status);
}

async function cancelTask(task: TaskRecord): Promise<void> {
  try {
    await bsd.cancel(task.id);
    message.info(`正在取消「${task.displayName}」`);
  } catch (error) {
    const text = String(error);
    message.error(text.includes("已经结束") ? "该任务已经结束" : `取消失败：${text}`);
  }
}

async function retryTask(task: TaskRecord): Promise<void> {
  try {
    const result = await bsd.start(task.job);
    removeTask(task.id);
    addTask(result);
    message.success("已重新开始下载");
  } catch (error) {
    notification.error({ title: "无法重新开始下载", content: String(error) });
  }
}

async function openTask(task: TaskRecord): Promise<void> {
  try {
    await bsd.openFolder(task.filePath ?? task.job.dir);
  } catch (error) {
    notification.error({ title: "无法打开文件夹", content: String(error) });
  }
}

function removeOne(task: TaskRecord): void {
  if (!isFinished(task)) {
    message.warning("任务仍在进行中，请先取消下载");
    return;
  }
  removeTask(task.id);
}

const filtered = computed(() => {
  switch (filter.value) {
    case "active":
      return tasks.filter((task) => ["queued", "running", "retrying"].includes(task.status));
    case "success":
      return tasks.filter((task) => task.status === "success");
    case "failed":
      return tasks.filter((task) => ["failed", "cancelled", "interrupted"].includes(task.status));
    default:
      return tasks;
  }
});

const counts = computed(() => ({
  all: tasks.length,
  active: tasks.filter((task) => ["queued", "running", "retrying"].includes(task.status)).length,
  success: tasks.filter((task) => task.status === "success").length,
  failed: tasks.filter((task) => ["failed", "cancelled", "interrupted"].includes(task.status))
    .length,
}));

function askClearFinished(): void {
  const removable = counts.value.all - counts.value.active;
  if (removable <= 0) {
    message.info("没有可清除的记录");
    return;
  }
  dialog.warning({
    title: "清除已结束的记录",
    content: `将移除 ${removable} 条已完成 / 失败 / 已取消的任务记录，已下载的文件不会被删除。`,
    positiveText: "清除",
    negativeText: "取消",
    onPositiveClick: () => {
      const removed = clearFinished();
      message.success(`已清除 ${removed} 条记录`);
    },
  });
}

function onRateChange(value: unknown): void {
  settings.limitRate = Number(value ?? 0);
}

function setThreads(value: number | null): void {
  settings.threads = clamp(value ?? 8, 1, 64);
}

function setTimeoutValue(value: number | null): void {
  settings.timeout = clamp(value ?? 90, 5, 3600);
}

function setRetry(value: number | null): void {
  settings.retry = clamp(value ?? 3, 0, 10);
}

onMounted(async () => {
  if (!settings.dir.trim()) {
    try {
      settings.dir = await bsd.defaultDir();
    } catch {
      /* 核心未加载时留空，由用户手动选择 */
    }
  }
});
</script>

<template>
  <div class="view">
    <header class="page-head">
      <h1 class="page-title">
        下载任务
        <span v-if="counts.active > 0" class="live-dot" />
      </h1>
      <button v-if="counts.all > 0" class="btn" @click="askClearFinished">
        <AppIcon name="trash" :size="14" />清除已结束
      </button>
    </header>

    <section class="card composer">
      <div class="composer-head">
        <span class="field-label">下载链接</span>
        <div class="head-actions">
          <button class="link-btn" type="button" @click="addRow">
            <AppIcon name="plus" :size="13" />添加一行
          </button>
          <button class="link-btn" type="button" :disabled="busy" @click="openImport('curl')">
            <AppIcon name="copy" :size="13" />导入 cURL
          </button>
          <button class="link-btn" type="button" :disabled="busy" @click="openImport('cookie')">
            <AppIcon name="shield" :size="13" />导入 Cookie
          </button>
          <button class="link-btn" type="button" @click="pasteFromClipboard">
            <AppIcon name="paste" :size="13" />从剪贴板粘贴
          </button>
        </div>
      </div>

      <div class="rows">
        <div v-for="r in rows" :key="r.id" class="row-grid">
          <NInput
            v-model:value="r.url"
            class="f-url"
            :disabled="busy"
            :placeholder="urlPlaceholder"
            @paste="onRowPaste($event, r)"
          />
          <div class="row-meta">
            <NInput
              v-model:value="r.output"
              class="f-name"
              :disabled="busy"
              placeholder="文件名（可选）"
            />
            <NSelect
              v-model:value="r.alg"
              class="f-alg"
              :options="algOptions"
              :disabled="busy"
            />
            <NInput
              v-model:value="r.checksum"
              class="f-hash"
              :disabled="busy || !r.alg"
              placeholder="哈希值（可选）"
            />
          </div>
          <button class="icon-btn row-remove" title="移除这一行" type="button" @click="removeRow(r.id)">
            <AppIcon name="trash" :size="15" />
          </button>
        </div>
      </div>

      <div class="composer-foot">
        <span class="hint">每行一个任务；文件名与哈希留空则自动处理</span>
        <button class="btn-primary" :disabled="busy || !coreReady" @click="startDownload">
          <AppIcon :name="busy ? 'clock' : 'download'" :size="17" />
          {{ busy ? "正在提交…" : "开始下载" }}
        </button>
      </div>

      <div class="quick-grid">
        <div class="field">
          <span class="field-label">保存到</span>
          <div class="dir-row">
            <NInput v-model:value="settings.dir" placeholder="选择保存目录" />
            <button class="icon-btn dir-btn" title="选择目录" @click="pickDir">
              <AppIcon name="folder" :size="17" />
            </button>
          </div>
        </div>
        <div class="field">
          <span class="field-label">线程数</span>
          <NInputNumber
            :value="settings.threads"
            :min="1"
            :max="64"
            :step="1"
            style="width: 100%"
            @update:value="setThreads"
          />
        </div>
        <div class="field">
          <span class="field-label">限速</span>
          <NSelect :value="settings.limitRate" :options="rateOptions" @update:value="onRateChange" />
        </div>
        <div class="field field-toggle">
          <span class="field-label">&nbsp;</span>
          <button class="btn toggle-btn" @click="advancedOpen = !advancedOpen">
            <AppIcon name="settings" :size="15" />
            高级选项
            <AppIcon
              name="chevron"
              :size="14"
              class="chevron"
              :class="{ open: advancedOpen }"
            />
          </button>
        </div>
      </div>

      <div v-if="advancedOpen" class="advanced">
        <div class="field">
          <span class="field-label">代理</span>
          <NInput v-model:value="settings.proxy" placeholder="http://127.0.0.1:7890" />
        </div>
        <div class="field">
          <span class="field-label">User-Agent</span>
          <NInput v-model:value="settings.userAgent" placeholder="默认 UA" />
        </div>
        <div class="field">
          <span class="field-label">超时（秒）</span>
          <NInputNumber
            :value="settings.timeout"
            :min="5"
            :max="3600"
            :step="5"
            style="width: 100%"
            @update:value="setTimeoutValue"
          />
        </div>
        <div class="field">
          <span class="field-label">失败重试</span>
          <NInputNumber
            :value="settings.retry"
            :min="0"
            :max="10"
            :step="1"
            style="width: 100%"
            @update:value="setRetry"
          />
        </div>
        <div class="field switch-field">
          <span class="field-label">断点续传</span>
          <div class="switch-row">
            <NSwitch v-model:value="settings.resume" />
            <span class="hint">{{ settings.resume ? "已开启" : "已关闭" }}</span>
          </div>
        </div>
        <div class="field span-all">
          <span class="field-label">自定义请求头（每行一个 Header: Value）</span>
          <NInput
            v-model:value="settings.headersText"
            type="textarea"
            :autosize="{ minRows: 2, maxRows: 5 }"
            placeholder="Referer: https://example.com&#10;Cookie: key=value"
            class="headers-input"
          />
        </div>
      </div>
    </section>

    <div v-if="!coreReady" class="core-warn">
      <AppIcon name="alert" :size="15" />
      <span>核心未加载，无法开始下载</span>
    </div>

    <section v-if="counts.all > 0" class="list-head">
      <div class="filters">
        <button class="filter" :class="{ active: filter === 'all' }" @click="filter = 'all'">
          全部<span class="count">{{ counts.all }}</span>
        </button>
        <button class="filter" :class="{ active: filter === 'active' }" @click="filter = 'active'">
          进行中<span class="count">{{ counts.active }}</span>
        </button>
        <button class="filter" :class="{ active: filter === 'success' }" @click="filter = 'success'">
          已完成<span class="count">{{ counts.success }}</span>
        </button>
        <button class="filter" :class="{ active: filter === 'failed' }" @click="filter = 'failed'">
          失败<span class="count">{{ counts.failed }}</span>
        </button>
      </div>
    </section>

    <TransitionGroup v-if="filtered.length > 0" name="task" tag="div" class="task-list">
      <TaskCard
        v-for="task in filtered"
        :key="task.id"
        :task="task"
        @cancel="cancelTask(task)"
        @retry="retryTask(task)"
        @open="openTask(task)"
        @remove="removeOne(task)"
      />
    </TransitionGroup>

    <section v-else class="card">
      <div class="empty">
        <div class="empty-icon">
          <AppIcon :name="counts.all === 0 ? 'download' : 'search'" :size="24" />
        </div>
        <div class="empty-title">
          {{ counts.all === 0 ? "还没有下载任务" : "当前筛选下没有任务" }}
        </div>
        <div class="empty-sub">
          {{ counts.all === 0 ? "粘贴链接即可开始" : "换一个筛选条件" }}
        </div>
      </div>
    </section>

    <NModal
      v-model:show="importOpen"
      preset="card"
      :title="importTitle"
      :bordered="false"
      :style="{ width: '620px', maxWidth: '94vw' }"
    >
      <p class="import-hint">{{ importHint }}</p>
      <NInput
        v-model:value="importText"
        type="textarea"
        :autosize="{ minRows: 8, maxRows: 14 }"
        :placeholder="importMode === 'cookie' ? 'key=value; key2=value2' : '粘贴完整的 curl 命令'"
      />
      <div v-if="importSummary.length > 0" class="import-summary">
        <span class="field-label">识别到</span>
        <div class="import-tags">
          <span v-for="item in importSummary" :key="item" class="import-tag">{{ item }}</span>
        </div>
      </div>
      <template #footer>
        <div class="import-foot">
          <button class="btn" type="button" @click="importOpen = false">取消</button>
          <button class="btn-primary" type="button" @click="applyImport">填入</button>
        </div>
      </template>
    </NModal>
  </div>
</template>

<style scoped>
.composer {
  padding: 16px 18px 18px;
  background:
    radial-gradient(420px 180px at 100% 0%, var(--accent-soft), transparent 70%),
    linear-gradient(180deg, var(--card), var(--card-2));
}

.composer-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}
.composer-head .field-label {
  margin-bottom: 0;
}

.head-actions {
  display: flex;
  gap: 4px;
}

.link-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 7px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text-3);
  font-size: 11.5px;
  cursor: pointer;
  transition: color var(--ease), background var(--ease);
}
.link-btn:hover {
  color: var(--accent-2);
  background: var(--accent-soft);
}

.rows {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.row-grid {
  display: flex;
  align-items: center;
  gap: 8px;
}
.f-url {
  flex: 1 1 260px;
  min-width: 0;
}
.row-meta {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 8px;
}
.f-name {
  width: 148px;
}
.f-alg {
  width: 108px;
}
.f-hash {
  width: 178px;
}
.row-remove {
  flex: 0 0 32px;
}

.composer-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 12px;
}
.composer-foot .hint {
  min-width: 0;
}

.quick-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 100px 128px auto;
  gap: 10px;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid var(--line);
}

.field {
  min-width: 0;
}
.field .field-label {
  margin-bottom: 5px;
}
.field-toggle {
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
}

.dir-row {
  display: flex;
  gap: 7px;
}
.dir-btn {
  flex: 0 0 36px;
  width: 36px;
  height: 36px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
}

.toggle-btn {
  height: 36px;
  white-space: nowrap;
}
.chevron {
  transition: transform var(--ease);
}
.chevron.open {
  transform: rotate(180deg);
}

.advanced {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid var(--line);
}
.span-all {
  grid-column: 1 / -1;
}
.switch-field {
  align-self: end;
}
.switch-row {
  display: flex;
  align-items: center;
  gap: 9px;
  height: 34px;
}
.headers-input :deep(textarea) {
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.6;
}

.core-warn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 13px;
  border: 1px solid rgba(245, 158, 11, 0.3);
  border-radius: var(--radius-md);
  background: var(--warning-soft);
  color: var(--warning);
  font-size: 12.5px;
}

.list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 2px;
}

.filters {
  display: flex;
  gap: 3px;
  padding: 3px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--bg-soft);
}
.filter {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 11px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text-3);
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
  transition: background var(--ease), color var(--ease);
}
.filter:hover {
  color: var(--text-2);
}
.filter.active {
  background: var(--accent-soft);
  color: var(--accent-2);
  box-shadow: inset 0 0 0 1px var(--accent-line);
}
.count {
  color: inherit;
  font-family: var(--mono);
  font-size: 11px;
  opacity: 0.72;
}

.task-list {
  display: flex;
  flex-direction: column;
  gap: 9px;
}

.live-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--success);
  box-shadow: 0 0 0 3px var(--success-soft);
  animation: chip-pulse 1.6s ease-in-out infinite;
}

.import-hint {
  margin: 0 0 10px;
  color: var(--text-3);
  font-size: 12px;
  line-height: 1.6;
}

.import-summary {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 12px;
}

.import-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.import-tag {
  padding: 3px 9px;
  border-radius: 999px;
  background: var(--accent-soft);
  box-shadow: inset 0 0 0 1px var(--accent-line);
  color: var(--accent-2);
  font-size: 12px;
}

.import-foot {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

@media (max-width: 1040px) {
  .row-grid {
    flex-wrap: wrap;
  }
  .row-meta {
    flex: 1 1 100%;
  }
  .f-hash {
    flex: 1 1 auto;
    width: auto;
  }
}

@media (max-width: 980px) {
  .quick-grid {
    grid-template-columns: minmax(0, 1fr) 96px;
  }
  .advanced {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
