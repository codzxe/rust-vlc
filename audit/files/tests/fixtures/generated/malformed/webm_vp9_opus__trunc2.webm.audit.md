# AUDIT: tests/fixtures/generated/malformed/webm_vp9_opus__trunc2.webm
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A05) |
| Size / sha256 | 441 B / 9529f6c3cb1e0b4fb7ad30b903a7a70a634fdd9c9cb539a63f609c4608c500f1 |
| Type | WebM — exact prefix of the valid parent (EBML header + early Segment element) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A data — consumed by 1 green demux test (below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 sha256/size/prefix-compare; file(1); rg consumers/MANIFEST/CORPUS; baseline greps |

## 1. One-paragraph verdict
Verified derivative: EXACT byte-prefix of the valid parent webm_vp9_opus.webm
(441 of 27,259 B, cut fraction 0.0162 — an early cut: EBML magic `1a 45 df a3`,
DocType webm, the Segment element's head, then truncation mid-header-level
elements). Consumed by the mkv_fixtures derivative loop, green in baseline.
The family is unregistered in the drift MANIFEST, and the loop pins outcome
SHAPE, not per-file codes — weaker than the ogg family's pinning table.

## 2. What actually works (verified)
— python3: 441 B; sha256 above; d == parent[:441] TRUE; parent sha256
8b5c6ba741cb843b5a0fc8009d6390de7cf127470c05197bfa35aff234685aea.
— Consumer green: baseline cargo-test-workspace.txt:2357
`malformed_derivatives_are_typed_or_recover_never_panics ... ok`
(crates/demux/tests/mkv_fixtures.rs:344-414: this file at :361 in the 18-name
list; MkvDemuxer::new → start → typed-refusal (stage Demux, code prefix
"demux.") or bounded walk with event-storm cap 1,000,000; checked==18 asserted).
— Generated, not handmade: `cargo xtask fixtures` B1 truncation
(truncate_derivative, tools/xtask/src/cmd/fixtures.rs:2515-2537).

## 3. Findings
F-001 [S3]
Where: tests/fixtures/generated/MANIFEST.toml + tests/fixtures/CORPUS.md (absences)
Code: `rg 'webm' tests/fixtures/generated/MANIFEST.toml tests/fixtures/CORPUS.md -> No matches found`
What is wrong: the webm_vp9_opus derivative family (9 files) is unregistered in
the MANIFEST drift gate and absent from CORPUS.md — no sha256/size pin; behavior
pins only. Same family defect as mkv_h264_aac (W16-RR2 :31) and mp4_h264_aac
(W16-RR2 :39, this run :10).
Confidence: HIGH — Verification: read (rg executed, zero hits).

F-002 [S4]
Where: crates/demux/tests/mkv_fixtures.rs:386-392 (the refusal arm)
Code: `assert_eq!(e.stage, media_types::PipelineStage::Demux, "{name}: {e}"); let code = e.context.code.as_str(); assert!(code.starts_with("demux."), "{name}: {code}");`
What is wrong: for the webm rows the loop pins only the stage + code prefix — no
per-file outcome (unlike ogg_fixtures.rs:446-530's DERIVATIVES table which pins
bytes/packets/reasons/terminal/damage per derivative). A regression that swaps
this file's typed code from, say, demux.mkv.truncated to demux.mkv.unknown-doctype
would still pass.
What correct looks like: a DERIVATIVES-style pin table for the webm family.
Confidence: HIGH — Verification: read (both consumer files compared).

## 4. Matrix evidence
Class-B corpus member; no feature-matrix row cites it; the demux-level half of
the class-B recovery contract (recovery_matrix.rs:6-9) is this mkv_fixtures loop.

## 5. Open questions for the next agent
- What typed code does trunc2 actually produce? The test prints it
  (println!("{name}: typed open refusal {code}")) but asserts only the prefix —
  run with --nocapture to record the specific code, then pin it.
