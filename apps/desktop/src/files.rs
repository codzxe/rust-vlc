//! Drag-and-drop + native file dialogs as trait seams (card 4.H1).
//!
//! # The rule both seams obey
//!
//! A file open is a **command**, never a shell-side engine call
//! (ARCH-5): every path — dropped or picked — funnels through
//! [`open_file_request`](crate::dispatch::open_file_request) and then
//! [`lumen_dispatch`](crate::dispatch::lumen_dispatch). The seams here
//! exist because the *acquisition* of the path is webview-shell
//! surface:
//!
//! - **Drag-and-drop** (tauri `DragDropEvent` on the window) — paths
//!   arrive from the OS; the shell's job is only to translate them
//!   into the open request.
//! - **Native file dialog** (tauri-plugin-dialog) — a modal the
//!   webview layer hosts.
//!
//! Neither can link in this sandbox (no webkit2gtk-4.1, no display —
//! `pkg-config --exists webkit2gtk-4.1` fails; the 2.C3/ADR-004
//! posture), so the real impls are the typed
//! [`HostError::WebviewPending`] state and the **test impls** carry
//! the logic proof. The tauri build swaps the test impls for the
//! plugin-backed ones at the composition root — the seam, not the
//! rule, is what moves.

use std::path::Path;
use std::path::PathBuf;

use crate::dispatch::open_file_request;
use crate::error::HostError;
use commands::WireRequest;
use commands::envelope::RequestId;

/// Media-file suffixes the drop handler accepts (the open path owns
/// format truth; this is the shell's pre-filter so a dropped README
/// never becomes a session.open error the UI must handle).
#[must_use = "pure reporter"]
pub fn is_media_path(path: &Path) -> bool {
    const MEDIA_SUFFIXES: &[&str] = &[
        "avi", "mkv", "mp4", "mov", "webm", "m4v", "mpg", "mpeg", "ts", "flv", "wmv", "wav", "mp3",
        "flac", "ogg", "opus", "aac", "m4a", "aiff",
    ];
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| MEDIA_SUFFIXES.contains(&ext.to_ascii_lowercase().as_str()))
}

/// The drag-and-drop seam: OS-dropped paths → the open request.
pub trait DragDropHandler {
    /// Translate one drop event's paths into the command(s) to
    /// dispatch. Multi-file drops answer the first media file and
    /// ignore the rest (playlist enqueue is a Phase-4 card; the
    /// honest v1 surface is one session).
    ///
    /// # Errors
    /// The shell seam's [`HostError`] (a drop with no media file is
    /// a no-op answer, not an error — `Ok(None)`).
    fn on_files_dropped(
        &mut self,
        paths: &[PathBuf],
        request_id: RequestId,
    ) -> Result<Option<WireRequest>, HostError>;
}

/// The native file-dialog seam: pick one media file.
pub trait NativeFileDialog {
    /// Open the modal, answer the picked path (or `None` on cancel).
    ///
    /// # Errors
    /// [`HostError::WebviewPending`] for the not-linked state in this
    /// build; the tauri-backed impl answers the plugin's own errors.
    fn pick_media_file(&mut self) -> Result<Option<PathBuf>, HostError>;
}

/// The production drag-drop half — path logic only, no webview
/// needed: this compiles and is exact in any build.
#[derive(Debug, Default)]
pub struct PathDragDrop;

impl DragDropHandler for PathDragDrop {
    fn on_files_dropped(
        &mut self,
        paths: &[PathBuf],
        request_id: RequestId,
    ) -> Result<Option<WireRequest>, HostError> {
        let Some(first) = paths.iter().find(|path| is_media_path(path)) else {
            return Ok(None);
        };
        Ok(Some(open_file_request(request_id, first.as_path())))
    }
}

/// The production dialog half — the honest pending state: the modal
/// IS webview surface, and this build links none.
#[derive(Debug, Default)]
pub struct NativeDialog;

impl NativeFileDialog for NativeDialog {
    fn pick_media_file(&mut self) -> Result<Option<PathBuf>, HostError> {
        Err(HostError::WebviewPending { adr: "ADR-004" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::Command;

    /// A drop of media files answers exactly one session.open — the
    /// first media file, non-media skipped, playlist deferred.
    #[test]
    fn drops_become_the_open_command_first_media_wins() {
        let mut dnd = PathDragDrop;
        let request = match dnd.on_files_dropped(
            &[
                PathBuf::from("/tmp/notes.txt"),
                PathBuf::from("/tmp/movie.avi"),
            ],
            RequestId(5),
        ) {
            Ok(Some(request)) => request,
            Ok(None) => unreachable!("a media file was dropped"),
            Err(err) => unreachable!("drop translation is pure logic: {err}"),
        };
        assert_eq!(request.request_id, RequestId(5));
        match request.command {
            Command::SessionOpen { source, .. } => match source {
                commands::catalog::types::SourceSpec::File { path } => {
                    assert_eq!(path, PathBuf::from("/tmp/movie.avi"));
                }
                other => unreachable!("drop open must be a Path source: {other:?}"),
            },
            other => unreachable!("drop must open a session: {other:?}"),
        }
    }

    #[test]
    fn drops_without_media_are_noops_not_errors() {
        let mut dnd = PathDragDrop;
        let answer = dnd
            .on_files_dropped(&[PathBuf::from("/tmp/README.md")], RequestId(6))
            .unwrap_or_else(|err| unreachable!("no media is a no-op: {err}"));
        assert!(answer.is_none());
        // Case-insensitive suffix matching.
        let answer = dnd
            .on_files_dropped(&[PathBuf::from("/tmp/Song.MP3")], RequestId(7))
            .unwrap_or_else(|err| unreachable!("one media file: {err}"));
        assert!(answer.is_some());
    }

    /// The dialog seam in this build answers the typed pending
    /// state — never a fake path, never a silent `None`.
    #[test]
    fn native_dialog_is_the_typed_pending_state() {
        let mut dialog = NativeDialog;
        let err = match dialog.pick_media_file() {
            Err(err) => err,
            Ok(path) => unreachable!("no webview is linked, got {path:?}"),
        };
        assert_eq!(err, HostError::WebviewPending { adr: "ADR-004" });
        assert!(err.to_string().contains("ADR-004"));
    }
}
