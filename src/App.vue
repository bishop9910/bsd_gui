<script setup lang="ts">
import { computed } from "vue";
import {
  NConfigProvider,
  NDialogProvider,
  NMessageProvider,
  NNotificationProvider,
  darkTheme,
  dateZhCN,
  zhCN,
} from "naive-ui";
import AppShell from "./AppShell.vue";
import { getThemeOverrides } from "./theme";
import { initTheme, isDark } from "./lib/theme-state";

initTheme();

const naiveTheme = computed(() => (isDark.value ? darkTheme : null));
const themeOverrides = computed(() => getThemeOverrides(isDark.value));
</script>

<template>
  <NConfigProvider
    :theme="naiveTheme"
    :theme-overrides="themeOverrides"
    :locale="zhCN"
    :date-locale="dateZhCN"
  >
    <NMessageProvider :max="3" placement="top">
      <NDialogProvider>
        <NNotificationProvider :max="4" placement="bottom-right">
          <AppShell />
        </NNotificationProvider>
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>
