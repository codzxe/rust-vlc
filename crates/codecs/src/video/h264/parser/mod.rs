//! H.264 bitstream parser (card 5.F1): NAL-unit framing, parameter
//! sets, both byte-stream entry forms. Clean-room from ITU-T H.264
//! syntax (§7.3.2/§7.4.2); **parse only — no decode claim** (doc 04
//! §4.2, see the parent module's honesty note).
//!
//! # Module map
//!
//! - [`nal`] — Annex-B start-code splitting, AVC length-prefixed
//!   splitting, the `avcC` (AVCDecoderConfigurationRecord) record,
//!   emulation-prevention removal, NAL classification, and the shared
//!   Exp-Golomb [`nal::BitReader`].
//! - [`sps`] — the sequence parameter set (§7.3.2.1.1) incl. cropping
//!   arithmetic and VUI basics.
//! - [`pps`] — the picture parameter set (§7.3.2.2).
//! - (`slice`, `sei`: sibling card 5.F1b appends its files here.)
//!
//! # Conventions
//!
//! All functions take/return **complete NAL units** (payload *with*
//! their one-byte header) unless a name says `rbsp` (emulation
//! prevention already removed). Slice views are zero-copy; the RBSP
//! unescape offers a caller-owned reusable buffer so steady-state
//! parsing of a stream performs no allocation per NAL.

pub mod nal;
pub mod pps;
pub mod sps;
