//! The SubRip (.srt) parser and render path (card 5.M1; doc 04 §5
//! table row "SubRip (.srt) + variants", registry fuzz target
//! `fuzz_sub_srt`; doc 01 §2.2 "format-specific parsing belongs to
//! demux/codecs/subtitles").
//!
//! # What this module owns
//!
//! * [`encoding`] — BOM detection (UTF-8 / UTF-16LE / UTF-16BE),
//!   UTF-8 fallback with lossy degradation, line-ending
//!   normalization;
//! * [`parser`] — the block walk: optional index line, the
//!   `HH:MM:SS,mmm --> HH:MM:SS,mmm` timing line (with the
//!   real-world tolerances: `.`-vs-`,` milliseconds, glued `-->`,
//!   rolled hours, 1–3-digit ms), text lines, blank-line block
//!   separation, run-together resync;
//! * [`tags`] — the inline-markup subset (`<b>/<i>/<u>`,
//!   `<font color=…>`, entities) mapped onto the 5.L2 model's
//!   [`CueText`](crate::model::CueText) style runs;
//! * [`render`] — cue → compositing [`CueLayout`] +
//!   plain-text-box bounds (the renderer owns fonts; doc 04 §5);
//! * [`damage`] — the capped malformed-tolerance ledger;
//! * [`fuzz`] — the `fuzz_sub_srt` harness.
//!
//! # The contract (dirty-world, D-4)
//!
//! [`parse`] takes raw bytes and returns an [`SrtDocument`]: the
//! cues that survived plus the damage ledger. It **never** returns
//! an error, **never** aborts the whole file on mid-file garbage
//! (blocks are skipped to the next block boundary with a damage
//! event) and **never** panics — every malformation is typed data
//! (MP §8). Std-only (C5); no unsafe (workspace deny).
//!
//! # Example
//!
//! ```
//! use subtitles::srt::{self, parse};
//!
//! let doc = srt::parse(b"1\r\n00:00:01,000 --> 00:00:02,000\r\nHello!\r\n");
//! assert!(doc.is_clean());
//! assert_eq!(doc.cues.len(), 1);
//! assert_eq!(doc.cues[0].interval.start_us(), 1_000_000);
//!
//! // Mid-file garbage never aborts the parse:
//! let dirty = b"garbage\n\n1\n00:00:01,000 --> 00:00:02,000\nkept\n";
//! let doc = parse(dirty);
//! assert_eq!(doc.cues.len(), 1);
//! assert!(!doc.is_clean());
//! ```
//!
//! # Never
//!
//! Font stacks, rasterization, GPU paths (doc 01 §2.2 registry row
//! "Never: rendering (that is presentation)") — geometry only.

pub mod damage;
pub mod encoding;
pub mod fuzz;
pub mod parser;
pub mod render;
pub mod tags;

pub use damage::{MAX_DAMAGE_EVENTS, SrtDamage, SrtDamageKind};
pub use encoding::SourceEncoding;
pub use parser::{SrtDocument, parse, parse_str};
pub use render::{
    SrtRenderError, TextBoxLayout, TextMetrics, cue_layout, place_plain_text, plain_text,
};
