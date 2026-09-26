//! Reading the files the tools leave on disk.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Bytes read at a time from the end of a file.
const BLOCK: u64 = 256 * 1024;
/// How far back a search goes before giving up.
const SEARCH_LIMIT: u64 = 64 * 1024 * 1024;

/// The last line of `path` that contains `needle` and that `keep` accepts,
/// read from the end, so a long session log costs only its tail.
pub fn last_line(path: &Path, needle: &str, keep: impl Fn(&str) -> bool) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut end = file.metadata().ok()?.len();
    // The start of a line cut by the previous block.
    let mut rest: Vec<u8> = Vec::new();
    let mut searched = 0;
    loop {
        let start = end.saturating_sub(BLOCK);
        let mut block = vec![0; (end - start) as usize];
        file.seek(SeekFrom::Start(start)).ok()?;
        file.read_exact(&mut block).ok()?;
        block.extend_from_slice(&rest);
        let mut lines = block.split(|&byte| byte == b'\n');
        // Unless the block starts the file, its first line may be cut short.
        let first = if start > 0 { lines.next() } else { None };
        let found = lines.rev().find_map(|line| {
            let line = std::str::from_utf8(line).ok()?;
            (line.contains(needle) && keep(line)).then(|| line.to_owned())
        });
        if found.is_some() {
            return found;
        }
        rest = first?.to_vec();
        searched += end - start;
        end = start;
        if searched >= SEARCH_LIMIT {
            return None;
        }
    }
}

/// When a file last changed.
pub fn modified(path: &Path) -> Option<SystemTime> {
    path.metadata().ok()?.modified().ok()
}

/// Files in `dir` named `prefix…suffix`, with when each last changed.
pub fn listed(dir: &Path, prefix: &str, suffix: &str) -> Vec<(PathBuf, SystemTime)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with(prefix) && name.ends_with(suffix)
        })
        .filter_map(|entry| {
            let path = entry.path();
            let changed = modified(&path)?;
            Some((path, changed))
        })
        .collect()
}

/// The user's home folder.
pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn scratch(name: &str, text: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("vitals-{}-{name}", std::process::id()));
        File::create(&path)
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
        path
    }

    #[test]
    fn finds_the_last_matching_line() {
        let path = scratch("lines", "a 1\nb 2\na 3\nc 4\n");
        assert_eq!(last_line(&path, "a ", |_| true).as_deref(), Some("a 3"));
        assert_eq!(
            last_line(&path, "a ", |line| line.ends_with('1')).as_deref(),
            Some("a 1")
        );
        assert_eq!(last_line(&path, "z", |_| true), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn finds_lines_across_blocks() {
        // A long line that crosses the edge between two blocks, then filler.
        let long = format!("wanted {}", "x".repeat(BLOCK as usize));
        let filler = "y\n".repeat(BLOCK as usize / 4);
        let path = scratch("blocks", &format!("first\n{long}\n{filler}"));
        assert_eq!(last_line(&path, "wanted", |_| true), Some(long));
        assert_eq!(
            last_line(&path, "first", |_| true).as_deref(),
            Some("first")
        );
        let _ = std::fs::remove_file(path);
    }
}
