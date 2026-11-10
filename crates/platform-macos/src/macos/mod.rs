//! The macOS integrations (cfg-gated to `target_os = "macos"`): the
//! CoreAudio device client, the Metal adapter scanner, the AppKit
//! open/save panels. On any other OS every one of these is replaced
//! by the typed fallbacks in the seam modules (the crate still
//! compiles and its capability report says `WrongOs` honestly).
//!
//! **Evidence status (sandbox honesty, card 7.A3):** this Linux
//! sandbox has no macOS SDK — everything under this module is
//! compile-verified via `cargo check --target aarch64-apple-darwin`
//! and `x86_64-apple-darwin` (declarations typechecked, no linking,
//! no execution). Runtime behavior on real Apple hardware is
//! `[VERIFY]`; the host-runnable tests cover the typed seams and
//! negotiation instead.

#[cfg(target_os = "macos")]
pub mod coreaudio;
#[cfg(target_os = "macos")]
pub mod dialogs;
#[cfg(target_os = "macos")]
pub mod metal;
#[cfg(target_os = "macos")]
pub mod objc;
