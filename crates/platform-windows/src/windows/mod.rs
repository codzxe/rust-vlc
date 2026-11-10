//! The Windows integrations (cfg-gated to `target_os = "windows"`):
//! the WASAPI MMDevice client, the DXGI adapter scanner, the Common
//! File Dialogs. On any other OS every one of these is replaced by
//! the typed fallbacks in the seam modules (the crate still compiles
//! and its capability report says `WrongOs` honestly).
//!
//! **Evidence status (sandbox honesty, card 7.A1):** this Linux
//! sandbox has no Windows SDK — everything under this module is
//! compile-verified via `cargo check --target x86_64-pc-windows-msvc`
//! (declarations typechecked, no linking, no execution). Runtime
//! behavior on real Windows is `[VERIFY]`; the host-runnable tests
//! cover the typed seams and negotiation instead.

#[cfg(target_os = "windows")]
pub mod com;
#[cfg(target_os = "windows")]
pub mod dialogs;
#[cfg(target_os = "windows")]
pub mod dxgi;
#[cfg(target_os = "windows")]
pub mod wasapi;
