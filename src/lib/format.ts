const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** 人类可读的字节数，例如 1.4 GB。数值未知时返回 "—"。 */
export function formatBytes(bytes: number, digits = 1): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const text = unit === 0 || value >= 100 ? String(Math.round(value)) : value.toFixed(digits);
  return `${text} ${UNITS[unit]}`;
}

export function formatSpeed(bytesPerSecond: number): string {
  if (!Number.isFinite(bytesPerSecond) || bytesPerSecond <= 0) return "—";
  return `${formatBytes(bytesPerSecond)}/s`;
}

/** 毫秒 → "8 秒" / "3 分 12 秒" / "1 小时 4 分"。 */
export function formatDuration(millis: number): string {
  if (!Number.isFinite(millis) || millis < 0) return "—";
  const totalSeconds = Math.floor(millis / 1000);
  if (totalSeconds < 1) return "不到 1 秒";
  if (totalSeconds < 60) return `${totalSeconds} 秒`;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes < 60) return seconds > 0 ? `${minutes} 分 ${seconds} 秒` : `${minutes} 分`;
  const hours = Math.floor(minutes / 60);
  const restMinutes = minutes % 60;
  return restMinutes > 0 ? `${hours} 小时 ${restMinutes} 分` : `${hours} 小时`;
}

/** 秒 → 剩余时间。 */
export function formatEta(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "—";
  return formatDuration(seconds * 1000);
}

function pad(value: number, width = 2): string {
  return String(value).padStart(width, "0");
}

/** HH:MM:SS.mmm（日志时间戳）。 */
export function formatClock(timestamp: number): string {
  const date = new Date(timestamp);
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(
    date.getMilliseconds(),
    3,
  )}`;
}

export function formatDateTime(timestamp: number | null): string {
  if (!timestamp) return "—";
  const date = new Date(timestamp);
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}`;
}

export function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, value));
}

/** 判断 URL 里是否含协议前缀，用于输入即时校验。 */
export function looksLikeUrl(text: string): boolean {
  const trimmed = text.trim().toLowerCase();
  return trimmed.startsWith("http://") || trimmed.startsWith("https://");
}

/** 核心版本串可能形如 "0.1.9+2(beta)-[SHA256:…]"，卡片里只展示主版本号。 */
export function shortVersion(version: string | null | undefined): string {
  if (!version) return "?";
  const main = version.split(/[+([]/)[0]?.trim();
  return main && main.length > 0 ? main : version;
}
