//! lumen-desktop-tauri — the ADR-004 admission crate's library face.
//!
//! `app::run()` is the whole entry; see `src/app.rs` for the wrapper's
//! contract (single command, event pump, window spec, file-open seams)
//! and `tests/single_command.rs` for the §7.4 anti-bypass audit.

pub mod app;
