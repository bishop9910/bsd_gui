import { reactive, ref, watch } from "vue";
import { bsd } from "./bsd";
import type { AppSettings, CoreStatus, LogEntry, StartedTask, TaskRecord, TaskStatus } from "./types";

const SETTINGS_KEY = "bsd.settings.v1";
const TASKS_KEY = "bsd.tasks.v1";
const LOG_LIMIT = 1200;
const TASK_LIMIT = 80;

export function defaultSettings(): AppSettings {
  return {
    dir: "",
    threads: 8,
    limitRate: 0,
    timeout: 90,
    retry: 3,
    resume: true,
    proxy: "",
    userAgent: "",
    headersText: "",
    logLevel: 2,
    autoOpenOnComplete: false,
    notifyOnComplete: true,
  };
}

function readSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return defaultSettings();
    const parsed = JSON.parse(raw) as Partial<AppSettings>;
    return { ...defaultSettings(), ...parsed };
  } catch {
    return defaultSettings();
  }
}

export const settings = reactive<AppSettings>(readSettings());

export function saveSettings(): void {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    /* 本地存储不可用时忽略 */
  }
}

watch(settings, saveSettings, { deep: true });

export function resetSettings(): void {
  Object.assign(settings, defaultSettings());
  saveSettings();
}

/* ------------------------------ 任务列表 ------------------------------ */

const UNFINISHED: TaskStatus[] = ["queued", "running", "retrying"];

function readTasks(): TaskRecord[] {
  try {
    const raw = localStorage.getItem(TASKS_KEY);
    if (!raw) return [];
    const list = JSON.parse(raw) as TaskRecord[];
    if (!Array.isArray(list)) return [];
    return list.slice(0, TASK_LIMIT).map((task) => {
      if (UNFINISHED.includes(task.status)) {
        return {
          ...task,
          status: "interrupted" as TaskStatus,
          speed: 0,
          error: task.error ?? "应用上次退出时该任务尚未完成，可点击重试继续",
        };
      }
      return { ...task, speed: 0 };
    });
  } catch {
    return [];
  }
}

export const tasks = reactive<TaskRecord[]>(readTasks());

export function persistTasks(): void {
  try {
    localStorage.setItem(TASKS_KEY, JSON.stringify(tasks.slice(0, TASK_LIMIT)));
  } catch {
    /* 忽略写入失败 */
  }
}

export function addTask(started: StartedTask): TaskRecord {
  const record: TaskRecord = {
    ...started,
    status: "queued",
    downloaded: 0,
    total: 0,
    speed: 0,
    error: null,
    filePath: null,
    elapsedMs: 0,
    attempts: 0,
    finishedAt: null,
  };
  tasks.unshift(record);
  if (tasks.length > TASK_LIMIT) tasks.splice(TASK_LIMIT);
  persistTasks();
  return record;
}

export function findTask(id: string): TaskRecord | undefined {
  return tasks.find((task) => task.id === id);
}

export function removeTask(id: string): void {
  const index = tasks.findIndex((task) => task.id === id);
  if (index >= 0) tasks.splice(index, 1);
  persistTasks();
}

export function clearFinished(): number {
  let removed = 0;
  for (let index = tasks.length - 1; index >= 0; index -= 1) {
    if (!UNFINISHED.includes(tasks[index].status)) {
      tasks.splice(index, 1);
      removed += 1;
    }
  }
  persistTasks();
  return removed;
}

export function activeCount(): number {
  return tasks.filter((task) => UNFINISHED.includes(task.status)).length;
}

/* ------------------------------ 运行日志 ------------------------------ */

export const logs = reactive<LogEntry[]>([]);
let logSeq = 0;

export function pushLog(level: number, message: string, ts = Date.now()): void {
  logSeq += 1;
  logs.push({ id: logSeq, level, message, ts });
  if (logs.length > LOG_LIMIT) logs.splice(0, logs.length - LOG_LIMIT);
}

export function clearLogs(): void {
  logs.splice(0, logs.length);
}

export const LOG_LEVELS = [
  { label: "TRACE", value: 0 },
  { label: "DEBUG", value: 1 },
  { label: "INFO", value: 2 },
  { label: "WARN", value: 3 },
  { label: "ERROR", value: 4 },
  { label: "OFF", value: 5 },
] as const;

export function levelName(level: number): string {
  return LOG_LEVELS.find((item) => item.value === level)?.label ?? "LOG";
}

/* ------------------------------ 核心状态 ------------------------------ */

export const coreStatus = ref<CoreStatus | null>(null);

export async function refreshCoreStatus(): Promise<void> {
  try {
    coreStatus.value = await bsd.coreStatus();
  } catch (error) {
    coreStatus.value = {
      loaded: false,
      version: null,
      dllPath: null,
      error: `无法与后端通信：${String(error)}`,
      searched: [],
    };
  }
}
