<script setup lang="ts">
import type { TaskStatus } from "../lib/types";

defineProps<{
  status: TaskStatus;
}>();

const labels: Record<TaskStatus, string> = {
  queued: "等待中",
  running: "下载中",
  retrying: "重试中",
  success: "已完成",
  failed: "失败",
  cancelled: "已取消",
  interrupted: "已中断",
};
</script>

<template>
  <span class="chip" :class="status">
    <span class="chip-dot" />
    {{ labels[status] }}
  </span>
</template>
