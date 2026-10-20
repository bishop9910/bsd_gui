<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { NInput, NSwitch, useMessage } from "naive-ui";
import AppIcon from "../components/AppIcon.vue";
import { formatClock } from "../lib/format";
import { clearLogs, levelName, logs } from "../lib/store";

const message = useMessage();

const minLevel = ref(-1);
const keyword = ref("");
const autoScroll = ref(true);
const box = ref<HTMLElement | null>(null);

const levelFilters = [
  { label: "全部", value: -1 },
  { label: "信息+", value: 2 },
  { label: "警告+", value: 3 },
  { label: "错误", value: 4 },
];

const filtered = computed(() => {
  const word = keyword.value.trim().toLowerCase();
  return logs.filter((entry) => {
    if (minLevel.value >= 0 && entry.level < minLevel.value) return false;
    if (word && !entry.message.toLowerCase().includes(word)) return false;
    return true;
  });
});

const counts = computed(() => {
  let errors = 0;
  let warnings = 0;
  for (const entry of logs) {
    if (entry.level >= 4) errors += 1;
    else if (entry.level === 3) warnings += 1;
  }
  return { errors, warnings };
});

async function scrollToBottom(): Promise<void> {
  await nextTick();
  const element = box.value;
  if (element) element.scrollTop = element.scrollHeight;
}

watch(
  () => logs.length,
  () => {
    if (autoScroll.value) void scrollToBottom();
  },
);

watch([minLevel, keyword], () => {
  if (autoScroll.value) void scrollToBottom();
});

function levelClass(level: number): string {
  if (level >= 4) return "error";
  if (level === 3) return "warn";
  if (level === 2) return "info";
  if (level === 1) return "debug";
  return "trace";
}

async function copyAll(): Promise<void> {
  if (filtered.value.length === 0) {
    message.warning("没有可复制的日志");
    return;
  }
  const text = filtered.value
    .map((entry) => `${formatClock(entry.ts)} [${levelName(entry.level)}] ${entry.message}`)
    .join("\n");
  try {
    await navigator.clipboard.writeText(text);
    message.success(`已复制 ${filtered.value.length} 条日志`);
  } catch {
    message.error("复制失败，请手动选择文本");
  }
}

function clearAll(): void {
  const removed = logs.length;
  clearLogs();
  if (removed > 0) message.success(`已清空 ${removed} 条日志`);
}

onMounted(() => {
  void scrollToBottom();
});
</script>

<template>
  <div class="view logs-view">
    <header class="page-head">
      <h1 class="page-title">日志</h1>
      <div class="stats">
        <span v-if="counts.warnings > 0" class="stat-pill alert">警告 {{ counts.warnings }}</span>
        <span v-if="counts.errors > 0" class="stat-pill danger">错误 {{ counts.errors }}</span>
      </div>
    </header>

    <section class="card console-card">
      <div class="toolbar">
        <div class="filters">
          <button
            v-for="item in levelFilters"
            :key="item.value"
            class="filter"
            :class="{ active: minLevel === item.value }"
            @click="minLevel = item.value"
          >
            {{ item.label }}
          </button>
        </div>

        <NInput
          v-model:value="keyword"
          class="search"
          size="small"
          clearable
          placeholder="搜索日志"
        >
          <template #prefix>
            <AppIcon name="search" :size="14" />
          </template>
        </NInput>

        <div class="tool-actions">
          <label class="auto-scroll">
            <NSwitch v-model:value="autoScroll" size="small" />
            <span>自动滚动</span>
          </label>
          <button class="icon-btn" title="复制当前日志" @click="copyAll">
            <AppIcon name="copy" :size="16" />
          </button>
          <button class="icon-btn danger" title="清空日志" @click="clearAll">
            <AppIcon name="trash" :size="16" />
          </button>
        </div>
      </div>

      <div ref="box" class="console">
        <div v-if="filtered.length === 0" class="console-empty">
          <div class="empty-icon"><AppIcon name="logs" :size="22" /></div>
          <div class="empty-title">{{ logs.length === 0 ? "还没有日志" : "没有匹配的日志" }}</div>
        </div>

        <div
          v-for="entry in filtered"
          :key="entry.id"
          class="line"
          :class="levelClass(entry.level)"
        >
          <span class="time">{{ formatClock(entry.ts) }}</span>
          <span class="level">{{ levelName(entry.level) }}</span>
          <span class="msg">{{ entry.message }}</span>
        </div>
      </div>

      <div class="console-foot">
        <span class="hint">共 {{ filtered.length }} / {{ logs.length }} 条</span>
      </div>
    </section>
  </div>
</template>

<style scoped>
.logs-view {
  height: 100%;
}

.stats {
  display: flex;
  gap: 8px;
}

.stat-pill {
  display: inline-flex;
  align-items: center;
  height: 24px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 999px;
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.stat-pill.alert {
  color: var(--warning);
  border-color: rgba(245, 158, 11, 0.3);
  background: var(--warning-soft);
}
.stat-pill.danger {
  color: var(--danger);
  border-color: rgba(244, 63, 94, 0.3);
  background: var(--danger-soft);
}

.console-card {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  padding: 0;
  overflow: hidden;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--line);
}

.filters {
  display: flex;
  gap: 3px;
  padding: 3px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--bg-soft);
}
.filter {
  padding: 4px 10px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-3);
  font-size: 12px;
  cursor: pointer;
  transition: background var(--ease), color var(--ease);
}
.filter.active {
  background: var(--accent-soft);
  color: var(--accent-2);
}

.search {
  flex: 1;
  max-width: 300px;
}

.tool-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}

.auto-scroll {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  color: var(--text-3);
  font-size: 11.5px;
  cursor: pointer;
}

.console {
  flex: 1;
  min-height: 300px;
  max-height: calc(100vh - 250px);
  padding: 8px 0;
  overflow-y: auto;
  background: var(--bg-soft);
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.7;
}

.line {
  display: flex;
  gap: 10px;
  padding: 2px 14px;
  border-left: 2px solid transparent;
}
.line:hover {
  background: var(--accent-soft);
}

.time {
  flex: 0 0 auto;
  color: var(--text-3);
  font-variant-numeric: tabular-nums;
  opacity: 0.8;
}

.level {
  flex: 0 0 50px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.02em;
}

.msg {
  flex: 1;
  min-width: 0;
  color: var(--text-2);
  white-space: pre-wrap;
  word-break: break-word;
}

.line.trace .level,
.line.debug .level {
  color: var(--text-3);
}
.line.trace .msg,
.line.debug .msg {
  color: var(--text-3);
}
.line.info .level {
  color: var(--accent-2);
}
.line.warn {
  background: var(--warning-soft);
  border-left-color: rgba(245, 158, 11, 0.6);
}
.line.warn .level,
.line.warn .msg {
  color: var(--warning);
}
.line.error {
  background: var(--danger-soft);
  border-left-color: rgba(244, 63, 94, 0.6);
}
.line.error .level,
.line.error .msg {
  color: var(--danger);
}

.console-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 100%;
  min-height: 240px;
  color: var(--text-3);
  font-family: var(--font);
  text-align: center;
}

.console-foot {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  border-top: 1px solid var(--line);
}
</style>
