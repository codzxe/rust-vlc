# AUDIT: crates/codecs/src/video/vp9/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-21 |
| Lines / functions | 71 / 5 (module docs + doctest + 2 mod decls + 1 re-export) |
| File verdict | REAL (facade whose checkable claims verify; the decoder it fronts is shipped-reachable and differentially executed in baseline) |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:313 `"vp9" => { let mut decoder = Vp9Decoder::new(); … }`; descriptor producers: IVF `VP90` → `Some("vp9")` (crates/demux/src/ivf/mod.rs:211) and MKV/WebM `V_VP9` → `"vp9"` (crates/demux/src/mkv/header.rs:805); `codec-vp9` is a default feature (crates/codecs/Cargo.toml:58). So `lumen open` on a VP9 IVF/WebM reaches this module's decoder. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `ls fuzz/fuzz_targets/` (35 targets — NO fuzz_codec_vp9); `rg -n "vp9" fuzz/README.md fuzz/Cargo.toml` → zero hits; `rg -n "unsafe" crates/codecs/src/video/vp9/` → comments only; `rg -n "vp9.bitdepth\|vp9.layout"` (decoder.rs:187,199); `rg -n "V_VP9" crates/demux/src/mkv/header.rs` (:805); `rg -n "vp9" audit/runs/baseline/test-workspace.log.txt` (unit tests :776-780 ok; differential :1286-1295 4/4 ok); `awk` Cargo.lock rusty_vp9 entry (no deps) |

## 1. One-paragraph verdict
A 71-line facade: module documentation, a doctest, and the decoder/error declarations. The docs are unusually dense with verifiable claims, and the ones checkable in this tree all hold: the exact pin `rusty_vp9 = "=0.1.3"` with defaults-on (Cargo.toml:145), zero unsafe in the vp9 subtree (rg), the `vp9.bitdepth`/`vp9.layout` typed refusals (decoder.rs:187/199), and both container descriptor producers (IVF VP90, MKV V_VP9). The decoder ran and passed its ffmpeg-libvpx differential plus a hostile-garbage/truncation battery in the baseline workspace run. The two real issues: the only default-enabled shipped VIDEO decoder without a fuzz target (every sibling — mjpeg, h264, hevc, av1 — has one; the module doc's safety story leans on a one-time record battery instead of a campaign), and the "default features (an empty set)" claim about the external crate that cannot be verified from this tree (only lock-corroborated).

## 2. What actually works (verified)
— VP9 decode evidence executed in baseline run #3: `video::vp9::decoder::tests::{wrong_descriptor_is_a_typed_refusal, decode_before_configure_is_typed_bad_state, garbage_packet_is_recovered_not_fatal, zero_length_packet_is_recovered_not_fatal}` + `video::vp9::error::tests::taxonomy_maps_each_variant_to_its_public_kind` all `ok` (test-workspace.log.txt:776-780); `tests/vp9_differential.rs` 4/4 `ok` incl. `differential_lanes_vs_ffmpeg_libvpx_vp9`, `hostile_garbage_is_recovered_and_the_stream_survives`, `off_by_one_truncation_sweep_is_panic_free`, `tenbit_lane_is_a_typed_refusal_not_a_misdecode` (:1286-1295).
— Pin verified: crates/codecs/Cargo.toml:145 `rusty_vp9 = { version = "=0.1.3", optional = true }` behind `codec-vp9 = ["dep:rusty_vp9"]` (:81), in `default` (:58).
— Lock corroboration of "ZERO dependencies": the Cargo.lock entry for rusty_vp9 0.1.3 (lines 816-819) lists no dependencies — an enabled `prometheus-telemetry` would have dragged prometheus crates into the lock.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/vp9/mod.rs:68-71 (the module surface), evidence fuzz/fuzz_targets/
Code:
```rust
pub mod decoder;
mod error;

pub use decoder::Vp9Decoder;
```
What is wrong: VP9 has no fuzz harness. Every sibling video codec module ships one — `av1/fuzz.rs`, `h264/fuzz.rs`, `hevc/fuzz.rs`, mjpeg's — each with a registered cargo-fuzz target (`fuzz_codec_av1.rs`, `fuzz_codec_h264.rs`, `fuzz_codec_hevc.rs`, `fuzz_codec_mjpeg.rs`); `ls fuzz/fuzz_targets/` shows 35 targets, none for vp9, and `rg "vp9" fuzz/README.md fuzz/Cargo.toml` returns nothing. The module doc's hostile-input narrative (:12-13 "the hostile-input no-panic battery") points at the admission record's one-time battery, not a continuous campaign.
Why it matters: this is a shipped, default-enabled decoder consuming arbitrary IVF/WebM payloads — exactly the class the fuzz registry exists to protect; its absence is a coverage hole the passing registry check (baseline run #18, "35 targets, diff empty") silently blesses, because the registry only verifies itself against what exists.
What correct looks like: a `vp9/fuzz.rs` (configure → garbage packet sweep → flush, the av1 pattern) plus `fuzz_codec_vp9.rs` registration; the module doc updated to cite it.
Confidence: HIGH — Verification: ls + rg of the fuzz tree; baseline registry-check log.

### F-002 [S4]
Where: crates/codecs/src/video/vp9/mod.rs:9-10
Code:
```rust
//! `rusty_vp9 = "=0.1.3"`, **default features (an empty set)** — the
//! `prometheus-telemetry` feature stays OFF.
```
What is wrong: the claim that rusty_vp9 0.1.3's default feature set is empty is a property of the external crate's manifest — not verifiable from this tree (no vendored source; no network in this audit). The Cargo.toml pin relies on it: `rusty_vp9 = { version = "=0.1.3", optional = true }` has no `default-features = false`, so if the crate ever shipped a non-empty default, telemetry would silently compile in.
Why it matters: the whole admission shape ("the lockfile stays telemetry-free") hangs on an unverified external property; only the lock's empty dependency list indirectly corroborates it today.
What correct looks like: pin `default-features = false` explicitly (harmless for an empty default set, load-bearing if it ever changes), or re-verify the manifest on every bump per doc 11 §3.7 and say so here.
Confidence: MEDIUM — Verification: UNVERIFIED (external crate metadata); lock corroboration cited in §2.

### F-003 [S4]
Where: crates/codecs/src/video/vp9/mod.rs:47-66
Code:
```rust
//! let refused = decoder.configure(&DecoderConfig::new(
//!     media_types::ids::CodecDescriptor::new("av1")
//!         .unwrap_or_else(|e| unreachable!("{e}")),
```
What is wrong: the doctest asserts only the foreign-descriptor refusal — the facade's one executable claim. Two nits: it never exercises the accept path (`"vp9"` configure → ok), and the baseline test log contains no `Doc-tests` sections at all (`rg -c "Doc-tests" test-workspace.log.txt` → 0), so there is no captured evidence this doctest ran in the baseline run.
Why it matters: thin doc-level coverage plus absent execution evidence — the facade's accept-path claim ("The descriptor gate speaks the demuxers' vocabulary") is only pinned by the unit test in decoder.rs (`wrong_descriptor_is_a_typed_refusal` is refusal-only too; the vocabulary test is `descriptor_gate_accepts_the_delivery_vocabulary` in the av1 sibling's suite, not vp9's).
What correct looks like: extend the doctest with the accepted `"vp9"` descriptor, and record why the baseline log lacks doc-test sections (orchestrator evidence question).
Confidence: HIGH — Verification: read + rg of the baseline log.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-44 | REAL | S3/S4 | claims verified except F-002 (external); F-001 gap documented |
| doctest | 47-66 | REAL-UNPROVEN | S4 | F-003 — refusal-only, no baseline doc-test evidence captured |
| `pub mod decoder` | 68 | REAL | — | 577 lines, zero unsafe (rg), vp9.bitdepth/layout refusals at :187/:199 (other agent's file) |
| `mod error` | 69 | REAL | — | private; taxonomy test ok in baseline |
| `pub use decoder::Vp9Decoder` | 71 | REAL | — | the shipped symbol |

## 5. Test honesty (tests for this module live in decoder.rs / error.rs / tests/vp9_differential.rs)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| doctest (mod.rs:47-66) | YES (refusal path) | would fail if `accepts` admitted "av1" |
| wrong_descriptor_is_a_typed_refusal | YES | same, unit-level |
| garbage_packet_is_recovered_not_fatal / zero_length_packet_is_recovered_not_fatal | YES | per-frame isolation policy |
| differential_lanes_vs_ffmpeg_libvpx_vp9 | YES | bit-exactness vs the real encoder oracle (ran ok, 1.58s) |
| hostile_garbage_is_recovered_and_the_stream_survives / off_by_one_truncation_sweep_is_panic_free | PANIC class | the "no-panic battery" the docs cite — deterministic, not a campaign (F-001) |
| tenbit_lane_is_a_typed_refusal_not_a_misdecode | YES | pins the honest v0 boundary claim |
All `ok` in baseline run #3 (:776-780, :1286-1295).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`rusty_vp9 = "=0.1.3"`, default features (an empty set) — prometheus-telemetry stays OFF" | mod.rs :9-10 | REAL-UNPROVEN | pin verified (Cargo.toml:145); empty-default claim external — lock's empty dep list corroborates |
| "all **safe** Rust … rusty_vp9's decode surface is safe end to end" | mod.rs :18-21 | REAL | rg "unsafe" in vp9/ → comments only (decoder.rs:22 "This module contains **zero `unsafe`**") |
| "IVF … `VP90` fourcc → descriptor `vp9` … Matroska/WebM `V_VP9` tracks map to the same descriptor" | mod.rs :27-31 | REAL | ivf/mod.rs:211, mkv/header.rs:805 |
| "8-bit 4:2:0 → I420; 10/12-bit … 4:2:2/4:4:4 … typed Unsupported (`vp9.bitdepth` / `vp9.layout`)" | mod.rs :36-39 | REAL | decoder.rs:187/199; `tenbit_lane_is_a_typed_refusal_not_a_misdecode` ok in baseline |
| "six-lane bit-exactness differential against ffmpeg libvpx-vp9" | mod.rs :11-13 | REAL | `differential_lanes_vs_ffmpeg_libvpx_vp9 ... ok` in baseline |
| "the hostile-input no-panic battery" | mod.rs :13-14 | REAL (one-time) | battery tests ran ok — but no fuzz campaign exists (F-001) |
| matrix row `codec-video-vp9` | docs/feature-matrix.toml:473+ | (other agent's row range) | not audited here |

## 7. Dead / duplicated / unreachable code
None in this file. The gap is absence, not dead weight (F-001).

## 8. Dependencies & flags
`codec-vp9 = ["dep:rusty_vp9"]` (Cargo.toml:81), default-on (:58) — compiled into shipped builds and reachable (IVF/MKV lanes). rusty_vp9 exact-pinned, defaults enabled (see F-002).

## 9. Open questions for the next agent
- vp9/decoder.rs auditor (not my file): the pooled tight-I420 copy-out and superframe handling are the real decode surface — check the pool rotation for the stale-frame class of bug (the dnx packed_slot twin).
- Fuzz owner: dnx is the other default-enabled codec with no fuzz target — same registry gap as F-001; note it in the same fix.
- Orchestrator: why does the baseline test log contain no Doc-tests sections? If doc tests were skipped in the baseline run, several doctest-backed claims across the repo lack execution evidence.
