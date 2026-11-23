# AUDIT: crates/streaming/src/hls/master.rs
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-23 |
| Lines / functions | 1189 / 31 production fns+methods, 8 tests, 6 test helpers |
| File verdict | UNREACHABLE (real, tested code — no shipped entry point opens it) |
| Reachable from shipped entry point? | NO — `open_master` has zero non-test callers; lumen-cli opens http(s) via `streaming::ranged` only (apps/cli/src/demux_host.rs:488-490) |
| Findings | S0: 0 S1: 1 S2: 1 S3: 3 S4: 4 |
| Commands run | `rg -n "open_master|HlsAdaptiveStream|..."` (only tests/fuzz hit); `cargo test -p streaming --lib hls::master` -> 8 passed, 0 failed (exit 0); `sed -n` quote verification |

## 1. One-paragraph verdict
The ABR core (`select_variant_index`, `AbrController`) is genuinely implemented and genuinely tested — hysteresis band, dwell, stall-forced down-steps, bounded switch ledger all behave as the module docs claim, and I ran the 8 unit tests myself (all pass). But the entire adaptive master session is dead weight in the shipped product: nothing outside `crates/streaming` tests and the fuzz target ever calls `open_master`/`HlsAdaptiveStream`; the CLI's HTTP lane goes through `RangedResource`, not HLS. Beyond reachability, the retained-prefix byte server carries a real arithmetic bug: a segment body larger than `max_retained_bytes` (a config `validate()` happily accepts) makes the next `read()` subtract-underflow and panic in both debug and release.

## 2. What actually works (verified)
— verified by `cargo test -p streaming --lib hls::master` -> `test result: ok. 8 passed; 0 failed; 0 ignored` (selection, hysteresis, stall-forced down, bounded history, scripted end-to-end switching, typed refusals, misaligned-variant refusal, in-playlist discontinuity).
— verified by `rg -n "open_master"` over the repo -> only crates/streaming tests + netfuzz/fuzz.rs:104. No app/input/playback-engine caller.

## 3. Findings
F-001 [S1]
Where: crates/streaming/src/hls/master.rs:603-617 and 620-627
Code:
```
    fn absorb(&mut self, body: Vec<u8>) {
        let start = self.fetched_upto;
        self.fetched_upto += body.len() as u64;
        if self.retained.len() + body.len() <= self.config.max_retained_bytes {
            self.retained.extend_from_slice(&body);
        } else {
            self.retained.clear();
            self.retained_from = start;
            if body.len() <= self.config.max_retained_bytes {
                self.retained = body;
            } else {
                self.retained_from = self.fetched_upto;
            }
        }
    }
```
plus in `serve`:
```
            if self.pos < self.fetched_upto {
                let at = (self.pos - self.retained_from) as usize;
                let have = self.retained.len() - at;
```
What is wrong: when one segment body exceeds `max_retained_bytes`, `absorb` sets `retained_from = fetched_upto` (end of body) with `retained` empty while `pos` still equals `start` (< `fetched_upto`). The next `serve` iteration computes `pos - retained_from` → u64 underflow. Debug: "attempt to subtract with overflow" panic. Release: `at` wraps to `2^64 - body.len()`, `have` wraps to `body.len()`, and `&self.retained[at..at+take]` on an empty `retained` panics ("range start index out of range for slice of length 0"). Why it matters: `HlsMasterConfig::validate` (lines 362-384) accepts any non-zero `segment_response_limit` and any `max_retained_bytes <= 1 GiB`, so a low-memory host (e.g. 4 MiB retention, default 8 MiB response budget) plus a server returning one >4 MiB TS segment panics the reader thread. What correct looks like: clamp retention to the tail (keep last `max_retained_bytes` of the body) or refuse typed; never let `retained_from > pos`. Same pattern is cloned in hls/media.rs and hls/live.rs (flag for their auditors).
Confidence: HIGH — Verification: read (arithmetic traced both profiles); not executed (no test harness permitted outside audit/).

F-002 [S2]
Where: crates/streaming/src/hls/master.rs:422-446 and 479-487
Code:
```
pub struct HlsAdaptiveStream {
    fetch: Arc<dyn HttpFetch>,
```
and
```
    pub fn note_throughput(&mut self, bytes: u64, wall_ns: u64) {
        self.abr.observe_throughput(bytes, wall_ns);
    }
```
What is wrong: the ABR is observation-driven by design, but no code in the repo — and no shipped host — ever calls `note_throughput`/`on_buffer_signal` on a session; grep shows the only callers are the test files. Any future integrator that opens a session and just reads gets a controller whose EWMA stays `None`, so `decide` (line 267 `self.throughput_bps()?`) always returns `None`: zero switching, silently, forever. The doc says "production measures each segment fetch" (L30-31) — there is no such production code. What correct looks like: the session measuring its own fetches (`fetch_next_segment` already has bytes + wall clock available) with the host seam as an override, or an explicit "ABR disabled without samples" event.
Confidence: HIGH — Verification: read + repo-wide rg (callers: tests only).

F-003 [S3]
Where: crates/streaming/src/hls/master.rs:274-282
Code:
```
        // ---- Down: starvation or a 6.D1 stall prediction. A stall
        // bypasses the dwell (an emergency is an emergency); plain
        // starvation does not (one bad sample must not flap the
        // ladder — the EWMA already smooths, the dwell double-guards).
```
What is wrong: the comment asserts plain starvation does not bypass the dwell and that "the dwell double-guards", but the down branch `if pos > 0 && (self.stalled || starved)` contains no `segments_on_current` check at all — starvation down-steps ignore dwell exactly like stall down-steps. The stated invariant is false; only the EWMA smooths. What correct looks like: either the code checks `segments_on_current >= min_dwell` for the starvation (non-stall) path, or the comment tells the truth.
Confidence: HIGH — Verification: read (both branches compared).

F-004 [S3]
Where: crates/streaming/src/hls/master.rs:787-805
Code:
```
    impl HttpFetch for RouterFetch {
        fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
            let path = crate::fetch::Url::parse(&request.url)?.path_and_query;
```
What is wrong: unlike the media.rs test double (hls/media.rs:778-783, which enforces `ResponseTooLarge` when `resp.body.len() > request.response_limit`), this `RouterFetch` ignores `request.response_limit` entirely. The master unit tests therefore cannot detect a regression that drops `with_response_limit` from segment/playlist fetches — the port's core MP §12 bounded-memory contract is untested in this module's unit lane. What correct looks like: copy the budget check from media.rs's double.
Confidence: HIGH — Verification: read (side-by-side with media.rs:770-786).

F-005 [S4]
Where: crates/streaming/src/hls/master.rs:704-711
Code:
```
    config.validate()?;
    crate::fetch::Url::parse(url).map_err(map_fetch_err)?;
    let uri = SourceUri::parse(url)?;
```
What is wrong: the URL is fully parsed twice (streaming's `Url` then `SourceUri`), doubling validation cost per open and creating two vocabularies for the same string. Cosmetic, but it is duplicated parsing on the open path.
Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/streaming/src/hls/master.rs:651-655
Code:
```
    fn supports_seek(&self) -> bool {
        // Within the retained prefix (the same honest boundary as
        // the VOD session).
        true
    }
```
What is wrong: unconditional `true` while both out-of-window directions are typed refusals at seek time (`streaming.hls.seek-unretained`, `streaming.hls.adaptive-seek-unfetched`, lines 663-682). A host probing capability gets "full seek" and then errors. The comment describes a condition the return value does not encode.
Confidence: HIGH — Verification: read.

F-007 [S4]
Where: crates/streaming/src/hls/master.rs:199-211
Code:
```
        let rate = u128::from(bytes) * 1_000_000 / u128::from(wall_ns);
        self.ewma_bytes_per_ms = Some(match self.ewma_bytes_per_ms {
            // 3/4 old + 1/4 new — shift-only fixed point (D-2).
            Some(prev) => prev - (prev >> 2) + (rate >> 2),
            None => rate,
        });
```
What is wrong: shift-only quantization: any sample below 4 B/ms contributes `rate >> 2 == 0`, and once `prev == 1` the EWMA can never decay to 0 (`1 - 0 + 0 = 1`) — a single tiny sample then zero-rate links report a permanent 8 kbps floor. Bounded impact (only feeds switching decisions that are unreachable today).
Confidence: MEDIUM — Verification: read (arithmetic), not executed.

F-008 [S4]
Where: crates/streaming/src/hls/master.rs:568-569, 494-495
Code:
```
        self.stats.requests += 1;
        let body = match self.fetch.fetch(request) {
```
What is wrong: `requests`/`playlists_fetched` are incremented before the fetch resolves, so failed and refused fetches are counted as served requests; stats semantics drift from the wire truth (`bytes_fetched` is only counted on success, lines 581-582). Metrics-only impact.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| select_variant_index | 142-150 | REAL | — | Ran: selection test passes; ties keep order (stable max_by_key) |
| AbrConfig::Default::default | 127-136 | REAL | — | Constants wired |
| AbrController::new | 183-194 | REAL | — | Plain ctor |
| AbrController::observe_throughput | 199-211 | REAL | S4 | F-007 quantization |
| AbrController::on_buffer_signal | 215-221 | REAL | — | Stall arms, Filled/Cleared disarm, Steady/Drained ignored (by design) |
| AbrController::throughput_bps | 227-230 | REAL | — | u128 clamp before `as u64` — correct |
| AbrController::current | 233-236 | REAL | — | Getter |
| AbrController::snapshot | 241-249 | REAL | — | Copy out |
| AbrController::segment_served | 253-256 | REAL | — | u32 dwell counter can wrap only after 4G segments |
| AbrController::decide | 263-300 | REAL | S3 | F-003 comment lies about dwell; ladder Vec alloc + sort per boundary (trivial) |
| AbrController::commit | 303-312 | REAL | — | Bounded history (cap 16) verified by test |
| HlsMasterConfig::Default::default | 345-354 | REAL | — | 8 MiB response / 64 MiB retention budgets |
| HlsMasterConfig::validate | 362-384 | SHALLOW | S1 | Checks zero-budget and 1 GiB retention cap; accepts the F-001 panic config (retention < segment budget) |
| HlsAdaptiveStream::load_variant_plan | 491-522 | REAL | — | B1a refusal table present (live/empty/encrypted/byterange/map) |
| HlsAdaptiveStream::switch_variant | 528-552 | REAL | — | Sequence alignment + typed refusal `variant-sequence-missing`; seam flagged |
| HlsAdaptiveStream::fetch_next_segment | 555-599 | REAL | S4 | F-008 stats-before-result; `plan[next].clone()` per segment (needless String clone) |
| HlsAdaptiveStream::absorb | 603-617 | REAL (buggy) | S1 | F-001 underflow setup |
| HlsAdaptiveStream::serve | 620-636 | REAL (buggy) | S1 | F-001 panic site |
| HlsAdaptiveStream::master/abr_snapshot/stats/segment_boundaries | 451-474 | REAL | — | Getters |
| HlsAdaptiveStream::note_throughput/on_buffer_signal | 479-487 | REAL | S2 | F-002: no production caller feeds them |
| SourceStream::uri/size/read/position | 640-649, 657-659, 687-689 | REAL | — | size() honest-None until drained; read clamped by MAX_READ_BYTES |
| SourceStream::supports_seek | 651-655 | REAL | S4 | F-006 constant true |
| SourceStream::seek | 661-685 | REAL | — | Typed refusals both directions; `resolve_seek_origin` shared |
| open_master | 704-770 | REAL | S4 | F-005 double parse; error mapping table matches playlists errors |
| tests (8) | 772-1189 | REAL | S3 | See §5; RouterFetch fidelity gap F-004 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| selection_is_highest_under_cap | YES | 5 cases incl. exact-cap boundary and 1-under-cap → None |
| abr_hysteresis_holds_inside_the_band | YES | Holds in band, starve→down after EWMA crossing, no flap at exact margin, rise→up; EWMA arithmetic re-derived by me matches assertions |
| abr_stall_prediction_forces_down | YES | Stall arms → down; cleared + dwell-0 → hold |
| abr_history_is_bounded | YES | 48 toggles; asserts len <= 16 and switch_count >= 16 |
| adaptive_session_switches_on_scripted_bandwidth | YES | Verifies variant bytes ('H'/'M'), continuity +1, 12 boundaries, exact size() after drain |
| adaptive_refusals_are_typed | YES | Asserts exact error codes `no-variant`, `media-playlist` |
| switch_to_misaligned_variant_refuses | YES | Asserts `variant-sequence-missing` on a mis-sequenced rung |
| in_playlist_discontinuity_is_recorded_without_switching | YES | Positional discontinuity flags checked per-boundary |
Caveat: all in-module tests run against RouterFetch, which skips the response-limit contract (F-004) and never exercises real sockets; the real-socket e2e lane is tests/hls_master_e2e.rs (3 tests, mockhttp server, request log as oracle — honest).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "highest BANDWIDTH at or under the cap … never a silent clamp" | master.rs L11-18 | REAL | select_variant_index 142-150 + refusal test |
| "a switch never tears a segment in half" | master.rs L25-27 | REAL | decide only consulted before fetch (560-564) |
| "The controller never reads a clock itself" | master.rs L35 | REAL | No clock access in AbrController (157-313) |
| "a gap is never served silently" | master.rs L50-51 | REAL | variant-sequence-missing refusal 532-542 + test |
| "size() stays None until every segment is fetched" | master.rs L65-66 | REAL | size() 644-649 |
| "production measures each segment fetch" | master.rs L30-31 | FALSE | No production caller of note_throughput exists (F-002) |
| "plain starvation does not [bypass the dwell]" | master.rs L276-277 | FALSE | Down branch has no dwell check (F-003) |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable from shipped entry points: `open_master` callers = master.rs tests, tests/hls_master_e2e.rs, and netfuzz (`select_variant_index` only). apps/cli/src/demux_host.rs:488-490 opens http(s) via `streaming::ranged::RangedResource` — HLS is never opened by lumen-cli or desktop.
- `AbrSnapshot` (318-330) is consumed only by tests; "milestone 3's [PROPOSAL] wiring point" (L238-239) does not exist yet.
- `DEFAULT_MAX_BANDWIDTH_BPS` is a `[PROPOSAL]` constant not wired to any product config (L87-91).
- absorb/serve retained-prefix logic is a near-copy of hls/media.rs and hls/live.rs (three-way duplication; F-001 likely replicated there — flagged to those files' auditors).

## 8. Dependencies & flags
- No feature flags gate this module: compiled unconditionally in `crates/streaming` (Cargo.toml has no [features]); lumen-cli depends on streaming (apps/cli/Cargo.toml:62), so the code ships in the binary but is never called.
- Deps: media-types, plugin-api, playlists (path deps, std-only per crate header).

## 9. Open questions for the next agent
- Does hls/media.rs `absorb`/`serve` (VOD) and hls/live.rs have the same F-001 underflow? Same code shape appears at media.rs:97 constants and live.rs — verify, since VOD/live are the sessions actually closer to wiring.
- Who is ever supposed to call `note_throughput`? If the playback-engine "host" lanes exist, check whether any of them open HLS at all.
