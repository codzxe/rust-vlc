//! Shared helpers for the 2.A1 integration tests: unique temp files
//! cleaned up on drop. (Std-only by C5: no `tempfile` crate.)
//!
//! `#[allow(dead_code)]` is the shared-module idiom: each integration
//! binary links this module whole while using only part of it; nothing
//! here hides a defect (every helper is exercised by at least one
//! binary in this directory).

#![allow(dead_code)]

use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique temp directory with one file inside, removed recursively on
/// drop.
pub struct TempFile {
    pub path: PathBuf,
}

impl TempFile {
    /// Create `…/lumen-2a1-<tag>-<pid>-<n>/source.bin` with `bytes`.
    pub fn create(tag: &str, bytes: &[u8]) -> io::Result<TempFile> {
        let unique = format!(
            "lumen-2a1-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("source.bin");
        std::fs::write(&path, bytes)?;
        Ok(TempFile { path })
    }

    /// The `file:` URI for the file. Temp paths are ASCII on every CI
    /// host; a non-UTF-8 path here is a broken fixture (testkit's
    /// `unreachable!`-based `must` vocabulary).
    pub fn file_uri(&self) -> String {
        match self.path.to_str() {
            Some(text) => format!("file://{text}"),
            None => unreachable!("temp paths are UTF-8 on CI hosts"),
        }
    }

    /// Truncate the file to `len` bytes (the truncation fault).
    pub fn truncate_to(&self, len: u64) -> io::Result<()> {
        let handle = std::fs::OpenOptions::new().write(true).open(&self.path)?;
        handle.set_len(len)
    }

    /// Append `bytes` to the file (the growth case).
    pub fn append(&self, bytes: &[u8]) -> io::Result<()> {
        use std::io::Write;
        let mut handle = std::fs::OpenOptions::new().append(true).open(&self.path)?;
        handle.write_all(bytes)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// A deterministic byte pattern (period 251: every offset distinguishable).
pub fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// A unique temp DIRECTORY (no file inside).
pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn create(tag: &str) -> io::Result<TempDir> {
        let unique = format!(
            "lumen-2a1-dir-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path)?;
        Ok(TempDir { path })
    }

    pub fn file_uri(&self) -> String {
        match self.path.to_str() {
            Some(text) => format!("file://{text}"),
            None => unreachable!("temp paths are UTF-8 on CI hosts"),
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// True when the process effectively runs as root (user namespaces with
/// uid 0 mapped): the permission-denial test cannot assert denial there.
pub fn running_privileged() -> bool {
    std::fs::read_to_string("/proc/self/uid_map")
        .ok()
        .and_then(|text| {
            text.lines().next().and_then(|first| {
                first
                    .split_whitespace()
                    .nth(2)
                    .and_then(|count| count.parse::<u32>().ok())
                    .map(|count| count == 0)
            })
        })
        .unwrap_or(false)
}
