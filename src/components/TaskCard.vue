<script setup lang="ts">
import { computed } from "vue";
import type { TaskRecord } from "../lib/types";
import { formatBytes, formatDuration, formatEta, formatSpeed } from "../lib/format";
import AppIcon from "./AppIcon.vue";
import StatusChip from "./StatusChip.vue";

const props = defineProps<{ task: TaskRecord }>();

defineEmits<{
  (event: "cancel"): void;
  (event: "retry"): void;
  (event: "open"): void;
  (event: "remove"): void;
}>();

const isActive = computed(
  () =>
    props.task.status === "running" ||
    props.task.status === "queued" ||
    props.task.status === "retrying",
);

const canCancel = computed(() => isActive.value);
const canRetry = computed(
  () =>
    props.task.status === "failed" ||
    props.task.status === "cancelled" ||
    props.task.status === "interrupted",
);

const pct = computed(() => {
  if (props.task.total <= 0) return 0;
  return Math.min(100, Math.round((props.task.downloaded / props.task.total) * 1000) / 10);
});

const barClass = computed(() => {
  switch (props.task.status) {
    case "success":
      return "success";
    case "failed":
      return "failed";
    case "cancelled":
    case "interrupted":
      return "cancelled";
    default:
      return "";
  }
});

const sizeText = computed(() => {
  const { downloaded, total, status } = props.task;
  if (status === "running" || status === "queued" || status === "retrying") {
    return `${formatBytes(downloaded)} / ${total > 0 ? formatBytes(total) : "未知大小"}`;
  }
  if (status === "success") return formatBytes(total > 0 ? total : downloaded);
  return `${formatBytes(downloaded)} 已下载`;
});

const speedText = computed(() => formatSpeed(props.task.speed));

const etaText = computed(() => {
  const remaining = props.task.total - props.task.downloaded;
  if (remaining <= 0 || props.task.speed <= 0) return "—";
  return formatEta(remaining / props.task.speed);
});

const durationText = computed(() => formatDuration(props.task.elapsedMs));

const host = computed(() => {
  try {
    return new URL(props.task.job.url).host;
  } catch {
    return props.task.job.url;
  }
});

const dirText = computed(() => {
  const parts = props.task.job.dir.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : props.task.job.dir;
});
</script>

<template>
  <article class="task" :class="task.status">
    <div class="glyph" :class="task.status">
      <AppIcon name="file" :size="19" />
    </div>

    <div class="body">
      <div class="head">
        <div class="titles">
          <div class="name" :title="task.displayName">{{ task.displayName }}</div>
          <div class="url" :title="task.job.url">{{ host }}</div>
        </div>
        <div class="head-right">
          <span v-if="isActive && task.total > 0" class="pct">{{ pct }}%</span>
          <StatusChip :status="task.status" />
        </div>
      </div>

      <div class="progress-track">
        <div class="progress-fill" :class="barClass" :style="{ width: `${pct}%` }" />
        <div v-if="task.total === 0 && isActive" class="progress-indeterminate" />
      </div>

      <div class="foot">
        <span class="stat">
          <AppIcon name="layers" :size="12" />{{ task.job.threads }} 线程
        </span>
        <span class="stat">{{ sizeText }}</span>
        <span v-if="isActive && task.speed > 0" class="stat accent">{{ speedText }}</span>
        <span v-if="isActive && etaText !== '—'" class="stat">剩余 {{ etaText }}</span>
        <span v-if="task.status === 'success'" class="stat ok">{{ durationText }}</span>
        <span v-if="task.attempts > 1" class="stat">第 {{ task.attempts }} 次尝试</span>
        <span class="grow" />
        <span class="stat path" :title="task.filePath || task.job.dir">
          <AppIcon name="folder" :size="12" />{{ dirText }}
        </span>
      </div>

      <div v-if="task.error" class="err">
        <AppIcon name="alert" :size="14" />
        <span class="err-text">{{ task.error }}</span>
      </div>

      <div v-if="task.status === 'success' && task.filePath" class="file-path" :title="task.filePath">
        {{ task.filePath }}
      </div>
    </div>

    <div class="actions">
      <button v-if="canCancel" class="icon-btn" title="取消下载" @click="$emit('cancel')">
        <AppIcon name="stop" :size="16" />
      </button>
      <button v-if="canRetry" class="icon-btn" title="重新下载" @click="$emit('retry')">
        <AppIcon name="retry" :size="16" />
      </button>
      <button
        v-if="task.status === 'success'"
        class="icon-btn"
        title="在文件夹中显示"
        @click="$emit('open')"
      >
        <AppIcon name="folder" :size="16" />
      </button>
      <button class="icon-btn danger" title="移除记录" @click="$emit('remove')">
        <AppIcon name="trash" :size="16" />
      </button>
    </div>
  </article>
</template>

<style scoped>
.task {
  display: flex;
  gap: 13px;
  padding: 14px 15px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--card);
  box-shadow: var(--shadow-card);
  transition: border-color var(--ease), background var(--ease);
}
.task:hover {
  border-color: var(--accent-line);
  background: var(--card-hover);
}
.task.failed {
  border-color: rgba(244, 63, 94, 0.28);
}
.task.success {
  border-color: rgba(16, 185, 129, 0.22);
}

.glyph {
  display: grid;
  flex: 0 0 38px;
  place-items: center;
  width: 38px;
  height: 38px;
  border: 1px solid var(--line);
  border-radius: 11px;
  background: var(--bg-soft);
  color: var(--text-3);
}
.glyph.running,
.glyph.queued,
.glyph.retrying {
  color: var(--accent-2);
  border-color: var(--accent-line);
  background: var(--accent-soft);
}
.glyph.success {
  color: var(--success);
  border-color: rgba(16, 185, 129, 0.28);
  background: var(--success-soft);
}
.glyph.failed {
  color: var(--danger);
  border-color: rgba(244, 63, 94, 0.28);
  background: var(--danger-soft);
}

.body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}

.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.titles {
  min-width: 0;
}
.name {
  overflow: hidden;
  color: var(--text);
  font-size: 13.5px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.url {
  overflow: hidden;
  margin-top: 1px;
  color: var(--text-3);
  font-family: var(--mono);
  font-size: 11.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.head-right {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 9px;
}
.pct {
  color: var(--accent-2);
  font-family: var(--mono);
  font-size: 12.5px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 13px;
  color: var(--text-3);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.stat {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
}
.stat.accent {
  color: var(--accent-2);
  font-weight: 550;
}
.stat.ok {
  color: var(--success);
}
.path {
  overflow: hidden;
  max-width: 220px;
  text-overflow: ellipsis;
}
.path :deep(svg) {
  flex: 0 0 auto;
  opacity: 0.75;
}

.err {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  padding: 8px 10px;
  border: 1px solid rgba(244, 63, 94, 0.26);
  border-radius: var(--radius-sm);
  background: var(--danger-soft);
  color: var(--danger);
  font-size: 12px;
  line-height: 1.55;
}
.err :deep(svg) {
  flex: 0 0 auto;
  margin-top: 2px;
}
.err-text {
  max-height: 96px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--mono);
}

.file-path {
  overflow: hidden;
  color: var(--text-3);
  font-family: var(--mono);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.actions {
  display: flex;
  flex: 0 0 auto;
  flex-direction: column;
  gap: 2px;
  align-items: center;
  align-self: center;
}
</style>
