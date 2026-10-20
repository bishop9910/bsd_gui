<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useNotification } from "naive-ui";
import AppIcon from "../components/AppIcon.vue";
import { bsd } from "../lib/bsd";
import { shortVersion } from "../lib/format";
import { coreStatus, refreshCoreStatus } from "../lib/store";

const notification = useNotification();
const appVersion = ref("—");
const refreshing = ref(false);

async function refresh(): Promise<void> {
  refreshing.value = true;
  try {
    await refreshCoreStatus();
  } finally {
    refreshing.value = false;
  }
}

async function openDll(): Promise<void> {
  const path = coreStatus.value?.dllPath;
  if (!path) return;
  try {
    await bsd.openFolder(path);
  } catch (error) {
    notification.error({ title: "无法打开所在目录", content: String(error) });
  }
}

onMounted(async () => {
  try {
    appVersion.value = await bsd.appVersion();
  } catch {
    appVersion.value = "—";
  }
});
</script>

<template>
  <div class="view">
    <header class="page-head">
      <h1 class="page-title">关于</h1>
      <span class="version-pill">v{{ appVersion }}</span>
    </header>

    <section class="card core-card">
      <div class="core-left" :class="{ ok: coreStatus?.loaded, bad: coreStatus && !coreStatus.loaded }">
        <div class="core-badge">
          <AppIcon :name="coreStatus?.loaded ? 'check' : 'cube'" :size="24" />
        </div>
        <div>
          <h2 class="core-title">
            {{ coreStatus === null ? "检测中…" : coreStatus.loaded ? "核心已就绪" : "核心未加载" }}
          </h2>
          <p class="core-sub">
            {{
              coreStatus === null
                ? "bsd_core.dll"
                : coreStatus.loaded
                  ? `BSD Core v${shortVersion(coreStatus.version)} · API v1`
                  : "下载功能不可用"
            }}
          </p>
        </div>
      </div>

      <div class="core-right">
        <button class="btn" :disabled="refreshing" @click="refresh">
          <AppIcon name="refresh" :size="14" />{{ refreshing ? "检测中…" : "重新检测" }}
        </button>
        <button v-if="coreStatus?.loaded && coreStatus.dllPath" class="btn" @click="openDll">
          <AppIcon name="folder" :size="14" />DLL 位置
        </button>
      </div>
    </section>

    <section v-if="coreStatus?.loaded" class="card card-pad path-card">
      <span class="field-label">DLL 路径</span>
      <code class="path">{{ coreStatus.dllPath }}</code>
      <span class="field-label sub-label">版本</span>
      <code class="path">{{ coreStatus.version ?? "—" }}</code>
    </section>

    <section v-else-if="coreStatus && !coreStatus.loaded" class="card card-pad error-card">
      <span class="field-label">失败原因</span>
      <pre class="error-text">{{ coreStatus.error }}</pre>
      <span class="field-label sub-label">已尝试的路径</span>
      <ul class="searched">
        <li v-for="path in coreStatus.searched" :key="path" class="mono">{{ path }}</li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
.version-pill {
  padding: 4px 11px;
  border: 1px solid var(--line);
  border-radius: 999px;
  color: var(--text-3);
  font-family: var(--mono);
  font-size: 11.5px;
}

.core-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 20px;
}

.core-left {
  display: flex;
  align-items: center;
  gap: 13px;
  min-width: 0;
}

.core-badge {
  display: grid;
  flex: 0 0 46px;
  place-items: center;
  width: 46px;
  height: 46px;
  border: 1px solid var(--line);
  border-radius: 13px;
  background: var(--bg-soft);
  color: var(--text-3);
}
.core-left.ok .core-badge {
  border-color: rgba(16, 185, 129, 0.3);
  background: var(--success-soft);
  color: var(--success);
}
.core-left.bad .core-badge {
  border-color: rgba(244, 63, 94, 0.3);
  background: var(--danger-soft);
  color: var(--danger);
}

.core-title {
  font-size: 15px;
  font-weight: 650;
}
.core-sub {
  margin-top: 2px;
  color: var(--text-3);
  font-size: 12.5px;
}

.core-right {
  display: flex;
  flex: 0 0 auto;
  gap: 8px;
}

.path-card .path {
  display: block;
  overflow: hidden;
  padding: 8px 11px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--bg-soft);
  color: var(--text-2);
  font-family: var(--mono);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sub-label {
  margin-top: 12px;
}

.error-card {
  border-color: rgba(244, 63, 94, 0.26);
}
.error-text {
  margin: 0;
  padding: 9px 11px;
  max-height: 170px;
  overflow: auto;
  border: 1px solid rgba(244, 63, 94, 0.24);
  border-radius: var(--radius-sm);
  background: var(--danger-soft);
  color: var(--danger);
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.65;
  white-space: pre-wrap;
  word-break: break-word;
}

.searched {
  margin: 0;
  padding-left: 18px;
  color: var(--text-3);
  font-size: 11.5px;
  line-height: 1.9;
}
.searched .mono {
  word-break: break-all;
}
</style>
