# AUDIT: crates/codecs/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 59 / crate root (2 module decls, doc-only) |
| File verdict | REAL code, **FALSE crate-wide doc claims** (unsafe-free, zero-external-deps) |
| Reachable from shipped entry point? | YES — the crate root compiles into every shipped binary (codecs is a dependency of apps/cli) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
Two `pub mod` lines wrapped in a wave-4-era crate doc that has not been updated for the code it now fronts. The "Conventions (crate-wide)" section asserts **"No `unsafe`, no panics on any input"** — but the crate's Linux H.264 hardware lane is a module-level `#![allow(unsafe_code)]` zone with 27 `unsafe` uses in session.rs alone (plus device/dl/library/ffi siblings), compiled unconditionally on Linux into the shipped binary. The header also asserts "Std-only by C5: **zero external dependencies**" and then, 35 lines later in the same doc, introduces claxon as "the first external dependency of this crate" — self-contradiction, and both statements are stale against today's graph (claxon, symphonia ×4, rav1d, rusty_vp9, filmcraft-dnx, rust_h264). The per-module children are honest about all of this (vorbis/mod.rs carefully says "No `unsafe` **on this module's own paths**"; session.rs documents its ADR-001 exception) — only the crate root overclaims.

## 2. What actually works (verified)
- Module declarations resolve (audio, video) — the crate compiles and its suites run (my live runs across 12 binaries this session).
- The 2.B4/2.B5 in-house decision record (jpeg/pcm) is accurate history.

## 3. Findings

F-001 [S3 — crate-wide "No unsafe" claim is false on Linux]
Where: lib.rs:47-52 (esp. 49-50), vs session.rs:74, 499, 597...
Code:
```
//! ## Conventions (crate-wide)
//!
//! - No `unsafe`, no panics on any input: every decode path is
//!   bounds-checked and overflow-checked (the fuzz targets
```
```
#![allow(unsafe_code)]
```
What is wrong: `rg -c "unsafe impl|unsafe \{" crates/codecs/src/video/h264/hw/vaapi/session.rs` → 27 (plus the lane's device/dl/library/ffi modules). The vaapi lane is `cfg(target_os = "linux")` and unconditional, so the shipped Linux binary carries this `unsafe`. The claim is a security-posture statement that a reader (or an automated unsafe-audit) would trust and be wrong. The exception is properly recorded in ADR-001/session.rs — the crate root simply never learned of it.
Why it matters: doc-claimed invariants that the binary contradicts are exactly the class of lie this audit exists to surface.
What correct looks like: "No `unsafe` outside the ADR-001 §5 OS-ABI lanes (h264 hw)".
Confidence: HIGH — Verification: rg + read.

F-002 [S4 — "zero external dependencies" self-contradicts and is stale]
Where: lib.rs:8 vs lib.rs:41-45
Code:
```
//! §2.5(b), a later card). Std-only by C5: **zero external dependencies**.
```
```
//! [`audio::flac`] is the FLAC decoder (card 5.A1, doc 04 §3.1 row
//! "FLAC") through the purity-verified `claxon` engine — the first
//! external dependency of this crate, recorded in
```
What is wrong: the same doc comment asserts zero external dependencies and names the first external dependency; today's Cargo.toml lists 8 optional external engines (claxon, symphonia-bundle-mp3/core/aac/alac/vorbis, ruopus, rav1d, rusty_vp9, filmcraft-dnx, rust_h264) with most in the default feature set. Stale wave-4 prose never revised.
Confidence: HIGH — Verification: read + crates/codecs/Cargo.toml.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crate doc | 1-57 | FALSE in 2 claims | S3/S4 | F-001, F-002 |
| `pub mod audio; pub mod video;` | 58-59 | REAL | — | resolves; all children compile |

## 5. Test honesty
No tests in this file (crate root). Its claims are the test surface — see findings.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No `unsafe`, no panics on any input" (crate-wide) | lib.rs:49-50 | FALSE (unsafe half) | vaapi lane: 27 unsafe uses in session.rs, module allow at :74 |
| "Std-only by C5: zero external dependencies" | lib.rs:8 | FALSE (stale + self-contradictory) | claxon named at :42-45; Cargo.toml graph |
| fuzz targets "fuzz_codec_mjpeg / fuzz_codec_pcm" as the guarantee hammers | :51-52 | STALE | fuzz/ now carries 12 codec + 17 demux + 6 other targets; the two named are the wave-4 set |
| in-house mjpeg/pcm decision records | :10-33 | TRUE (history) | consistent with codecs docs |

## 7. Dead / duplicated / unreachable code
None (the root itself). Note: the false claims live in doc comments only — no runtime effect.

## 8. Dependencies & flags
Root of the codecs crate; children gated per-codec (default = full set incl. external engines).

## 9. Open questions for the next agent
- If any doc/matrix row cites "codecs: std-only / no unsafe" as a product property, that row is FALSE — cross-check in the docs audit.
