export interface DownloadJob {
  url: string;
  dir: string;
  output: string;
  checksum: string;
  threads: number;
  retry: number;
  timeout: number;
  resume: boolean;
  limitRate: number;
  proxy: string;
  userAgent: string;
  headers: string[];
}

export interface StartedTask {
  id: string;
  job: DownloadJob;
  displayName: string;
  createdAt: number;
}

export type TaskStatus =
  | "queued"
  | "running"
  | "retrying"
  | "success"
  | "failed"
  | "cancelled"
  | "interrupted";

export interface TaskRecord extends StartedTask {
  status: TaskStatus;
  downloaded: number;
  total: number;
  speed: number;
  error: string | null;
  filePath: string | null;
  elapsedMs: number;
  attempts: number;
  finishedAt: number | null;
}

export interface CoreStatus {
  loaded: boolean;
  version: string | null;
  dllPath: string | null;
  error: string | null;
  searched: string[];
}

export interface LogEntry {
  id: number;
  level: number;
  message: string;
  ts: number;
}

export interface AppSettings {
  dir: string;
  threads: number;
  limitRate: number;
  timeout: number;
  retry: number;
  resume: boolean;
  proxy: string;
  userAgent: string;
  headersText: string;
  logLevel: number;
  autoOpenOnComplete: boolean;
  notifyOnComplete: boolean;
}
