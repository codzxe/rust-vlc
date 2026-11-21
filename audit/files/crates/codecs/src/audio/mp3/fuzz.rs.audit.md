# AUDIT: crates/codecs/src/audio/mp3/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-21 |
| Lines / functions | 214 / 5 (harness fn + 2 types + 2 tests) |
| File verdict | REAL (as a harness; see reachability) |
| Reachable from shipped entry point? | NO — by design: this is a fuzz harness, not a playback lane. Reachable from `fuzz/fuzz_targets/fuzz_codec_mp3.rs:19` (libFuzzer shim, verified present) and from the test suite (`tests/mp3_fuzz_survival.rs` drives `decode_stream` directly; the in-crate tests here drive `fuzz_mp3_stream`). The shipped `lumen` binary contains none of it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cat fuzz/fuzz_targets/fuzz_codec_mp3.rs` (shim exists, calls `codecs::audio::mp3::fuzz::fuzz_mp3_stream`); `rg -n "libfuzzer" fuzz/Cargo.toml` (scaffolding EXISTS: `libfuzzer-sys = "0.4.13"`, package `lumen-fuzz`); `rg -n "fuzz-registry" audit/runs/baseline/full-gate.log.txt` (`:47-48` CLEAN — 35 targets, diff empty; INDEX.md #18 PASS); `rg -n "mp3" audit/runs/baseline/test-workspace.log.txt` (in-crate fuzz tests `:418,:450` ok; survival suite ran `:1175`; full mp3 family green `:409-445`); `sed -n '1,60p' crates/codecs/tests/mp3_fuzz_survival.rs` (real LCG-mutation survival suite); Read on the full file. |

## 1. One-paragraph verdict
A real six-pass fuzz harness (ID3 walk, header walk, Xing/VBRI, full `decode_stream`, contract path, both seek planners) with a correctly narrow contract: absence of panic/hang/unbounded allocation, damage swallowed, outcomes surfaced as an assertable `FuzzOutcome`. All internal indexing goes through `data.get(...)` (no slice panics), the contract pass is bounded (`take(64)`), and the outcome type is honest (no fake "passed" boolean). The two defects are doc/lint level: the "Wiring status (honest)" paragraph is stale — it claims the cargo-fuzz target is merely "proposed" and the libfuzzer-sys/fuzz-Cargo.toml scaffolding absent, while both exist and the registry gate is green; and `id3v2_len` is parsed twice with a dead `let _ = len;`. The harness is not shipped code and the libFuzzer campaign itself has no baseline run evidence (compiles-on-stable claim UNVERIFIED).

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/test-workspace.log.txt:418,450` → `audio::mp3::fuzz::tests::{a_header_only_buffer_is_survived, empty_and_garbage_inputs_never_panic}` `ok` (baseline run #3); `:1175` → `tests/mp3_fuzz_survival.rs` executed in the same green run (the broader LCG-mutation coverage of the same decode surfaces).
— Verified by `cat fuzz/fuzz_targets/fuzz_codec_mp3.rs` + `rg libfuzzer fuzz/Cargo.toml`: the shim and scaffolding exist; `audit/runs/baseline/full-gate.log.txt:46-48` → "fuzz-registry: CLEAN — 35 targets, diff empty, corpora present" (INDEX.md #18).
— Verified by trace (read): every array access in the harness is `data.get(range)` (107, 160-162) — a wrapped/huge range yields `None`/`&[]`, not a panic; `first.offset + frame_size` cannot overflow on libFuzzer's bounded inputs, and even under wrap `get` returns `None`.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/mp3/fuzz.rs:22-33
Code:
```
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_mp3.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned
//! dependency decisions (AGENTS.md §3.2, C5) — so until they land
```
What is wrong: The "honest" wiring status is stale: `fuzz/fuzz_targets/fuzz_codec_mp3.rs` exists (verified — a real `fuzz_target!` shim calling this harness), `fuzz/Cargo.toml` carries `libfuzzer-sys = "0.4.13"` and the full 35-target package, and the registry gate is green (baseline full-gate.log.txt:47-48, "diff empty"). The paragraph describes a pre-scaffolding world that no longer exists.
Why it matters: The paragraph is the file's own honesty anchor ("Wiring status (honest)") — a reader using it to decide what is proven (in-crate sweeps only) would wrongly conclude the libFuzzer wiring is absent and the campaign infrastructure hypothetical. Stale honesty sections are worse than none: they get cited.
What correct looks like: Update to: target + scaffolding landed; campaign runs need nightly (`cargo +nightly fuzz run fuzz_codec_mp3`, as the shim's own comment says); CI wiring for the campaign is the remaining gap.
Confidence: HIGH — Verification: read + cat/rg of the named files + baseline gate log.

### F-002 [S4]
Where: crates/codecs/src/audio/mp3/fuzz.rs:94-103
Code:
```
    if let Ok(Some(len)) = id3v2_len(data) {
        outcome.id3 = true;
        let _ = parse_id3v2(data);
        let _ = len;
    }
    // 2. Header walk from post-ID3 (or 0).
    let start = match id3v2_len(data) {
        Ok(Some(len)) => len,
        _ => 0,
    };
```
What is wrong: `id3v2_len(data)` is computed twice (94 and 100) — the length probe re-parses the tag header — and the first result is immediately discarded with `let _ = len;` (a lint-shaped dead assignment) instead of being reused as `start`.
Why it matters: Redundant work per fuzz call (trivial) and a confusing dead statement in a file whose whole job is to be readable as the panic-free contract; the double-parse also invites the two call sites to drift if the probe signature changes.
What correct looks like: `let start = match id3v2_len(data) { Ok(Some(len)) => { outcome.id3 = true; parse_id3v2(data); len } _ => 0 };` — one call, no dead binding.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CountingSink` + `FrameSink::accept` | 55-65 | REAL | — | Saturating counter, no per-frame allocation; ignores the frame payload (contract-correct for a panic target). |
| `FuzzOutcome` struct | 68-80 | REAL | — | Observable digest; no fake success flag — honest shape. |
| `fuzz_mp3_stream` | 84-182 | REAL | F-001, F-002 | Six passes as documented; all input access via `get` (no panic path found); VBR TOC seek probed with two targets (12345/0) and `bytes.unwrap_or(1)`/`frames.unwrap_or(1)` defaults keep the planner in-domain; contract pass bounded at 64 frames, `break` on first decode error — note: early-break means per-packet error-RECOVERY isolation (error packet → next packet decodes) is NOT exercised on the contract path (the full-buffer `decode_stream` pass covers resync instead); `Layer::Reserved → "mp3"` descriptor is a fuzz-only mapping (the real provider lane never sees Reserved); `bytes.to_vec()` per frame — harness-only allocation, acceptable. |
| test `empty_and_garbage_inputs_never_panic` | 188-206 | REAL | — | Full-outcome equality for empty; garbage asserts !id3 + frames==0 (0xFF-fill must fail header validation) — would fail if the walk started accepting sync-only garbage. |
| test `a_header_only_buffer_is_survived` | 208-213 | REAL | — | 4-byte valid header, no body → frames==0 — pins the truncated-tail rule. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_garbage_inputs_never_panic (188) | YES (for the classes it covers) | Exact `FuzzOutcome` equality on empty; garbage asserts the walk rejects 0xFF-fill. Coverage is 2 inputs — the breadth lives in `tests/mp3_fuzz_survival.rs` (deterministic LCG mutants over 6 committed fixtures, green in baseline) and the owed libFuzzer campaign. |
| a_header_only_buffer_is_survived (208) | YES | Asserts the no-body walk yields zero frames. |
| External: tests/mp3_fuzz_survival.rs | YES | Per-mutant classification (Decoded/Typed) with recorded tally — real mutation testing of the same surfaces; executed green (baseline :1175 under run #3's 1681-passed total). |
| External: libFuzzer campaign | N/A — UNVERIFIED | No baseline run builds or runs the fuzz package (workspace build excludes it); the shim's "compiles on stable" claim is untested by any recorded run. REQUEST-ORCH-RUN (optional): `cargo check --manifest-path fuzz/Cargo.toml`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | module docs 17-20 | REAL | Code matches: all errors swallowed, outcome is a digest; in-crate tests + survival suite pin it. |
| "The cargo-fuzz target file is proposed … until they land the harness logic is proven by the in-crate deterministic smoke tests" | module docs 22-33 | FALSE (stale) | Target + scaffolding exist; registry gate green — F-001. |
| "Memory is bounded by the input: … walked_frames × 1152 × 2 × 4 bytes with frame count bounded by input_len / frame_size (frame_size ≥ 24)" | module docs 35-42 | REAL-UNPROVEN | The bound formula is coherent; `frame_size ≥ 24` depends on header.rs's table math (another agent's file) — not independently verified here. |
| fuzz registry row `fuzz_codec_mp3` (doc 04 §8.5) | fuzz/README.md:71 | REAL | Target exists; registry check CLEAN (full-gate.log.txt:47). |

## 7. Dead / duplicated / unreachable code
`fuzz_mp3_stream` is unreachable from shipped entry points (harness by design — not a defect). The duplicate `id3v2_len` call is F-002. No other dead code. The same stale wiring-status paragraph likely exists in the sibling fuzz.rs files (flac/aac/alac/adpcm/vorbis — all their shims exist per fuzz/README and the registry gate) — flag for those files' owners.

## 8. Dependencies & flags
Module rides `codec-mp3` (default-on). The harness itself is std+crate-only; the libFuzzer dependency lives in the excluded `fuzz` package (`libfuzzer-sys = "0.4.13"`), so the shipped workspace graph is unaffected. No unsafe.

## 9. Open questions for the next agent
- mp3/decoder.rs owner: the harness's early-`break` on contract-path error means post-error packet isolation is only exercised via `decode_stream`; if the decoder claims per-packet isolation (as FLAC does), a two-packet fuzz case (bad → good) would pin it.
- Whoever owns fuzz/README.md: confirm the registry table's "wiring status" column matches the now-landed scaffolding (the in-crate doc drift suggests the README may be stale too).
