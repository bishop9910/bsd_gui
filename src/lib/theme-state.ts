import { computed, ref } from "vue";

export type ThemeMode = "dark" | "light";

const STORAGE_KEY = "bsd.theme.v1";

function readStoredMode(): ThemeMode {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark") return stored;
  } catch {
    /* 忽略存储不可用 */
  }
  return "dark";
}

/** 当前主题（仅影响外观，与下载逻辑无关）。 */
export const themeMode = ref<ThemeMode>(readStoredMode());
export const isDark = computed(() => themeMode.value === "dark");

function apply(mode: ThemeMode): void {
  const root = document.documentElement;
  root.dataset.theme = mode;
  root.style.colorScheme = mode;
}

export function setTheme(mode: ThemeMode): void {
  themeMode.value = mode;
  try {
    localStorage.setItem(STORAGE_KEY, mode);
  } catch {
    /* 忽略存储不可用 */
  }
  apply(mode);
}

export function toggleTheme(): void {
  setTheme(themeMode.value === "dark" ? "light" : "dark");
}

/** 启动时把已保存的主题写到 <html> 上，避免首屏闪烁。 */
export function initTheme(): void {
  apply(themeMode.value);
}
