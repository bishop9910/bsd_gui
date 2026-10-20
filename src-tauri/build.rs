use std::path::PathBuf;

/// 把 binaries/ 下的 BSD 运行库复制到可执行文件旁边（target/debug、target/release），
/// 这样开发运行时 Windows 也能直接找到 bsd_core.dll 及其依赖。
fn copy_runtime_dlls() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let bin_dir = manifest.join("binaries");
    println!("cargo:rerun-if-changed=binaries");

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default());
    // OUT_DIR = <target>/<profile>/build/<pkg>-<hash>/out
    let Some(profile_dir) = out.ancestors().nth(3) else {
        return;
    };

    let Ok(entries) = std::fs::read_dir(&bin_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let src = entry.path();
        let is_dll = src
            .extension()
            .map(|ext| ext.eq_ignore_ascii_case("dll"))
            .unwrap_or(false);
        if !is_dll {
            continue;
        }
        let Some(name) = src.file_name() else { continue };
        let dest = profile_dir.join(name);
        let stale = match (std::fs::metadata(&dest), std::fs::metadata(&src)) {
            (Ok(old), Ok(new)) => {
                old.len() != new.len() || old.modified().ok() < new.modified().ok()
            }
            _ => true,
        };
        if stale {
            let _ = std::fs::copy(&src, &dest);
        }
    }
}

fn main() {
    copy_runtime_dlls();
    tauri_build::build()
}
