pub struct KeepAwake {
    #[cfg(target_os = "macos")]
    process: Option<std::process::Child>,
    #[cfg(windows)]
    stop: Option<std::sync::mpsc::Sender<()>>,
}

impl KeepAwake {
    pub fn new() -> Self {
        #[cfg(windows)]
        let (stop, receive) = std::sync::mpsc::channel();
        #[cfg(windows)]
        std::thread::spawn(move || unsafe {
            use windows_sys::Win32::System::Power::{
                SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
            };
            // This API is thread-local; acquire and release on the same dedicated thread.
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
            let _ = receive.recv();
            SetThreadExecutionState(ES_CONTINUOUS);
        });
        Self {
            #[cfg(windows)]
            stop: Some(stop),
            #[cfg(target_os = "macos")]
            process: std::process::Command::new("/usr/bin/caffeinate")
                .args(["-i", "-w", &std::process::id().to_string()])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .ok(),
        }
    }
}

impl Default for KeepAwake {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        #[cfg(windows)]
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
