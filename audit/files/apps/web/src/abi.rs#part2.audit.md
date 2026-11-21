# AUDIT: apps/web/src/abi.rs#part2 (lines 406–811)
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-21 |
| Lines / functions | 406 (of 811; chunk = the entire `#[cfg(test)] mod tests`, boundary from wave-00 assignments.tsv:720) / 11 items |
| File verdict | REAL (the test chunk itself: all six tests execute natively and assert real behavior) — with the standing caveat that what they test is product-unreachable and the browser entry symbols are never driven |
| Reachable from shipped entry point? | Tests: compiled+run by baseline #3 (`cargo test --workspace`, test-workspace.log.txt:21 "Compiling lumen-web"); worklog.md:749 records `cargo test -p lumen-web` = 111 passed (abi 6). Subject under test: UNREACHABLE from `lumen`/`lumen-tauri` (crate not linked by any shipped lane). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read abi.rs 1–812 (full file; part1 1–405 audited by A08, read here for context); rg `MAX_PENDING_MESSAGES` (enforced at host.rs:989 — part2 range, verified by Read 980–1009); rg `lumen_web_` in apps/web/worker.js:13-98 (all six exports called by the JS glue); rg `lumen_web_dispatch` in src (definitions only — no native test drives the exports). |

## 1. One-paragraph verdict
This chunk is the ABI's test module, and it is the strongest test code I have audited in this wave: a real in-memory MessagePort double drives the actual JSON dispatch core through the full lifecycle (boot → blob → open → pump → end-of-media → snapshot → shutdown) over the real AVI fixture, the word lane is round-tripped exactly as the JS glue pushes it, the frame lane's byte-level contract (w×h×4 RGBA, valid-until-next-dispatch, cleared when no present) is verified byte-for-byte, and malformed input and the 32 MiB input bound are both proven typed-and-recoverable. The two gaps are both coverage-shaped, not correctness-shaped: nothing ever calls the six `#[unsafe(no_mangle)]` export functions the browser actually calls (not even natively, where they are callable), and the outbound-queue backpressure refusal has no test at all.

## 2. What actually works (verified)
- Full lane proof over the ABI core: `the_lane_proof_over_the_abi` (464–601) asserts the boot handshake (host `web`, threading `single`, blocked rows, mjpeg in decoders), `BlobLoaded` echo, `Accepted` for session open, `Playing` + `PlaybackEndOfMedia` events, final `Ended`/`!busy`, `MediaGetCurrentTime` position in 0.95..=1.0, and `ShutdownComplete`. Ran in baseline #3 (worklog.md:749: lumen-web 111/0).
- The word lane (the input path the JS glue actually uses): `the_word_lane_matches_the_json_lane` (696–739) space-pads to word boundaries exactly per the glue contract and asserts Boot + Pump replies — proving the trailing-whitespace-JSON claim and the `from_le_bytes` framing.
- The frame lane's byte contract: `the_frame_lane_stages_the_present_bytes` (610–690) — exact `width×height×4` lengths, non-zero (converted) pixels, `pts` present, and empty-lane-when-no-present. Would catch a stale or mis-sized staging buffer.
- Malformed input (744–762): typed `web.abi.parse` control error + the worker still boots after.
- Input bound (767–787): pushes to the 32 MiB cap, asserts `STATUS_OVERFLOW` + typed `web.abi.overflow`.
- Output staging invariant (793–810): addr/len describe the exact bytes; re-read between dispatches is stable (idempotent `stage_output`).

## 3. Findings

### F-001 [S4] — The exports the browser calls are never executed by any test
Where: apps/web/src/abi.rs:412-415 (claim) and 347-402 (untested layer, part1 range)
Code:
```
    /// The in-memory MessagePort double: what the JS glue does —
    /// serialize a message, run it through the ABI's dispatch core,
    /// parse the outbound batch — with no port at all. Every test
    /// below drives the SAME paths the browser would.
```
What is wrong: "The SAME paths" overstates. Every test drives `AbiWorker` methods (`dispatch_json`, `write_word`, `output_json`) — the shared core. The six `extern "C"` exports (`lumen_web_write_u32/dispatch/output_addr/output_len/frame_addr/frame_len`, lines 347–402) — the only functions the browser ever calls, confirmed real consumers in worker.js:71–98 — have zero native execution, though they are plain Rust functions callable from tests. `lumen_web_dispatch` contains the module's only non-trivial wrapper logic (`if status != STATUS_OK { worker.stage_output(); }`, 360–362) and `lumen_web_write_u32` does a `u32::try_from(..).unwrap_or(0)` width fold (350) — both unexercised.
Why it matters: The module's own docs (59–66) honestly label the browser runtime `[VERIFY]`, but the export wrappers are NOT browser-runtime — they are testable native code left untested. A broken export (wrong lock handling, dropped error staging) passes the entire suite and only fails in a browser nobody runs in CI.
What correct looks like: One native test calling `lumen_web_dispatch()` etc. directly after booting through `lumen_web_write_u32` words (they are `pub` Rust fns in the same crate).
Confidence: HIGH — Verification: rg (`lumen_web_dispatch` has no test callers) + read.

### F-002 [S4] — Outbound-queue backpressure is enforced but untested
Where: apps/web/src/abi.rs:764-787 (the covered sibling) vs host.rs:989-1000 (uncovered path)
Code:
```
    /// The input bound is enforced: words past [`MAX_INPUT_BYTES`]
    /// answer 0 and the next dispatch answers the typed overflow.
    #[test]
    fn the_input_bound_is_typed() {
```
What is wrong: The input bound gets a dedicated test; the outbound bound (`MAX_PENDING_MESSAGES`, abi.rs:97) — enforced at host.rs:989–1000 by clearing the queue and answering the typed `web.worker.output_backlog` refusal — has zero coverage anywhere in this module (or in host.rs's own tests, which I scanned: no test grows the queue to 4096).
Why it matters: A regression in that path (panic on clear, wrong message shape, bound never tripping) would pass CI. The two bounds deserve symmetric treatment.
What correct looks like: A test that pumps without draining until the control error arrives (cheap: shrink the constant under `#[cfg(test)]` or push 4096 no-op messages).
Confidence: HIGH — Verification: rg over both files' test names.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| tests module docs | 412-415 | SHALLOW | F-001 | "SAME paths the browser would" — same core, different (untested) entry. |
| PortDouble (struct + new) | 416-425 | REAL | — | Owns a fresh AbiWorker per test. |
| PortDouble::post | 428-437 | REAL | — | Serialize → dispatch_json → parse batch; asserts STATUS_OK (valid messages only — invalid lane is post_raw). |
| PortDouble::post_raw | 440-449 | REAL | — | The malformed-input lane; returns (status, replies). |
| PortDouble::frame_bytes | 453-455 | REAL | — | Delegates to the worker's staged bytes. |
| the_lane_proof_over_the_abi | 463-601 | REAL | — | Full lifecycle, real fixture, meaningful asserts at every step; native-profile decoder assert (`mjpeg`) would fail if the native table regressed. |
| the_frame_lane_stages_the_present_bytes | 609-690 | REAL | — | Byte-exact frame contract incl. the cleared-when-no-present invariant; the non-zero-pixel assert is weak (any byte) but paired with exact length. |
| the_word_lane_matches_the_json_lane | 696-739 | REAL | — | Drives `write_word`+`dispatch` (the real input lane, minus the export wrapper); space-padding contract verified. |
| malformed_input_is_typed_and_recoverable | 744-762 | REAL | — | Parse error typed + recovery proven. |
| the_input_bound_is_typed | 767-787 | REAL | — | 32 MiB cap proven; ~8M write_word iterations (bounded, fast). |
| output_addr_and_len_describe_the_staged_bytes | 793-810 | REAL | — | u32-width comparison on native honestly acknowledged (800–802); re-read stability pinned. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_lane_proof_over_the_abi | YES | Mutation check: break `handle`'s Boot reply, blob echo, open acceptance, event flush, position math, or shutdown ack — each has a dedicated assert. |
| the_frame_lane_stages_the_present_bytes | YES | Wrong staging length, stale lane, zeroed buffer, or missing pts each trips an assert. |
| the_word_lane_matches_the_json_lane | YES | If `write_word` mis-framed (endianness, order) or padding broke JSON, dispatch would parse-error and the status/reply asserts fail. |
| malformed_input_is_typed_and_recoverable | YES | Wrong status/code or an unrecoverable worker fails it. |
| the_input_bound_is_typed | YES | If the bound latched early/late or dispatch mishandled the latch, status or code assert fails. |
| output_addr_and_len_describe_the_staged_bytes | YES | Non-idempotent staging or addr/len mismatch fails the re-read asserts. |
| (exports) | NO test exists | F-001. |
| (outbound backpressure) | NO test exists | F-002. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every test below drives the SAME paths the browser would" | abi.rs:415 | SHALLOW | Same dispatch core, but the browser's entry exports are never called (F-001); worker.js:71–98 is their only consumer. |
| "dispatch_json … is pure logic and unit-tested on the native host with the REAL fixture" (module docs, part1) | abi.rs:61-63 | REAL | the_lane_proof (464) does exactly this over `generated/valid/avi_mjpeg_pcm.avi`. |
| "The exports themselves … are exercised only by a browser runtime — `[VERIFY]`, not claimed here" | abi.rs:63-66 | REAL (honest) | rg: no native caller of any `lumen_web_*` export. |
| worklog "abi 6" tests | worklog.md:749 | REAL | Six `#[test]` fns in this range (464/610/696/744/767/793). |

## 7. Dead / duplicated / unreachable code
None in this range. (Part1's `output_json`/`frame_bytes` "native test lane" accessors exist solely for these tests — used here, so not dead.)

## 8. Dependencies & flags
Test-only: `testkit` (Must/assert/fixture_bytes), `serde_json`, `commands`, `crate::protocol`/`crate::render`. No feature gates in the chunk. The chunk compiles under `--all-features` too (baseline #4 ran the workspace suite; the native-profile asserts hold because native decoders are unconditional).

## 9. Open questions for the next agent
- A08 (part1 + worker.js): the JSON-blob inflation — `LoadBlob { bytes: Vec<u8> }` serializes as a JSON number array (~4× expansion), so `MAX_INPUT_BYTES` (32 MiB of JSON text) caps real media at roughly 6–8 MiB on the word lane; the module doc admits the raw blob lane is 7.H4. Worth an explicit number in the doc.
- A05 (host.rs part2): the backpressure refusal message says "pending messages were dropped at the 4096-message bound" — that drop-everything behavior (host.rs:990) deserves its own test (F-002) and maybe a policy look (drop-all vs refuse-new).
