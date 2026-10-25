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
    let windows10 = version
        .as_ref()
        .is_some_and(|o| is_windows_10(&String::from_utf8_lossy(&o.stdout)));
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

#[cfg(any(windows, test))]
fn is_windows_10(version: &str) -> bool {
    version
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter_map(|v| v.strip_prefix("10.0."))
        .filter_map(|v| v.split('.').next())
        .filter_map(|v| v.parse::<u32>().ok())
        .any(|build| build < 22000)
}

#[cfg(test)]
mod tests {
    #[test]
    fn windows_10_builds_with_update_revisions_need_runtime_permissions() {
        for output in [
            "Microsoft Windows [Version 10.0.19045.6456]\r\n",
            "Microsoft Windows [Version 10.0.17763.0]",
            "Microsoft Windows [Version 10.0.19045]",
        ] {
            assert!(super::is_windows_10(output), "{output}");
        }
    }

    #[test]
    fn windows_11_older_windows_and_invalid_output_do_not_match() {
        for output in [
            "Microsoft Windows [Version 10.0.22000.0]",
            "Microsoft Windows [Version 10.0.26100.1]",
            "Microsoft Windows [Version 6.3.9600]",
            "Microsoft Windows [Version 10.0.invalid]",
            "",
        ] {
            assert!(!super::is_windows_10(output), "{output}");
        }
    }
}
