#[cfg(windows)]
pub fn configure_runtime() {
    use std::{os::windows::process::CommandExt, process::Command};
    let Some(parent) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
    else {
        return;
    };
    let runtime = parent.join("WebView2");
    if !runtime.is_dir() {
        return;
    }
    // Microsoft's Fixed Version 120+ runtime needs these read/execute ACLs on Windows 10.
    // Only the public browser binaries are exposed; no app data or credentials are included.
    let version = Command::new("cmd.exe")
        .args(["/C", "ver"])
        .creation_flags(0x08000000)
        .output()
        .ok();
    let windows10 = version.as_ref().is_some_and(|o| {
        String::from_utf8_lossy(&o.stdout)
            .split(|c: char| !c.is_ascii_digit() && c != '.')
            .filter_map(|v| v.strip_prefix("10.0."))
            .filter_map(|v| v.parse::<u32>().ok())
            .any(|build| build < 22000)
    });
    if windows10 {
        for identity in ["*S-1-15-2-2:(OI)(CI)(RX)", "*S-1-15-2-1:(OI)(CI)(RX)"] {
            let _ = Command::new("icacls.exe")
                .arg(&runtime)
                .args(["/grant", identity, "/Q"])
                .creation_flags(0x08000000)
                .output();
        }
    }
    // Called before creating Tauri threads or webviews.
    std::env::set_var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", runtime);
}
