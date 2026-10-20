<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { NInput, NInputNumber, NSelect, NSwitch, useDialog, useMessage, useNotification } from "naive-ui";
import AppIcon from "../components/AppIcon.vue";
import TaskCard from "../components/TaskCard.vue";
import { bsd } from "../lib/bsd";
import { clamp, looksLikeUrl } from "../lib/format";
import { addTask, clearFinished, coreStatus, removeTask, settings, tasks } from "../lib/store";
import type { DownloadJob, TaskRecord } from "../lib/types";

type FilterKey = "all" | "active" | "success" | "failed";

const message = useMessage();
const notification = useNotification();
const dialog = useDialog();

const urlText = ref("");
const output = ref("");
const checksumAlg = ref("sha256");
const checksumValue = ref("");
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
  { label: "SHA-256", value: "sha256" },
  { label: "SHA-512", value: "sha512" },
  { label: "SHA-1", value: "sha1" },
  { label: "MD5", value: "md5" },
];

const coreReady = computed(() => coreStatus.value === null || coreStatus.value.loaded);

function parseUrls(): string[] {
  return urlText.value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function buildJob(url: string, single: boolean): DownloadJob {
  const digest = checksumValue.value.trim();
  return {
    url,
    dir: settings.dir,
    output: single ? output.value.trim() : "",
    checksum: digest ? `${checksumAlg.value}:${digest}` : "",
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
  if (!text.trim()) {
    message.warning("剪贴板里没有文本");
    return;
  }
  urlText.value = urlText.value.trim() ? `${urlText.value.trim()}\n${text.trim()}` : text.trim();
}

async function startDownload(): Promise<void> {
  const urls = parseUrls();
  if (urls.length === 0) {
    message.warning("请先粘贴下载链接");
    return;
  }
  const invalid = urls.find((url) => !looksLikeUrl(url));
  if (invalid) {
    message.error(`链接必须以 http:// 或 https:// 开头：${invalid}`);
    return;
  }
  if (urls.length > 1 && output.value.trim()) {
    message.warning("批量下载时不能指定统一文件名，已忽略文件名");
  }
  if (!settings.dir.trim()) {
    message.warning("请先选择保存目录");
    return;
  }

  busy.value = true;
  const failures: string[] = [];
  let started = 0;
  for (const url of urls) {
    try {
      const result = await bsd.start(buildJob(url, urls.length === 1));
      addTask(result);
      started += 1;
    } catch (error) {
      failures.push(`${url}\n${String(error)}`);
    }
  }
  busy.value = false;

  if (started > 0) {
    urlText.value = "";
    output.value = "";
    checksumValue.value = "";
    filter.value = "all";
    message.success(urls.length === 1 ? "任务已开始下载" : `已开始 ${started} 个下载任务`);
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

function onRateChange(value: string | number | null): void {
  settings.limitRate = Number(value ?? 0);
}

function onAlgChange(value: string | number | null): void {
  checksumAlg.value = String(value ?? "sha256");
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
        <button class="link-btn" type="button" @click="pasteFromClipboard">
          <AppIcon name="paste" :size="13" />从剪贴板粘贴
        </button>
      </div>

      <div class="url-row">
        <NInput
          v-model:value="urlText"
          type="textarea"
          class="url-input"
          :autosize="{ minRows: 2, maxRows: 4 }"
          :disabled="busy"
          placeholder="每行一个链接，支持批量"
        />
        <div class="url-actions">
          <button class="btn-primary" :disabled="busy || !coreReady" @click="startDownload">
            <AppIcon :name="busy ? 'clock' : 'download'" :size="17" />
            {{ busy ? "正在提交…" : "开始下载" }}
          </button>
        </div>
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
          <span class="field-label">文件名（可选）</span>
          <NInput v-model:value="output" placeholder="留空自动识别" />
        </div>
        <div class="field">
          <span class="field-label">校验算法</span>
          <NSelect :value="checksumAlg" :options="algOptions" @update:value="onAlgChange" />
        </div>
        <div class="field">
          <span class="field-label">校验值（可选）</span>
          <NInput v-model:value="checksumValue" placeholder="十六进制哈希" />
        </div>
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
  margin-bottom: 6px;
}
.composer-head .field-label {
  margin-bottom: 0;
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

.url-row {
  display: flex;
  gap: 10px;
  align-items: stretch;
}

.url-input {
  flex: 1;
}

.url-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: stretch;
}
.url-actions .btn-primary {
  height: auto;
  min-height: 46px;
  min-width: 132px;
  padding: 0 22px;
}

.quick-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 100px 128px auto;
  gap: 10px;
  margin-top: 12px;
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

@media (max-width: 980px) {
  .quick-grid {
    grid-template-columns: minmax(0, 1fr) 96px;
  }
  .advanced {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
