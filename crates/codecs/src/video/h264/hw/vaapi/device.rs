//! DRM device-node discovery for the VA-API lane (card 7.B1) —
//! pure std, no `unsafe`, no ioctls.
//!
//! VA-API needs a DRM render node (`/dev/dri/renderD*`, or a card
//! node on legacy drivers) to construct its display
//! (`vaGetDisplayDRM(fd)`). Discovery answers *which node to open*
//! before any library is touched — on this sandbox the answer is
//! "none" (`/dev/dri` absent), which is exactly the typed
//! [`NoDeviceNode`](super::super::capability::HwUnavailableReason::NoDeviceNode)
//! path the tests pin.
//!
//! The scanner mirrors `platform-linux`'s sysfs DRM scanner design
//! (injectable roots → deterministic fixture tests; the layering
//! wall keeps this L3 crate from importing that L5 code).

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

/// One discoverable DRM node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriNode {
    /// The node's kernel name (`renderD128`, `card0`).
    pub name: String,
    /// Preferred over card nodes: render nodes exist exactly for
    /// non-display compute/decode clients (VA-API's first choice).
    pub render: bool,
}

/// Where discovery stopped, typed (never a bare io error).
#[derive(Debug)]
pub enum DriDiscovery {
    /// At least one node exists (render nodes first).
    Found(Vec<DriNode>),
    /// The `/dev/dri` directory does not exist — no GPU stack at
    /// all (the sandbox reality: the typed "no hardware" path).
    NoDriDirectory,
    /// `/dev/dri` exists but holds no `renderD*`/`card*` entries.
    Empty,
    /// The directory could not be read (permission or I/O error —
    /// carried verbatim for the report).
    Unreadable(String),
}

/// A `/dev/dri` scanner with injectable roots (the test seam).
#[derive(Debug, Clone)]
pub struct DriScanner {
    root: PathBuf,
}

impl DriScanner {
    /// Scan the real system (`/dev`).
    #[must_use = "constructs the scanner"]
    pub fn new() -> DriScanner {
        DriScanner {
            root: PathBuf::from("/"),
        }
    }

    /// Scan a fixture tree: `root` plays the role of `/`
    /// (`root/dev/dri/...`).
    #[must_use = "constructs the fixture-driven scanner"]
    pub fn with_root(root: &Path) -> DriScanner {
        DriScanner {
            root: root.to_path_buf(),
        }
    }

    /// The devfs root being scanned (`<root>/dev/dri`).
    #[must_use = "pure accessor"]
    pub fn dri_dir(&self) -> PathBuf {
        self.root.join("dev/dri")
    }

    /// Enumerate the DRM nodes (render nodes first, then cards,
    /// each group name-sorted for determinism).
    pub fn discover(&self) -> DriDiscovery {
        let dir = self.dri_dir();
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return DriDiscovery::NoDriDirectory,
            Err(e) => return DriDiscovery::Unreadable(format!("{dir:?}: {e}")),
        };
        let mut render: Vec<DriNode> = Vec::new();
        let mut cards: Vec<DriNode> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("renderD")
                && name.len() > 7
                && name[7..].chars().all(|c| c.is_ascii_digit())
            {
                render.push(DriNode { name, render: true });
            } else if name.starts_with("card")
                && name.len() > 4
                && name[4..].chars().all(|c| c.is_ascii_digit())
            {
                cards.push(DriNode {
                    name,
                    render: false,
                });
            }
            // Everything else (by-path symlinks, `version`) is not a
            // usable node — ignored, not guessed at.
        }
        if render.is_empty() && cards.is_empty() {
            return DriDiscovery::Empty;
        }
        render.sort_by(|a, b| a.name.cmp(&b.name));
        cards.sort_by(|a, b| a.name.cmp(&b.name));
        render.extend(cards);
        DriDiscovery::Found(render)
    }

    /// Open a discovered node read-write (the access VA-API needs)
    /// under the scanner's root.
    ///
    /// # Errors
    /// The raw [`io::Error`] — the caller maps `PermissionDenied` /
    /// `NotFound` to the typed capability reasons verbatim.
    pub fn open_node(&self, node: &DriNode) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.dri_dir().join(&node.name))
    }
}

impl Default for DriScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A per-test temp directory under the system temp root (the
    /// platform-linux fixture pattern; no dev-dependencies by C5).
    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("lumen-codecs-hw-vaapi-tests")
            .join(format!("{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap_or_else(|e| unreachable!("{e}"));
        dir
    }

    #[test]
    fn discovers_render_nodes_first_then_cards() {
        let root = temp_root("dri-order");
        let dri = root.join("dev/dri");
        fs::create_dir_all(&dri).unwrap_or_else(|e| unreachable!("{e}"));
        for name in [
            "card1",
            "card0",
            "renderD129",
            "renderD128",
            "version",
            "by-path",
        ] {
            let path = dri.join(name);
            if name == "by-path" {
                fs::create_dir_all(&path).unwrap_or_else(|e| unreachable!("{e}"));
            } else {
                fs::write(&path, b"").unwrap_or_else(|e| unreachable!("{e}"));
            }
        }
        let scanner = DriScanner::with_root(&root);
        match scanner.discover() {
            DriDiscovery::Found(nodes) => {
                assert_eq!(
                    nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
                    ["renderD128", "renderD129", "card0", "card1"],
                    "render nodes first, name-sorted, scratch entries ignored"
                );
                assert!(nodes[0].render);
                assert!(!nodes[2].render);
            }
            other => unreachable!("must discover, got {other:?}"),
        }
    }

    #[test]
    fn missing_dri_directory_is_the_typed_no_hardware_path() {
        let root = temp_root("dri-none");
        // Nothing created under root/dev — the sandbox reality.
        let scanner = DriScanner::with_root(&root);
        assert_eq!(
            scanner.discover_debug_shape(),
            ("NoDriDirectory", 0),
            "absent /dev/dri is the typed NoDeviceNode path"
        );
    }

    #[test]
    fn empty_dri_directory_is_typed_empty() {
        let root = temp_root("dri-empty");
        fs::create_dir_all(root.join("dev/dri")).unwrap_or_else(|e| unreachable!("{e}"));
        let scanner = DriScanner::with_root(&root);
        assert_eq!(scanner.discover_debug_shape(), ("Empty", 0));
    }

    #[test]
    fn scratch_names_are_not_nodes() {
        let root = temp_root("dri-scratch");
        let dri = root.join("dev/dri");
        fs::create_dir_all(&dri).unwrap_or_else(|e| unreachable!("{e}"));
        for name in ["renderD", "card", "renderD12x", "card0x", "render"] {
            fs::write(dri.join(name), b"").unwrap_or_else(|e| unreachable!("{e}"));
        }
        let scanner = DriScanner::with_root(&root);
        assert_eq!(
            scanner.discover_debug_shape(),
            ("Empty", 0),
            "only renderD<digits>/card<digits> count"
        );
    }

    #[test]
    fn open_node_maps_permission_denied_verbatim() {
        // Skip under root (CI service accounts): chmod-000 denial is
        // only enforceable for unprivileged users.
        if nix_euid_is_root() {
            return;
        }
        let root = temp_root("dri-perm");
        let dri = root.join("dev/dri");
        fs::create_dir_all(&dri).unwrap_or_else(|e| unreachable!("{e}"));
        let node_path = dri.join("renderD128");
        fs::write(&node_path, b"").unwrap_or_else(|e| unreachable!("{e}"));
        fs::set_permissions(&node_path, fs::Permissions::from_mode(0o000))
            .unwrap_or_else(|e| unreachable!("{e}"));
        let scanner = DriScanner::with_root(&root);
        let node = match scanner.discover() {
            DriDiscovery::Found(mut nodes) => nodes.remove(0),
            other => unreachable!("must discover, got {other:?}"),
        };
        match scanner.open_node(&node) {
            Err(e) => assert_eq!(
                e.kind(),
                io::ErrorKind::PermissionDenied,
                "EACCES carried verbatim"
            ),
            Ok(_) => unreachable!("mode-000 node must not open"),
        }
    }

    #[test]
    fn open_node_succeeds_on_an_openable_fixture() {
        let root = temp_root("dri-open");
        let dri = root.join("dev/dri");
        fs::create_dir_all(&dri).unwrap_or_else(|e| unreachable!("{e}"));
        fs::write(dri.join("renderD128"), b"").unwrap_or_else(|e| unreachable!("{e}"));
        let scanner = DriScanner::with_root(&root);
        let node = match scanner.discover() {
            DriDiscovery::Found(mut nodes) => nodes.remove(0),
            other => unreachable!("must discover, got {other:?}"),
        };
        assert!(
            scanner.open_node(&node).is_ok(),
            "rw-openable fixture opens"
        );
    }

    /// Test-shape helper: (variant, node count) for the assertions
    /// above (the real API returns the full enum).
    impl DriScanner {
        fn discover_debug_shape(&self) -> (&'static str, usize) {
            match self.discover() {
                DriDiscovery::Found(nodes) => ("Found", nodes.len()),
                DriDiscovery::NoDriDirectory => ("NoDriDirectory", 0),
                DriDiscovery::Empty => ("Empty", 0),
                DriDiscovery::Unreadable(_) => ("Unreadable", 0),
            }
        }
    }

    fn nix_euid_is_root() -> bool {
        // /proc/self/status Uid line, first field — std-only root
        // check (tests skip the chmod-000 branch under root).
        fs::read_to_string("/proc/self/status")
            .map(|s| {
                s.lines().any(|line| {
                    let mut fields = line.split_whitespace();
                    fields.next() == Some("Uid:") && fields.next() == Some("0")
                })
            })
            .unwrap_or(false)
    }
}
