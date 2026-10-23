use super::*;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
async fn connection<S: AsyncRead + AsyncWrite + Unpin>(mut socket: S, service: Arc<Service>) {
    let response = async {
        let size = socket.read_u32_le().await? as usize;
        if size > 1024 * 1024 {
            return Err(message("IPC message exceeds 1 MiB"));
        }
        let mut bytes = vec![0; size];
        socket.read_exact(&mut bytes).await?;
        let value = dispatch(service, serde_json::from_slice(&bytes)?).await?;
        Ok::<_, music_core::error::Error>(json!({"ok":true,"data":value}))
    };
    let result = match tokio::time::timeout(std::time::Duration::from_secs(120), response).await {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => json!({"ok":false,"error":e.to_string()}),
        Err(_) => json!({"ok":false,"error":"IPC timeout"}),
    };
    if let Ok(bytes) = serde_json::to_vec(&result) {
        let _ = socket.write_u32_le(bytes.len() as u32).await;
        let _ = socket.write_all(&bytes).await;
    }
}
#[cfg(unix)]
pub async fn serve(service: Arc<Service>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let path = service.root.join("service.sock");
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let listener = tokio::net::UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    loop {
        tokio::select! {
            accepted=listener.accept()=>{let (socket,_)=accepted?;tokio::spawn(connection(socket,service.clone()));}
            _=tokio::time::sleep(std::time::Duration::from_secs(30))=>{
                let busy=service.jobs.lock().await.values().any(|j|["running","queued"].contains(&j.value["status"].as_str().unwrap_or("")));
                if !busy && seconds().saturating_sub(service.last_request.load(std::sync::atomic::Ordering::Relaxed))>300 {break;}
            }
        }
    }
    std::fs::remove_file(path)?;
    Ok(())
}
#[cfg(windows)]
pub async fn serve(service: Arc<Service>) -> Result<()> {
    let mut pipe = windows_pipe::create(&service.root, true)?;
    loop {
        pipe.connect().await?;
        let next = windows_pipe::create(&service.root, false)?;
        tokio::spawn(connection(pipe, service.clone()));
        pipe = next;
    }
}
