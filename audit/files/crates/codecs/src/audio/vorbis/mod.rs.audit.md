# AUDIT: crates/codecs/src/audio/vorbis/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-21 |
| Lines / functions | 76 / 0 code functions (module organizer: 6 `mod` declarations, 14 re-exports, doc header) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for the module's public surface: `lumen open file.ogg` → demux ogg (`OggCodec::Vorbis => "vorbis"`, demux/src/ogg/mod.rs:152; ID-header probe `parse_identification` at apps/cli/src/demux_host.rs:189) → `pipeline_host.rs:271-275` seeded `VorbisDecoder` (three-header chain in `extra_data`); MKV lane: `A_VORBIS => "vorbis"` (demux/src/mkv/header.rs:811) with CodecPrivate harvested (header.rs:158-160,566). NO shipped-entry EXECUTION evidence in baseline (smoke corpus is AVI+MP4 only; baseline #20 is "AVI first-vertical only, by design") — the lane is unit/differential-tested (`tests/vorbis_bitexact.rs`, `decode_through_ogg` in the differential helper) but has no end-to-end run: REAL-UNPROVEN at the entry level. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "pub fn\|pub struct" vorbis/*.rs` → all 14 re-exported symbols verified to exist (comments.rs:41, decoder.rs:212/391/318, headers.rs:54/170/76/90, seek.rs:44/54/77/93/99/106); `rg -n "decode_block\b"` → `pub fn decode_block` at decoder.rs:318; `rg -n "fuzz_codec_vorbis" fuzz/ crates/codecs/tests/` → target shim + survival suite both call it from outside the crate (fuzz.rs's `pub` justification verified); `ls docs/dependencies/records/` → symphonia-vorbis-0.6.1.md exists; `head -40 crates/codecs/tests/vorbis_vlc_differential.rs` → PENDING-INFRA status honest; `sed -n` on quoted ranges |

## 1. One-paragraph verdict
A clean module organizer whose every structural claim checks out: all six submodules exist, all 14 re-exports resolve to real symbols, the fuzz module's `pub` justification (target shim + out-of-crate survival suite) is verifiably true, the VLC-differential "honestly pending infra" file exists and does exactly what its status says (computes the Lumen half, skips the VLC half), and the dependency record it cites exists. The Vorbis decode lane is compiled into the shipped binary and dispatched from two container lanes (Ogg, Matroska), but the baseline smoke corpus contains no Ogg/Vorbis sample, so the shipped lane has unit/differential evidence only — no end-to-end execution. The single hygiene finding is that the unconditional `pub mod fuzz` ships harness code in release binaries that no shipped caller invokes (a documented, repo-wide precedent — flagging it once here for the cross-codec ledger).

## 2. What actually works (verified)
— Re-export integrity: every name in the three `pub use` lists (70-76) resolves (symbol-by-symbol grep, see Commands run). A typo'd re-export would not compile, but the audit point is the doc's module map (18-33) matches the real surface — it does.
— The fuzz-`pub` claim ("the registered target file and the in-tree survival tests call it from outside the crate"): `fuzz/fuzz_targets/fuzz_codec_vorbis.rs` exists and `crates/codecs/tests/vorbis_fuzz_survival.rs:17` does `use codecs::audio::vorbis::fuzz::fuzz_codec_vorbis;` — both verified.
— The differential-posture claims: `tests/vorbis_bitexact.rs` and `tests/vorbis_vlc_differential.rs` both exist; the latter's PENDING-INFRA banner (lines 1-24) is accurate about what runs (ffmpeg-oracle pin asserted always-on, VLC half skipped) — verified by reading its head.
— Engine pin: `symphonia-codec-vorbis = "=0.6.1"` (codecs/Cargo.toml:103) matches the doc's "0.6.1" claim; the cited purity record `docs/dependencies/records/symphonia-vorbis-0.6.1.md` exists.

## 3. Findings

### F-001 [S4] `pub mod fuzz` ships fuzz-harness code in release binaries with no shipped caller
Where: crates/codecs/src/audio/vorbis/mod.rs:62-66
Code:
```rust
/// Fuzz harness (doc 04 §8.5 registry name `fuzz_codec_vorbis`):
/// `pub` because the registered target file and the in-tree survival
/// tests call it from outside the crate (the 2.B2/2.B4/5.A1
/// precedent).
pub mod fuzz;
```
What is wrong: nothing functionally — the `pub` is required for the out-of-crate callers (verified) and the justification is honest. The cost is that every shipped `lumen`/web release binary carries the whole vorbis fuzz harness (plus the same for flac/pcm/adpcm/mp3/mp2/aac) as dead weight, with no `cfg` to strip it from non-test builds.
Why it matters: binary-size debt and attack-surface surface area in a project that tracks wasm size budgets (docs/web/size-report.md); the "precedent" is a choice, not a law.
What correct looks like: `#[cfg(any(test, feature = "fuzz-harnesses"))]` on the fuzz modules with the fuzz package enabling the feature — the registry check (baseline #18) would need a matching tweak.
Confidence: HIGH — Verification: read + caller census (`rg "vorbis::fuzz" apps/` → no shipped caller).

## 4. Per-function table (module items — no functions in this file)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module doc (format slice, module map, conventions, differential posture) | 1-57 | REAL | — | Every structural claim verified (§2); the "no panic on any input" convention spans submodules owned by W07-A07/A08 — delegated, see §6. |
| `pub(crate) mod comments/decoder/error/headers/seek` | 59-61, 67-68 | REAL | — | All five files exist (ls). |
| `pub mod fuzz` | 66 | REAL | S4 (F-001) | Justification verified; ships in release binaries. |
| `pub use comments::VorbisComments` | 70 | REAL | — | comments.rs:41. |
| `pub use decoder::{DecodedVorbis, VorbisDecoder, decode_block}` | 71 | REAL | — | decoder.rs:212, 391, 318. |
| `pub use headers::{Identification, VorbisHeaders, is_vorbis_ident, parse_identification}` | 72 | REAL | — | headers.rs:54, 170, 76, 90. |
| `pub use seek::{cold_block_emission, join_emission, landing_first_sample, measure_shift, nanos_to_sample, sample_to_nanos}` | 73-76 | REAL | — | seek.rs:99, 106, 93, 77, 54, 44. |

## 5. Test honesty
No tests in this file. The submodules' and out-of-crate suites (`tests/vorbis_bitexact.rs`, `tests/vorbis_fuzz_survival.rs`, `tests/vorbis_vlc_differential.rs`, `tests/vorbis/`) are other agents' scope (W07-A07/A08 own decoder.rs parts); their existence and out-of-crate imports verified here.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "block decode through the pinned symphonia-codec-vorbis 0.6.1 engine" | module doc 10-11 | REAL | Cargo.toml:103 exact pin; record exists. |
| "Native-Vorbis-in-Matroska reuses the same header chain verbatim (the extra_data seam is container-agnostic)" | module doc 13-16 | REAL-UNPROVEN | MKV mints "vorbis" (header.rs:811) and harvests CodecPrivate naming "Vorbis headers" (header.rs:158-160) — the seam exists; no MKV-Vorbis end-to-end run exists in baseline (demux wave owns the proof). |
| "no panic on any input — every decode/parse path is bounds-checked" | module doc 37-43 | REAL-UNPROVEN (delegated) | The paths live in decoder/headers/seek/comments (W07-A07/A08); this file has no code. Cannot be verified from here; note the FLAC harness precedent found a violated sibling claim this wave (flac S2 is a *harness* defect, not a decoder panic). |
| "the VLC differential hook … is registered and honestly pending infra" | module doc 56-57 | REAL | vorbis_vlc_differential.rs:1-24 PENDING-INFRA banner does exactly this (Lumen half always-on, VLC half skipped). |
| "a digital silence fixture … gives the true byte-exact MD5 lane" | module doc 53-55 | REAL-UNPROVEN | The claim's proof lives in tests/vorbis_bitexact.rs (other agents' scope); file exists. |

## 7. Dead / duplicated / unreachable code
- The fuzz module in release binaries (F-001) — repo-wide pattern, flagged once.
- Nothing else: no dead re-export (each has at least one in-crate or out-of-crate consumer; `is_vorbis_ident`/`parse_identification` consumed by apps/cli demux_host.rs:189 and containers/ogg.rs).

## 8. Dependencies & flags
- Feature `codec-vorbis` (default on; codecs/Cargo.toml:57,62,80) — module compiled into shipped CLI/web builds.
- symphonia-codec-vorbis =0.6.1 (+ shared symphonia-core), purity record docs/dependencies/records/symphonia-vorbis-0.6.1.md.

## 9. Open questions for the next agent
- W07-A07/A08 (vorbis decoder parts): the mod doc's "one-block overlap scar … keeps decoding after a damaged packet" is the strongest convention claim — verify it in decoder.rs's damage path; if untrue it belongs in the cross-codec conventions ledger.
- Smoke/orchestrator: one Ogg-Vorbis sample in the rc-smoke corpus would move the whole lane from REAL-UNPROVEN to REAL at the entry level (the corrected-baseline precedent shows how much a single sample settles).
