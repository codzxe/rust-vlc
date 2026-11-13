//! Build script — the tauri v2 standard form for THIS crate.
//!
//! tauri.conf.json and capabilities/ live at the crate root (the
//! conventional layout), so default discovery finds them:
//! `tauri_build::build()` embeds both into the context
//! `tauri::generate_context!()` (src/app.rs) reads at compile time —
//! including the ui/dist assets via `build.frontendDist`, which is why
//! every lane that compiles this crate builds the UI first.

fn main() {
    tauri_build::build();
}
