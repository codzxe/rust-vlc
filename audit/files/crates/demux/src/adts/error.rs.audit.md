# AUDIT: crates/demux/src/adts/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Lines / functions | 252 / 17 (mint + 14 constructors + 2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli `containers/mod.rs:52` "adts" → `adts::open` → RawAacDemuxer, which calls every constructor (crates/demux/src/adts/mod.rs:250-942, rg-verified call sites for all 14) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p demux --lib adts::error` → 2 passed (exit 0); rg call-site census for all 14 constructors |

## 1. One-paragraph verdict
A clean, fully-wired typed-error factory: 14 constructors each minting an EngineError
at stage Demux with a stable `demux.adts.*` code, all 14 reachable from the ADTS/ADIF
demuxer that the CLI opens for probe id "adts". The in-module tests execute and pass,
enumerating every constructor's stage/code/Display shape. The only defect is a
doc-precision one: the "# No secrets" header claims constructors take "byte counts
and offsets only — never paths, URIs or payload content", but `not_adts` takes and
stores a byte slice of the file's leading bytes (capped to 16 in Display, but stored
at caller-chosen length).

## 2. What actually works (verified)
- `cargo test -p demux --lib adts::error` → errors_carry_demux_stage_and_adts_codes
  + not_adts_sniff_capped_by_display, both pass — every one of the 14 codes pinned,
  Display prefix "[demux] " and suffix "(code)" asserted.
- rg census: every constructor has live call sites in adts/mod.rs (bad_descriptor
  at :250/:291, bad_header :489/:531, truncated :496/:535/:541/:673/:714,
  bad_sampling_index :524, bad_channel_config :527, crc_mismatch :538, not_adts
  :546/:555, adif family :567-:600, recovery_budget_exhausted :734, not_started
  :764/:871, seek_out_of_range :909/:912/:934/:942). No dead constructors.
- recovery_budget_exhausted binds the real budget constant
  `super::MAX_RECOVERY_SCAN_BYTES` (adts/mod.rs) — the error reports the true limit.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/adts/error.rs:9-12 vs 26-32
Code:
```
//! # No secrets
//!
//! Constructors take byte counts and offsets only — never paths, URIs
//! or payload content.
```
vs
```
pub fn not_adts(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("adts".to_string()),
                sniffed: sniffed.to_vec(),
```
What is wrong: not_adts takes file-content bytes (the sniff window) and embeds them
in the error — the "byte counts and offsets only" claim is false for this
constructor. The blast radius is small: Display caps the sniff at 16 bytes (pinned
by the executed test), and callers pass a 16-byte window (adts/mod.rs:546
`read_prefix_at(16, from)`), but a misidentified secrets file's first 16 bytes would
still surface in an error message. What correct looks like: scope the doc claim
("offsets, counts, and a capped 16-byte sniff window"). Confidence: HIGH —
Verification: read + ran the cap test.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 19-21 | REAL | — | stage+code factory |
| not_adts | 26-36 | REAL | S4 | sniffed bytes vs "No secrets" doc (F-001) |
| bad_header | 40-49 | REAL | — | InvalidStructure + offset |
| bad_descriptor | 56-63 | REAL | — | documented panic-free residue; call sites exist |
| bad_sampling_index | 69-76 | REAL | — | first-frame declaration refusal |
| bad_channel_config | 83-92 | REAL | — | channel 0 = PCE-in-band refusal, documented |
| crc_mismatch | 99-108 | REAL | — | header-half CRC scope documented |
| truncated | 114-121 | REAL | — | expected/found byte counts |
| recovery_budget_exhausted | 125-134 | REAL | — | real budget constant bound |
| adif_vbr | 141-151 | REAL | — | honest VBR refusal, ffmpeg-parity noted |
| adif_multi_program | 156-166 | REAL | — | decode-domain split refusal |
| adif_bad_params | 172-179 | REAL | — | umbrella for 5 param classes |
| adif_no_data | 182-192 | REAL | — | Truncated{1,0} |
| not_started | 195-200 | REAL | — | contract-order guard |
| seek_out_of_range | 204-209 | REAL | — | 4 call sites in seek path |
| tests: errors_carry… | 216-243 | REAL | — | all 14 codes pinned; executed |
| tests: not_adts_sniff… | 246-250 | REAL | — | 16-byte Display cap; executed |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_adts_codes | YES | any stage/code/Display-format regression fails it (14 cases) |
| not_adts_sniff_capped_by_display | YES | uncapped sniff Display fails the <=16 "ee" count |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Constructors take byte counts and offsets only — never paths, URIs or payload content" | error.rs:10-12 | SHALLOW | not_adts stores sniff bytes (F-001); no paths/URIs anywhere (rg-verified) |
| "sniffed carries the leading bytes … capped by the taxonomy's Display" | error.rs:24-25 | REAL | executed cap test |
| "Unreachable from parsed data" (bad_descriptor) | error.rs:52-53 | REAL-UNPROVEN | static-ASCII mapping claim; not falsifiable without the descriptor grammar, call sites never fire per doc |
| "ffmpeg has no ADIF demuxer at all" | error.rs:139-140 | UNVERIFIED | external-tool claim, ffmpeg not runnable in sandbox |

## 7. Dead / duplicated / unreachable code
None — all constructors called; bad_descriptor is defensive-but-wired (map_err at
mod.rs:250/291), honestly documented as such.

## 8. Dependencies & flags
media_types taxonomy only; no feature gates; no unsafe.

## 9. Open questions for the next agent
- Whoever audits adts/mod.rs: confirm the seek path's 4 seek_out_of_range sites
  (:909-:942) cover the linear-scan budget exhaustion case the doc mentions.
