//! The AVI1 field-polarity vocabulary (card 5.H3).
//!
//! Kept in a file of its own — not folded into `mod.rs` — because the
//! `stage_bench` example `#[path]`-includes the decoder and parser
//! sources without `mod.rs`, and both refer to the type as
//! `super::FieldPolarity`; a dedicated module keeps that resolution
//! working on both trees.

/// The AVI1 APP0 field polarity of a field-based MJPEG frame (card
/// 5.H3; the "AVI 1.1" extension — OpenDML AVI M-JPEG / Motion JPEG
/// DIB). Absence of the marker (the JFIF world) is a progressive
/// frame and is not represented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldPolarity {
    /// Polarity 0: the JPEG holds the **first field only** — the SOF
    /// height is the field height.
    FirstField,
    /// Polarity 1: the JPEG holds the **second field only**.
    SecondField,
    /// Polarity 2: both fields, lines interleaved in one frame (a
    /// complete interlaced picture; field 1 = the even/top lines).
    Interleaved,
}
