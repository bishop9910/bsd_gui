//! bsd_core.dll 的最小动态加载层。
//!
//! 这里不用 `libloading`，而是直接调用 kernel32 的 `LoadLibraryExW` /
//! `GetProcAddress`，避免给项目新增任何 crate 依赖。
//! 加载时使用 `LOAD_WITH_ALTERED_SEARCH_PATH`，让 bsd_core.dll 的依赖
//! （libcurl / libcrypto / libssl / fmt / z）从它所在目录解析。

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::{Path, PathBuf};

// ---- 与 include/bsd/bsd_core.h 保持一致的状态码 ----
pub const BSD_OK: c_int = 0;
pub const BSD_ERROR: c_int = 1;
pub const BSD_ERROR_RETRYABLE: c_int = 2;
pub const BSD_ERROR_CANCELLED: c_int = 3;
pub const BSD_ERROR_INVALID_ARG: c_int = 4;

pub const LOG_TRACE: c_int = 0;
#[allow(dead_code)]
pub const LOG_DEBUG: c_int = 1;
pub const LOG_INFO: c_int = 2;
#[allow(dead_code)]
pub const LOG_WARN: c_int = 3;
#[allow(dead_code)]
pub const LOG_ERROR: c_int = 4;
pub const LOG_OFF: c_int = 5;

pub type ProgressFn =
    extern "C" fn(user: *mut c_void, downloaded: u64, total: u64, bytes_per_second: f64, finished: c_int);
pub type LogFn = extern "C" fn(user: *mut c_void, level: c_int, message: *const c_char);

#[repr(C)]
pub struct BsdOptions {
    pub struct_size: usize,
    pub dir: *const c_char,
    pub threads: c_int,
    pub retry: c_int,
    pub timeout_seconds: c_int,
    pub continue_download: c_int,
    pub limit_rate_bytes_per_second: u64,
    pub proxy: *const c_char,
    pub user_agent: *const c_char,
    pub headers: *const *const c_char,
    pub header_count: usize,
}

#[repr(C)]
pub struct BsdRequest {
    pub struct_size: usize,
    pub url: *const c_char,
    pub output: *const c_char,
    pub checksum: *const c_char,
}

#[cfg(windows)]
mod sys {
    use std::ffi::c_void;

    pub const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x0000_0008;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn LoadLibraryExW(lib_file_name: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
        pub fn GetProcAddress(module: *mut c_void, proc_name: *const u8) -> *mut c_void;
        pub fn FreeLibrary(module: *mut c_void) -> i32;
        pub fn GetLastError() -> u32;
    }
}

struct Funcs {
    version: unsafe extern "C" fn() -> *const c_char,
    init: unsafe extern "C" fn() -> c_int,
    handle_create: unsafe extern "C" fn(*const BsdOptions) -> *mut c_void,
    handle_destroy: unsafe extern "C" fn(*mut c_void),
    handle_cancel: unsafe extern "C" fn(*mut c_void),
    download: unsafe extern "C" fn(
        *mut c_void,
        *const BsdRequest,
        Option<ProgressFn>,
        *mut c_void,
    ) -> c_int,
    last_error: unsafe extern "C" fn() -> *const c_char,
    set_log_callback: unsafe extern "C" fn(Option<LogFn>, *mut c_void),
    set_log_level: unsafe extern "C" fn(c_int),
    set_log_console: unsafe extern "C" fn(c_int),
}

pub struct Core {
    module: *mut c_void,
    path: PathBuf,
    version: String,
    funcs: Funcs,
}

// 模块句柄与函数指针在进程生命周期内保持有效，可以跨线程共享。
unsafe impl Send for Core {}
unsafe impl Sync for Core {}

#[cfg(windows)]
unsafe fn open_module(path: &Path) -> Result<*mut c_void, String> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let module = unsafe { sys::LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), sys::LOAD_WITH_ALTERED_SEARCH_PATH) };
    if module.is_null() {
        let code = unsafe { sys::GetLastError() };
        let detail = std::io::Error::from_raw_os_error(code as i32);
        return Err(format!("LoadLibraryExW 失败（错误 {code}）：{detail}"));
    }
    Ok(module)
}

#[cfg(windows)]
unsafe fn sym<T: Copy>(module: *mut c_void, name: &str) -> Result<T, String> {
    let cname = CString::new(name).map_err(|_| format!("非法符号名 {name}"))?;
    let proc = unsafe { sys::GetProcAddress(module, cname.as_bytes().as_ptr()) };
    if proc.is_null() {
        return Err(format!("bsd_core.dll 缺少导出符号 {name}"));
    }
    // 所有 T 都是函数指针，与 *mut c_void 同宽。
    Ok(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&proc) })
}

impl Core {
    /// 依次尝试候选路径，返回第一个成功加载并完成 `bsd_init` 的核心。
    pub fn load(candidates: &[PathBuf]) -> Result<Core, String> {
        #[cfg(not(windows))]
        {
            let _ = candidates;
            Err("BSD 核心目前仅提供 Windows 版本".to_string())
        }

        #[cfg(windows)]
        {
            let mut had_candidate = false;
            let mut errors: Vec<String> = Vec::new();
            for path in candidates {
                if !path.is_file() {
                    continue;
                }
                had_candidate = true;
                let module = match unsafe { open_module(path) } {
                    Ok(module) => module,
                    Err(err) => {
                        errors.push(format!("{}：{err}", path.display()));
                        continue;
                    }
                };
                match unsafe { Self::bind(module, path.clone()) } {
                    Ok(core) => return Ok(core),
                    Err(err) => {
                        errors.push(format!("{}：{err}", path.display()));
                        unsafe { sys::FreeLibrary(module) };
                    }
                }
            }

            if !had_candidate {
                Err(format!(
                    "未找到 bsd_core.dll，已检查以下位置：\n{}",
                    candidates
                        .iter()
                        .map(|p| format!("  - {}", p.display()))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))
            } else {
                Err(format!("加载 bsd_core.dll 失败：\n{}", errors.join("\n")))
            }
        }
    }

    #[cfg(windows)]
    unsafe fn bind(module: *mut c_void, path: PathBuf) -> Result<Core, String> {
        let funcs = unsafe {
            Funcs {
                version: sym(module, "bsd_version")?,
                init: sym(module, "bsd_init")?,
                handle_create: sym(module, "bsd_handle_create")?,
                handle_destroy: sym(module, "bsd_handle_destroy")?,
                handle_cancel: sym(module, "bsd_handle_cancel")?,
                download: sym(module, "bsd_download")?,
                last_error: sym(module, "bsd_last_error")?,
                set_log_callback: sym(module, "bsd_set_log_callback")?,
                set_log_level: sym(module, "bsd_set_log_level")?,
                set_log_console: sym(module, "bsd_set_log_console")?,
            }
        };

        let version_ptr = unsafe { (funcs.version)() };
        let version = if version_ptr.is_null() {
            "unknown".to_string()
        } else {
            unsafe { CStr::from_ptr(version_ptr) }.to_string_lossy().trim().to_string()
        };

        let init_rc = unsafe { (funcs.init)() };
        if init_rc != BSD_OK {
            return Err(format!("bsd_init() 返回错误码 {init_rc}"));
        }

        Ok(Core { module, path, version, funcs })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn create_handle(&self, options: &BsdOptions) -> Result<usize, String> {
        let handle = unsafe { (self.funcs.handle_create)(options) };
        if handle.is_null() {
            Err(self.last_error_or("创建下载句柄失败"))
        } else {
            Ok(handle as usize)
        }
    }

    pub fn destroy_handle(&self, handle: usize) {
        if handle != 0 {
            unsafe { (self.funcs.handle_destroy)(handle as *mut c_void) };
        }
    }

    pub fn cancel_handle(&self, handle: usize) {
        if handle != 0 {
            unsafe { (self.funcs.handle_cancel)(handle as *mut c_void) };
        }
    }

    /// 阻塞执行一次下载；返回状态码与（失败时的）错误文本。
    pub fn download(&self, handle: usize, request: &BsdRequest, cb: ProgressFn, user: *mut c_void) -> (c_int, Option<String>) {
        let rc = unsafe { (self.funcs.download)(handle as *mut c_void, request, Some(cb), user) };
        if rc == BSD_OK {
            (rc, None)
        } else {
            (rc, Some(self.last_error_or("下载失败（核心未提供错误详情）")))
        }
    }

    fn last_error(&self) -> String {
        let ptr = unsafe { (self.funcs.last_error)() };
        if ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(ptr) }.to_string_lossy().trim().to_string()
        }
    }

    fn last_error_or(&self, fallback: &str) -> String {
        let msg = self.last_error();
        if msg.is_empty() { fallback.to_string() } else { msg }
    }

    pub fn set_log_callback(&self, cb: LogFn) {
        unsafe { (self.funcs.set_log_callback)(Some(cb), std::ptr::null_mut()) };
    }

    pub fn set_log_level(&self, level: c_int) {
        unsafe { (self.funcs.set_log_level)(level) };
    }

    pub fn set_log_console(&self, enabled: bool) {
        unsafe { (self.funcs.set_log_console)(c_int::from(enabled)) };
    }

    #[allow(dead_code)]
    pub fn module_handle(&self) -> *mut c_void {
        self.module
    }
}
