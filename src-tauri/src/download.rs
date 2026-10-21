//! 下载任务的编排层：校验参数、创建句柄、后台线程执行、事件上报。

use crate::ffi::{self, BsdOptions, BsdRequest, Core};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const MAX_THREADS: u32 = 64;
const MAX_RETRY: u32 = 10;
const MIN_TIMEOUT: u32 = 5;
const MAX_TIMEOUT: u32 = 3600;
const MIN_RATE: u64 = 16 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub url: String,
    #[serde(default)]
    pub dir: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub checksum: String,
    #[serde(default)]
    pub threads: u32,
    #[serde(default)]
    pub retry: u32,
    #[serde(default)]
    pub timeout: u32,
    #[serde(default = "default_true")]
    pub resume: bool,
    #[serde(default)]
    pub limit_rate: u64,
    #[serde(default)]
    pub proxy: String,
    #[serde(default)]
    pub user_agent: String,
    #[serde(default)]
    pub headers: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartedTask {
    pub id: String,
    pub job: DownloadJob,
    pub display_name: String,
    pub created_at: u64,
}

pub struct TaskHandle {
    pub handle: usize,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct Registry {
    inner: Mutex<HashMap<String, TaskHandle>>,
}

impl Registry {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, TaskHandle>> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn insert(&self, id: String, handle: usize, cancel: Arc<AtomicBool>) {
        self.lock().insert(id, TaskHandle { handle, cancel });
    }

    /// 在锁内完成取消，避免与 `retire` 释放句柄产生竞争。
    pub fn cancel(&self, id: &str, core: &Core) -> bool {
        let guard = self.lock();
        match guard.get(id) {
            Some(task) => {
                task.cancel.store(true, Ordering::SeqCst);
                core.cancel_handle(task.handle);
                true
            }
            None => false,
        }
    }

    /// 在锁内移除并销毁句柄，保证取消命令不会操作已释放的句柄。
    pub fn retire(&self, id: &str, core: &Core) {
        let mut guard = self.lock();
        if let Some(task) = guard.remove(id) {
            core.destroy_handle(task.handle);
        }
    }

    #[allow(dead_code)]
    pub fn cancel_all(&self, core: &Core) -> usize {
        let guard = self.lock();
        for task in guard.values() {
            task.cancel.store(true, Ordering::SeqCst);
            core.cancel_handle(task.handle);
        }
        guard.len()
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    task_id: String,
    downloaded: u64,
    total: u64,
    speed: f64,
    finished: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TaskStatePayload {
    task_id: String,
    state: String,
    attempt: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FinishedPayload {
    pub task_id: String,
    pub ok: bool,
    pub cancelled: bool,
    pub code: i32,
    pub status_text: String,
    pub message: Option<String>,
    pub elapsed_ms: u64,
    pub attempts: u32,
    pub file_path: Option<String>,
}

struct JobCtx {
    app: AppHandle,
    task_id: String,
}

extern "C" fn progress_cb(user: *mut c_void, downloaded: u64, total: u64, bytes_per_second: f64, finished: c_int) {
    if user.is_null() {
        return;
    }
    // SAFETY: user 指向的 JobCtx 在 bsd_download 返回前一直存活。
    let ctx = unsafe { &*(user as *const JobCtx) };
    let _ = ctx.app.emit(
        "bsd://progress",
        ProgressPayload {
            task_id: ctx.task_id.clone(),
            downloaded,
            total,
            speed: bytes_per_second,
            finished: finished != 0,
        },
    );
}

pub fn status_text(code: i32) -> &'static str {
    match code {
        ffi::BSD_OK => "已完成",
        ffi::BSD_ERROR => "失败",
        ffi::BSD_ERROR_RETRYABLE => "可重试错误",
        ffi::BSD_ERROR_CANCELLED => "已取消",
        ffi::BSD_ERROR_INVALID_ARG => "参数错误",
        _ => "未知错误",
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// 把核心返回的英文异常整理成界面能一眼看懂的中文说明。
fn friendly_message(raw: Option<String>) -> Option<String> {
    let raw = raw?;
    let lower = raw.to_lowercase();
    if lower.contains("rename failed: file already exists") {
        return Some("目标文件已存在，请换一个文件名，或先删除同名文件".to_string());
    }
    if lower.contains("diff hash") {
        return Some("哈希校验失败，下载的文件已删除".to_string());
    }
    if lower.contains("url mismatch") || lower.contains("cannot resume") {
        return Some(format!("无法续传：{}", raw));
    }
    Some(raw)
}

fn next_task_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("t{}-{}", now_ms(), COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn default_dir() -> PathBuf {
    for key in ["USERPROFILE", "HOME"] {
        if let Ok(home) = std::env::var(key) {
            if !home.trim().is_empty() {
                return Path::new(home.trim()).join("Downloads");
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 从 URL 里猜测一个便于展示的文件名（仅用于界面提示）。
pub fn guess_name(url: &str) -> String {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    let tail = without_query.rsplit('/').next().unwrap_or("").trim();
    if tail.is_empty() {
        "未命名文件".to_string()
    } else {
        tail.to_string()
    }
}

fn display_name(job: &DownloadJob) -> String {
    if !job.output.trim().is_empty() {
        job.output.trim().to_string()
    } else {
        guess_name(&job.url)
    }
}

fn normalize(mut job: DownloadJob) -> Result<DownloadJob, String> {
    job.url = job.url.trim().to_string();
    if job.url.is_empty() {
        return Err("请填写下载链接".to_string());
    }
    let lower = job.url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(format!("链接必须以 http:// 或 https:// 开头：{}", job.url));
    }
    if job.url.chars().any(|c| c == '\0' || c == '\n' || c == '\r') {
        return Err("链接包含非法字符（换行或空字符）".to_string());
    }

    job.dir = job.dir.trim().to_string();
    if job.dir.is_empty() {
        job.dir = default_dir().to_string_lossy().into_owned();
    }
    let dir = PathBuf::from(&job.dir);
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建保存目录 {}：{err}", dir.display()))?;
    if !dir.is_dir() {
        return Err(format!("保存路径不是目录：{}", dir.display()));
    }

    job.output = job.output.trim().to_string();
    if job.output.contains(['\\', '/']) {
        return Err("文件名不能包含路径分隔符".to_string());
    }

    job.checksum = job.checksum.trim().to_string();
    if !job.checksum.is_empty() {
        let (alg, digest) = job
            .checksum
            .split_once(':')
            .ok_or_else(|| "校验值格式应为 算法:哈希，例如 sha256:abc123".to_string())?;
        let alg = alg.trim().to_ascii_lowercase();
        if !matches!(alg.as_str(), "md5" | "sha1" | "sha256" | "sha512") {
            return Err(format!("不支持的校验算法：{alg}（可选 md5 / sha1 / sha256 / sha512）"));
        }
        let digest = digest.trim().to_ascii_lowercase();
        if digest.is_empty() || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("校验哈希必须是非空十六进制字符串".to_string());
        }
        job.checksum = format!("{alg}:{digest}");
    }

    job.threads = job.threads.clamp(1, MAX_THREADS);
    job.retry = job.retry.min(MAX_RETRY);
    job.timeout = job.timeout.clamp(MIN_TIMEOUT, MAX_TIMEOUT);
    if job.limit_rate > 0 {
        job.limit_rate = job.limit_rate.max(MIN_RATE);
    }

    job.proxy = job.proxy.trim().to_string();
    job.user_agent = job.user_agent.trim().to_string();

    let mut headers = Vec::new();
    for raw in &job.headers {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if !line.contains(':') {
            return Err(format!("请求头缺少冒号：{line}"));
        }
        if line.chars().any(|c| c == '\0' || c == '\n' || c == '\r') {
            return Err(format!("请求头包含非法字符：{line}"));
        }
        headers.push(line.to_string());
    }
    job.headers = headers;

    Ok(job)
}

pub fn start(core: Arc<Core>, app: AppHandle, registry: Arc<Registry>, job: DownloadJob) -> Result<StartedTask, String> {
    let job = normalize(job)?;
    let name = display_name(&job);

    let dir_c = CString::new(job.dir.as_str()).map_err(|_| "保存目录包含空字符".to_string())?;
    let proxy_c = CString::new(job.proxy.as_str()).map_err(|_| "代理包含空字符".to_string())?;
    let ua_c = CString::new(job.user_agent.as_str()).map_err(|_| "User-Agent 包含空字符".to_string())?;

    let header_store: Vec<CString> = job
        .headers
        .iter()
        .map(|h| CString::new(h.as_str()).map_err(|_| format!("请求头包含空字符：{h}")))
        .collect::<Result<_, _>>()?;
    let header_ptrs: Vec<*const std::ffi::c_char> = header_store.iter().map(|h| h.as_ptr()).collect();

    let options = BsdOptions {
        struct_size: std::mem::size_of::<BsdOptions>(),
        dir: dir_c.as_ptr(),
        threads: job.threads as c_int,
        retry: job.retry as c_int,
        timeout_seconds: job.timeout as c_int,
        continue_download: c_int::from(job.resume),
        limit_rate_bytes_per_second: job.limit_rate,
        proxy: proxy_c.as_ptr(),
        user_agent: ua_c.as_ptr(),
        headers: if header_ptrs.is_empty() { std::ptr::null() } else { header_ptrs.as_ptr() },
        header_count: header_ptrs.len(),
    };

    // bsd_handle_create 会把字符串复制进核心对象，函数返回后这些 CString 即可释放。
    let handle = core.create_handle(&options)?;

    let id = next_task_id();
    let cancel = Arc::new(AtomicBool::new(false));
    registry.insert(id.clone(), handle, cancel.clone());

    let created_at = now_ms();
    let worker_core = core.clone();
    let worker_registry = registry.clone();
    let worker_app = app.clone();
    let worker_id = id.clone();
    let worker_job = job.clone();
    let worker_cancel = cancel.clone();

    std::thread::Builder::new()
        .name(format!("bsd-download-{id}"))
        .spawn(move || run(worker_app, worker_core, worker_registry, worker_id, worker_job, handle, worker_cancel))
        .map_err(|err| {
            registry.retire(&id, &core);
            format!("无法启动下载线程：{err}")
        })?;

    Ok(StartedTask { id, job, display_name: name, created_at })
}

fn sleep_cancel(millis: u64, cancel: &AtomicBool) -> bool {
    let mut left = millis;
    while left > 0 {
        if cancel.load(Ordering::SeqCst) {
            return false;
        }
        let step = left.min(100);
        std::thread::sleep(Duration::from_millis(step));
        left -= step;
    }
    !cancel.load(Ordering::SeqCst)
}

fn effective_file_path(job: &DownloadJob) -> Option<PathBuf> {
    let name = if !job.output.trim().is_empty() {
        job.output.trim().to_string()
    } else {
        let tail = guess_name(&job.url);
        if tail == "未命名文件" { return None; } else { tail }
    };
    Some(Path::new(&job.dir).join(name))
}

fn run(
    app: AppHandle,
    core: Arc<Core>,
    registry: Arc<Registry>,
    task_id: String,
    job: DownloadJob,
    handle: usize,
    cancel: Arc<AtomicBool>,
) {
    let started = Instant::now();

    let url_c = CString::new(job.url.as_str()).unwrap_or_default();
    let output_c = CString::new(job.output.as_str()).unwrap_or_default();
    let checksum_c = CString::new(job.checksum.as_str()).unwrap_or_default();
    let request = BsdRequest {
        struct_size: std::mem::size_of::<BsdRequest>(),
        url: url_c.as_ptr(),
        output: output_c.as_ptr(),
        checksum: checksum_c.as_ptr(),
    };

    let ctx = Box::new(JobCtx { app: app.clone(), task_id: task_id.clone() });
    let user = (&*ctx as *const JobCtx).cast_mut().cast::<c_void>();

    let mut attempts: u32 = 0;
    let mut last_code;
    let mut last_message;

    loop {
        attempts += 1;
        let (code, message) = core.download(handle, &request, progress_cb, user);
        last_code = code;
        last_message = message;

        let retryable = code == ffi::BSD_ERROR_RETRYABLE;
        if retryable && attempts <= job.retry && !cancel.load(Ordering::SeqCst) {
            let _ = app.emit(
                "bsd://task-state",
                TaskStatePayload { task_id: task_id.clone(), state: "retrying".to_string(), attempt: attempts },
            );
            if !sleep_cancel(800 * u64::from(attempts), &cancel) {
                last_code = ffi::BSD_ERROR_CANCELLED;
                last_message = Some("已取消".to_string());
                break;
            }
            continue;
        }
        break;
    }

    drop(ctx);

    registry.retire(&task_id, &core);

    let cancelled = cancel.load(Ordering::SeqCst) || last_code == ffi::BSD_ERROR_CANCELLED;
    let ok = last_code == ffi::BSD_OK;
    let mut message = last_message;
    if cancelled {
        message = Some("已取消".to_string());
    } else if ok {
        message = None;
    } else {
        message = friendly_message(message);
    }

    let payload = FinishedPayload {
        task_id: task_id.clone(),
        ok,
        cancelled,
        code: last_code,
        status_text: status_text(last_code).to_string(),
        message,
        elapsed_ms: started.elapsed().as_millis() as u64,
        attempts,
        file_path: if ok {
            effective_file_path(&job).map(|p| p.to_string_lossy().into_owned())
        } else {
            None
        },
    };
    let _ = app.emit("bsd://finished", payload);
}
