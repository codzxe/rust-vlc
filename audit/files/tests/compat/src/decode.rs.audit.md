# AUDIT: tests/compat/src/decode.rs
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 561 / 11 functions + 3 types + 2 sink impls |
| File verdict | REAL-UNPROVEN (real driver code, no own tests, crate not in baseline) |
| Reachable from shipped entry point? | NO — standalone test tool (`compat-matrix`, run via `cargo run -p` per main.rs:64-66). Not shipped. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | Read full file; rg callers (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
Real decode-pipeline driver (no stubs): opens files through the genuine player composition
(`lumen_cli::demux_host::open_source`, `HostDecoderProvider`), routes packets per track, and
serializes typed per-track outcomes. The design is honest about refusals-as-records. But the
core driver has ZERO unit tests of its own (sibling modules have 31; decode.rs has none —
rg "#\[test\]" -> no matches), its crate is a standalone workspace ABSENT from the committed
per-package baseline run, so its only run evidence is the historical jsonl (2026-11-10, from a
different checkout path). One real classification defect: decode-stage fatals are recorded as
`NoDecoder` rows, mislabeling "decoder existed and died" as "no decoder in this build".

## 2. What actually works (verified)
- Real composition verified by import graph: `use lumen_cli::demux_host::open_source;` /
  `use lumen_cli::pipeline_host::HostDecoderProvider;` (lines 31-32) — the real open path,
  not a mock. Callers: tests/compat/src/report.rs:298 `decode::decode_file(&file.rel, &file.path, file.dump_pcm)`
  and oracle.rs (DecodeDump consumer).
- Historical run evidence: docs/corpus/compat-matrix-20261110.raw-rows.jsonl:39 records a
  genuine refused_demux outcome for a flip4 fixture — the driver demonstrably ran once.
- Budget exhaustion is typed, not a panic (verified sed -n '286,295p'):
  `self.consecutive = self.consecutive.saturating_add(1);` / `if self.consecutive >= CONCEALMENT_BUDGET {` / `self.fatal = Some(...)`.

## 3. Findings
F-001 [S2]
Where: tests/compat/src/decode.rs:476-484
Code (verified):
```
        if let Some(message) = fatal {
            // A decode-stage fatal on an otherwise-open file: record
            // it as the honest NoDecoder-shaped refusal (the limb
            // produced no complete decode).
            tracks.push(TrackOutcome::NoDecoder {
                codec: codec_of(&facts, limb.track).unwrap_or_default(),
                message: format!("decode fatal: {message}"),
```
What is wrong: a mid-walk decode fatal (budget exhaustion included) is serialized under the
`no_decoder` status tag. The enum's own doc (lines 176-183) defines NoDecoder as "No decoder in
this build for the track's codec". Any consumer aggregating the matrix (compat transitions,
parity reports) counts broken decodes as missing decoders — support-coverage statistics and
diff-vs-VLC classification are both wrong for these rows.
What correct looks like: a `DecodeFatal { message }` TrackOutcome variant.
Confidence: HIGH — Verification: read.
Severity rationale: the file feeds the compat-matrix evidence chain (docs/corpus + xtask
compat_matrix transitions); misclassified evidence rows are S2.

F-002 [S3]
Where: tests/compat/src/decode.rs:49-57 + doc lines 40-48
Code (verified):
```
fn per_frame_damage(error: &EngineError) -> bool {
    matches!(
        &error.kind,
        ErrorKind::Corrupt { .. }
```
What is wrong: hand-mirrored classification of "per-frame damage" ("mirrored from the CLI
bench, which mirrors playback-engine; the engine's own is crate-private"). Three copies of
one rule (engine, cli bench, this) with no shared source — if the engine reclassifies a cause,
this matrix silently conceals-or-fatals differently from the player.
What correct looks like: the engine exporting its classifier (or a contract test that fails
on drift). Confidence: HIGH — Verification: read.

F-003 [S3]
Where: tests/compat/src/decode.rs (entire file) + tests/compat/Cargo.toml:27-76
What is wrong: the crate's CORE module has no #[test]; the crate is a standalone workspace so
`cargo test` baselines (audit/runs/baseline/cargo-test-workspace.txt, all PACKAGEs listed)
never ran it. UNVERIFIED-run for the current tree; the only evidence is a 16-day-old jsonl
from another checkout path (`/home/z/my-project/wt/w24d/...`).
Confidence: HIGH — Verification: rg (no #[test] in file; no PACKAGE compat-matrix in baseline).
What correct looks like: unit tests for Limb::feed outcome handling + decode_file refusal
routing, and a lane in the gate that runs the crate.

F-004 [S4]
Where: tests/compat/src/decode.rs:465-474 (verified): `pcm: audio.payload.unwrap_or_default()`
What is wrong: when `dump_audio` is false the dump is still constructed (empty pcm, real md5) —
a DecodeDump whose pcm silently doesn't match md5_hex. Consumers must know the convention.
Also 111-117: a non-audio frame on an audio limb inflates `format_changes` (a
format-anomaly counter repurposed as wiring-anomaly counter).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CONCEALMENT_BUDGET / per_frame_damage | 44-57 | REAL | S3 | mirrored rule (F-002) |
| AudioDigestSink::{new,digest_hex,accept} | 63-135 | REAL | S4 | format_changes dual use (F-004) |
| sample_name | 99-108 | REAL | — | total enum match |
| CountSink::accept | 146-151 | REAL | — | counts only, by design |
| TrackOutcome / FileOutcome / FileRecord | 154-232 | REAL | S2 | NoDecoder mislabel (F-001) |
| Limb::feed | 270-300 | REAL | — | budget exhaustion typed |
| Limb::drain | 303-320 | REAL | — | flush error only if no prior fatal |
| decode_file | 332-518 | REAL | S2/S4 | main driver; mislabel + dump wart |
| codec_of | 521-527 | REAL | — | linear facts lookup |
| refused_record | 530-560 | REAL | — | stage-tagged refusals |

## 5. Test honesty
No tests in this file — N/A. Sibling coverage exists (oracle.rs 11, report.rs 9, sha256.rs 5,
config.rs 6 tests) but none exercise decode_file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decodes the committed + regenerable corpus through the real Lumen open/decode path" | tests/compat/Cargo.toml:48 | REAL-UNPROVEN | imports real seams (lines 31-32); historical jsonl; no current run |
| "Every outcome is a typed, serializable record — ... never a crash" | decode.rs:21-23 | REAL | refusal paths return Ok(record) (347-352, 530-560); panics absent (no unwrap/expect/index in file) |

## 7. Dead / duplicated code
`DecodeDump.sample` duplicates `TrackOutcome::Decoded.sample`. `first_audio` flag retained
after sink choice (used only at 465-474). No dead functions found.

## 8. Dependencies & flags
Path deps only (lumen-cli, commands, codec-api, media-types, plugin-api, playback-engine,
bench-vs-vlc, serde/serde_json) — matches Cargo.toml:54-75. unsafe_code denied crate-wide.

## 9. Open questions for the next agent
- Does report.rs's row rendering use the NoDecoder rows to compute "support" percentages?
  (W-next: audit tests/compat/src/report.rs — the mislabel lands there.)
