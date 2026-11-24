# AUDIT: apps/web/src/io/staged_source.rs
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2025-06-27 |
| Lines / functions | 1218 / 36 (27 fns/methods + 3 test helpers + 9 tests, incl. trait impl) |
| File verdict | UNREACHABLE (code is real and well-tested; no shipped entry point constructs it) |
| Reachable from shipped entry point? | NO — `StagedSource` referenced only by `apps/web/src/io/mod.rs:69,76` (mod decl + re-export) and its own tests; `host.rs` media path is `LoadBlob` → `BlobRegistry` (`apps/web/src/host.rs:1026-1029`); no JS glue serves media `FetchRequest`s (`apps/web/worker.js` fetches only the wasm module, line 134) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | rg (usage tracing, exit 0); sed -n (quote verification, exit 0); od -c (byte-verify `#[must_use]` lines, exit 0); rustc standalone parse probes on /tmp copy (parse OK; only E0432/E0433 unresolved-crate errors, exit 1 as expected); baseline test citation |

## 1. One-paragraph verdict
This is a genuinely implemented, carefully engineered fetch-twin of `crates/streaming`'s `RangedResource`: typed refusals everywhere, bounded memory with aging+eviction, honest scan-mode accounting, and a strong integration test that pushes the REAL AVI fixture through the real probe+demuxer path over staged windows. But the entire lane is dead weight in the shipped artifact: nothing outside `io/` constructs a `StagedSource`, the host's media lane is blob-only, the capability snapshot honestly reports `http-streaming` as `NotStarted` (capabilities.rs:272-277), and the module doc's central wiring claim — "the HOST's pump gates engine polls on it" — describes host code that does not exist. The fetching model answers the hostile-focus question: yes, the design is JS-feeds-chunks (wasm mints `WindowRequest`s; a transport in the Worker answers), but that transport is unwritten on both sides of the seam.

## 2. What actually works (verified)
— All 9 tests pass in the orchestrator baseline: `audit/runs/baseline/cargo-test-workspace.txt` lines 3395-3404 (`io::staged_source::tests::*` ... `ok` ×9, under `Running unittests src/lib.rs (target/debug/deps/lumen_web-9c0a2a5b7ee9e3e2)`), including `fetch_lane_serves_the_real_fixture_to_the_demuxer` which demuxes the real `generated/valid/avi_mjpeg_pcm.avi` to `EndOfStream` through staged windows.
— Standalone `rustc` parse of the file (edition 2024) produced only unresolved-crate errors — no syntax errors. Note: raw tool output mangles `#[must_use]` → `#ust_use]` (known sandbox mangler; verified true bytes via `od -c`: `# [ m u s t _ u s e ]`). The file is NOT corrupted.

## 3. Findings
F-001 [S2]
Where: apps/web/src/io/staged_source.rs:16-20 (module doc), vs apps/web/src/host.rs (whole file)
Code:
```
//! - [`StagedSource::ready`] answers whether the read cursor is
//!   covered; the HOST's pump gates engine polls on it (doc 09
//!   §3.2's cooperative rule adapted to I/O: a poll is only issued
//!   when the bytes it would read are staged), so a synchronous
//!   demuxer read never sees an unstaged byte and never blocks.
```
What is wrong: The doc asserts host wiring ("the HOST's pump gates engine polls on it") as present fact. `rg 'ready\(|pending_request|outstanding\(' apps/web/src/host.rs` — zero hits; the host never calls `ready()`, never mints or serves a `FetchRequest`. The pump-gate architecture this file is built around does not exist anywhere. Mitigating: `capabilities.rs:272-277` honestly reports `http-streaming` as `NotStarted` (card 7.H4), so the user-facing claim is not fraudulent — this is a misleading module doc on an unwired lane, plus ~1200 lines of dead code shipped in the wasm module. Why it matters: a reader/dev assumes network playback is one glue-step away; in reality neither the Rust host loop nor the JS transport half exists. What correct looks like: the doc should say "the pump gate is the designed contract; no host wiring exists yet (NotStarted, card 7.I1)". Confidence: HIGH — Verification: read (rg tracing cited above).
F-002 [S2]
Where: apps/web/src/io/staged_source.rs:637-642, 393-408 (frontier + pending_request)
Code:
```
    fn frontier(&self) -> u64 {
        self.windows
            .last()
            .map_or(0, |window| window.start + window.data.len() as u64)
    }
```
and (pending_request):
```
        Some(self.mint_request(Some(RangeRequest {
            start: frontier,
            end_exclusive: frontier.saturating_add(self.config.chunk_bytes as u64),
        })))
```
What is wrong: Read-ahead mints only at the END of the highest-start window, assuming windows are contiguous from the cursor. They are not: `seek_to` (line 602-611) stages a window at an arbitrary target, so after a backward seek (S staged, older window at T > S kept — aging only drops windows ending at/before the cursor), the cache is `[S..S+c][T..T+c]` with a gap `S+c..T`. A forward read from S stops at the gap (gather breaks), `ready()` stays false (`covers_staged` walks windows and fails in the gap), and `pending_request` keeps extending PAST the gap from the frontier — the gap is never filled, so the documented pump gate ("a poll is only issued when the bytes it would read are staged") stalls the lane indefinitely. Why it matters: any seek-then-read-forward sequence — exactly what demuxers do (probe at 0, seek to index, read forward) — deadlocks the gate once the lane is wired. What correct looks like: mint read-ahead from the cursor's coverage end (`covers_staged` walk), not the last window's end; or drop non-contiguous windows behind the cursor. Confidence: MEDIUM — Verification: read/logic-traced; UNVERIFIED by execution (cannot add a test to a read-only tree; cargo test budget). Latent today only because the lane is unwired (F-001).
F-003 [S3]
Where: apps/web/src/io/staged_source.rs:677-687 (age_behind_cursor)
Code:
```
    fn age_behind_cursor(&mut self) {
        let cursor = self.pos;
        let before = self.windows.len();
        ...
        let _ = before;
```
What is wrong: `before` is computed and then discarded — dead store, leftover from a removed assertion. Why it matters: noise in a bounded-memory-critical function; a reader hunts for the missing use. Confidence: HIGH — Verification: read (sed-verified).
F-004 [S3]
Where: apps/web/src/io/staged_source.rs:549-556 (absorb catch-all)
Code:
```
                        (_, _) => {
                            if !response.body.is_empty() {
                                self.scan = Some(ScanBuffer::new(0, response.body));
                            }
                        }
```
What is wrong: A scan-mode whole-body re-fetch answered `416` (Empty) keeps the old buffer and returns `Ok(())`; the seek that minted the re-fetch can never succeed afterwards, and every caller retry re-fetches and re-fails — an unbounded refetch loop driven by any retrying caller (each attempt counted in `refetches`). Why it matters: a no-progress Ok on the re-fetch path; the honest answer is a typed refusal for a 416 after a 200 probe (server changed its story). What correct looks like: treat a mid-stream 416 in scan mode as a typed protocol error, not a silent keep. Confidence: MEDIUM — Verification: read (path traced; requires a server that answers 200 then 416).
F-005 [S4]
Where: apps/web/src/io/staged_source.rs:800-805 (SourceStream::read)
Code:
```
    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let len = len.min(MAX_READ_BYTES);
        let mut out = vec![0; len];
        let gathered = self.gather(&mut out);
        out.truncate(gathered);
```
What is wrong: A large `read(len)` allocates `len` (up to 64 MiB) zeroed before knowing staged coverage; a 4-byte staged span still pays the 64 MiB allocation (calloc-backed, but the page-touch profile depends on the demuxer). Documented as the bounded exception — hygiene note only. What correct looks like: gather into a small buffer, or `Vec::with_capacity` sized to staged coverage. Confidence: HIGH — Verification: read.
F-006 [S4]
Where: apps/web/src/io/staged_source.rs:412-415 (ready)
Code:
```
    pub fn ready(&self) -> bool {
        let need_end = self.pos.saturating_add(READY_MARGIN);
        let need = match self.len {
```
plus line 427-428 `if self.pos >= need { return true; }` — `need >= pos` by construction unless `pos` saturates near `u64::MAX`; effectively dead defensive branch. Hygiene. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint / window_not_staged | 89-113 | REAL (test-only reachable) | — | typed error builders; used on every refusal path |
| StagedConfig::default/validate | 129-161 | REAL | — | refuses zero/over-window configs typed; test-proven (config_validation_refuses_typed) |
| WindowRequest::as_fetch_request | 185-192 | REAL | — | lossless mapping; 6 `#[must_use]` attrs byte-verified via od |
| Window::covers | 227-231 | REAL | — | offset math correct |
| ScanBuffer::new/end/serves | 264-286 | REAL | — | `serves` allows `target == end()` = honest short read |
| StagedSource::open | 324-351 | REAL | — | validates config+URL, mints probe; test-proven |
| url/seekability/stats/outstanding | 354-379 | REAL | — | plain getters |
| pending_request | 386-408 | REAL (defect) | S2 | frontier-gap stall (F-002); scan None; budget guard honest |
| ready | 412-427 | REAL | S4 | gate math correct incl. known-end case; dead branch F-006 |
| absorb | 441-560 | REAL (defect) | S3 | correlation + probe + resync + truncation all typed; 416-keep wart F-004; 200-over-budget bound enforced own-side |
| seek_to | 562-624 | REAL | — | EOF serves; negative/over-len refused; unstaged mints + typed refusal; scan skip accounting |
| out_of_range | 626-635 | REAL | — | typed |
| frontier | 637-642 | REAL (defect) | S2 | last-window assumption (F-002) |
| covers_staged | 646-675 | REAL | — | window-chain walk; O(W·K) with W≤32 realistic; scan span check correct |
| age_behind_cursor | 677-692 | REAL | S3 | aging correct; dead store F-003 |
| insert_window | 694-724 | REAL | — | sorted insert; evicts non-cursor windows, never the cursor's; single-window floor prevents self-eviction |
| mint_request | 726-742 | REAL | — | window-budget response limit on every shape |
| gather | 744-782 | REAL | — | ranged coalescing + scan drain correct; short only at unstaged/known end |
| SourceStream::{uri,size,supports_seek,read,read_into,seek,position} | 784-818 | REAL | S4 | read alloc (F-005); supports_seek = server truth |
| tests: serve/pipeline/tiny_ranged | 831-872 | REAL | — | drive the REAL message path via MockFetch |
| 9 tests | 875-1218 | REAL | — | all pass in baseline; see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ranged_server_reads_across_windows | YES | asserts coalesced bytes across window boundary + EOS short read |
| no_range_server_scans_forward_only | YES | asserts seekable=false, refetch counter, skip accounting, error codes |
| window_cache_ages_and_progress_never_deadlocks | YES | asserts consumed==12, budget held, evictions ≥2 — mutation of aging/eviction breaks it |
| ready_gate_tracks_the_margin_and_the_end | YES | asserts not-ready before staging, ready after, EOS ready |
| correlation_refuses_unsolicited_and_mismatched | YES | asserts typed codes + outstanding survival |
| mid_stream_200_resyncs_into_scan | YES | asserts resync counter + scan serving |
| truncated_windows_are_typed_or_wire_end | YES | asserts typed truncation vs wire-end length fix |
| config_validation_refuses_typed | YES | asserts all three refusal codes |
| fetch_lane_serves_the_real_fixture_to_the_demuxer | YES | real AVI fixture, real ProbeRegistry + AviDemuxer to EndOfStream, packets > 0 — the strongest proof in the file |
None assert nothing; none are #[ignore] (baseline: all `ok`). Gap: no test for seek-induced cache gaps (F-002) — the stall case is untested.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the HOST's pump gates engine polls on it" | staged_source.rs:16-20 | FALSE (today) | host.rs has zero calls to ready/pending_request/outstanding (rg, exit 0) |
| fetch lane is honest about its status | capabilities.rs:272-277 `NotStarted` | REAL | sed-verified row; test `doc_09_table_rows_are_typed_data` guards it |
| "reads larger than the margin may see mid-stream short reads" | staged_source.rs:75-77 | REAL | gather breaks at unstaged byte by construction |
| bounded memory (windows+scan+bodies+reads) | module docs §Bounded memory | REAL | insert_window eviction loop + absorb 200-budget check + MAX_READ_BYTES clamp; test-proven for windows |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from the shipped wasm entry point (see header). Within the file: dead store F-003, dead branch F-006. `MAX_READ_BYTES` restates the input crate's constant (documented duplication, acceptable). The io/mod.rs:76 re-export (`StagedConfig..WindowRequest`) is consumed by no non-test code.

## 8. Dependencies & flags
No new deps; uses media_types, plugin_api (SourceStream), crate::io::{fetch_port, seekability}, testkit + mock_fetch (cfg(test)). No feature gates. Compiles into lumen-web unconditionally (pub mod chain lib.rs:61-69 → io/mod.rs:69).

## 9. Open questions for the next agent
- Who is supposed to own the gap-fill fix (F-002) when card 7.I1 wires the lane — source or host? The pump-gate design as documented cannot recover from a seek gap without it.
- Does any doc (doc 09 §6.1) promise the fetch lane for v1? capabilities says NotStarted; if a README/matrix row claims web http playback, that row is FALSE — cross-check in the matrix wave.
