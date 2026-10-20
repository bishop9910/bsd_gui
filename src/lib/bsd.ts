import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CoreStatus, DownloadJob, StartedTask } from "./types";

export interface ProgressEvent {
  taskId: string;
  downloaded: number;
  total: number;
  speed: number;
  finished: boolean;
}

export interface TaskStateEvent {
  taskId: string;
  state: "retrying" | string;
  attempt: number;
}

export interface FinishedEvent {
  taskId: string;
  ok: boolean;
  cancelled: boolean;
  code: number;
  statusText: string;
  message: string | null;
  elapsedMs: number;
  attempts: number;
  filePath: string | null;
}

export interface LogEvent {
  level: number;
  message: string;
  ts: number;
}

/** 与 src-tauri 中注册的命令一一对应。 */
export const bsd = {
  coreStatus: () => invoke<CoreStatus>("core_status"),
  start: (job: DownloadJob) => invoke<StartedTask>("start_download", { job }),
  cancel: (id: string) => invoke<void>("cancel_download", { id }),
  setLogLevel: (level: number) => invoke<void>("set_log_level", { level }),
  defaultDir: () => invoke<string>("default_download_dir"),
  openFolder: (path: string) => invoke<void>("open_folder", { path }),
  pickFolder: (defaultPath: string) => invoke<string | null>("pick_folder", { defaultPath }),
  readClipboard: () => invoke<string>("read_clipboard"),
  appVersion: () => invoke<string>("app_version"),
};

export function onProgress(handler: (event: ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<ProgressEvent>("bsd://progress", (event) => handler(event.payload));
}

export function onTaskState(handler: (event: TaskStateEvent) => void): Promise<UnlistenFn> {
  return listen<TaskStateEvent>("bsd://task-state", (event) => handler(event.payload));
}

export function onFinished(handler: (event: FinishedEvent) => void): Promise<UnlistenFn> {
  return listen<FinishedEvent>("bsd://finished", (event) => handler(event.payload));
}

export function onLog(handler: (event: LogEvent) => void): Promise<UnlistenFn> {
  return listen<LogEvent>("bsd://log", (event) => handler(event.payload));
}

/** 后端在核心加载失败时通过该事件通知，payload 为空对象。 */
export function onCoreError(handler: () => void): Promise<UnlistenFn> {
  return listen("bsd://core-error", () => handler());
}
