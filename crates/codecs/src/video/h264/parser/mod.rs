//! H.264 bitstream parsers (card 5.F1; ITU-T H.264 03/2023 syntax,
//! clean-room; wave-4 file-disjoint split unified at Lead merge).
//!
//! # Module map
//!
//! - [`nal`] — Annex-B start-code splitting, AVC length-prefixed
//!   splitting, the `avcC` (AVCDecoderConfigurationRecord) record,
//!   emulation-prevention removal, NAL classification, and the shared
//!   Exp-Golomb [`nal::BitReader`] (card 5.F1a).
//! - [`sps`] — the sequence parameter set (§7.3.2.1.1) incl. cropping
//!   arithmetic and VUI basics (5.F1a).
//! - [`pps`] — the picture parameter set (§7.3.2.2) (5.F1a).
//! - [`slice`] — slice-header parser (§7.3.3) with the minimal
//!   SPS/PPS context seams (5.F1b).
//! - [`sei`] — supplemental-enhancement-information parser (§7.3.2.3
//!   and Annex D payloads: buffering period, pic timing, unregistered
//!   user data, recovery point, filler; 5.F1b).
//!
//! # Conventions
//!
//! All functions take/return **complete NAL units** (payload *with*
//! their one-byte header) unless a name says `rbsp` (emulation
//! prevention already removed — the NAL layer's job). Slice views are
//! zero-copy; the RBSP unescape offers a caller-owned reusable buffer
//! so steady-state parsing of a stream performs no allocation per
//! NAL. All parsers return typed errors: no panic, no hang, no decode
//! claim.

pub mod nal;
pub mod pps;
pub mod sei;
pub mod slice;
pub mod sps;
