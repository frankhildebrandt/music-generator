use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

/// Read only this synthesis's recent log tail; old inference steps never leak across jobs.
pub fn synthesis_step(path: &Path, offset: u64) -> Option<(u32, u32)> {
    let mut file = File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    if size < offset {
        return None;
    }
    file.seek(SeekFrom::Start(offset.max(size.saturating_sub(65536))))
        .ok()?;
    let mut text = String::new();
    file.take(65536).read_to_string(&mut text).ok()?;
    parse_step(&text)
}

fn parse_step(text: &str) -> Option<(u32, u32)> {
    text.lines().rev().find_map(|line| {
        let counts = line
            .split_once("[DiT] Step ")?
            .1
            .split_whitespace()
            .next()?;
        let (current, total) = counts.split_once('/')?;
        let (current, total) = (current.parse::<u32>().ok()?, total.parse::<u32>().ok()?);
        (total > 0 && current <= total).then_some((current, total))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_real_steps_only_for_the_current_generation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("engine.log");
        std::fs::write(&path, "[DiT] Step 50/50 t=0.02\n").unwrap();
        let offset = path.metadata().unwrap().len();
        assert_eq!(synthesis_step(&path, offset), None);
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(file, "[DiT] Step 7/50 t=0.86").unwrap();
        assert_eq!(synthesis_step(&path, offset), Some((7, 50)));
        assert_eq!(parse_step("[DiT] Step 2/0"), None);
        assert_eq!(parse_step("[DiT] Step 51/50"), None);
        assert_eq!(parse_step("loading XL"), None);
    }
}
