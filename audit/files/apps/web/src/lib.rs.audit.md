# AUDIT: apps/web/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-02-14 |
| Lines / functions | 76 / 0 (9 module declarations + 6 re-export lines + crate docs) |
| File verdict | REAL |
| Reachable from shipped entry point? | WEB LANE ONLY — this IS the wasm composition root; artifact `lumen_web.wasm` (release, 1,294,311 bytes, sha256 `4a0f33…87` per docs/web/browser-matrix-20261110.md) proven built (rc-smoke addendum) and driven by a real Chromium run. Not in the native release lane; not bundled by the UI `vite build`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg (placeholders, OpaqueHandle, matrix record); ls docs/web; baseline + rc-smoke citations |

## 1. One-paragraph verdict
A clean crate root: 9 modules, all present and substantial (11,260 LOC total), no production placeholders (every `todo!/unimplemented!/unreachable!` in the crate sits in `#[cfg(test)]` bodies; the production `OpaqueHandle` paths answer typed errors — render/present.rs:188, 361-363), and honest v1 posture (decoder-less wasm module, single-threaded cooperative lane, capability snapshot at boot). The native test lane is real: 111 lumen-web tests passed in the baseline (:3343), driving boot→loadBlob→open→pump→events through the ABI dispatch core exactly as the header claims. The one defect is a stale evidence label: the header says browser-runtime behavior is `[VERIFY]` because "no browser exists in this sandbox, and nothing here claims it was exercised" — while the repo's own committed browser-matrix record documents a real Playwright Chromium run (157.0.8103.50) driving the real Worker, real wasm instantiation, and real fetch transport. The doc underclaims the evidence; it is factually wrong about the repo's state.

## 2. What actually works (verified)
— Module inventory: abi (811), audio (1,152 across 4), capabilities (440), host (2,651), io (2,899 across 5), open (575), protocol (417), render (2,367 across 4), source (277) — all real files, none empty/stub.
— baseline cargo-test-workspace.txt:3324-3451: PACKAGE lumen-web, 111 passed / 0 failed (incl. abi dispatch, pump slices to end, autoplay gate, surface-loss fallback).
— baseline rc-smoke.txt addendum: `cargo build -p lumen-web --target wasm32-unknown-unknown --locked` completes after the rust-std component install; web-ui-browser-load: PASS (Chromium 143, 0 page errors).
— Browser-matrix evidence: docs/web/browser-matrix-20261110.md — "The chromium lane is a real Playwright run (evidence tier `playwright-run`)", wasm artifact sha256-stamped, fixture-driven, honesty notes for never-executed Firefox/WebKit/Safari lanes.

## 3. Findings
F-001 [S4]
Where: apps/web/src/lib.rs:57-59
Code:
```
//! transport, `performance.now` arming) is `[VERIFY]` — no browser
//! exists in this sandbox, and nothing here claims it was exercised.
```
What is wrong: stale and factually wrong about the repo. The committed record docs/web/browser-matrix-20261110.md proves a real Playwright Chromium run (captured 2026-11-11) exercised exactly those things: real Worker instantiation (harness.js:85 `new Worker("worker.js", { type: "module" })`), real postMessage transport, real wasm instantiation of the release artifact. The header predates card 7.I1 (issue #49, the browser-matrix lane) and was never updated.
Why it matters: evidence labels are load-bearing in this project's audit posture; a `[VERIFY]` marker that contradicts committed tier-1 evidence misleads the next reader (in the conservative direction here, but it is still a false doc claim).
What correct looks like: point the label at the matrix record (`docs/web/browser-matrix-20261110.md`, tier `playwright-run`) and keep `[VERIFY]` only for the never-run browser lanes.
Confidence: HIGH — Verification: read (record + harness).

F-002 [S4]
Where: apps/web/src/lib.rs:14-15
Code:
```
//! 2. a hand-rolled `extern "C"` ABI over shared linear memory
//!    ([`abi`]) — **zero new dependencies**: doc 09 §2.2 recommends
```
What is wrong: "zero new dependencies" is true only for the ABI boundary (no wasm-bindgen); the crate itself adds serde + serde_json (Cargo.toml dependencies), admitted there as "the command layer's own wire family". As written in the crate-root doc, a reader concludes the crate has no external deps at all. Ambiguity, not a lie.
Confidence: HIGH — Verification: read (Cargo.toml [dependencies]).

## 4. Per-function table
Not applicable (76 LOC, no functions). Module inventory verified in §2.

## 5. Test honesty
No tests in this file. Crate-level test honesty verified via the modules' own suites (baseline 111/111) and my per-file audits of open.rs/protocol.rs/source.rs paths.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the wasm32 module ships the engine core WITHOUT the builtin decoders" | L43-46 | REAL | Cargo.toml: wasm32 codecs dep optional, `default-features = false`, no default feature enabled; capability snapshot rows (capabilities.rs:376-392 tests). |
| "Native builds link the real MJPEG + PCM decoders so the headless lane proof decodes for real" | L47-48 | REAL | rc-smoke headless-decode PASS: v_frames=60 v_decoded=60 a_frames=60 a_decoded=60 (:131). |
| "a `cargo test` lane drives the SAME message paths ... through the wasm ABI's dispatch core" | L52-56 | REAL | baseline :3328-3443 (abi/host tests). |
| "Browser-runtime behavior ... is `[VERIFY]` — no browser exists in this sandbox, and nothing here claims it was exercised" | L57-59 | FALSE | F-001: browser-matrix-20261110.md is a committed `playwright-run` record exercising exactly this. |
| "the 64-command / 23-event catalog rides inside, it is never re-defined here" | L11-13 | REAL | protocol.rs re-uses commands::Wire* (verified in protocol.rs audit). |

## 7. Dead / duplicated / unreachable code
None in this file. Cross-module: io/staged_source.rs H4 lane is built+tested but unwired (see open.rs audit F-001) — the crate-root docs do not mention that gap either way.

## 8. Dependencies & flags
commands, media-types, plugin-api, codec-api, media-probe, demux, media-clock, playback-engine, audio-engine, audio-output (+serde/serde_json); native-only codecs edge; wasm32 opt-in codec features (none default). crate-type cdylib+rlib — the rlib is what the native test lane links.

## 9. Open questions for the next agent
- Should the `[VERIFY]` label's correction (F-001) also update docs/web/web-column-pass-20261110.md cross-references?
- The H4 staged-source wiring gap (open.rs F-001): is there a card that closed 7.H4's source side but not its open-path wiring? Check card/issue notes before calling it abandoned.
