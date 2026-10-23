use std::{
    io::Read,
    process::{Command, Stdio},
    time::Duration,
};
pub fn supervise() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--engine-supervisor")) {
        return None;
    }
    let Some(engine) = args.next() else {
        return Some(2);
    };
    let mut command = Command::new(engine);
    command.args(args).stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Engine launch: {e}");
            return Some(1);
        }
    };
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = std::io::stdin().read(&mut [0u8; 1]);
        let _ = send.send(());
    });
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return Some(status.code().unwrap_or(1));
        }
        if receive.recv_timeout(Duration::from_millis(100)).is_ok() {
            let _ = child.kill();
            let _ = child.wait();
            return Some(0);
        }
    }
}
