//! H.265/HEVC **bitstream parser** (card 5.G1, doc 04 §4.3's "parser
//! self-build M" — the same shape as the H.264 parser of card 5.F1,
//! one spec generation later).
//!
//! # The honesty boundary (doc 04 §4.3, verbatim scope)
//!
//! There is **no software HEVC decode here** — this module parses and
//! classifies bitstream structure only (NAL framing, VPS/SPS/PPS, a
//! minimal slice-segment header, the `hvcC` record). Software decode
//! remains `blocked`/`hw-only` in the feature matrix (doc 04 §4.3:
//! "ADR-001 hw-only, parser self-build M, software decode blocked
//! with next-step recorded"). Phase-7 hardware adapters are the
//! consumer of these parsed structures. No `Decoder` impl lives under
//! this module tree.
//!
//! # Damage model
//!
//! Every entry point is total: any byte string yields either a parsed
//! structure or a typed [`parser::HevcError`] — no panics, no
//! unbounded work (the fuzz-lite suite pins this).

pub mod fuzz;
pub mod parser;
