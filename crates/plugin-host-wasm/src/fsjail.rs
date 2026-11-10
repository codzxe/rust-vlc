//! The `fs-sandboxed-read` path jail (doc 06 §2.2, MP §12 safe
//! paths).
//!
//! Read-only, prefix-based: a granted path resolves only if its
//! **canonical** form falls under one of the canonical jail roots.
//! `std::fs::canonicalize` resolves symlinks on every path
//! component, so a symlink pointing outside the root canonicalizes
//! outside the root and is denied — the §9.4
//! `plugin_sandbox_escape` symlink test.
//!
//! Known limit (honest, `[VERIFY]`): the canonicalize-then-open
//! sequence has a TOCTOU window — a local process swapping a
//! directory symlink between check and open can redirect the read.
//! Closing it needs `openat`/`O_NOFOLLOW` (a libc dependency — MP §9
//! forbids without an ADR). v1 accepts the window: roots are
//! user-granted media directories and the attacker model is the
//! *plugin* (which cannot create symlinks at all — it has no write
//! permission), not a hostile local user.

use std::path::{Path, PathBuf};

use crate::error::HostError;

/// A set of read-only jail roots.
#[derive(Debug, Clone)]
pub struct PathJail {
    roots: Vec<PathBuf>,
}

impl PathJail {
    /// Build a jail from user-granted roots; each root is
    /// canonicalized immediately (a root that does not exist yet is
    /// an error — honest refusal, not a silent no-op jail).
    ///
    /// # Errors
    /// [`HostError::Io`] naming the offending root when
    /// canonicalization fails.
    pub fn new<I, P>(roots: I) -> Result<PathJail, HostError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut canon = Vec::new();
        for root in roots {
            let canonical = std::fs::canonicalize(root.as_ref()).map_err(|e| {
                HostError::Io(format!(
                    "jail root {} cannot be canonicalized: {e}",
                    root.as_ref().display()
                ))
            })?;
            canon.push(canonical);
        }
        Ok(PathJail { roots: canon })
    }

    /// The canonical roots (for display and events).
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Resolve `path` to a canonical path that must live under a
    /// jail root. Relative paths try each root in order (first hit
    /// wins); absolute paths must already be under a root.
    ///
    /// # Errors
    /// [`HostError::PermissionDenied`] when the path escapes the
    /// jail (symlink or otherwise), does not exist, or no root is
    /// granted; the message carries shapes, not file contents.
    pub fn resolve(&self, path: &str) -> Result<PathBuf, HostError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(HostError::PermissionDenied {
                detail: "empty path".to_string(),
            });
        }
        // Length guard before touching the filesystem (dirty-world
        // inputs; D-2 bounded memory).
        if trimmed.len() > 4096 {
            return Err(HostError::PermissionDenied {
                detail: "path longer than 4096 bytes".to_string(),
            });
        }
        if self.roots.is_empty() {
            return Err(HostError::PermissionDenied {
                detail: "no sandbox roots granted (fs-sandboxed-read has \
                         no user-granted directories)"
                    .to_string(),
            });
        }
        let requested = Path::new(trimmed);
        let candidates: Vec<PathBuf> = if requested.is_absolute() {
            vec![requested.to_path_buf()]
        } else {
            self.roots.iter().map(|r| r.join(requested)).collect()
        };
        for candidate in candidates {
            // Canonicalize resolves ALL symlink components; a link
            // pointing outside the root lands outside and fails the
            // prefix check below.
            match std::fs::canonicalize(&candidate) {
                Ok(canonical) => {
                    if self.under_root(&canonical) {
                        return Ok(canonical);
                    }
                    // Escaped via symlink: fall through to denial with
                    // the shape of the attempt.
                    return Err(HostError::PermissionDenied {
                        detail: format!(
                            "path resolves outside the sandbox roots \
                             (resolved prefix {})",
                            shape(&canonical)
                        ),
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    return Err(HostError::PermissionDenied {
                        detail: format!("path cannot be resolved: {e}"),
                    });
                }
            }
        }
        Err(HostError::PermissionDenied {
            detail: format!("path `{}` not found inside the sandbox roots", shape(requested)),
        })
    }

    /// Prefix check against every canonical root. Component-aware
    /// (`starts_with` on `Path` compares components, not bytes, so
    /// `/root-evil` does not pass a `/root` jail).
    fn under_root(&self, canonical: &Path) -> bool {
        self.roots.iter().any(|root| canonical.starts_with(root))
    }
}

/// A path shape for events: the first two components plus an
/// ellipsis (doc 06 §5.3: never raw buffers, never full paths that
/// could carry secrets).
fn shape(path: &Path) -> String {
    let comps: Vec<String> = path
        .components()
        .take(3)
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    if comps.is_empty() {
        "…".to_string()
    } else {
        format!("{}…", comps.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// House test helper (media-probe precedent).
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    /// Unique temp dir helper (std-only, self-cleaning).
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let dir = std::env::temp_dir().join(format!(
                "lumen-pjail-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            ok(std::fs::create_dir_all(&dir)
                .map_err(|e| format!("test temp dir {}: {e}", dir.display())));
            TempDir(dir)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_file(path: &Path, bytes: &[u8]) {
        let mut f = ok(std::fs::File::create(path)
            .map_err(|e| format!("create {}: {e}", path.display())));
        ok(f.write_all(bytes)
            .map_err(|e| format!("write {}: {e}", path.display())));
    }

    #[test]
    fn inside_reads_resolve() {
        let tmp = TempDir::new("inside");
        let media = tmp.path().join("media");
        ok(std::fs::create_dir_all(&media).map_err(|e| e.to_string()));
        write_file(&media.join("a.lrc"), b"lyrics");
        let jail = ok(PathJail::new([&media]));
        // Relative and absolute both work.
        assert!(jail.resolve("a.lrc").is_ok());
        let abs = ok(jail.resolve(&media.join("a.lrc").to_string_lossy()));
        assert!(abs.ends_with("a.lrc"));
    }

    #[test]
    fn symlink_escape_is_denied() {
        let tmp = TempDir::new("symlink");
        let media = tmp.path().join("media");
        let secret = tmp.path().join("secret");
        ok(std::fs::create_dir_all(&media).map_err(|e| e.to_string()));
        ok(std::fs::create_dir_all(&secret).map_err(|e| e.to_string()));
        write_file(&secret.join("secret.txt"), b"no");
        #[cfg(unix)]
        ok(std::os::unix::fs::symlink(&secret, media.join("escape"))
            .map_err(|e| e.to_string()));
        #[cfg(windows)]
        {
            // Junction-free environments: skip the symlink fixture,
            // the prefix logic is covered by the sibling test.
            ok(std::fs::copy(secret.join("secret.txt"), media.join("escape.txt"))
                .map_err(|e| e.to_string()));
        }
        let jail = ok(PathJail::new([&media]));
        #[cfg(unix)]
        {
            let Err(err) = jail.resolve("escape/secret.txt") else {
                unreachable!("symlink escape must be denied")
            };
            assert!(
                matches!(err, HostError::PermissionDenied { .. }),
                "symlink escape must be a typed denial"
            );
        }
        // Parent traversal is equally refused (canonicalize of
        // ../secret lands outside).
        let Err(err) = jail.resolve("../secret/secret.txt") else {
            unreachable!("parent traversal must be denied")
        };
        assert!(matches!(err, HostError::PermissionDenied { .. }));
    }

    #[test]
    fn component_aware_prefix() {
        let tmp = TempDir::new("prefix");
        let root = tmp.path().join("root");
        let sibling = tmp.path().join("root-evil");
        ok(std::fs::create_dir_all(&root).map_err(|e| e.to_string()));
        ok(std::fs::create_dir_all(&sibling).map_err(|e| e.to_string()));
        write_file(&sibling.join("x.txt"), b"no");
        let jail = ok(PathJail::new([&root]));
        let Err(err) = jail
            .resolve(&sibling.join("x.txt").to_string_lossy())
        else {
            unreachable!("sibling-prefix path must be denied")
        };
        assert!(matches!(err, HostError::PermissionDenied { .. }));
    }

    #[test]
    fn empty_jail_refuses_everything() {
        let jail = ok(PathJail::new(Vec::<PathBuf>::new()));
        assert!(matches!(
            jail.resolve("anything"),
            Err(HostError::PermissionDenied { .. })
        ));
    }

    #[test]
    fn nonexistent_root_is_a_typed_error() {
        let Err(err) = PathJail::new(["/definitely/not/a/lumen/jail/root"]) else {
            unreachable!("nonexistent root must error")
        };
        assert!(matches!(err, HostError::Io(_)));
    }
}
