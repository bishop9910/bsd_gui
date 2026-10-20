<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { NInput, NInputNumber, NSelect, NSwitch, useDialog, useMessage } from "naive-ui";
import AppIcon from "../components/AppIcon.vue";
import { bsd } from "../lib/bsd";
import { clamp } from "../lib/format";
import { resetSettings, settings } from "../lib/store";

const message = useMessage();
const dialog = useDialog();
const savedFlash = ref(false);
let savedTimer: number | undefined;

const rateOptions = [
  { label: "不限速", value: 0 },
  { label: "1 MB/s", value: 1 << 20 },
  { label: "2 MB/s", value: 2 << 20 },
  { label: "5 MB/s", value: 5 << 20 },
  { label: "10 MB/s", value: 10 << 20 },
  { label: "20 MB/s", value: 20 << 20 },
];

const logOptions = [
  { label: "详细（TRACE）", value: 0 },
  { label: "调试（DEBUG）", value: 1 },
  { label: "信息（INFO）", value: 2 },
  { label: "警告（WARN）", value: 3 },
  { label: "仅错误（ERROR）", value: 4 },
  { label: "关闭（OFF）", value: 5 },
];

watch(
  settings,
  () => {
    savedFlash.value = true;
    window.clearTimeout(savedTimer);
    savedTimer = window.setTimeout(() => {
      savedFlash.value = false;
    }, 1400);
  },
  { deep: true },
);

watch(
  () => settings.logLevel,
  (level) => {
    bsd.setLogLevel(level).catch(() => {
      /* 核心未加载时忽略 */
    });
  },
);

function onRateChange(value: string | number | null): void {
  settings.limitRate = Number(value ?? 0);
}

function onLogChange(value: string | number | null): void {
  settings.logLevel = Number(value ?? 2);
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

async function pickDir(): Promise<void> {
  try {
    const picked = await bsd.pickFolder(settings.dir);
    if (picked) settings.dir = picked;
  } catch (error) {
    message.error(String(error));
  }
}

function confirmReset(): void {
  dialog.warning({
    title: "恢复默认设置",
    content: "将把所有下载、网络与界面选项恢复为初始值，不会影响已有的下载记录与文件。",
    positiveText: "恢复默认",
    negativeText: "取消",
    onPositiveClick: async () => {
      resetSettings();
      try {
        settings.dir = await bsd.defaultDir();
      } catch {
        /* 忽略 */
      }
      message.success("已恢复默认设置");
    },
  });
}

onMounted(async () => {
  if (!settings.dir.trim()) {
    try {
      settings.dir = await bsd.defaultDir();
    } catch {
      /* 忽略 */
    }
    // 首次进入时不要把默认目录当成一次「修改」
    savedFlash.value = false;
  }
});
</script>

<template>
  <div class="view">
    <header class="page-head">
      <h1 class="page-title">设置</h1>
      <div class="row">
        <span class="save-state" :class="{ flash: savedFlash }">
          <AppIcon :name="savedFlash ? 'check' : 'shield'" :size="14" />
          {{ savedFlash ? "已保存" : "自动保存" }}
        </span>
        <button class="btn" @click="confirmReset">
          <AppIcon name="refresh" :size="14" />恢复默认
        </button>
      </div>
    </header>

    <section class="card card-pad section">
      <div class="section-head">
        <span class="section-icon"><AppIcon name="download" :size="16" /></span>
        <h2 class="section-title">下载</h2>
      </div>

      <div class="rows">
        <div class="srow">
          <span class="slabel">保存目录</span>
          <div class="scontrol dir-row">
            <NInput v-model:value="settings.dir" placeholder="选择保存目录" />
            <button class="icon-btn dir-btn" title="选择目录" @click="pickDir">
              <AppIcon name="folder" :size="17" />
            </button>
          </div>
        </div>

        <div class="srow">
          <span class="slabel">线程数 <em>1–64</em></span>
          <div class="scontrol narrow">
            <NInputNumber
              :value="settings.threads"
              :min="1"
              :max="64"
              :step="1"
              style="width: 100%"
              @update:value="setThreads"
            />
          </div>
        </div>

        <div class="srow">
          <span class="slabel">限速</span>
          <div class="scontrol narrow">
            <NSelect :value="settings.limitRate" :options="rateOptions" @update:value="onRateChange" />
          </div>
        </div>

        <div class="srow">
          <span class="slabel">超时 <em>秒</em></span>
          <div class="scontrol narrow">
            <NInputNumber
              :value="settings.timeout"
              :min="5"
              :max="3600"
              :step="5"
              style="width: 100%"
              @update:value="setTimeoutValue"
            />
          </div>
        </div>

        <div class="srow">
          <span class="slabel">失败重试 <em>0 为不重试</em></span>
          <div class="scontrol narrow">
            <NInputNumber
              :value="settings.retry"
              :min="0"
              :max="10"
              :step="1"
              style="width: 100%"
              @update:value="setRetry"
            />
          </div>
        </div>

        <div class="srow">
          <span class="slabel">断点续传</span>
          <div class="scontrol">
            <NSwitch v-model:value="settings.resume" />
          </div>
        </div>
      </div>
    </section>

    <section class="card card-pad section">
      <div class="section-head">
        <span class="section-icon"><AppIcon name="globe" :size="16" /></span>
        <h2 class="section-title">网络</h2>
      </div>

      <div class="rows">
        <div class="srow">
          <span class="slabel">代理</span>
          <div class="scontrol">
            <NInput v-model:value="settings.proxy" placeholder="http://127.0.0.1:7890，留空直连" />
          </div>
        </div>

        <div class="srow">
          <span class="slabel">User-Agent</span>
          <div class="scontrol">
            <NInput v-model:value="settings.userAgent" placeholder="默认 UA" />
          </div>
        </div>

        <div class="srow align-top">
          <span class="slabel">默认请求头 <em>每行一个</em></span>
          <div class="scontrol">
            <NInput
              v-model:value="settings.headersText"
              type="textarea"
              :autosize="{ minRows: 3, maxRows: 8 }"
              placeholder="Referer: https://example.com&#10;Cookie: key=value"
              class="mono-input"
            />
          </div>
        </div>
      </div>
    </section>

    <section class="card card-pad section">
      <div class="section-head">
        <span class="section-icon"><AppIcon name="zap" :size="16" /></span>
        <h2 class="section-title">体验</h2>
      </div>

      <div class="rows">
        <div class="srow">
          <span class="slabel">完成后打开文件夹</span>
          <div class="scontrol">
            <NSwitch v-model:value="settings.autoOpenOnComplete" />
          </div>
        </div>

        <div class="srow">
          <span class="slabel">完成通知</span>
          <div class="scontrol">
            <NSwitch v-model:value="settings.notifyOnComplete" />
          </div>
        </div>

        <div class="srow">
          <span class="slabel">核心日志级别</span>
          <div class="scontrol narrow">
            <NSelect :value="settings.logLevel" :options="logOptions" @update:value="onLogChange" />
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.section {
  padding: 18px 20px 6px;
}

.section-head {
  display: flex;
  align-items: center;
  gap: 9px;
  margin-bottom: 4px;
}

.section-icon {
  display: grid;
  flex: 0 0 28px;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: var(--accent-soft);
  color: var(--accent-2);
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.rows {
  display: flex;
  flex-direction: column;
  margin-top: 6px;
}

.srow {
  display: grid;
  grid-template-columns: 190px minmax(0, 1fr);
  gap: 16px;
  align-items: center;
  padding: 11px 0;
  border-top: 1px solid var(--line);
}
.srow:first-child {
  border-top: none;
}
.srow.align-top {
  align-items: flex-start;
}
.srow.align-top .slabel {
  padding-top: 8px;
}

.slabel {
  display: flex;
  align-items: baseline;
  gap: 7px;
  color: var(--text-2);
  font-size: 13px;
  font-weight: 550;
}
.slabel em {
  color: var(--text-3);
  font-size: 11.5px;
  font-style: normal;
  font-weight: 400;
}

.scontrol {
  min-width: 0;
}
.scontrol.narrow {
  max-width: 200px;
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

.mono-input :deep(textarea) {
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.6;
}

.save-state {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--text-3);
  font-size: 12px;
  transition: color 0.2s ease;
}
.save-state.flash {
  color: var(--success);
}

@media (max-width: 860px) {
  .srow {
    grid-template-columns: 1fr;
    gap: 7px;
  }
}
</style>
