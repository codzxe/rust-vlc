//! The file-dialog seam (card 7.A2): open/save requests.
//!
//! The Linux lane's honest v0: an **xdg-desktop-portal probe** (the
//! session-bus reachability check, std `UnixStream`) followed by a
//! real **zenity/kdialog command fallback**. The portal's D-Bus
//! `FileChooser` client itself is `[PROPOSAL]` follow-up work — it
//! needs a raw D-Bus wire client (SASL EXTERNAL auth + message
//! marshalling + signal matching) that this sandbox cannot exercise
//! (no portal daemon), and shipping unexercised protocol code would
//! violate the no-fabrication rule. The typed
//! [`crate::capability::UnavailableReason::NotYetImplemented`] row in
//! the platform report says exactly this; the zenity/kdialog chain
//! is the working path today.

use std::path::PathBuf;

use crate::error::PlatformError;

/// One file-type filter row (label + glob patterns).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFilter {
    /// Human label ("Video files").
    pub label: String,
    /// Glob patterns ("*.mkv", "*.mp4").
    pub patterns: Vec<String>,
}

impl FileFilter {
    /// Build one filter row.
    #[must_use = "constructs a filter row"]
    pub fn new(label: &str, patterns: &[&str]) -> FileFilter {
        FileFilter {
            label: label.to_string(),
            patterns: patterns.iter().map(|p| (*p).to_string()).collect(),
        }
    }
}

/// A dialog request, OS-neutral (ARCH-6).
#[derive(Debug, Clone, Default)]
pub struct FileDialogRequest {
    /// Window title (None = OS default).
    pub title: Option<String>,
    /// Type filters (empty = all files).
    pub filters: Vec<FileFilter>,
    /// Suggested default location.
    pub default_dir: Option<PathBuf>,
    /// Allow picking several files at once (open dialogs only).
    pub multiple: bool,
}

/// The outcome of a modal dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePick {
    /// The user cancelled — not an error.
    Cancelled,
    /// One or more chosen paths (one for save dialogs unless the
    /// user typed a name that already exists — overwrite
    /// confirmation is the dialog's own).
    Paths(Vec<PathBuf>),
}

/// The dialog seam.
pub trait FileDialogs: Send {
    /// Show a modal file-open dialog.
    ///
    /// # Errors
    /// Typed [`PlatformError`] when no dialog backend could run
    /// (integration absent, every fallback program missing).
    fn open_file(&mut self, request: &FileDialogRequest) -> Result<FilePick, PlatformError>;

    /// Show a modal file-save dialog.
    ///
    /// # Errors
    /// As [`FileDialogs::open_file`].
    fn save_file(&mut self, request: &FileDialogRequest) -> Result<FilePick, PlatformError>;
}

/// The typed fallback (mirrors [`crate::audio::UnavailableAudioDevices`]).
#[derive(Debug, Clone)]
pub struct UnavailableDialogs {
    reason: PlatformError,
}

impl UnavailableDialogs {
    /// Wrap the reason every method will return.
    #[must_use = "constructs the typed fallback"]
    pub fn because(reason: PlatformError) -> UnavailableDialogs {
        UnavailableDialogs { reason }
    }

    /// The reason this integration is unavailable.
    #[must_use = "pure accessor"]
    pub fn reason(&self) -> &PlatformError {
        &self.reason
    }
}

impl FileDialogs for UnavailableDialogs {
    fn open_file(&mut self, _request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
        Err(self.reason.clone())
    }

    fn save_file(&mut self, _request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
        Err(self.reason.clone())
    }
}
