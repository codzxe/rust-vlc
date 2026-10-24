//! HEVC bitstream parsers (card 5.G1; ITU-T H.265 v4 syntax,
//! clean-room per the h264/parser pattern of card 5.F1).
//!
//! # Module map
//!
//! - [`nal`] — the two-byte NAL header (§7.3.1), Annex-B and
//!   length-prefixed framing, emulation-prevention removal (§7.4.2.1),
//!   NAL classification (Table 7-1), the `hvcC`
//!   HEVCDecoderConfigurationRecord (ISO/IEC 14496-15 §8.3.3.1 /
//!   A.4.1), and the shared Exp-Golomb [`nal::BitReader`].
//! - [`vps`] — the video parameter set (§7.3.2.1) incl. the shared
//!   profile_tier_level() (§7.3.3) and the HRD walker.
//! - [`sps`] — the sequence parameter set (§7.3.2.2): geometry with
//!   conformance-window arithmetic, sub-layer ordering, CTB/transform
//!   sizes, VUI basics.
//! - [`pps`] — the picture parameter set (§7.3.2.3): ids, tile grid,
//!   loop-filter bits, init QP.
//! - [`slice`] — the slice segment header (§7.3.6.1), **minimal
//!   honest subset**: first-slice flag, PPS id, dependent/address,
//!   slice type, POC lsb — the reference-picture-set machinery and
//!   everything after it is deliberately unparsed (recorded, not
//!   smoothed).
//!
//! # Conventions
//!
//! All functions take/return **complete NAL units** (payload *with*
//! their two-byte header) unless a name says `rbsp` (emulation
//! prevention already removed — the NAL layer's job). The RBSP
//! unescape offers a caller-owned reusable buffer so steady-state
//! parsing of a stream performs no allocation per NAL. All parsers
//! return typed errors: no panic, no hang, no decode claim.

pub mod nal;
pub mod pps;
pub mod slice;
pub mod sps;
pub mod vps;
