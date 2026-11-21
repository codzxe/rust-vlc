# AUDIT: apps/desktop/examples/emit_stream.rs
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-21 |
| Lines / functions | 252 / 6 items (single-binary example) |
| File verdict | REAL (its documented consumer does not exist) |
| Reachable from shipped entry point? | NO — it is a `cargo run -p desktop --example emit_stream` dev tool, not a shipped binary; compiled by baseline clippy `--all-targets` (runs #5/#6 CLEAN, which include examples); no baseline run executes it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `ls apps/desktop/prototype/` → only `lumen-frames.mjs` (no harness.mjs); `rg --files -g 'harness*'` → no harness under apps/desktop; `rg -ln "expected.json\|stream.bin"` (whole repo) → ZERO consumers outside audit/; `rg -n "harness.mjs" .github/workflows` → no CI invocation; `rg -n "emit_stream\|harness" docs/prototypes/video-in-webview.md apps/desktop/README.md` → both cite the phantom workflow; `rg -n "deterministic" crates/renderer-wgpu/src/synthetic.rs` → pattern determinism has its own test |

## 1. One-paragraph verdict
The generator itself is competently built: a fully deterministic LUF1 stream (fixed size schedule, 30 fps PTS grid, fixed virtual emit stamps, deterministic renderer-wgpu pattern with a marker pixel), a correct FNV-1a expectation JSON, an LCG-chunked decode measurement that exercises the real `WireDecoder`, and clean typed argument errors. But the tool's entire stated purpose is to feed "the Node conformance harness (`prototype/harness.mjs`)" — a file that does not exist anywhere in the tree, is invoked by no CI workflow, and whose documented command in this file's own usage block (`node apps/desktop/prototype/harness.mjs target/proto`) cannot run. The golden pair (`stream.bin` + `expected.json`) has zero in-tree consumers. The cross-language conformance story that wire.rs, this file, apps/desktop/README.md, and docs/prototypes/video-in-webview.md all tell is a pipeline with a generator and no checker.

## 2. What actually works (verified)
- Compiles clean under baseline clippy `--all-targets` (runs #5/#6, INDEX.md rows 5-6) — examples are in `--all-targets`.
- Determinism is real at every layer I could trace: `synthetic_pattern` is pinned deterministic by its own test (renderer-wgpu/src/synthetic.rs:93 `pattern_is_deterministic_and_sized`); PTS/emit stamps are fixed virtual values (`(i as i64) * 33_333_334`, `+ ENCODE_COST`); no clock, no randomness in the output path (the LCG only chunks the decode measurement, never the written bytes).
- The repeat-frame semantics are correct: every 7th delivery re-presents the previous slot — same extent, same marker (`(i-1) % 256`), same `pts_ns`, fresh record (lines 91-116), matching `FrameFlags.repeat` bit 0.
- Decode measurement drives the REAL `WireDecoder::feed`/`next_frame`/`finish` path with 1..=4096-byte chunks (the hostile-alignment shape), and `finish()` would catch a partial trailing record.
- Byte-identity check math verified by hand: header overhead 48/(1920·1080·4+48)·100 ≈ 0.00058% matches wire.rs's spec-table claim.

## 3. Findings

### F-001 [S3] The tool's documented consumer — `prototype/harness.mjs` — does not exist; the golden pair it writes has zero in-tree consumers
Where: apps/desktop/examples/emit_stream.rs:5-16 (doc block)
Code:
```rust
//! Emits a deterministic frame stream in the v1 wire format plus the
//! JSON expectation file the Node conformance harness
//! (`prototype/harness.mjs`) checks, and measures the encode/decode
//! wall-clock cost of the protocol on this machine (recorded
//! evidence — the 2.D1 staging precedent: measured, printed, never
//! asserted by tests).
//!
//! ```text
//! cargo run -p desktop --example emit_stream -- --out target/proto
//! node apps/desktop/prototype/harness.mjs target/proto
```
What is wrong: `apps/desktop/prototype/` contains exactly one file, `lumen-frames.mjs` (451 lines, the JS decoder reference). There is no `harness.mjs` anywhere in the repository (`rg --files -g 'harness*'` → harnesses exist only under tests/ and ui/, unrelated), no CI workflow invokes it (`rg harness.mjs .github/workflows` → no match), and nothing in the tree reads the golden pair (`rg "expected.json|stream.bin"` → zero consumers outside audit/). The second command of the documented two-step workflow fails with "Cannot find module".
Why it matters: this example exists to generate cross-language conformance evidence; the "cross-language" half is unverifiable with committed tooling. The same phantom citation appears in wire.rs:14-16 (my part1 audit, F-002), apps/desktop/README.md:23-36, and — most seriously — docs/prototypes/video-in-webview.md, whose "What WAS measured (this sandbox…)" list includes "the JS parser conformance runs over the Rust-emitted golden stream" as a measured result. A measured-evidence claim whose measuring instrument is absent is exactly the class of inflation this audit hunts.
What correct looks like: commit the harness (a ~100-line Node script reading expected.json + stream.bin and running lumen-frames.mjs's parser), wire it into a CI lane, or delete the claim from all four files and mark the JS-side conformance PENDING like the rest of the prototype to-run list.
Confidence: HIGH — Verification: read + `ls`/`rg` enumerations above (all four citation sites opened).

### F-002 [S4] Determinism self-check asserts the weak half and only prints the strong half
Where: apps/desktop/examples/emit_stream.rs:202-214
Code:
```rust
        let encode_ns = encode_start.elapsed().as_nanos() as u64;
        assert_eq!(sink.len(), stream.len(), "encode determinism (length)");
        let mut identical = true;
        for (a, b) in sink.iter().zip(stream.iter()) {
            identical &= a == b;
        }
        println!(
            "encode:  {} bytes in {} ns -> {:.1} MiB/s (byte-identical rebuild: {})",
```
What is wrong: the re-encode is length-asserted but byte-identity is only `println!`ed. The doc at line 21-22 claims "Two runs produce byte-identical outputs (the digest self-check in `--measure` mode re-proves it)" — no digest comparison happens in-process at all; a content divergence with equal length would print `byte-identical rebuild: false` and exit 0.
Why it matters: the tool's own determinism claim is enforced at its weakest link; anyone scripting `--measure` for CI (the documented future use) gets a passing run on a non-deterministic encoder.
What correct looks like: `assert!(identical, "encode determinism (bytes)")` (and optionally compare the FNV digests of sink vs stream, which the doc already implies exists).
Confidence: HIGH — Verification: read.

### F-003 [S4] frame_id convention off-by-one with the production bridge, undocumented
Where: apps/desktop/examples/emit_stream.rs:123 and apps/desktop/src/bridge.rs (contrast)
Code:
```rust
            frame_id: i as u64,
```
What is wrong: the example's stream starts `frame_id` at 0; the production `CanvasBridge` starts at 1 (`next_frame_id: 0` then ships `i+1` — pinned by apps/desktop/tests/zero_alloc_transport.rs:93 `assert_eq!(frame.header.frame_id, i as u64 + 1)`). Both are legal "monotonic per stream" values, but the reference stream that the (absent) JS consumer and humans compare against the bridge's output uses a different origin, and no doc on either side states the convention.
Why it matters: a conformance/golden tool whose numbering differs from the shipped producer invites false-mismatch debugging; the fix is one doc line or one `+ 1`.
Confidence: HIGH — Verification: read (both sites + the pinning test line).
Cross-ref: bridge.rs is W13-A06's file.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-22 | REAL (purpose claim FALSE) | S3 | F-001. |
| `fnv1a64` | 34-41 | REAL | — | Correct FNV-1a 64 constants (0xcbf29ce484222325 / 0x100000001b3); self-described as non-crypto conformance digest — honest. |
| `SIZES` | 44 | REAL | — | 64×48 … 640×360; `--big` appends 1080p (line 83). |
| `ENCODE_COST` | 49 | REAL | — | 5 ms virtual stamp; documented as matching the pipeline suite's constant. |
| `Entry` | 52-55 | REAL | — | — |
| `main` | 57-252 | REAL | S3/S4 | Arg parsing: typed errors, unknown-arg refusal, `--frames` parse error surfaces (69-73). Composition: repeat logic correct (91-116); marker pixel at (5,5) — safe for all schedule widths (min 64). `payload.len() as u32` safe (max 8,294,400). JSON emission: hand-rolled but all fields static/numeric — no escaping hazard; head/tail hex correct (`.rev().take(8).rev()`). Measurement: encode rebuild (F-002 weak assert); decode via real WireDecoder with LCG chunks 1..=4096 (terminates, `.min` guards the tail); `finish()` catches partial records; MiB/s and overhead math hand-verified. No unwrap/expect/panic path reachable from CLI args (all errors typed). |

## 5. Test honesty
No tests in this file (it is an example — the generator FOR tests). Its output is consumed by no test in the tree (F-001), so nothing would fail if this example's format drifted from wire.rs — except that `encode_into`/`WireDecoder` are the shared production functions, so format drift would fail wire.rs's own 14 tests. The determinism claim is only as strong as F-002's weak assert.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Evidence |
|---|---|---|---|
| "the JSON expectation file the Node conformance harness (`prototype/harness.mjs`) checks" | emit_stream.rs:5-7 | FALSE (consumer absent) | `ls apps/desktop/prototype/`; no CI invocation; no reader of expected.json. F-001. |
| "node apps/desktop/prototype/harness.mjs target/proto" (usage) | emit_stream.rs:14 | FALSE (unrunnable) | Same. |
| "Two runs produce byte-identical outputs (the digest self-check in `--measure` mode re-proves it)" | emit_stream.rs:21-22 | SHALLOW | Length asserted, bytes printed (F-002); no digest comparison executed. Pattern-level determinism itself is real (renderer-wgpu's own test). |
| "measures the encode/decode wall-clock cost … measured, printed, never asserted by tests" | emit_stream.rs:8-10 | REAL | Measurement code exists; numbers land on stdout only. |
| "the JS parser conformance runs over the Rust-emitted golden stream" (context: this tool's output) | docs/prototypes/video-in-webview.md §"What WAS measured" | FALSE (instrument absent) | Same enumeration as F-001 — doc-wave should re-verdict this row. |
| README "Emits the golden stream.bin + expected.json the Node harness checks" | apps/desktop/README.md:24,35-36 | FALSE (same phantom) | Same. |

## 7. Dead / duplicated / unreachable code
- The golden-pair OUTPUT (stream.bin/expected.json) is dead weight in the current tree — written, never read (F-001's root cause).
- `--big`'s 1080p size is used only by the measurement narrative; harmless.

## 8. Dependencies & flags
- `desktop::wire` (production codec), `renderer_wgpu` (synthetic pattern + Extent2D). No feature gates; example target of a workspace crate (clippy-clean in baseline). `std::time::Instant` only in the `--measure` path — output files never depend on wall-clock.

## 9. Open questions for the next agent
- Doc wave: re-verdict docs/prototypes/video-in-webview.md's "What WAS measured" list — the JS-parser-conformance bullet cites this tool plus a harness that does not exist; the latency/drop numbers there cite the apps/desktop/tests suites (32 tests) which DO exist and passed in baseline.
- W13-A06 (bridge.rs): the frame_id origin convention (F-003) — decide one origin and document it on the wire.
- If the orchestrator wants the conformance lane real: REQUEST-ORCH-RUN: `cargo run -p desktop --example emit_stream -- --out /tmp/proto && node apps/desktop/prototype/harness.mjs /tmp/proto` — expected to fail at the second command (module not found), demonstrating F-001 at the gate.
