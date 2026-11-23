# AUDIT: crates/media-probe/src/report.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 475 / 16 (5 types, 16 constructors/accessors/helpers) + 7 unit tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open` → apps/cli/src/host.rs:904 → demux_host.rs:444-448 (`media_probe::probe_stream` → `ProbeRegistry::probe` consumes `ProbeHit`/`SourceMeta` and mints `ProbeReport`; `report.format.as_str()` at demux_host.rs:448 drives the container dispatch). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `target/debug/deps/media_probe-440f45029f776138 report::` → 7 passed, 0 failed (exit 0) |

## 1. One-paragraph verdict
Clean vocabulary module, no fakeness: validated `FormatId` grammar (1..=24 bytes, lowercase
alnum/`-`/`.`), `Confidence` as a rejecting (never clamping) newtype, `TrackHint` validating its
descriptor through the L0 grammar, bounded strings everywhere (≤24 id, ≤16 extension). The one
design soft spot — `ProbeHit.score` is a raw `u8` a plugin could set to 255 — is closed at the
registry: `Confidence::from_score(hit.score)` (signature/mod.rs:187) drops out-of-range hits
before selection, exactly as this file's field doc promises. All 7 unit tests executed and pass,
including the extension-extraction edge matrix (uppercase, query string, dotfile, double suffix,
trailing slash). Two S4 doc nits only.

## 2. What actually works (verified)
- `target/debug/deps/media_probe-440f45029f776138 report::` → `test result: ok. 7 passed; 0 failed` (exit 0).
- Cross-verified the `score: u8` validation chain: report.rs:251-253's doc claim "validated when
  the registry accepts the hit" is true — signature/mod.rs:184-199 filters every hit through
  `Confidence::from_score` and silently drops mis-scaled hits from selection.

## 3. Findings
F-001 [S4]
Where: crates/media-probe/src/report.rs:5-9
Code:
```rust
//! House rules (MP §8): a value and its absence are different types
//! (`SourceMeta::size` is an [`Option`], never a `0` sentinel); every
//! public constructor validates and returns
//! [`Result`](std::result::Result); nothing here allocates
```
What is wrong: "every public constructor validates and returns Result" is not literally true
here: `FormatId::avi()` (and 15 siblings) return `Self` (infallible vocabulary — fine), and
`ProbeHit`/`ProbeReport`/`SourceMeta` have NO constructors at all (pub-field structs —
`SourceMeta::new` returns `Self` without validation). The fallible constructors that exist
(`FormatId::parse`, `TrackHint::new`) do validate. Overstated house-rule sentence.
What correct looks like: "every fallible constructor validates". Confidence: HIGH —
Verification: read.
F-002 [S4]
Where: crates/media-probe/src/report.rs:328
Code:
```rust
    let path = after_scheme.1.split(['?', '#']).next()?;
```
What is wrong: `split(...).next()` on a `&str` always returns `Some` — the `?` is dead code
(never the None path). Harmless; mildly misleading about a failure mode that cannot happen.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FormatId::{avi,mp4,wav,aiff,ogg,matroska,adts,mpegts,mpegps,ivf,mp3,mp2,annexb,h264,hevc,flac} | 36-124 | REAL | — | 16 builtin ids; no asf() — consistent with the no-ASF-signature reality |
| FormatId::parse | 135-144 | REAL | — | grammar enforced, tested both ways |
| FormatId::as_str / Display | 146-159 | REAL | — | — |
| Confidence::from_score / as_u8 / meets / Display | 171-201 | REAL | — | rejecting not clamping; tested incl. 255 |
| TrackHint::new | 224-237 | REAL | — | descriptor validated through CodecDescriptor |
| SourceMeta::new / from_stream | 300-322 | REAL | — | size None for live; extension lowered |
| extension_of | 325-340 | REAL | S4 | dead `?` (F-002); edge matrix tested |
| (tests) | 342-475 | REAL | — | 7 tests, executed green |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| format_id_rejects_bad_shapes | YES | empty/upper/space/25-char/tab all refused with exact code |
| confidence_bounds_are_enforced_not_clamped | YES | 101 and 255 → None; meets boundary exact |
| source_meta_extension_extraction | YES | 6-URI edge matrix incl. query/dotfile/double-suffix/trailing-slash |
| probe_report_carries_the_card_shape | YES | field round-trip + "95/100" Display |
| all 7 | YES | executed 7/7 green on the prebuilt binary |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every public constructor validates and returns Result" | report.rs:8-9 | FALSE-as-written | F-001 (infallible/fallible mix) |
| ProbeHit.score "validated when the registry accepts the hit" | report.rs:251-253 | REAL | signature/mod.rs:187 `Confidence::from_score(hit.score)` gate read + registry tests |
| "nothing here allocates unboundedly" | report.rs:9-12 | REAL | id ≤ 24 B, extension ≤ 16 B, hints bounded by the window scan |

## 7. Dead / duplicated / unreachable code
- The dead `?` at line 328 (F-002). Nothing else.

## 8. Dependencies & flags
- `media_types` (ids, EngineError), `plugin_api` (SourceStream), `crate::error`. No external
  deps, no features, no unsafe, no panics in non-test code.

## 9. Open questions for the next agent
- `error::no_demuxer` (signature/mod.rs:207-211) embeds the losing best hit + sniffed prefix in
  the refusal — check its Display redaction bound (the wav `not_wav` twin caps sniffed bytes at
  16 in Display; verify the probe error does the same).
