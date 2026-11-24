# AUDIT: apps/web/src/abi.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-14 |
| Lines / functions | 811 / 35 (16 AbiWorker methods + Default + worker/lock_worker + 6 exports + 4 PortDouble + 6 tests + send_words closure) |
| File verdict | REAL-UNPROVEN (dispatch core REAL by baseline test run; the `extern "C"` export surface and the browser lane never executed anywhere) |
| Reachable from shipped entry point? | NO — apps/web is outside the lumen-cli/desktop release lane; the product UI never spawns the worker (`rg "new Worker" ui/src` -> 0 hits); only the e2e matrix harness (ui/e2e/web/site/harness.js) drives it. wasm artifact builds (rc-smoke.txt:134-146, worklog:746: 6 exports + memory verified in the real binary). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | rg/sed/awk/Read over abi.rs, worker.js, host.rs, protocol.rs, lib.rs, Cargo.toml; rg over ui/src, scripts/, docs/web, worklog.md; baseline citation cargo-test-workspace.txt:3331-3360 |

## 1. One-paragraph verdict
The dispatch core (`AbiWorker`) is genuine, bounded, well-tested logic: word-lane input with a typed overflow latch, destructive-drain output staging with an idempotence flag, a raw frame lane, and six native tests that all pass in the orchestrator baseline. The module's honesty discipline is otherwise high (`[VERIFY]` labels on the browser lane). The hostile findings are structural: (1) the downstream word lane costs ONE FFI call per 4 bytes for the entire message INCLUDING media bytes serialized as a JSON number array — ~8.4M mutex-locking FFI calls plus ~4x JSON inflation and ≥5 full-buffer copies to load one ≤~8MB file — while the doc claims this was "measured cheap enough for v1" with zero measurement existing anywhere in the repo; (2) the exported `extern "C"` functions — the actual JS-facing surface — have ZERO test or harness coverage even headless (tests call the inner methods only); (3) the whole lane exists to feed a wasm module that ships decoder-less, so in the default build the big message it is optimized around (`loadBlob`) feeds an engine that refuses every open with `web.decoders_absent`.

## 2. What actually works (verified)
— `abi::tests::*` 6/6 pass (baseline cargo-test-workspace.txt:3331-3360: `the_lane_proof_over_the_abi`, `the_frame_lane_stages_the_present_bytes`, `the_word_lane_matches_the_json_lane`, `malformed_input_is_typed_and_recoverable`, `the_input_bound_is_typed`, `output_addr_and_len_describe_the_staged_bytes` all `ok`). Not re-run here (cargo budget; baseline cited per spec §2).
— Export surface exists in the real artifact: worklog.md:746 "parsed the export section of the real 5,989,631-byte lumen_web.wasm — all 6 function exports present (write_u32, dispatch, output_addr, output_len, NEW frame_addr, NEW frame_len) + memory (kind 2, index 0)"; wasm build passes in rc-smoke.txt:134-146.
— `MAX_PENDING_MESSAGES` (4096) IS enforced — host.rs:989-1003 refuses typed when the queue is undrained (quote in host.rs audit domain, verified by Read).

## 3. Findings
F-001 [S2]
Where: apps/web/src/abi.rs:24-30, 91, 155-162; apps/web/src/protocol.rs:69-75; apps/web/worker.js:62-75
Code:
```
//!   `TextEncoder`, pads the byte string to a multiple of four with
//!   SPACES (JSON permits trailing whitespace), and pushes it one
//!   `u32` word at a time through [`lumen_web_write_u32`]. One
//!   call per 4 bytes — the no-`unsafe` price, measured cheap
//!   enough for v1 (a `loadBlob` message is the large one; the raw
//!   linear-memory blob lane is card 7.H4's follow-up).
```
plus protocol.rs:72-74:
```
        /// The container bytes (JSON array in v1 — see the module
        /// docs).
        bytes: Vec<u8>,
```
What is wrong: the entire downstream message — including media bytes as a JSON *number array* — crosses at 4 bytes per FFI call, each call locking the global `HOST` mutex (abi.rs:348-351). Arithmetic: `Vec<u8>` in serde_json is `[n,n,...]`, avg ≈4 chars/byte, so MAX_INPUT_BYTES=32 MiB caps effective media at ≈8 MiB; loading it costs ~8.4M FFI crossings + TextEncoder pass + `String::from_utf8_lossy` full copy (abi.rs:191) + serde_json parse of an 8M-element number array + the structured-clone/JSON.stringify passes on the JS side (worker.js:63,123) — ≥5 full-size copies and one FFI storm per file open. The doc's justification "measured cheap enough for v1" cites a measurement that does not exist: `rg -i "write_u32|word.lane" docs/web/size-report.md docs/web/size-record.toml worklog.md` -> 0 measurement hits (only the claim itself at abi.rs:28). Why it matters: this is the only load path for local files in the web lane; the cost claim is the sole evidence the design is viable. What correct looks like: either the 7.H4 raw-memory blob lane, or a committed measurement (bytes/sec, ms for an N-MB loadBlob) backing the sentence. Confidence: HIGH — Verification: read + arithmetic; no browser to measure (UNVERIFIED runtime cost).

F-002 [S3]
Where: apps/web/src/abi.rs:347-364
Code:
```
#[unsafe(no_mangle)]
pub extern "C" fn lumen_web_write_u32(word: u32) -> u32 {
    let mut worker = lock_worker();
    u32::try_from(worker.write_word(word)).unwrap_or(0)
}
```
What is wrong: the six exported symbols — the only surface the browser touches — are never invoked by any test, harness, or CI step, even though they are plain `pub fn`s callable from native tests. Every test drives `worker.write_word`/`worker.dispatch`/`worker.output_json` instead (e.g. the_word_lane_matches_the_json_lane at 696-713 calls `worker.write_word`). A regression in a wrapper (lock inversion, return-width truncation, staging call order) is invisible to the entire evidence chain; the worklog export-section check proves the symbols EXIST, not that they behave. Why it matters: the module's own doc says the exports are the `[VERIFY]` boundary — but nothing verifies even the trivially testable half. What correct looks like: one native test calling `lumen_web_write_u32`/`lumen_web_dispatch`/`lumen_web_output_addr` directly (the global `HOST` makes this possible today). Confidence: HIGH — Verification: read (rg `lumen_web_` over tests/harness: only worker.js:71-98 calls them, and no browser run exists in-repo).

F-003 [S3]
Where: apps/web/src/abi.rs:38-50 vs the decoder-less shipped module
Code:
```
//! The two ways to
//! cross the wasm boundary with raw pointers both dereference
//! host-written memory — `slice::from_raw_parts` — i.e. `unsafe`.
//! The word-push input lane and the JS-read output lane keep the
//! whole module `unsafe`-free at zero new dependencies, trading
//! input throughput for the policy.
```
What is wrong: context, not the quote itself: the throughput trade is paid to ship a module whose default build carries NO decoders (apps/web/Cargo.toml:85-88 "the SHIPPED module enables none of them: the v1 wasm host stays decoder-less"; rc-smoke.sh:130 builds with no `--features`; the e2e runner itself documents `open-play-pause-stop` failing with typed `web.decoders_absent` — ui/scripts/web-matrix-run.mjs:280-284). The expensive lane's "large one" (`loadBlob`) can only ever stage bytes that the same build refuses to play. Why it matters: the v1 web lane's cost center exists to serve a path that dead-ends one message later. What correct looks like: the honesty is documented on all sides (capabilities row, harness interpretation), so this is an architecture-cost finding, not fraud — the Lead decision (builtin-codecs opt-in) should either ship or the lane should wait for 7.H4. Confidence: HIGH — Verification: read of Cargo.toml + rc-smoke.sh + matrix runner.

F-004 [S3]
Where: apps/web/src/abi.rs:207-209, 230-232 — re-staging interplay with worker.js:112-115
Code:
```
        self.stage_frame();
        // New outbound may exist: the next read re-stages.
        self.staged = false;
```
What is wrong: the JS re-arm contract depends on `Pumped` being the LAST element of the staged array: worker.js:112-115 reads `messages[messages.length - 1]` and re-arms only `if last.type === "pumped"`. That ordering is a host.rs property (`Pumped` pushed at host.rs:1044 after `self.pump(ticks)` returns, and nothing pushes after it in the `Pump` arm) — but it is NOWHERE pinned as a protocol invariant in protocol.rs or abi.rs; a future push-after-Pumped (e.g. a trailing event flush) silently stalls the run loop forever. Why it matters: cross-repo silent-coupling on array position. What correct looks like: a protocol.rs doc line "Pumped is always the final message of a Pump dispatch batch" + a host.rs test asserting it. Confidence: MEDIUM — Verification: read (host.rs:1040-1045 confirms current ordering; no invariant test found).

F-005 [S4]
Where: apps/web/src/abi.rs:257-265
Code:
```
        match serde_json::to_string(&messages) {
            Ok(json) => self.output.push_str(&json),
            Err(_) => {
```
(the Err arm pushes `"[]"`.)
What is wrong: a serialization failure would drop the ENTIRE outbound batch and answer an empty array with no error surfaced — the comment argues it cannot fail for these types, which is true today (no non-string keys, no cycles), but the failure mode chosen is maximal silent loss rather than a staged `ControlError`. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: apps/web/src/abi.rs:68-80
Code:
```
// AGENTS.md hard rule 3 exempts NOTHING outright — but this module
// contains ZERO `unsafe` blocks. The allow below exists solely for
// the four `#[unsafe(no_mangle)]` export attributes
```
What is wrong: stale count — the file carries SIX `#[unsafe(no_mangle)]` exports (348, 357, 371, 379, 391, 399), the comment says four. Trivial doc rot in the very block that justifies the `#![allow(unsafe_code)]` (workspace lint otherwise denies). Confidence: HIGH — Verification: read (grep count 6).

## 4. Per-function table (production surface)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AbiWorker::new | 129-139 | REAL | — | builds WebHost with build_profile() profile |
| AbiWorker::host / host_mut | 141-150 | REAL | — | test accessors |
| AbiWorker::write_word | 152-162 | REAL | — | overflow latch correct; returns byte count; bound exact at MAX_INPUT_BYTES |
| AbiWorker::input_overflowed / clear_input | 164-174 | REAL | — | trivial |
| AbiWorker::dispatch | 176-211 | REAL | — | overflow -> typed control error + STATUS_OVERFLOW; parse failure -> typed + STATUS_PARSE_ERROR; stages frame; invalidates staging |
| AbiWorker::dispatch_json | 213-234 | REAL | — | same lane minus word staging; the tests' entry |
| AbiWorker::stage_frame | 236-246 | REAL | — | take_pending_frame single-slot; clears on no-present (tested) |
| AbiWorker::stage_output | 248-268 | REAL | S4 | F-005 silent "[]" fallback |
| AbiWorker::output_addr | 270-278 | REAL-UNPROVEN | — | `as u32` exact on wasm32; never executed against real JS |
| AbiWorker::output_len | 280-284 | REAL | — | u32::try_from clamp |
| AbiWorker::output_json | 286-291 | REAL | — | native test lane; clones |
| AbiWorker::frame_addr | 293-301 | REAL-UNPROVEN | — | 0 = no-frame sentinel; fine |
| AbiWorker::frame_len | 303-307 | REAL | — | |
| AbiWorker::frame_bytes | 309-316 | REAL | — | zero-copy read for tests; JS twin reads memory.buffer |
| Default::default | 319-323 | REAL | — | delegates |
| HOST / worker / lock_worker | 325-343 | REAL | — | OnceLock+Mutex, poison recovery; single-threaded in practice |
| lumen_web_write_u32 | 347-351 | REAL-UNPROVEN | S3 | F-002: zero coverage; F-001: called up to ~8.4M times per loadBlob |
| lumen_web_dispatch | 356-364 | REAL-UNPROVEN | — | eager stage_output on error only (redundant but harmless — output_addr stages lazily anyway) |
| lumen_web_output_addr / output_len | 370-382 | REAL-UNPROVEN | — | worker.js:79-80 consumes |
| lumen_web_frame_addr / frame_len | 390-402 | REAL-UNPROVEN | — | worker.js:97-98 consumes |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_lane_proof_over_the_abi (463-601) | YES | full boot->loadBlob->open->pump->end walk with state/position asserts; executed in baseline |
| the_frame_lane_stages_the_present_bytes (609-690) | YES | asserts staged len == w*h*4 AND non-zero bytes AND empty-lane-on-no-present |
| the_word_lane_matches_the_json_lane (695-739) | YES for write_word+dispatch; NO for the export fn | drives `worker.write_word`, never `lumen_web_write_u32` (F-002) |
| malformed_input_is_typed_and_recoverable (743-762) | YES | status + code + recovery |
| the_input_bound_is_typed (766-787) | YES | pushes to 32 MiB, asserts STATUS_OVERFLOW + code |
| output_addr_and_len_describe_the_staged_bytes (792-810) | YES | addr/len vs buffer; re-read stability |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call per 4 bytes … measured cheap enough for v1" | abi.rs:27-29 | FALSE (no measurement exists) | rg over docs/web/*, worklog.md, size-record: 0 hits (F-001) |
| "The buffer is valid until the next dispatch" (output staging) | abi.rs:31-37, 269-273 | REAL | test 792-810 re-read stability + worker.js reads before further calls |
| "the raw frame lane … pixels never ride the JSON" | abi.rs:384-389 | REAL | frame lane stages RGBA apart; test 646-663; worker.js:97-105 |
| "exports … exercised only by a browser runtime — [VERIFY], not claimed here" | abi.rs:59-66 | REAL (honest) | no browser run exists; rc-smoke browser-load only asserts an `<h1>` (rc-smoke-web.mjs:71-79) |
| "the wasm32 module ships the engine core WITHOUT the builtin decoders" | lib.rs:43-48 | REAL | Cargo.toml:85-90; matrix runner expects web.decoders_absent |

## 7. Dead / duplicated / unreachable code
- None dead in-file. The module is unreachable from the shipped product surface (UI never spawns the worker — `rg "new Worker" ui/src` -> 0 hits); its only consumers are the e2e matrix harness and the never-product-tested rc-smoke artifact.
- `MAX_PENDING_MESSAGES` is referenced by host.rs (enforced) — not dead.

## 8. Dependencies & flags
- serde/serde_json only externals (Cargo.toml:70-71) — matches the "zero new dependencies" claim.
- `#![allow(unsafe_code)]` for the 6 `#[unsafe(no_mangle)]` attrs; comment says "four" (F-006).
- No feature gates in-file; decoder-less posture comes from Cargo.toml features (F-003).

## 9. Open questions for the next agent
- worker.js is NOT in my assignment: its `pumpOutputs` trusts `messages[length-1].type === "pumped"` (F-004) and never validates `lumen_web_write_u32` returning 0 mid-message (a mid-message overflow latch silently corrupts the JSON tail -> parse error, recoverable but confusing). Someone should audit worker.js as a file.
- Does any lane ever build lumen-web with `builtin-codecs` for real playback evidence? docs/web/size-report.md measures sizes only.
