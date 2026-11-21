# AUDIT: crates/demux/src/adts/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Lines / functions | 251 / 17 (14 constructors + `mint` + 2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (every constructor has a live call site in `crates/demux/src/adts/mod.rs` — verified by `rg -n "error::<name>" adts/mod.rs`: lines 250, 291, 489-600, 673-948; the ADTS lane is dispatched at `apps/cli/src/containers/mod.rs:52` `"adts" => adts::open(...)`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read; `rg -n "error::(not_adts|bad_header|…)" crates/demux/src/adts/mod.rs` (call-site census — no dead vocabulary, unlike aiff's `payload_too_large`); `rg -n "enum LimitKind" -A 10 crates/media-types/src/error.rs` (taxonomy variants); `Grep "LimitKind::" crates/demux/src` (cross-demuxer convention census); `rg -n "adts" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (error tests + 24 demuxer tests + corpus 3/3 ok) |

## 1. One-paragraph verdict
Clean typed-error module: 14 constructors, each minting a stable `demux.adts.*` code at
stage `Demux` with the right `ErrorKind`/`CorruptDetail` shape, every constructor wired
to at least one live call site, and a test that pins all 14 codes + stage + Display
format (plus a real sniff-redaction cap test) — all green in the committed baseline.
One hygiene defect: the recovery-scan budget exhaustion is reported as
`LimitKind::Memory` when the budget is a *scan depth in bytes*; the taxonomy offers
`ProbeDepth` ("Probing depth in bytes") and `RecoveryAttempts`, and the MP4 error
module already uses `RecoveryAttempts` for its resync limit — same concept, two
different `LimitKind`s across demuxers, and the Display tells the user a memory
ceiling was hit when none was.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt` lines 1492-1493: `adts::error::tests::errors_carry_demux_stage_and_adts_codes ... ok` (all 14 codes), `not_adts_sniff_capped_by_display ... ok` (64 sniffed bytes → ≤16 "ee" hex matches in Display — the redaction cap is enforced and tested).
— verified by baseline lines 1494-1517 and 1957-1962: 24 adts demuxer tests + `corpus_adts_lanes_match_the_ffmpeg_truth`, `corpus_adts_seek_roundtrip`, `corpus_adts_survive_the_fuzz_harness` all ok — the codes this module mints are exercised end-to-end against ffmpeg-produced corpus files.
— verified by read + rg: no dead vocabulary — contrast `aiff/error.rs:141` (`payload_too_large`, "Vocabulary without a minting site today"), every constructor here is reachable.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/adts/error.rs:125-133
Code:
```rust
pub fn recovery_budget_exhausted(scanned: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget: super::MAX_RECOVERY_SCAN_BYTES,
            attempted: scanned,
        },
        "demux.adts.recovery-budget",
    )
}
```
What is wrong: `MAX_RECOVERY_SCAN_BYTES` (adts/mod.rs:121, 4 MiB) caps how many bytes
the resync scan may *read* — a scan/probe depth, not a memory ceiling. The taxonomy
has `LimitKind::ProbeDepth` ("Probing depth in bytes") and `RecoveryAttempts` ("A
recovery attempt budget", used by `mp4/error.rs:164` for the same resync-exhaustion
concept). Reporting it as `Memory` renders a misleading limit kind in Display and
gives limit-keyed metrics/telemetry a wrong axis. The ogg twin
(`ogg/error.rs:156-165`) makes the identical choice, so the drift is cross-module.
Why it matters: Error-kind consumers (recovery matrix, dashboards) group by
`LimitKind`; a scan-depth exhaustion filed under Memory miscounts memory incidents.
What correct looks like: `LimitKind::ProbeDepth` with `budget =
MAX_RECOVERY_SCAN_BYTES` (bytes probed), or `RecoveryAttempts` if the semantics are
per-attempt — and the same fix in ogg/error.rs.
Confidence: HIGH — Verification: read (taxonomy enum at media-types/src/error.rs:171-181; mp4/error.rs:164 contrast; adts/mod.rs:691 confirms the budget is a byte-scan bound)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` | 19-21 | REAL | — | shared constructor, stage pinned |
| `not_adts` | 26-36 | REAL | — | `Unsupported` + FormatInfo; sniffed slice bounded at the call sites (16 bytes at mod.rs:546, head at 555) — no amplification |
| `bad_header` | 40-49 | REAL | — | `InvalidStructure` + offset |
| `bad_descriptor` | 56-63 | REAL | — | documented unreachable (static ASCII literals from profile bits); honest residue of the 8.A1 sweep |
| `bad_sampling_index` | 69-76 | REAL | — | first-frame declaration refusal; mid-stream is resync-skip damage (correct split, documented) |
| `bad_channel_config` | 83-92 | REAL | — | channel_configuration 0: legal-but-decode-domain → typed refusal (never invent) — correct per MP §8 |
| `crc_mismatch` | 99-108 | REAL | — | header-half CRC only, scope honestly documented (raw-data-block CRC is decode-domain) |
| `truncated` | 114-121 | REAL | — | expected=frame end, found=stream length — real numbers, no fabrication |
| `recovery_budget_exhausted` | 125-133 | REAL | F-001 | wrong LimitKind (F-001) |
| `adif_vbr` | 141-151 | REAL | — | honest VBR refusal with ffmpeg/GStreamer precedent named |
| `adif_multi_program` | 156-166 | REAL | — | decode-domain split refusal |
| `adif_bad_params` | 172-179 | REAL | — | covers 5 structural ADIF defects (one code — coarser than the ADTS side's per-cause codes, cosmetic) |
| `adif_no_data` | 182-192 | REAL | — | `Truncated { expected: 1, found: 0 }` — minimal honest encoding ("at least one raw byte"); acceptable, noted |
| `not_started` | 195-200 | REAL | — | misuse typed |
| `seek_out_of_range` | 204-209 | REAL | — | both refusal causes named in doc |
| test `errors_carry_demux_stage_and_adts_codes` | 215-243 | REAL | — | all 14 codes + stage + Display suffix pinned |
| test `not_adts_sniff_capped_by_display` | 245-250 | REAL | — | redaction cap enforced and tested |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_adts_codes | YES | exact code+stage+Display per constructor; renaming a code or changing the kind fails it |
| not_adts_sniff_capped_by_display | YES | a Display that leaks the full 64-byte sniff would show >16 "ee" matches |
Both green in baseline (1492-1493). The demuxer-level suite (1494-1517) additionally
pins the *minting* behavior: `crc_mismatch_is_typed_and_good_crc_walks`,
`frame_shorter_than_its_header_is_typed_bad_header`, `adif_vbr_is_the_typed_refusal` —
the codes are exercised by real parse outcomes, not just constructor smoke.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Constructors take byte counts and offsets only — never paths, URIs or payload content" | module doc 9-12 | REAL | signatures verified (offsets/counts; `sniffed` is leading bytes for the format hint, capped in Display by test) |
| "this module mints EngineError values with adts-scoped stable codes at stage Demux" | module doc 3-5 | REAL | test pin + baseline |
| (implied) limit kinds are semantically accurate | ResourceLimit usage | FALSE at one site | F-001 |

## 7. Dead / duplicated / unreachable code
None. `bad_descriptor` is documented-unreachable but deliberately kept as the
panic-free residue — correct pattern. The module doc itself notes the R3
consolidation plan (per-crate error files mirror each other by scope discipline, not
accident).

## 8. Dependencies & flags
`media_types` only; no features, no unsafe, no unwrap/expect, no I/O. The one
cross-module coupling is `super::MAX_RECOVERY_SCAN_BYTES` (the budget constant lives
in the demuxer module — fine).

## 9. Open questions for the next agent
- The ogg twin (`ogg/error.rs:156-165`) shares F-001 — whoever owns ogg/error.rs
  should carry the same LimitKind fix (not in my file list).
- `adif_bad_params` bundles five distinct structural failures under one code while
  the ADTS side uses per-cause codes — worth splitting when the recovery matrix
  starts keying on ADIF causes.
