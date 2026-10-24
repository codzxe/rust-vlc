//! H.264 bitstream parsers (card 5.F1; ITU-T H.264 03/2023 syntax,
//! clean-room).
//!
//! Module map (wave-4 file-disjoint split, worklog 5.F1b):
//!
//! - [`slice`] — slice-header parser (§7.3.3) with the minimal
//!   SPS/PPS context seams (5.F1b, this card);
//! - [`sei`] — supplemental-enhancement-information parser (§7.3.2.3
//!   and Annex D payloads: buffering period, pic timing, unregistered
//!   user data, recovery point, filler; 5.F1b, this card);
//! - `nal` / `sps` / `pps` — NAL-unit layer, parameter-set parsers
//!   (card 5.F1a, parallel agent; Lead unifies into this module).
//!
//! All parsers take **RBSP** payloads (emulation-prevention bytes
//! already removed — the NAL layer's job, card 5.F1a) and return
//! typed errors: no panic, no hang, no decode claim.

pub mod sei;
pub mod slice;
