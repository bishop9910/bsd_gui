<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import { RouterLink, RouterView } from "vue-router";
import { useNotification } from "naive-ui";
import AppIcon from "./components/AppIcon.vue";
import {
  bsd,
  onCoreError,
  onFinished,
  onLog,
  onProgress,
  onTaskState,
  type FinishedEvent,
  type LogEvent,
  type ProgressEvent,
  type TaskStateEvent,
} from "./lib/bsd";
import { formatDuration, shortVersion } from "./lib/format";
import {
  coreStatus,
  findTask,
  persistTasks,
  pushLog,
  refreshCoreStatus,
  settings,
  tasks,
} from "./lib/store";
import { isDark, toggleTheme } from "./lib/theme-state";
import type { UnlistenFn } from "@tauri-apps/api/event";

const notification = useNotification();

const nav = [
  { to: "/", label: "下载任务", icon: "download", badge: true },
  { to: "/logs", label: "日志", icon: "logs", badge: false },
  { to: "/settings", label: "设置", icon: "settings", badge: false },
  { to: "/about", label: "关于", icon: "info", badge: false },
];

const coreClass = computed(() => {
  if (coreStatus.value === null) return "pending";
  return coreStatus.value.loaded ? "ok" : "bad";
});
const coreTitle = computed(() => {
  if (coreStatus.value === null) return "检测核心";
  return coreStatus.value.loaded ? "核心已就绪" : "核心未加载";
});
const coreHint = computed(() => {
  const status = coreStatus.value;
  if (!status) return "正在检测 bsd_core.dll";
  return status.loaded ? `BSD Core v${shortVersion(status.version)}` : status.error ?? "点击查看原因";
});
const activeCount = computed(() =>
  tasks.filter((task) => task.status === "running" || task.status === "retrying" || task.status === "queued").length,
);

let unlisteners: UnlistenFn[] = [];

function handleProgress(event: ProgressEvent): void {
  const task = findTask(event.taskId);
  if (!task) return;
  if (task.status === "success" || task.status === "failed" || task.status === "cancelled" || task.status === "interrupted") {
    return;
  }
  task.status = "running";
  task.downloaded = event.downloaded;
  if (event.total > 0) task.total = event.total;
  task.speed = event.speed;
}

function handleTaskState(event: TaskStateEvent): void {
  const task = findTask(event.taskId);
  if (!task) return;
  task.status = "retrying";
  task.attempts = event.attempt;
  pushLog(3, `「${task.displayName}」第 ${event.attempt} 次重试即将开始`);
}

function handleFinished(event: FinishedEvent): void {
  const task = findTask(event.taskId);
  if (!task) return;
  task.finishedAt = Date.now();
  task.elapsedMs = event.elapsedMs;
  task.attempts = event.attempts;
  task.speed = 0;

  if (event.ok) {
    task.status = "success";
    task.error = null;
    task.filePath = event.filePath;
    if (task.total > 0) task.downloaded = task.total;
    pushLog(2, `「${task.displayName}」下载完成，用时 ${formatDuration(event.elapsedMs)}`);
    if (settings.notifyOnComplete) {
      notification.success({
        title: "下载完成",
        meta: task.displayName,
        content: event.filePath ?? "文件已保存",
        duration: 5000,
      });
    }
    if (settings.autoOpenOnComplete && event.filePath) {
      void bsd.openFolder(event.filePath).catch(() => {});
    }
  } else if (event.cancelled) {
    task.status = "cancelled";
    task.error = null;
    pushLog(3, `「${task.displayName}」已取消`);
  } else {
    task.status = "failed";
    task.error = event.message ?? event.statusText;
    pushLog(4, `「${task.displayName}」失败：${task.error}`);
    if (settings.notifyOnComplete) {
      notification.error({
        title: "下载失败",
        meta: task.displayName,
        content: event.message ?? event.statusText,
        duration: 9000,
      });
    }
  }
  persistTasks();
}

function handleLog(event: LogEvent): void {
  pushLog(event.level, event.message, event.ts);
}

onMounted(async () => {
  await refreshCoreStatus();
  try {
    await bsd.setLogLevel(settings.logLevel);
  } catch {
    /* 核心未加载时忽略 */
  }
  try {
    unlisteners.push(
      await onProgress(handleProgress),
      await onTaskState(handleTaskState),
      await onFinished(handleFinished),
      await onLog(handleLog),
      await onCoreError(() => {
        void refreshCoreStatus();
      }),
    );
    pushLog(2, "界面已就绪，等待下载任务");
  } catch (error) {
    pushLog(4, `事件通道初始化失败：${String(error)}`);
  }
  window.addEventListener("beforeunload", persistTasks);
});

onUnmounted(() => {
  unlisteners.forEach((unlisten) => unlisten());
  unlisteners = [];
  window.removeEventListener("beforeunload", persistTasks);
});
</script>

<template>
  <div class="app">
    <header class="topbar">
      <RouterLink to="/" class="brand">
        <span class="brand-mark"><AppIcon name="download" :size="18" :stroke="2.1" /></span>
        <span class="brand-name">BSD 下载器</span>
      </RouterLink>

      <nav class="tabs">
        <RouterLink
          v-for="item in nav"
          :key="item.to"
          :to="item.to"
          class="tab"
          active-class="active"
        >
          <AppIcon :name="item.icon" :size="16" />
          <span>{{ item.label }}</span>
          <span v-if="item.badge && activeCount > 0" class="tab-badge">{{ activeCount }}</span>
        </RouterLink>
      </nav>

      <div class="topbar-right">
        <RouterLink to="/about" class="core-pill" :class="coreClass" :title="coreHint">
          <span class="core-dot" :class="coreClass" />
          <span>{{ coreTitle }}</span>
        </RouterLink>
        <button
          class="icon-btn theme-btn"
          type="button"
          :title="isDark ? '切换到亮色模式' : '切换到暗色模式'"
          @click="toggleTheme"
        >
          <AppIcon :name="isDark ? 'sun' : 'moon'" :size="17" />
        </button>
      </div>
    </header>

    <main class="content">
      <div v-if="coreStatus && !coreStatus.loaded" class="banner">
        <span class="banner-icon"><AppIcon name="alert" :size="18" /></span>
        <div class="grow">
          <div class="banner-title">bsd_core.dll 未能加载</div>
          <pre v-if="coreStatus.error" class="banner-body">{{ coreStatus.error }}</pre>
          <RouterLink to="/about" class="btn banner-btn">查看原因</RouterLink>
        </div>
      </div>

      <RouterView v-slot="{ Component }">
        <Transition name="fade" mode="out-in">
          <component :is="Component" />
        </Transition>
      </RouterView>
    </main>
  </div>
</template>

<style scoped>
.banner-btn {
  margin-top: 9px;
  height: 30px;
  padding: 0 12px;
  font-size: 12px;
}
</style>
