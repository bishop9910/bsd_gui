/** 从 curl 命令 / Cookie 字符串里提取下载需要的请求信息。 */

export interface CurlImport {
  url: string;
  userAgent: string;
  referer: string;
  cookie: string;
  extraHeaders: string[];
}

/** 把 `curl ...` 文本切成参数（处理单/双引号、反斜杠转义、行尾的 \ 或 ^ 续行）。 */
function tokenize(input: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let quote: '"' | "'" | null = null;
  let started = false;
  for (let i = 0; i < input.length; i += 1) {
    const ch = input[i];
    if (quote) {
      if (ch === quote) {
        quote = null;
        continue;
      }
      if (ch === "\\" && quote === '"' && i + 1 < input.length) {
        current += input[i + 1];
        i += 1;
        continue;
      }
      current += ch;
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      started = true;
      continue;
    }
    if (ch === "\\" && i + 1 < input.length) {
      const nextCh = input[i + 1];
      if (nextCh === "\n") {
        i += 1;
        continue;
      }
      current += nextCh;
      i += 1;
      started = true;
      continue;
    }
    if (/\s/.test(ch)) {
      if (started) {
        tokens.push(current);
        current = "";
        started = false;
      }
      continue;
    }
    current += ch;
    started = true;
  }
  if (started) tokens.push(current);
  return tokens;
}

/** 归一化 cookie：去掉 `Cookie:` 前缀与换行，多个键用 `; ` 连接。 */
export function normalizeCookie(raw: string): string {
  const text = raw
    .replace(/\r?\n/g, "; ")
    .replace(/^\s*cookie\s*:\s*/i, "")
    .trim()
    .replace(/;+$/, "");
  if (!text || !text.includes("=")) return "";
  return text
    .split(";")
    .map((part) => part.trim())
    .filter((part) => part.length > 0)
    .join("; ");
}

/** cookie 里有几个键（用于提示）。 */
export function cookiePairCount(cookie: string): number {
  if (!cookie) return 0;
  return cookie.split(";").filter((part) => part.includes("=")).length;
}

function splitHeader(line: string): { name: string; value: string } | null {
  const index = line.indexOf(":");
  if (index <= 0) return null;
  const name = line.slice(0, index).trim();
  let value = line.slice(index + 1).trim();
  if (!name || !value) return null;
  if (name.toLowerCase() === "cookie") value = normalizeCookie(value);
  return { name, value };
}

/** 解析 curl 命令：自动识别 URL、Referer、User-Agent、Cookie，其余 -H 原样保留。 */
export function parseCurlCommand(raw: string): CurlImport {
  const text = raw.replace(/\r?\n/g, "\n").replace(/\^\s*\n/g, " ");
  const tokens = tokenize(text);
  const result: CurlImport = { url: "", userAgent: "", referer: "", cookie: "", extraHeaders: [] };

  const takeHeader = (line: string): void => {
    const header = splitHeader(line);
    if (!header) return;
    const name = header.name.toLowerCase();
    if (name === "user-agent") result.userAgent = header.value;
    else if (name === "referer") result.referer = header.value;
    else if (name === "cookie") result.cookie = header.value;
    else result.extraHeaders.push(`${header.name}: ${header.value}`);
  };

  for (let i = 0; i < tokens.length; i += 1) {
    const token = tokens[i];
    if (token === "curl" || token === "$" || token === ">") continue;

    let key = token;
    let inline: string | null = null;
    if (token.startsWith("--")) {
      const eq = token.indexOf("=");
      if (eq > 0) {
        key = token.slice(0, eq);
        inline = token.slice(eq + 1);
      }
    }
    const value = (): string => {
      if (inline !== null) {
        const current = inline;
        inline = null;
        return current;
      }
      i += 1;
      return tokens[i] ?? "";
    };

    switch (key) {
      case "-H":
      case "--header":
        takeHeader(value());
        break;
      case "-b":
      case "--cookie": {
        const cookie = normalizeCookie(value());
        if (cookie) result.cookie = cookie;
        break;
      }
      case "-A":
      case "--user-agent":
        result.userAgent = value();
        break;
      case "-e":
      case "--referer":
        result.referer = value();
        break;
      case "--url":
        result.url = value();
        break;
      default:
        if (!key.startsWith("-") && !result.url && /^https?:\/\//i.test(token)) result.url = token;
    }
  }
  return result;
}

/** 把新增请求头写进现有的「每行一个」请求头文本，同名（忽略大小写）覆盖。 */
export function mergeHeaderLines(existing: string, additions: string[]): string {
  const parsed = existing
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => splitHeader(line) ?? { name: line, value: "" });

  const names = new Set(additions.map((line) => (splitHeader(line)?.name ?? line).toLowerCase()));
  const kept = parsed
    .filter((item) => !names.has(item.name.toLowerCase()))
    .map((item) => (item.value ? `${item.name}: ${item.value}` : item.name));
  return [...kept, ...additions].join("\n");
}
