# AUDIT: apps/desktop/examples/emit_stream.rs
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-24 |
| Lines / functions | 252 / 2 (`main`, `fnv1a64`) + 2 consts |
| File verdict | REAL (executed end-to-end: golden pair generated, determinism proven, measurement runs) — its documented consumer does not exist |
| Reachable from shipped entry point? | NO — a `desktop` crate example (`cargo run -p desktop --example emit_stream`); examples are not part of the release lane (`cargo build --release -p lumen-cli -p desktop` builds the lib, not examples). Dev/evidence tool. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/examples/emit_stream --out audit/runs/proto-test1` (and -test2) → exit 0, identical sha256 `ad369cab…`; `--measure --frames 240` → exit 0 (encode 1710.7 MiB/s, decode 3285.3 MiB/s, byte-identical rebuild: true); `node audit/runs/W13-A03-substitute-harness.mjs` → 8/8 checks, ALL PASSED, exit 0 |

## 1. One-paragraph verdict
The tool is real and I executed its whole documented pipeline minus one step: two `--out` runs produce byte-identical 34,932,960-byte streams (sha256 equal — the determinism claim holds), `--measure --frames 240` prints encode/decode throughput with a byte-identical rebuild, and I substituted for the missing second step myself — feeding `stream.bin` through `lumen-frames.mjs`'s FrameDecoder with the same LCG chunking and verifying all 90 headers, all 90 FNV-1a payload digests, and all 90 head/tail windows against `expected.json` (8/8 checks, exit 0). So the cross-language conformance the docs claim is substantively TRUE on this stream — but the documented command `node apps/desktop/prototype/harness.mjs target/proto` is UNRUNNABLE: that file does not exist anywhere in the repo, and no CI step runs it. The tool's one-line purpose statement points at a phantom consumer. Note the happy interaction with the lumen-frames.mjs audit: this tool's streams keep frame ids and emit stamps small (ids 0..89; stamps < 3.1 s), so the JS safe-integer guard never trips here — the mirror only fails on wire.rs's own unit-test vector (1.23e18 frame id).

## 2. What actually works (verified)
— `./target/debug/examples/emit_stream --out audit/runs/proto-test1` + `--out audit/runs/proto-test2` → both exit 0, `emit_stream: 90 frames, 34932960 stream bytes (34928640 payload + 4320 header)`, `sha256sum` identical across runs (`ad369cabaf72e977b062252a0f48f5d4626893c0013bfe12ef7d3091bdbe1158` twice).
— `./target/debug/examples/emit_stream --measure --frames 240` → exit 0: `encode: 92936448 bytes in 51808859 ns -> 1710.7 MiB/s (byte-identical rebuild: true)`, `decode: 240 frames … 3285.3 MiB/s, 112407 ns/frame`, header-overhead line printed.
— `node audit/runs/W13-A03-substitute-harness.mjs` (committed) → `ok` ×8 incl. "all 90 headers match expected.json", "all 90 FNV-1a payload digests match", "all 90 head/tail hex windows match", `SUBSTITUTE HARNESS: ALL CHECKS PASSED`, exit 0.

## 3. Findings
F-001 [S3]
Where: apps/desktop/examples/emit_stream.rs:5-14
Code: `//! Emits a deterministic frame stream in the v1 wire format plus the\n//! JSON expectation file the Node conformance harness\n//! (`prototype/harness.mjs`) checks, and measures the encode/decode\n//! wall-clock cost of the protocol on this machine (recorded\n//! evidence — the 2.D1 staging precedent: measured, printed, never\n//! asserted by tests).\n//!\n//! ```text\n//! cargo run -p desktop --example emit_stream -- --out target/proto\n//! node apps/desktop/prototype/harness.mjs target/proto`
What is wrong: The documented consumer does not exist: `find . -name 'harness.mjs'` → zero hits; no CI step invokes it. The usage block's second command is unrunnable as written, and the file's stated purpose ("the JSON expectation file the Node conformance harness checks") describes a check nothing performs. Why it matters: the §4 evidence chain in docs/prototypes/video-in-webview.md leans on this pair; a developer following the doc hits a missing file at step 2. (The pipeline's SUBSTANCE is real — my substitute harness passes 8/8 — so this is a broken pointer, not fabricated results.) What correct looks like: commit the harness (my audit/runs/W13-A03-substitute-harness.mjs is a working 60-line seed) or drop the reference. Confidence: HIGH — Verification: ran (substitute harness) + find/rg (absence).

F-002 [S4]
Where: apps/desktop/examples/emit_stream.rs:18-22 with 203-207
Code: `//! Determinism: the synthetic pattern (renderer-wgpu's, fully\n//! specified), fixed size schedule, 30 fps PTS grid, fixed virtual\n//! emit stamps (slot + 5 ms — the pipeline test's ENCODE_COST), FNV-1a\n//! payload digests. Two runs produce byte-identical outputs (the\n//! digest self-check in `--measure` mode re-proves it).` … (L203-206) `assert_eq!(sink.len(), stream.len(), "encode determinism (length)");\n        let mut identical = true;\n        for (a, b) in sink.iter().zip(stream.iter()) {\n            identical &= a == b;\n        }`
What is wrong: The doc calls it a "digest self-check", but `--measure` performs no digest comparison — it asserts only the LENGTH and PRINTS byte-identity (`identical` is a println argument, not an assert). A non-deterministic encoder would still exit 0 with "byte-identical rebuild: false". The "never asserted by tests" posture is admitted up front, so this is a mislabeled claim, not a cover-up. What correct looks like: assert `identical` in `--measure` (it is a dev tool; a hard failure is fine) and call it a byte-identity check. Confidence: HIGH — Verification: ran + read.

F-003 [S4]
Where: apps/desktop/examples/emit_stream.rs:220-221
Code: `// Chunked like a real IPC feed (the deterministic LCG split\n        // the Node harness mirrors).`
What is wrong: Third phantom-harness reference (with L6-7 and L14): the "Node harness [that] mirrors" the LCG split does not exist. My substitute harness reimplemented the LCG split from this code to make the conformance run — the comment describes intent, not an artifact. Confidence: HIGH — Verification: read + ran (my mirror of the split decodes the stream correctly).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fnv1a64` | 34-41 | REAL | — | Correct FNV-1a constants (0xcbf29ce484222325 / 0x100000001b3); my JS twin reproduced all 90 digests. |
| `SIZES` / `ENCODE_COST` consts | 44-49 | REAL | — | Fixed schedule; 5 ms virtual encode cost, matches the pipeline suite's constant (claimed; UNVERIFIED against that suite). |
| `main` (arg parse) | 57-79 | REAL | — | `--out/--frames/--measure/--big`; unknown arg → typed error. No `--help` (example-grade). |
| `main` (stream compose) | 81-129 | REAL | — | Executed: repeat-every-7th logic keeps payload byte-identical to the previous frame (deterministic pattern + `(i-1)%256` marker); `entries[i-1]` only at i≥7 (no underflow); marker write at (5,5) in-bounds for all sizes (min 64x48). |
| `main` (golden pair) | 131-192 | REAL | — | Executed: stream.bin + expected.json written; JSON fields verified by my harness (headers, digests, head/tail windows, counts). |
| `main` (measurement) | 194-250 | REAL | S4 | Executed; byte-identity printed not asserted (F-002); LCG chunking correct (mirrored in my harness). |

## 5. Test honesty
No tests in this file (it is a measurement tool; the module doc declares "never asserted by tests"). The claimed external check (harness.mjs) is absent — F-001. My substitute check is committed at audit/runs/W13-A03-substitute-harness.mjs (8/8, exit 0).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Two runs produce byte-identical outputs" | emit_stream.rs:21-22 | REAL | Executed: two `--out` runs, identical sha256. |
| "the JSON expectation file the Node conformance harness (`prototype/harness.mjs`) checks" | emit_stream.rs:5-7 | FALSE (pointer) | No harness.mjs exists (find: 0 hits); the check itself passes when performed (my substitute, 8/8). |
| "`node apps/desktop/prototype/harness.mjs target/proto`" (usage) | emit_stream.rs:14 | PAPER-ONLY | Unrunnable command — file absent. |
| "the digest self-check in `--measure` mode re-proves it" | emit_stream.rs:22 | FALSE (mislabeled) | `--measure` checks length + prints byte-identity; no digest comparison (F-002). |
| "measured, printed, never asserted by tests" | emit_stream.rs:9-10 | REAL | Matches the code's posture exactly. |

## 7. Dead / duplicated / unreachable code
None dead. The example itself is unreachable from shipped binaries by design (evidence tool); the prebuilt binary exists in target/debug/examples and runs.

## 8. Dependencies & flags
`desktop` (wire, and through it the crate's own encoder/decoder), `renderer_wgpu` (synthetic_pattern). No feature gates. Deterministic: no wall-clock values enter the golden pair (emit stamps are virtual; only `--measure` timings vary, and they are printed, not written).

## 9. Open questions for the next agent
- docs/prototypes/video-in-webview.md §4 quotes measured encode/decode numbers as evidence — do the quoted figures match a current run (my run: 1710.7/3285.3 MiB/s on this 2-core box)? Owner: the doc auditor.
- The golden pair lands in `target/` by default (never committed): if §4's evidence predates a format change, nothing in-tree would catch the drift except a re-run. Consider committing one small golden pair under tests/fixtures.
