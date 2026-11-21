# AUDIT: apps/web/src/abi.rs#part1 (lines 1-405)
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = lines 1-405 per wave-00/assignments.tsv (LOC 405): the ENTIRE implementation — module docs, `#![allow(unsafe_code)]` justification, bounds/status consts, `AbiWorker` (word-in/JSON-out/frame staging), the `HOST` static + `lock_worker`, and the six `#[unsafe(no_mangle)] extern "C"` exports. Line 405 is `mod tests {` — part2 (A04) = 406-812: the `PortDouble` test harness + 6 tests. Boundary is clean: no function spans it. |
| Lines / functions | 405 / 22 (16 AbiWorker/worker fns + 6 exports) + 5 consts + 1 static |
| File verdict | REAL-UNPROVEN (implementation real, native-tested green; the exports executed in a real Chromium once — at a foreign commit, not the baseline SHA; no wasm32 build exists in the baseline runs) |
| Reachable from shipped entry point? | YES in the web lane, NO in any shipped product: the six exports are the wasm entry surface driven by apps/web/worker.js:71-98 (`write_u32` loop → `dispatch` → `output_addr`+`output_len` → `frame_addr`+`frame_len`) and instantiated only by `ui/e2e/web/site/harness.js` (rg "new Worker" repo → harness + web-matrix-run.mjs only). Real-browser execution: `docs/web/browser-matrix-20261110.md` (Playwright Chromium, commit 3755dc3 branch w24/a, 2026-11-11 — "real Worker, real wasm instantiation"). Baseline INDEX has NO wasm32 build/run — the cdylib lane is unexecuted at e83e5bd. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `wc -l` (812; split 405/406 per assignments.tsv) · `rg "MAX_PENDING_MESSAGES\|MAX_INPUT_BYTES" apps/web/src` (input bound self-enforced at 156; pending bound enforced at host.rs:989-997 — not dead) · `rg "output_addr\|output_len\|write_u32\|dispatch\|frame_addr\|frame_len" apps/web/worker.js` (glue call order addr→len — the safe order) · `sed -n '405,420p;795,812p'` (part2 boundary + final test) · `rg "unsafe" apps/web/src/abi.rs` (zero unsafe blocks; allow is attribute-only) · `rg "rgba" apps/web/src/render/present.rs` (UploadFrame.rgba bounded, non-empty for real frames) · `ls tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` (425,516 B — the real blob the matrix lane loaded) · baseline: `orchrun-test-workspace-nofailfast.log.txt:3090-3094` (`abi::tests::*` 5/5 green; full lumen_web 111/111 at :3089) · `sed -n` quote verification below |

## 1. One-paragraph verdict
This is a competently built no-`unsafe` wasm ABI: bounded input with an overflow latch that converts to a typed control error, lossy-tolerant decode that can never panic the worker, idempotent output staging with a `staged` flag that makes re-reads stable (test-pinned at the file's end), a raw frame lane staged beside the JSON, and poisoning-recovered global state behind `OnceLock<Mutex<_>>` instead of `static mut`. The six exports match the glue's call order exactly (verified against worker.js:71-98), and the native tests drive the same `dispatch` core the browser drives. The defects are small and mostly latent: a swallowed serialize failure that would silently drop the whole outbound batch with a bogus `[]`; the `from_utf8_lossy` decode that can turn corrupted input into *successfully parsed* JSON; `output_len` reading without staging (stale length if a caller asks len before addr); and a module-wide `unsafe_code` allow that is broader than the six exports need. The bigger caveat is evidential: the export surface has run in a real browser only at a foreign commit, and the baseline contains no wasm32 build at all — at e83e5bd this file is REAL-UNPROVEN on the target it exists for.

## 2. What actually works (verified)
— Word lane + overflow latch: traced `write_word` (155-162: 4-byte increments, latch at 32 MiB, returns 0) → `dispatch` (181-189: latch ⇒ clear + typed `web.abi.overflow` + STATUS_OVERFLOW). Input stays 4-aligned so the bound is exactly reachable; `the_input_bound_is_typed` (part2, 764-778) pushes to the bound and asserts the typed error — green in `orchrun-test-workspace-nofailfast.log.txt:3092`.
— Staging idempotence: `staged=false` at both dispatch sites (209, 232); `stage_output` early-returns when staged (252-254); the final part2 test (795-810) asserts re-read stability AND that addr/len describe the staged buffer — green (`output_addr_and_len_describe_the_staged_bytes`, :3090).
— Glue contract match: worker.js:79-80 calls `output_addr` then `output_len`; :97-98 `frame_addr` then `frame_len` — both match the staging functions' &mut self / &self split.
— `MAX_PENDING_MESSAGES` is not a paper constant: enforced at host.rs:989-997 (outbound queue refuses typed at 4096).

## 3. Findings

### F-001 [S3] `stage_output` swallows a serialization failure and answers `[]` — a whole outbound batch silently dropped, with the `staged` flag certifying the emptiness
Where: apps/web/src/abi.rs:257-266
Code:
```rust
        match serde_json::to_string(&messages) {
            Ok(json) => self.output.push_str(&json),
            Err(_) => {
                // Serialization cannot fail for these types (no
                // non-string map keys, no cycles); if it ever does,
                // the honest answer is an empty batch, never a
                // dead worker.
                self.output.push_str("[]");
            }
        }
```
What is wrong: The `Err(_)` arm discards the error AND the messages. It then sets `staged = true` (267), so every subsequent re-read certifies the empty batch as the dispatch's output. The parse-failure lane three lines up (198-204) stages a typed `web.abi.parse` control error — the serialize-failure lane stages silence.
Why it matters: If a future `WorkerToHost` variant ever becomes unserializable (a map with non-string keys, a new untagged type), every reply containing it vanishes with no diagnostic: the glue sees `[]`, the page sees nothing, and the `web.abi.*` typed-error vocabulary this module exists to provide is bypassed by its own fallback. The comment claims it "cannot fail" — true for today's types, and exactly the assumption a silent-drop fallback erodes over time.
What correct looks like: Stage a typed control error (e.g. `web.abi.serialize`) through `handle_control_error` like the parse lane, then stage THAT; or at minimum `debug_assert!` the error path.
Confidence: HIGH — Verification: read; unreachable with current wire types (traced: WorkerToHost is plain serde enums/structs in protocol.rs, no maps), so no runtime repro possible — defect is latent by construction.

### F-002 [S4] `from_utf8_lossy` can convert corrupted input into *valid* JSON — silent content corruption instead of a typed parse error
Where: apps/web/src/abi.rs:191
Code:
```rust
            let text = String::from_utf8_lossy(&self.input).into_owned();
            self.input.clear();
```
What is wrong: Invalid UTF-8 inside a JSON string payload becomes U+FFFD before parsing; if the surrounding JSON stays well-formed, `serde_json::from_str` succeeds and the message is applied with corrupted content. Strict `String::from_utf8` would refuse with the typed `web.abi.parse` error.
Why it matters: The word lane accepts arbitrary bytes; a truncated/misaligned word sequence that happens to land inside a string value is accepted rather than refused. The blast radius is one field of one message (recoverable, bounded), but it contradicts the module's typed-refusal posture for zero benefit — the lossy decode never *fixes* anything, it only downgrades refusals into corruptions.
What correct looks like: `match String::from_utf8(std::mem::take(&mut self.input)) { Ok(text) => …, Err(_) => typed web.abi.parse error }`.
Confidence: HIGH — Verification: read; no execution (mutation would require editing source — forbidden).

### F-003 [S4] `output_len` does not stage — a len-before-addr read after dispatch answers the PREVIOUS dispatch's length
Where: apps/web/src/abi.rs:280-284 (with 208-209)
Code:
```rust
    /// The staged output's byte length.
    #[must_use]
    pub fn output_len(&self) -> u32 {
        u32::try_from(self.output.len()).unwrap_or(u32::MAX)
    }
```
What is wrong: `output_addr` is `&mut self` and stages (275-278); `output_len` is `&self` and just reads the buffer. After a dispatch (which sets `staged=false` at 209 but does NOT clear `output`), a caller that asks `lumen_web_output_len()` before `lumen_web_output_addr()` receives the stale length of the previous staging; pairing it with the fresh address reads a truncated or over-long byte range.
Why it matters: The shipped glue (worker.js:79-80) uses the safe order, so this is latent — but the ABI is a published contract for a page the repo does not control, and the doc only says "pair with", not "addr first". A mismatched pair yields garbage JSON, which downstream surfaces as a parse error of the OUTPUT — misattributed blame.
What correct looks like: Make `output_len` take `&mut self` and call `stage_output()` first (symmetric with `output_addr`), so any order is safe.
Confidence: HIGH — Verification: read + trace of `staged`/`output` lifecycle; worker.js call order verified safe.

### F-004 [S4] Module-wide `#![allow(unsafe_code)]` is broader than the six exports need
Where: apps/web/src/abi.rs:68-80
Code:
```rust
// platform-adapter shape rule 3 anticipates ("approved ADR +
// documented invariants") — `[PROPOSAL: Lead ratifies the wasm
// export seam as the apps/web ADR, or supplies the bindgen
// dependency decision (doc 09 §2.2 OQ-1)]`. Every other module in
// this crate compiles under the deny.
#![allow(unsafe_code)]
```
What is wrong: The allow is needed only for the `#[unsafe(no_mangle)]` attributes (six functions), but a module-level `#![allow]` also silences `unsafe_code` for any future `unsafe {}` block added anywhere in this 400-line file — the workspace deny (root Cargo.toml `[workspace.lints.rust] unsafe_code = "deny"`) is disarmed file-wide.
Why it matters: The comment says the module contains "ZERO `unsafe` blocks" (verified true today — rg over 1-405: only the 6 attributes); the allow's scope does not enforce that invariant, it only documents it.
What correct looks like: `#[allow(unsafe_code)]` on each of the six export fns (or `#[expect(unsafe_code)]` so a future block re-lints), keeping the deny live for the rest of the file.
Confidence: HIGH — Verification: read; zero unsafe blocks confirmed by rg.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_INPUT_BYTES` / `MAX_PENDING_MESSAGES` consts | 87-97 | REAL | — | Input bound self-enforced (156); pending bound enforced at host.rs:989-997. Not dead. |
| `STATUS_OK/PARSE_ERROR/OVERFLOW` consts | 99-105 | REAL | — | Wire statuses; matched by glue + tests. |
| `AbiWorker` struct | 110-125 | REAL | — | Plain data; `staged`/`input_overflowed` latches documented. |
| `AbiWorker::new` | 129-139 | REAL | — | `WebHost::new(build_profile())` — profile from capabilities.rs (A02). |
| `AbiWorker::host`/`host_mut` | 141-150 | REAL | — | Test/diagnostic accessors; `host_mut` is `pub` — no non-test caller found (minor API width, noted not finding). |
| `AbiWorker::write_word` | 155-162 | REAL | — | 4-byte LE push; overflow latch; `len()+4` cannot overflow usize (≤32 MiB). Returns 0 on latch — 0 is also the empty-buffer answer? No: first word returns 4; 0 ONLY on latch. Unambiguous. |
| `AbiWorker::input_overflowed` | 165-168 | REAL | — | Trivial probe. |
| `AbiWorker::clear_input` | 171-174 | REAL | — | Resets both buffer and latch. |
| `AbiWorker::dispatch` | 180-211 | REAL | S4 | F-002 lossy decode. Overflow lane orders clear-before-error (bounded). Sets `staged=false` AFTER `stage_frame` — order traced correct. |
| `AbiWorker::dispatch_json` | 216-234 | REAL | — | Same core minus word staging (port-double lane); duplicate of dispatch's else-branch (shared shape, acceptable duplication — 8 lines). |
| `AbiWorker::stage_frame` | 241-246 | REAL | — | Clear-then-take: a dispatch with no present clears the lane (documented 238-239). |
| `AbiWorker::stage_output` | 251-268 | REAL | S3 | F-001 swallowed serialize error → silent `[]`. |
| `AbiWorker::output_addr` | 274-278 | REAL | — | Stages on demand; `as u32` exact on wasm32, native truncation documented in part2 test. |
| `AbiWorker::output_len` | 281-284 | REAL | S4 | F-003 no staging. `unwrap_or(u32::MAX)` unreachable (output ≤ pending-bound × msg size ≪ 4 GiB). |
| `AbiWorker::output_json` | 287-291 | REAL | — | Native test lane; clones. |
| `AbiWorker::frame_addr` | 295-301 | REAL | — | 0-sentinel when empty; a zero-byte present would alias the sentinel — present.rs always stages ≥1 byte for real frames (rgba_bytes bound > 0), so unreachable. |
| `AbiWorker::frame_len` | 304-307 | REAL | — | Pairs with frame_addr. |
| `AbiWorker::frame_bytes` | 313-316 | REAL | — | Native test lane. |
| `Default` | 319-323 | REAL | — | Trivial. |
| `HOST` static / `worker()` / `lock_worker()` | 325-343 | REAL | — | `OnceLock<Mutex<…>>` — no `static mut` (boundaries rule); poisoning recovered via `into_inner()` — sound for plain staging buffers; single-threaded in the Worker, `Send+Sync` for the type system. |
| `lumen_web_write_u32` export | 347-351 | REAL | — | `u32::try_from(...).unwrap_or(0)` — unreachable fallback (write_word ≤ 32 MiB). |
| `lumen_web_dispatch` export | 356-364 | REAL | — | Force-stages output on non-OK (361) so an error-reading glue sees the typed error without addr first — good asymmetry, documented. |
| `lumen_web_output_addr` / `output_len` exports | 370-382 | REAL | S4 | Thin wrappers; F-003 applies. |
| `lumen_web_frame_addr` / `frame_len` exports | 390-402 | REAL | — | Thin wrappers. |

## 5. Test honesty (tests are part2's range — cross-referenced, not re-audited)
| Test (part2) | Would it fail if the code were broken? | Reason |
|---|---|---|
| `the_lane_proof_over_the_abi` (464+) | YES | Full protocol lifecycle through `dispatch_json` — boot, open, pump, present — the same core the exports wrap. |
| `the_frame_lane_stages_the_present_bytes` (609+) | YES | Asserts staged RGBA bytes match the JSON-described present. |
| `the_word_lane_matches_the_json_lane` (695+) | YES | Word-pushed input ≡ JSON-dispatched input — pins write_word/le-order/padding contract. |
| `malformed_input_is_typed_and_recoverable` (744+) | YES | Garbage input → STATUS_PARSE_ERROR + typed control message; worker reusable after. |
| `the_input_bound_is_typed` (764+) | YES | Pushes to exactly MAX_INPUT_BYTES + 1 word; asserts latch + typed overflow. |
| `output_addr_and_len_describe_the_staged_bytes` (790+) | YES | addr/len/bytes triple + re-read stability — pins the staging invariant (and would catch F-003 only in the addr-first order it uses). |
| All 6 green: `orchrun-test-workspace-nofailfast.log.txt:3090-3094`. | — | NOTE: primary baseline `test-workspace.log.txt` fail-fast aborted before lumen_web (see backend.rs audit) — the no-fail-fast orchrun run is the evidence. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One call per 4 bytes — the no-`unsafe` price, measured cheap enough for v1" | abi.rs:28-30 | REAL-SHALLOW | A real 425,516-byte AVI blob rode the word lane in the Chromium matrix run (open scenarios reach probe/decode after load); but the 32 MiB worst case was never measured — the claim's "measured" covers ~1.3% of the bound. |
| "The buffer is valid until the next dispatch; the glue reads before any further call" | abi.rs:36-37 | REAL | staged-flag lifecycle + re-read test; worker.js reads addr/len/frame then no wasm call in between (worker.js:79-98, comment at 85). |
| "the staging buffer never moves while the glue reads it (no allocation between the two output calls and the read)" | abi.rs:47-50 | REAL | output_addr/output_len/stage_output allocate nothing between calls (traced). |
| "exports themselves … are exercised only by a browser runtime — `[VERIFY]`, not claimed here" | abi.rs:59-66 | REAL (honest label) | Exercised at 3755dc3 (browser matrix); UNVERIFIED at baseline SHA — no wasm32 build in baseline. |
| "JSON permits trailing whitespace" (space-padding to word alignment) | abi.rs:24-27 | REAL | Standard JSON grammar; `the_word_lane_matches_the_json_lane` pins the round-trip. |

## 7. Dead / duplicated / unreachable code
- `dispatch` vs `dispatch_json`: the parse-and-handle core is duplicated (193-205 vs 217-229) — 8 lines each, same shape; acceptable seam but a one-fn refactor would remove it.
- No dead items found in part1 (all consts enforced, all accessors used by tests or glue).
- The whole export surface is unreachable from the shipped `lumen` CLI / tauri (it is wasm-only by construction); its only consumers are the e2e harness lane.

## 8. Dependencies & flags
- Uses only serde_json + crate-internal host/protocol/capabilities — no new externals (matches Cargo.toml's zero-dep claim for the app).
- `#![allow(unsafe_code)]` (F-004) is the only lint deviation in the crate.
- cdylib target never built in baseline (INDEX has no wasm32 run); size/budget facts live in xtask web-budget (#16 PASS) against committed records.

## 9. Open questions for the next agent
- A04 (abi.rs part2): the tests drive `dispatch_json`/PortDouble — does ANY native test drive the actual `extern "C"` exports (via the static HOST)? If not, the export wrappers (locking + u32 casts) are test-uncovered natively and browser-only at a foreign SHA.
- A01 (worker.js): confirm the glue never calls `lumen_web_output_len` without a preceding `output_addr` in ANY branch (the safe order I verified is the main loop path).
- Wave-19 (docs/web): size-report.md's wasm byte counts vs `builtin-codecs-external` features — whether the "measured" word-lane claim has a recorded 32 MiB ceiling measurement anywhere.
- REQUEST-ORCH-RUN: `cargo build --release -p lumen-web --target wasm32-unknown-unknown --locked` at baseline SHA (proof the audited source still builds for its actual target; baseline never did).
