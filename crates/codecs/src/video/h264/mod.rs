//! H.264 / AVC (ITU-T H.264) parsing and decoding (card 5.F1).
//!
//! Wave-4 split: card 5.F1a owns the NAL-unit / SPS / PPS layer
//! (`parser::nal`, `parser::sps`, `parser::pps` — separate agent,
//! separate worktree); card 5.F1b (this commit) owns the slice-header
//! and SEI parsers under [`parser`]. The two halves meet at the
//! `parser::slice` context structs ([`parser::slice::SliceSpsInfo`],
//! [`parser::slice::SlicePpsInfo`]) — minimal, F1b-clean-room seams
//! that a parsed SPS/PPS from F1a's modules can populate at Lead
//! merge time. No decode claim in either half: this crate layer
//! parses bitstream syntax only.

pub mod parser;
