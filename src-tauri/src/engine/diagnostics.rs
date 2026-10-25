use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

// OwnedProcess merges stdout and stderr into ace-server.log on every platform.
// Capture per synthesis, never attribute an earlier job's memory error to this one.
pub(super) struct SynthesisLog {
    path: PathBuf,
    offset: u64,
}

impl SynthesisLog {
    pub fn capture(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            offset: path.metadata().map(|m| m.len()).unwrap_or(0),
        }
    }

    pub fn hint(&self, error: &str) -> Option<&'static str> {
        let mut bytes = Vec::new();
        if let Ok(mut file) = File::open(&self.path) {
            let size = file.metadata().map(|m| m.len()).unwrap_or(0);
            let start = if size < self.offset { 0 } else { self.offset };
            if file.seek(SeekFrom::Start(start)).is_ok() {
                let _ = file.take(200_000).read_to_end(&mut bytes);
            }
        }
        let text = format!("{error}\n{}", String::from_utf8_lossy(&bytes)).to_lowercase();
        if text.contains("exceeds silence_latent") {
            Some("Der Song ist für die Klangerzeugung zu lang (über 10 Minuten). Bitte eine kürzere Länge wählen. / The song is too long for synthesis (over 10 minutes). Please choose a shorter length.")
        } else if ["out of memory", "failed to allocate", "outofdevicememory"]
            .iter()
            .any(|s| text.contains(s))
        {
            Some("Der Speicher reicht für diese Songlänge nicht. Bitte einen kürzeren Song oder ein kleineres Modell wählen. Intel-Macs rechnen bereits auf der CPU. / Not enough memory for this song length. Choose a shorter song or a smaller model. Intel Macs already use the CPU.")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn only_new_synthesis_lines_explain_failures() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ace-server.log");
        std::fs::write(&path, "old: out of memory\n").unwrap();
        let mark = SynthesisLog::capture(&path);
        assert_eq!(mark.hint("unrelated failure"), None);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(file, "new: vk::OutOfDeviceMemory").unwrap();
        assert!(mark.hint("failed").unwrap().contains("Not enough memory"));
        let next = SynthesisLog::capture(&path);
        assert_eq!(next.hint("unrelated failure"), None);
        writeln!(file, "new: exceeds silence_latent").unwrap();
        assert!(next.hint("failed").unwrap().contains("over 10 minutes"));
        assert!(mark.hint("failed").unwrap().contains("over 10 minutes"));
    }

    #[test]
    fn missing_and_truncated_logs_and_direct_engine_errors_are_handled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ace-server.log");
        let mark = SynthesisLog::capture(&path);
        assert_eq!(mark.hint("unknown error"), None);
        std::fs::write(&path, "failed to allocate\n").unwrap();
        assert!(mark.hint("failed").is_some());
        let mark = SynthesisLog::capture(&path);
        std::fs::write(&path, "out of memory").unwrap();
        assert!(mark.hint("failed").is_some());
        std::fs::remove_file(&path).unwrap();
        assert!(mark
            .hint("Engine HTTP 500: exceeds silence_latent")
            .is_some());
        assert_eq!(mark.hint("generic failure"), None);
    }
}
