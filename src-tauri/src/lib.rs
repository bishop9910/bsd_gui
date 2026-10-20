mod download;
mod ffi;

use download::{DownloadJob, Registry, StartedTask};
use ffi::Core;
use serde::Serialize;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Manager, State};

/// 进程级核心状态：要么已加载，要么记录失败原因。
struct CoreState {
    core: Option<Arc<Core>>,
    error: Option<String>,
    searched: Vec<PathBuf>,
}

static CORE_STATE: OnceLock<CoreState> = OnceLock::new();
static LOG_APP: OnceLock<AppHandle> = OnceLock::new();

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LogPayload {
    level: i32,
    message: String,
    ts: u64,
}

extern "C" fn log_cb(_user: *mut c_void, level: c_int, message: *const c_char) {
    if message.is_null() {
        return;
    }
    let text = unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned();
    if let Some(app) = LOG_APP.get() {
        let _ = app.emit("bsd://log", LogPayload { level, message: text, ts: now_ms() });
    }
}

fn candidate_paths(app: &AppHandle) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("bsd_core.dll"));
            paths.push(dir.join("binaries").join("bsd_core.dll"));
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        paths.push(resource_dir.join("binaries").join("bsd_core.dll"));
        paths.push(resource_dir.join("bsd_core.dll"));
    }

    // 开发运行（cargo / tauri dev）时，target/debug 与 src-tauri/binaries 都可能命中。
    paths.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join("bsd_core.dll"));

    paths.dedup();
    paths
}

fn core_ref() -> Result<Arc<Core>, String> {
    let state = CORE_STATE.get().ok_or_else(|| "BSD 核心尚未初始化".to_string())?;
    match &state.core {
        Some(core) => Ok(core.clone()),
        None => Err(state.error.clone().unwrap_or_else(|| "BSD 核心未加载".to_string())),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoreStatus {
    loaded: bool,
    version: Option<String>,
    dll_path: Option<String>,
    error: Option<String>,
    searched: Vec<String>,
}

#[tauri::command]
fn core_status() -> CoreStatus {
    match CORE_STATE.get() {
        Some(state) => CoreStatus {
            loaded: state.core.is_some(),
            version: state.core.as_ref().map(|c| c.version().to_string()),
            dll_path: state.core.as_ref().map(|c| c.path().display().to_string()),
            error: state.error.clone(),
            searched: state.searched.iter().map(|p| p.display().to_string()).collect(),
        },
        None => CoreStatus {
            loaded: false,
            version: None,
            dll_path: None,
            error: Some("BSD 核心尚未初始化".to_string()),
            searched: Vec::new(),
        },
    }
}

#[tauri::command]
fn start_download(app: AppHandle, registry: State<'_, Arc<Registry>>, job: DownloadJob) -> Result<StartedTask, String> {
    let core = core_ref()?;
    download::start(core, app, registry.inner().clone(), job)
}

#[tauri::command]
fn cancel_download(registry: State<'_, Arc<Registry>>, id: String) -> Result<(), String> {
    let core = core_ref()?;
    if registry.cancel(&id, &core) {
        Ok(())
    } else {
        Err("任务不存在或已经结束".to_string())
    }
}

#[tauri::command]
fn set_log_level(level: i32) -> Result<(), String> {
    let core = core_ref()?;
    core.set_log_level(level.clamp(ffi::LOG_TRACE, ffi::LOG_OFF));
    Ok(())
}

#[tauri::command]
fn default_download_dir() -> String {
    download::default_dir().to_string_lossy().into_owned()
}

#[cfg(windows)]
fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    cmd
}

#[cfg(not(windows))]
fn hidden_command(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(path.trim());
    if p.as_os_str().is_empty() {
        return Err("路径为空".to_string());
    }
    if !p.exists() {
        return Err(format!("路径不存在：{}", p.display()));
    }
    let p_pare = p.parent().unwrap().to_path_buf();
    hidden_command("explorer")
        .arg(&p_pare)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("无法打开资源管理器：{err}"))
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 直接读取 Windows 剪贴板中的文本：WebView 里的 navigator.clipboard 在部分环境
/// 需要额外授权，这里提供一个无依赖的原生兜底。
#[cfg(windows)]
#[tauri::command]
fn read_clipboard() -> Result<String, String> {
    use std::os::windows::ffi::OsStringExt;

    const CF_UNICODETEXT: u32 = 13;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenClipboard(hwnd: *mut c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn GetClipboardData(format: u32) -> *mut c_void;
        fn IsClipboardFormatAvailable(format: u32) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalLock(mem: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(mem: *mut c_void) -> i32;
    }

    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            return Ok(String::new());
        }
        let mut opened = false;
        for _ in 0..5 {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if !opened {
            return Err("剪贴板正被其他程序占用，请稍后重试".to_string());
        }

        let result = (|| -> Result<String, String> {
            let handle = GetClipboardData(CF_UNICODETEXT);
            if handle.is_null() {
                return Ok(String::new());
            }
            let ptr = GlobalLock(handle) as *const u16;
            if ptr.is_null() {
                return Err("无法读取剪贴板内容".to_string());
            }
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let text = std::ffi::OsString::from_wide(std::slice::from_raw_parts(ptr, len))
                .to_string_lossy()
                .into_owned();
            GlobalUnlock(handle);
            Ok(text)
        })();

        CloseClipboard();
        result
    }
}

#[cfg(not(windows))]
#[tauri::command]
fn read_clipboard() -> Result<String, String> {
    Err("当前平台不支持读取剪贴板".to_string())
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode_utf16_base64(text: &str) -> String {
    let bytes: Vec<u8> = text.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[tauri::command]
fn pick_folder(default_path: Option<String>) -> Result<Option<String>, String> {
    let initial = default_path.unwrap_or_default().replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference = 'Stop'\n\
         $ProgressPreference = 'SilentlyContinue'\n\
         [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)\n\
         $OutputEncoding = [Console]::OutputEncoding\n\
         Add-Type -AssemblyName System.Windows.Forms | Out-Null\n\
         $owner = New-Object System.Windows.Forms.Form\n\
         $owner.TopMost = $true\n\
         $owner.ShowInTaskbar = $false\n\
         $owner.Opacity = 0\n\
         $owner.StartPosition = 'CenterScreen'\n\
         $dialog = New-Object System.Windows.Forms.FolderBrowserDialog\n\
         $dialog.Description = '选择下载保存目录'\n\
         $dialog.ShowNewFolderButton = $true\n\
         if ('{initial}' -ne '') {{ $dialog.SelectedPath = '{initial}' }}\n\
         $result = $dialog.ShowDialog($owner)\n\
         if ($result -eq [System.Windows.Forms.DialogResult]::OK) {{ [Console]::Out.Write($dialog.SelectedPath) }}\n\
         $dialog.Dispose()\n\
         $owner.Dispose()"
    );
    let encoded = encode_utf16_base64(&script);
    let output = hidden_command("powershell")
        .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-EncodedCommand", &encoded])
        .output()
        .map_err(|err| format!("无法启动目录选择器：{err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        if detail.is_empty() {
            return Err(format!("目录选择器退出码 {}", output.status));
        }
        return Err(format!("目录选择器出错：{detail}"));
    }
    let picked = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if picked.is_empty() { None } else { Some(picked) })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let _ = LOG_APP.set(handle.clone());

            let searched = candidate_paths(&handle);
            let state = match Core::load(&searched) {
                Ok(core) => {
                    core.set_log_console(false);
                    core.set_log_callback(log_cb);
                    core.set_log_level(ffi::LOG_INFO);
                    CoreState { core: Some(Arc::new(core)), error: None, searched }
                }
                Err(err) => CoreState { core: None, error: Some(err), searched },
            };
            let loaded = state.core.is_some();
            let _ = CORE_STATE.set(state);

            app.manage(Arc::new(Registry::default()));

            if !loaded {
                let _ = handle.emit("bsd://core-error", ());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core_status,
            start_download,
            cancel_download,
            set_log_level,
            default_download_dir,
            open_folder,
            pick_folder,
            app_version,
            read_clipboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
