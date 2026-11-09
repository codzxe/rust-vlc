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

use crate::dispatch::open_path_request;
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

/// Playlist-file suffixes (the engine-side playlists crate's reader
/// surface: M3U/M3U8, PLS, XSPF, CUE). A playlist file does not open
/// a session — it loads the list (`playlist.load`; 4.D1's handoff,
/// parse engine-side). Mirrors ui/src/client/open-path.ts exactly.
#[must_use = "pure reporter"]
pub fn is_playlist_path(path: &Path) -> bool {
    const PLAYLIST_SUFFIXES: &[&str] = &["m3u", "m3u8", "pls", "xspf", "cue"];
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| PLAYLIST_SUFFIXES.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Anything the shell can turn into an open command (media OR
/// playlist) — the drop pre-filter's honest surface.
#[must_use = "pure reporter"]
pub fn is_openable_path(path: &Path) -> bool {
    is_media_path(path) || is_playlist_path(path)
}

/// The drag-and-drop seam: OS-dropped paths → the open request.
pub trait DragDropHandler {
    /// Translate one drop event's paths into the command(s) to
    /// dispatch. Multi-file drops answer the first OPENABLE file
    /// (media or playlist) and ignore the rest (multi-enqueue is a
    /// later card; the honest v1 surface is one command per drop).
    ///
    /// # Errors
    /// The shell seam's [`HostError`] (a drop with no openable file is
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
/// needed: this compiles and is exact in any build. The path-kind
/// rule (media → `session.open`, playlist → `playlist.load`) is
/// [`crate::dispatch::open_path_request`], the shared funnel — this
/// half only acquires the path.
#[derive(Debug, Default)]
pub struct PathDragDrop;

impl DragDropHandler for PathDragDrop {
    fn on_files_dropped(
        &mut self,
        paths: &[PathBuf],
        request_id: RequestId,
    ) -> Result<Option<WireRequest>, HostError> {
        let Some(first) = paths.iter().find(|path| is_openable_path(path)) else {
            return Ok(None);
        };
        Ok(open_path_request(request_id, first.as_path()))
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

    /// Issue #3 (4.D1's handoff to 4.H1): a dropped PLAYLIST file
    /// loads the list — `playlist.load {path, append: false}`, the
    /// engine parses; the drop never opens a session for it.
    #[test]
    fn playlist_drops_load_the_list_not_a_session() {
        let mut dnd = PathDragDrop;
        for (index, path) in [
            PathBuf::from("/tmp/queue.m3u"),
            PathBuf::from("/tmp/Queue.M3U8"),
            PathBuf::from("/tmp/radio.pls"),
            PathBuf::from("/tmp/library.xspf"),
            PathBuf::from("/tmp/album.cue"),
        ]
        .into_iter()
        .enumerate()
        {
            assert!(is_playlist_path(&path), "{path:?} is a playlist");
            let request = match dnd
                .on_files_dropped(std::slice::from_ref(&path), RequestId(20 + index as u64))
            {
                Ok(Some(request)) => request,
                Ok(None) => unreachable!("a playlist file was dropped: {path:?}"),
                Err(err) => unreachable!("drop translation is pure logic: {err}"),
            };
            match request.command {
                Command::PlaylistLoad {
                    path: loaded,
                    append,
                } => {
                    assert_eq!(loaded, path);
                    assert!(!append, "a drop replaces the queue (append: false)");
                }
                other => unreachable!("a playlist drop must load the list: {other:?}"),
            }
        }
    }

    /// The classifier's negative space: media suffixes stay sessions,
    /// unknown suffixes stay no-ops (never a guessed kind).
    #[test]
    fn path_classification_is_exact() {
        assert!(is_playlist_path(&PathBuf::from("/tmp/x.m3u")));
        assert!(!is_playlist_path(&PathBuf::from("/tmp/x.m3u.bak")));
        assert!(!is_playlist_path(&PathBuf::from("/tmp/movie.avi")));
        assert!(!is_playlist_path(&PathBuf::from("/tmp/noext")));
        assert!(is_openable_path(&PathBuf::from("/tmp/movie.avi")));
        assert!(is_openable_path(&PathBuf::from("/tmp/queue.pls")));
        assert!(!is_openable_path(&PathBuf::from("/tmp/README.md")));
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
