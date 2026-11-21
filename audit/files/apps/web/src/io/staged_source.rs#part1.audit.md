# AUDIT: apps/web/src/io/staged_source.rs#part1 (lines 1-820, production)
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-21 |
| Lines / functions | 820 (of 1219; chunk boundary: part1 = 1-820 production — module docs, config, request/stats/window types, StagedSource, SourceStream impl; part2 = 821-1219 `mod tests`, another agent) / 36 |
| File verdict | UNREACHABLE (real, unit-tested code; ZERO callers outside its own tests — the wired open path typed-refuses http) |
| Reachable from shipped entry point? | NO. Proof: `rg "StagedSource"` across apps/web/src hits only io/staged_source.rs (33) and the io/mod.rs:76 re-export. The actual open path (`open.rs:182-186`) answers `"http" \| "https"` with `unsupported_for_host … "network streaming is card 7.H4 (fetch + Range seekability detection)"`. The repo's own e2e asserts the refusal: ui/e2e/web/site/harness.js:508-513 `"http open must refuse (the staged fetch lane is not wired to open)"`, recorded green in docs/web/browser-matrix-20261110.md:72,98 (`open-http-pending-lane … unsupported-for-host / http-streaming`). The module IS compiled into the shipped `lumen_web.wasm` unconditionally (io/mod.rs:69 `pub mod staged_source`) — whether wasm DCE strips it is UNVERIFIED (no wasm artifact in the workspace). |
| Findings | S0:0 S1:0 S2:0 S3:3 S4:1 |
| Commands run | full read of 1-820 + boundary peek at tests; `rg "StagedSource\|WindowRequest"` apps/web/src (only self + mod.rs re-export); read open.rs:175-195 (http refusal site) + io/mod.rs:1-50; harness.js:500-516 + docs/web/browser-matrix-20261110.md (committed refusal evidence); read seekability.rs detect_seekability (Truncated semantics); read tests `window_cache_ages_and_progress_never_deadlocks` (:954) and `truncated_windows_are_typed_or_wire_end` (cross-check, part2 agent owns them); baseline orchrun log :3154-3162 (9 staged_source tests green); hand-trace of eviction/seek/pending interactions; `sed -n` quote verification (16-20, 37-45, 537-549, 677-689, 699-708) |

## 1. One-paragraph verdict
Eight hundred twenty lines of a genuinely engineered HTTP-staging `SourceStream` — probe-driven seekability detection, ranged/scan modes with mid-stream resync, a bounded window cache with aging, a readiness gate, typed errors everywhere, and 9 green unit tests including a mock-fetch→AVI-demuxer full lane — that is wired to NOTHING: no constructor call exists outside its own tests, and the real web open path refuses http URLs with a typed "card 7.H4 pending" error that the e2e browser matrix asserts as the expected behavior. Worse, the module's docs narrate a live architecture ("the HOST's pump gates engine polls on it") that has no implementation anywhere in host.rs. And the unwired lane carries two latent correctness bugs its own doc comments explicitly promise cannot happen: (1) `insert_window`'s eviction loop drops the window immediately ahead of the read cursor — creating a coverage gap that `pending_request` (which only fetches at the frontier) never refills, deadlocking the readiness gate the module doc says "eviction cannot deadlock"; (2) `absorb`'s truncation arm accepts a 206 body short of its own Content-Range span as the "resource end" whenever the length is unknown — a silent truncation the module promises is "never a silent EOF".

## 2. What actually works (verified)
— 9/9 `io::staged_source::tests` green in baseline `orchrun-test-workspace-nofailfast.log.txt:3154-3162`, incl. `fetch_lane_serves_the_real_fixture_to_the_demuxer` (mock transport → StagedSource → real AviDemuxer over the SourceStream contract) and `ranged_server_reads_across_windows`, `no_range_server_scans_forward_only`, `mid_stream_200_resyncs_into_scan`.
— Config validation (147-161): zero and chunk>window refused typed, mirroring ranged.rs.
— LEB… no — the correctness items I traced: `seek_to` (562-624) never moves the cursor into an unstaged position (covered targets only, EOF position special-cased 579-586, negative/over-len refused 567-575); `gather` (744-781) copies only contiguous staged bytes and cannot index out of bounds (`covers` guards every window access); `covers_staged` (646-672) requires contiguity from `from` (no false-ready across a gap); scan mode is bounded by the window budget at absorb time (472-481) with the whole-body refusal; correlation/unsolicited responses refuse typed and restore the outstanding request (442-462, test `correlation_refuses_unsolicited_and_mismatched` green).
— All arithmetic is saturating/checked (`saturating_add`, `u64::try_from…unwrap_or(u64::MAX)`); no unwrap/expect/panic (workspace lint-deny, baseline clippy CLEAN #5/#6).
— The honest-refusal posture of the ACTUAL wired lane: open.rs:182-186 + harness.js:508-513 + browser-matrix-20261110.md:72/98 — the repo does not pretend http works end-to-end.

## 3. Findings
### F-001 [S3]
Where: apps/web/src/io/staged_source.rs:16-20 (claim) + 1-58 (module header); reality: whole file
Code:
```
//! - [`StagedSource::ready`] answers whether the read cursor is
//!   covered; the HOST's pump gates engine polls on it (doc 09
//!   §3.2's cooperative rule adapted to I/O: a poll is only issued
//!   when the bytes it would read are staged), so a synchronous
//!   demuxer read never sees an unstaged byte and never blocks.
```
What is wrong: There is no pump. `ready()`, `pending_request()`, `absorb()` have zero callers outside this file's own tests (repo-wide grep). No JS glue serves `FetchRequest`s either — worker.js's only `fetch()` loads the engine module itself (worker.js:134). The header narrates the staging bridge as operating architecture in present tense; io/mod.rs:22-28 repeats it ("the [`SourceStream`] the engine reads network media through … the pump gates engine polls").
Why it matters: ~820 production lines + their dependency subtree (seekability, fetch_port) describe themselves as the engine's network lane while the shipped lane is a typed refusal. A reader of the module docs believes web http playback is one wired seam away with a working gate; in reality the gate's consumer does not exist and two latent bugs (F-002/F-003) sit in the unwired path. The module is also compiled unconditionally into the size-budgeted wasm artifact (io/mod.rs:69) — dead weight unless wasm DCE removes it (UNVERIFIED: no built module in the workspace to symbol-scan).
What correct looks like: Either wire the lane (host pump + worker fetch glue, card 7.H4) or mark the module `#[cfg(any(test, feature = "http-lane"))]`-gated OUT of the shipped floor build, and rewrite the header in the repo's own honest-pending voice ("card 7.H4 pending; open.rs refuses http; this module is the lane's implementation awaiting its pump").
Confidence: HIGH — Verification: read (grep zero callers; open.rs/harness/browser-matrix evidence; worker.js read).
### F-002 [S3]
Where: apps/web/src/io/staged_source.rs:699-716 (eviction) vs 41-42 (the promise) and 386-406 (the gap-unfilling read-ahead)
Code:
```
        while self.cached > self.config.window_bytes && self.windows.len() > 1 {
            // Evict the lowest-start window that does not cover the
            // cursor (the oldest by position; sorted order makes the
            // scan linear).
            let cursor = self.pos;
            let index = self
                .windows
                .iter()
                .position(|window| !window.covers(cursor))
                .unwrap_or(0);
```
(and the promise at 39-42:)
```
//! Every buffer is bounded by [`StagedConfig`]: windows aged past the
//! read cursor drop first (forward reads can never touch them
//! again), then least-recently-staged windows evict — never the
//! cursor's own window (eviction cannot deadlock the gate);
```
What is wrong: After `age_behind_cursor` has removed every window ending at/before the cursor, the first window NOT covering the cursor is the one *immediately after* the cursor's window — the exact bytes the next forward read needs. Evicting it (the loop does, `position(!covers)`) punches a hole in the contiguous staged span ahead of the cursor. The eviction is also by POSITION, not "least-recently-staged" as documented. The hole is unrepairable: `pending_request` mints only at `frontier()` (the LAST window's end, 637-641), never at the gap, so once the read cursor reaches the gap edge, `ready()` (via `covers_staged`, which demands contiguity from `pos`) goes false forever — the gate deadlock the header says cannot happen — or, if a poll slips through at the margin boundary, `gather` short-reads mid-resource.
Trigger (traced): the eviction loop only runs when an insert exceeds budget; the pending path's own guard (`cached + chunk > window_bytes → None`, 399-401) keeps `cached ≤ window_bytes`, so the over-budget insert comes from `seek_to`'s unguarded `mint_request` (593-599, 618-620): cache full (32×256 KiB windows = the 8 MiB default, reachable by pipelining on any >8 MiB resource) + one demuxer seek → absorb → eviction of the cursor-adjacent window. A subsequent backward seek behind the gap (AVI/MP4 open patterns do header→index→media) then stalls.
Why it matters: The moment this module is wired (its sole purpose), real demuxer seek patterns on ranged HTTP resources >8 MiB deadlock playback. Today: latent (zero callers, F-001).
What correct looks like: Evict the HIGHEST-start window (farthest from the cursor — true "least recently needed"), or evict only windows whose end is behind the cursor (aging already does that — then refuse the over-budget seek-minted fetch typed instead of evicting ahead-of-cursor windows); plus make `pending_request` fall back to minting at the cursor's coverage end (the gap) when the frontier is not cursor-adjacent.
Confidence: HIGH — Verification: read (loop + invariant trace: pending guard ⇒ cached ≤ budget; seek mint unguarded). Not executed — would need a scripted mock scenario; the existing `window_cache_ages_and_progress_never_deadlocks` test (:954) never triggers insert-time eviction (forward-only, budget respected), so it cannot catch this.
### F-003 [S3]
Where: apps/web/src/io/staged_source.rs:537-549
Code:
```
            Err(FetchError::Truncated { expected, got }) => {
                let start = outstanding.range.map_or(0, |range| range.start);
                let end = start.saturating_add(got);
                let at_end = self.len.is_none() || self.len == Some(end);
                if at_end {
                    if self.len.is_none() {
                        self.len = Some(end);
```
What is wrong: `detect_seekability` returns `Truncated` when a 206 body is shorter than its own Content-Range span — including the unknown-total form (`bytes 0-1023/*`, seekability.rs:120-138: `at_end` is false whenever `total` is `None`). `absorb` then re-derives "at end" as "the length is unknown" and treats ANY short body as the resource's true end: `len = start + got`, staged, `Ok(())`. A server/proxy that promises ≥ span bytes but delivers fewer (cleanly-closed chunked transfer, lying CDN) silently shortens the resource: the demuxer reads a truncated file believing it complete — duration, index and EOF all wrong.
Why it matters: The module's stated invariant (533-536) is "mid-resource it is the typed truncation, never a silent EOF" — this path IS a silent EOF, on the first probe no less (`len` is always `None` before the probe lands, so the very first short-with-`*` 206 always passes `at_end`). Latent today (F-001), but it is a data-integrity bug in the lane's core honesty claim.
What correct looks like: `at_end` must also require the truncation to land at/after the promised span end (`start + got >= range.end_exclusive`) or a KNOWN total that equals `end`; an unknown total plus a body short of its own span is a typed truncation, full stop.
Confidence: HIGH — Verification: read (seekability.rs:120-138 Truncated conditions traced; absorb arm traced). Not executed — the mock has no unknown-total truncating route; `truncated_windows_are_typed_or_wire_end` covers only known-total truncation (mid-resource, typed ✓) and the exact-fit wire-end (✓), not the `*`-form.
### F-004 [S4]
Where: apps/web/src/io/staged_source.rs:677-688
Code:
```
    fn age_behind_cursor(&mut self) {
        let cursor = self.pos;
        let before = self.windows.len();
        self.windows.retain(|window| {
```
(ends:)
```
        let _ = before;
    }
```
What is wrong: `before` is computed and then explicitly discarded — dead store left in production code.
Why it matters: Noise that reads like an unfinished assertion (a count check that was never written); the `let _ =` pattern is exactly what the house lint policy exists to surface.
What correct looks like: Delete both lines, or make it the assertion it apparently wanted to be (debug_assert on evicted count vs `cache_evictions` delta).
Confidence: HIGH — Verification: read.

## 4. Per-function table (part1, lines 1-820)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` / `window_not_staged` | 89-110 | REAL | — | Typed error helpers; codes `web.fetch.window-not-staged` etc. |
| `MAX_READ_BYTES` / `READY_MARGIN` consts | 76, 84 | REAL | — | 64 MiB read ceiling (documented caller-owned); 256 KiB margin. `read(len)` allocates `len` up front — a 64 MiB request over a 256 KiB staged span transiently allocates 64 MiB for a short answer; documented design (71-84), noted not flagged |
| `StagedConfig` + `Default` + `validate` | 115-162 | REAL | — | Zero/over-sized refused typed; defaults from fetch_port |
| `WindowRequest` + `as_fetch_request` | 166-193 | REAL | — | Straight mapping to the port type; clones url (fine, per-request) |
| `StagedStats` | 197-217 | REAL | — | Counters; "no production path reads them" per its own doc — test/telemetry only (consistent with F-001) |
| `Window::covers` | 225-230 | REAL | — | Bounds-correct |
| `Mode` | 233-240 | REAL | — | Ranged/Scan |
| `ScanBuffer::{new,end,serves}` | 262-284 | REAL | — | `serves` allows `target == end()` (honest short read at staged end) — correct |
| `StagedSource::open` | 324-350 | REAL | — | Validates config+URL, mints probe `0..chunk`; probe doubles as first read-ahead |
| `url`/`seekability`/`stats`/`outstanding` | 353-376 | REAL | — | Accessors |
| `pending_request` | 386-406 | REAL | S3 (F-002) | Mints ONLY at the frontier — the reason a mid-span gap is never refilled; budget guard keeps `cached ≤ window_bytes` (the invariant that makes seek-mints the only over-budget inserts) |
| `ready` | 412-426 | REAL | — | Margin/end coverage via `covers_staged`; saturating arithmetic; correct — including going permanently false at a gap (F-002's stall surface) |
| `absorb` | 441-556 | REAL | S3 (F-003) | Correlation + budget + mode machinery correct (traced); the Truncated arm's `at_end` accepts span-short bodies when length unknown |
| `seek_to` | 562-624 | REAL | S3 (F-002 enabler) | Covered targets serve; unstaged mints (NO budget guard, 593-599/618-620) + typed refusal; EOF seek special case correct |
| `out_of_range` | 626-633 | REAL | — | Typed |
| `frontier` | 637-641 | REAL | — | Last window's end — max staged position under sortedness |
| `covers_staged` | 646-672 | REAL | — | Contiguity required from `from` — no false-ready; O(windows²) worst case bounded by budget/chunk (≤32 windows) |
| `age_behind_cursor` | 677-689 | REAL | S4 (F-004) | Correct aging; dead `before` variable |
| `insert_window` | 694-717 | REAL | S3 (F-002) | Sorted insert via partition_point; eviction loop evicts cursor-adjacent window — the gap bug |
| `mint_request` | 726-739 | REAL | — | Takes id, marks outstanding; `let _ =` at call sites drops the returned clone (mint cannot fail — not a swallow) |
| `gather` | 744-781 | REAL | — | Contiguous copy only; short at gap/end; scan branch uses `max(consumed)` defensively; no OOB (covers-guarded) |
| `SourceStream::uri/size/supports_seek` | 785-798 | REAL | — | Server truth for seekability (honest: serving is staged-window-bounded) |
| `SourceStream::read` | 800-806 | REAL | — | len capped at 64 MiB; allocate-then-truncate (see consts note) |
| `SourceStream::read_into`/`seek`/`position` | 808-818 | REAL | — | Delegation |

## 5. Test honesty (if tests exist in/for this file)
Tests live in part2 (821-1219, another agent); I cross-read two for bug-refutation only. Baseline: all 9 green (:3154-3162).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `window_cache_ages_and_progress_never_deadlocks` (:954) | For AGING yes; for F-002 NO | Forward-only 12-byte resource, 16-byte budget, 4-byte chunks: the pending-path budget guard keeps inserts within budget, so `insert_window`'s eviction loop never runs; no seek is issued. The name's "never deadlocks" claim is unproven for the eviction path. |
| `truncated_windows_are_typed_or_wire_end` | For known-total truncation yes; for F-003 NO | Mock routes emit `Content-Range …/8` (known totals) only; the `*`-unknown-total short-span form has no route, so the silent-truncation path is unreachable in tests. |
| `fetch_lane_serves_the_real_fixture_to_the_demuxer` | Yes | Real AVI demuxer driven through mock fetch + StagedSource — would fail on contract drift. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the HOST's pump gates engine polls on it … a synchronous demuxer read never sees an unstaged byte" | module doc 16-20 (doc 09 §3.2/§6.1) | PAPER-ONLY | No pump exists: zero callers of ready/pending_request/absorb; open.rs:182-186 refuses http; harness.js:513 + browser-matrix:72/98 assert the refusal |
| "windows … then least-recently-staged windows evict — never the cursor's own window (eviction cannot deadlock the gate)" | module doc 39-42 | FALSE | Eviction is by lowest-start-not-covering (position, not recency) and CAN deadlock the gate by evicting the cursor-adjacent window (F-002) |
| "mid-resource it is the typed truncation, never a silent EOF" | 533-536 | FALSE | The unknown-total short-span 206 is absorbed as the resource end (F-003) |
| "Seek = cancel + re-fetch with a Range header; seekability is detected, not assumed" (web adapter) | doc 09 §6.1 WebUrl row | REAL-UNPROVEN (as library) / UNREACHABLE (as product) | Detection implemented (seekability.rs, tested); the adapter is not wired into session.open (open.rs refusal) |
| web feature-matrix `access-http` row | docs/feature-matrix.toml (parity web=0/252, baseline #9) | Consistent | The web column does not claim http works |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable (F-001) — 820 production lines + the io::fetch_port/seekability subtree they exist to serve (those files have their own mock-tested logic; whether anything else uses fetch_port: only this lane + tests — the io/mod.rs doc says as much).
- `let before` dead store (F-004).

## 8. Dependencies & flags
- Deps: media-types, plugin-api (SourceStream/SeekOrigin), crate::io::fetch_port + seekability. No feature gate on the module — compiled into every lumen_web build incl. the shipped decoder-less floor (Cargo.toml has no http-lane feature to disable it).
- apps/web builds `lumen_web.wasm` (cdylib) — shipped via apps/web/worker.js + ui e2e web matrix (matrix.config.json `wasmArtifact`), size-tracked by the web-budget gate (baseline #16 PASS; floor 1.23 MiB raw / 0.29 MiB gz per docs/web/size-report.md).

## 9. Open questions for the next agent
- part2 holder (tests 821-1219): audit `window_cache_ages_and_progress_never_deadlocks` and `truncated_windows_are_typed_or_wire_end` against F-002/F-003 — both tests bypass the buggy paths (no seek-minted over-budget insert; no `*`-total route). A mutation-proof per test should record that.
- host.rs holder: confirm no pump hook ever lands in a #[cfg] corner I missed (I grepped ready/pending_request/absorb — zero hits in host.rs).
- Card 7.H4 implementer: F-002 (eviction gap) and F-003 (silent truncation) are preconditions to wiring this lane; also decide whether the module should be feature-gated out of the floor wasm until then (size budget).
- REQUEST-ORCH-RUN (non-essential, sharpens F-001's size claim): `cargo build -p lumen-web --target wasm32-unknown-unknown --release --locked && grep -ac "window-not-staged" target/wasm32-unknown-unknown/release/lumen_web.wasm` — a non-zero count proves the dead lane ships in the budgeted module; zero proves wasm DCE strips it.
