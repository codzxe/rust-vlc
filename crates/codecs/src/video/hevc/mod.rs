//! H.265/HEVC **bitstream parser + hardware decode-path seam** (the
//! parser is card 5.G1, doc 04 §4.3's "parser self-build M"; the
//! decode-path integration is card 5.G2 — the mirror of the H.264
//! shape of 5.F1/5.F2, one spec generation later).
//!
//! # The honesty boundary (doc 04 §4.3; ADR-001 §1)
//!
//! There is **no software HEVC decode here** — the module parses and
//! classifies bitstream structure, and routes the parsed result to a
//! **hardware session** ([`hw`]). Software decode remains
//! `blocked`/`hw-only` in the feature matrix until ADR-001's
//! conformance-gated software track produces a validated decoder
//! (doc 04 §4.3: "ADR-001 hw-only, parser self-build M, software
//! decode `blocked` with next-step recorded"). The one `Decoder`
//! impl under this tree ([`decode::HevcDecoder`]) feeds hardware
//! adapters only: with no accelerator negotiated it fails configure
//! with the typed `hevc.hw.required` `Unsupported` error — the
//! honest matrix row, never a fake success, never a silent skip.
//! Doc 04 §4.3 also carries the heavier-patent note: licensing
//! assessment for *distributing an HEVC decoder at all* is a §14
//! item (doc 11 / ADR-010 territory) — the hw-first posture keeps
//! that decision with the OS adapters, not this crate.
//!
//! # Structure (5.G1 split; 5.G2 additions)
//!
//! [`parser`] owns the whole parsing surface: NAL-unit framing (the
//! Annex-B start-code form and the length-prefixed form incl. the
//! `hvcC` record — 5.G1), the VPS/SPS/PPS parameter sets, and the
//! minimal-honest slice-segment header prefix. Card 5.G2 adds the
//! consumption side: [`hw`] (the adapter seam:
//! [`hw::HevcHwDecoder`] session contract, [`hw::HevcHwBridge`]
//! plug, [`hw::HevcHwPath`] consumption point) and [`decode`] (the
//! codec-api [`Decoder`](codec_api::Decoder) adapter: extradata
//! configure, §7.4.2.4.4 access-unit assembly, typed-unsupported
//! fallback).
//!
//! # Damage model
//!
//! Every parser entry point is total: any byte string yields either
//! a parsed structure or a typed [`parser::HevcError`] — no panics,
//! no unbounded work (the fuzz-lite suites pin this). The decode
//! path inherits the discipline: in-stream damage is
//! skipped-and-counted recovery
//! ([`DecodeOutcome::Recovered`](codec_api::DecodeOutcome::Recovered)),
//! never a panic.

pub mod decode;
pub mod hw;
pub mod fuzz;
pub mod parser;

pub use decode::HevcDecoder;
pub use hw::{HevcHwBridge, HevcHwDecoder, HevcHwPath};
