//! Video snapshot — the current-frame-to-PNG planning policy (issue #32,
//! doc 05 §2.7 `video.take-screenshot`; VLC Shift+S).
//!
//! # What this module is
//!
//! The **pure planning half** of a snapshot, mirroring
//! [`crate::frame_step`]: given the command's optional explicit path and
//! the `snapshot-*` config family, produce the output file the host
//! should write the presented frame to. Execution — reading pixels back
//! from the presenting renderer — is the structural gap recorded below.
//!
//! # Semantics (VLC `snapshot-*` options, inventory §Video/Snapshot)
//!
//! - **Explicit path wins**: `video.take-screenshot`'s optional `path`
//!   is a *file* path used verbatim (its extension selects the format;
//!   no extension → the configured format).
//! - **Derived path**: with no explicit path, the plan is
//!   `<dir>/<prefix><counter>.<ext>` — `snapshot-path` (directory),
//!   `snapshot-prefix` (default `vlcsnap-`), `snapshot-format`
//!   (enum png/jpg/tiff, default png). The counter is zero-padded to 6
//!   (VLC's `vlcsnap-000001.png` convention).
//!   **[PROPOSAL: modern VLC names snapshots by wall-clock time
//!   (`vlcsnap-YYYY-MM-DD-HHhMMmSSs`); the padded counter is the
//!   picked v0 — a monotonic sequence never collides within a session
//!   even across seeks.]**
//! - **Format**: the wire v1 encodes PNG only (doc 05 §2.7 result is
//!   `accepted`); `jpg`/`tiff` parse (the enum is honest about VLC's
//!   choices) but [`SnapshotFormat::is_supported_v1`] reports `false`
//!   so the host refuses typed instead of writing a lie.
//!
//! # The structural gap (honest)
//!
//! A real capture needs pixel readback from the **presenting renderer**
//! — the video-engine sink hands frames to the platform surface and
//! keeps no copy. No frame-grab seam exists there today; wiring one is
//! video-engine/renderer-owned work. This module pins everything the
//! command layer *can* decide ahead of that seam, so the readback lands
//! onto a planned, tested path.

use std::path::{Path, PathBuf};

/// `snapshot-format` (inventory enum; wire v1 encodes PNG only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotFormat {
    /// Portable Network Graphics — the v1 encoder.
    Png,
    /// JPEG — parsed, not encoded in v1.
    Jpg,
    /// TIFF — parsed, not encoded in v1.
    Tiff,
}

impl SnapshotFormat {
    /// Parse the config value (`png`/`jpg`/`tiff`, case-sensitive per
    /// the inventory enum). `None` is the typed refusal for anything
    /// else — never a silent fallback.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "png" => Some(Self::Png),
            "jpg" => Some(Self::Jpg),
            "tiff" => Some(Self::Tiff),
            _ => None,
        }
    }

    /// The canonical lower-case name + file extension.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Tiff => "tiff",
        }
    }

    /// Whether the v1 engine can encode this format (PNG only).
    #[must_use]
    pub fn is_supported_v1(self) -> bool {
        matches!(self, Self::Png)
    }
}

/// VLC's default `snapshot-prefix` (`vlcsnap-`).
pub const DEFAULT_SNAPSHOT_PREFIX: &str = "vlcsnap-";

/// One planned snapshot: where the presented frame goes, as what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPlan {
    /// The output file (verbatim explicit path, or derived
    /// dir/prefix/counter name).
    pub output_path: PathBuf,
    /// The format the file is written as.
    pub format: SnapshotFormat,
}

/// The `snapshot-*` config family (VLC defaults per the inventory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotConfig {
    /// `snapshot-path` — the directory (or `None` for the host's
    /// default pictures location, resolved by the host).
    pub directory: Option<PathBuf>,
    /// `snapshot-prefix` (default `vlcsnap-`).
    pub prefix: String,
    /// `snapshot-format` (default `png`).
    pub format: SnapshotFormat,
    /// The session's monotonic snapshot counter (drives the derived
    /// filename; the host increments it per capture).
    pub counter: u32,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            directory: None,
            prefix: DEFAULT_SNAPSHOT_PREFIX.to_string(),
            format: SnapshotFormat::Png,
            counter: 0,
        }
    }
}

impl SnapshotConfig {
    /// Plan the capture for an optional explicit `path` (a *file*
    /// path — VLC's snapshot-path "directory or filename" split lives
    /// in config; the wire command's argument is the explicit file).
    ///
    /// Typed refusals (`None`): an empty explicit path, an empty
    /// prefix, or an unknown format — never a guessed file.
    #[must_use]
    pub fn plan(&self, explicit_path: Option<&Path>) -> Option<SnapshotPlan> {
        if let Some(path) = explicit_path {
            if path.as_os_str().is_empty() {
                return None;
            }
            // The extension selects the format when present; anything
            // the enum does not know is refused typed.
            let format = match path.extension().and_then(|ext| ext.to_str()) {
                Some(ext) => SnapshotFormat::parse(ext)?,
                None => self.format,
            };
            return Some(SnapshotPlan {
                output_path: path.to_path_buf(),
                format,
            });
        }
        if self.prefix.is_empty() {
            return None;
        }
        // No directory configured: the plan stays relative — the host
        // resolves its default location at write time (its platform
        // truth, not ours).
        let file_name = format!(
            "{}{:06}.{}",
            self.prefix,
            self.counter,
            self.format.as_str()
        );
        let output_path = match &self.directory {
            Some(dir) => dir.join(file_name),
            None => PathBuf::from(file_name),
        };
        Some(SnapshotPlan {
            output_path,
            format: self.format,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn format_parses_the_inventory_enum_and_refuses_the_rest() {
        assert_eq!(SnapshotFormat::parse("png"), Some(SnapshotFormat::Png));
        assert_eq!(SnapshotFormat::parse("jpg"), Some(SnapshotFormat::Jpg));
        assert_eq!(SnapshotFormat::parse("tiff"), Some(SnapshotFormat::Tiff));
        assert_eq!(SnapshotFormat::parse("jpeg"), None, "not in the enum");
        assert_eq!(SnapshotFormat::parse("PNG"), None, "case-sensitive enum");
        assert!(SnapshotFormat::Png.is_supported_v1());
        assert!(!SnapshotFormat::Jpg.is_supported_v1());
        assert!(!SnapshotFormat::Tiff.is_supported_v1());
    }

    #[test]
    fn derived_path_uses_vlc_prefix_counter_and_extension() {
        let config = SnapshotConfig {
            directory: Some(PathBuf::from("/tmp/snaps")), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            prefix: "vlcsnap-".to_string(),
            format: SnapshotFormat::Png,
            counter: 1,
        };
        let plan = config.plan(None).must();
        assert_eq!(
            plan.output_path,
            PathBuf::from("/tmp/snaps/vlcsnap-000001.png") // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        );
        assert_eq!(plan.format, SnapshotFormat::Png);
    }

    #[test]
    fn explicit_path_wins_and_its_extension_selects_the_format() {
        let config = SnapshotConfig::default();
        let plan = config.plan(Some(Path::new("/home/me/frame.png"))).must(); // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        assert_eq!(plan.output_path, PathBuf::from("/home/me/frame.png")); // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        assert_eq!(plan.format, SnapshotFormat::Png);
        // A different extension overrides the configured format.
        let plan = config.plan(Some(Path::new("/home/me/frame.jpg"))).must(); // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        assert_eq!(plan.format, SnapshotFormat::Jpg);
        // No extension falls back to the configured format.
        let plan = config.plan(Some(Path::new("/home/me/frame"))).must(); // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        assert_eq!(plan.format, SnapshotFormat::Png);
    }

    #[test]
    fn unknown_extension_is_refused_typed_not_fallen_back() {
        let config = SnapshotConfig::default();
        assert!(config.plan(Some(Path::new("/tmp/x.webp"))).is_none()); // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
    }

    #[test]
    fn empty_explicit_path_or_prefix_is_refused() {
        let config = SnapshotConfig::default();
        assert!(config.plan(Some(Path::new(""))).is_none());
        let broken = SnapshotConfig {
            prefix: String::new(),
            ..SnapshotConfig::default()
        };
        assert!(broken.plan(None).is_none());
    }

    #[test]
    fn no_directory_keeps_the_plan_relative_for_the_host() {
        let config = SnapshotConfig {
            counter: 42,
            ..SnapshotConfig::default()
        };
        let plan = config.plan(None).must();
        assert_eq!(plan.output_path, PathBuf::from("vlcsnap-000042.png"));
    }

    #[test]
    fn counter_advances_names_monotonically() {
        let mut config = SnapshotConfig::default();
        let first = config.plan(None).must();
        config.counter += 1;
        let second = config.plan(None).must();
        assert_eq!(first.output_path, PathBuf::from("vlcsnap-000000.png"));
        assert_eq!(second.output_path, PathBuf::from("vlcsnap-000001.png"));
    }
}
