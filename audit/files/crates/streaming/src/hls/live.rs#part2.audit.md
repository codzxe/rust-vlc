# AUDIT: crates/streaming/src/hls/live.rs#part2 (lines 492–982)
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-21 |
| Lines / functions | 491 (of 1473) / 8 fns + tests (7 fns + 3 helpers) |
| File verdict | UNREACHABLE (real, e2e-tested live session — no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `open_live` callers: `crates/streaming/tests/{hls_live_e2e,net_interrupted,hls_conformance_e2e}.rs` + in-file tests only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "open_live"` (callers, above); `rg -n "segments.is_empty\|empty" crates/playlists/src/hls.rs` (parser accepts 0-segment media playlists — empty-reload reachable); `rg -n "fn fetch_playlist_model" -A 25 crates/streaming/src/hls/media.rs`; `sed -n` on all quoted ranges; baseline INDEX #3 (streaming tests green incl. hls_live_e2e). |

## 1. One-paragraph verdict
The live-session reload/window engine (my range: `reload`, the `SourceStream` impl, `open_live`, and the first tests) is a thoughtfully-typed state machine — window regression, gap detection, stall budget, mid-stream encryption refusal all fail typed — and its slide-by-absolute-sequence core is pinned by a byte-exact test. Two real logic defects sit inside it: the live-edge join keeps `back` segments while two doc statements promise "back segments behind the newest" / "0 = join at the newest" (an off-by-one that makes `live_edge_back: 0` skip the entire window and reload immediately), and an empty reload window bumps `max_seen_seq` by a phantom sequence — contradicting its own inline comment and poisoning the productivity detector for a following single-segment window. The same segment-body-over-retention-budget panic I proved for master.rs exists here too, but its code sits in part1 (A02) — cross-flagged, not double-counted. Nothing shipped reaches this lane.

## 2. What actually works (verified)
— Window slide by absolute sequence: pinned by `window_slide_maps_by_absolute_sequence` (:924-966) — asserts exact concatenation `BBBBCCDDDDEE` across two slides, `segments_fetched == 4` (no refetch), stall reset on progress. Mutation check: inverting the `>= next_abs_seq` filter (:567) or dropping the `max_seen_seq` update (:585) fails its assertions. Green in baseline `cargo test --workspace` (INDEX #3).
— Typed refusal surface verified by trace: `window-regression` (:543-550), `segment-lost-window` gap semantics (:554-559 + serve :436-449 — permanent refusal after a gap, never silently bridged), `live-timeout` stall budget (:594-603, `>` comparison ⇒ budget+1th consecutive stall errors), mid-live KEY/BYTERANGE/MAP refusals (:532-540), ENDLIST end + exact-total `size()` (:614-619 with `!gap_pending`).

## 3. Findings

### F-001 [S3] Live-edge join is off-by-one against both of its own doc statements
Where: crates/streaming/src/hls/live.rs:720-723 (doc at :10-13 and :130-132)
Code:
```
    let back = config.live_edge_back as usize;
    let skip = model.segments.len().saturating_sub(back);
    let media_sequence = model.media_sequence;
    let next_abs_seq = media_sequence + skip as u64;
```
What is wrong: `skip = len − back` keeps exactly the last `back` segments. The module doc says the session "starts [`live_edge_back`] segments **behind the newest** segment" (:10-13) — i.e. keep `back+1` segments (newest inclusive); the config doc says "0 = join at the newest segment" (:130-132) — keep 1. The code with `back = 0` keeps **zero** segments (`skip = len`, `pending` empty) and immediately reloads on the first read; with the default `back = 3` it joins 2 behind the newest, not 3.
Why it matters: the join point is the session's core latency/runway policy; both doc statements are wrong about it in opposite directions, and the "0 = join at newest" configuration is unusable as documented (it stalls until the next reload produces anything). The in-file test pins the code's semantics (:911-912 "back=2 of a 3-segment window: seg1+seg2") without flagging the doc contradiction.
What correct looks like: either `let skip = len.saturating_sub(back + 1)` (doc semantics) or fix both doc statements to "keep the last `back` segments of the window" and document `0` as "wait for the next reload".
Confidence: HIGH — Verification: read (arithmetic + test cross-check at :892-919).

### F-002 [S3] Empty reload window bumps `max_seen_seq` by a phantom sequence, contradicting the adjacent comment and poisoning progress detection
Where: crates/streaming/src/hls/live.rs:582-585
Code:
```
        // `saturating_sub`: an empty transient window must not
        // underflow (and cannot be progress either).
        let window_max = model.media_sequence + model.segments.len().saturating_sub(1) as u64;
        self.max_seen_seq = self.max_seen_seq.max(window_max);
```
What is wrong: with `len == 0` the expression yields `media_sequence + 0` — it records that a segment exists at `media_sequence` when the window shows none. The parser accepts 0-segment media playlists (no `empty-playlist` refusal exists on the reload path — only at open, :704-706), so a transient empty window (real CDN behavior) is reachable. Consequences: (a) the comment's claim "cannot be progress either" is false — `max_seen_seq` advances; (b) when the segment at that base finally appears, `abs_seq > max_seen_seq` (:580) is false for a single-segment window, so a genuinely productive reload is counted as stalled, double-spending the `max_stalled_reloads` budget per empty-window hiccup and able to produce a spurious `streaming.hls.live-timeout` on a live server.
Why it matters: premature typed failure on flaky-but-alive live sources; the bookkeeping invariant ("highest sequence any window has shown") is violated by construction.
What correct looks like: `let window_max = model.media_sequence + model.segments.len().checked_sub(1)? as u64;`-equivalent — skip the bump (or use `media_sequence - 1`) when the window is empty.
Confidence: MEDIUM-HIGH — Verification: read (empty-parse acceptance verified in playlists/src/hls.rs; no empty check in `reload`); execution UNVERIFIED (no cargo).

### F-003 [S4] Failed reloads are invisible in the stats
Where: crates/streaming/src/hls/live.rs:515-520
Code:
```
        let (model, timestamp_map, playlist_bytes) =
            fetch_playlist_model(&self.fetch, &self.url, self.config.playlist_response_limit)?;
        self.timestamp_map = timestamp_map;
        self.stats.requests += 1;
        self.stats.playlist_reloads += 1;
```
What is wrong: `requests` is documented as "HTTP requests issued (playlist fetches + segment fetches)" (:215-216) but is incremented only after a successful fetch+parse; a transport failure or HTTP error on a reload issued a request that is never counted.
Why it matters: the stats are the observability surface for "honest logging"; retry storms against a dead origin show `requests == 1`.
What correct looks like: increment `requests` before the fetch (or count failures separately).
Confidence: HIGH — Verification: read.

## 4. Per-function table (part2 range; boundary: `reload_wait_ms` :476-494 straddles — its fallback logic :480-487 is part1/A02's, only the stalled/full tail :489-494 is mine)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `reload_wait_ms` (tail) | 489-494 | REAL | — | half-wait after stall ✓; see §9 cross-flag on the fallback |
| `reload` | 499-606 | REAL | S3,S4 | F-002 (:582-585), F-003 (:515-520); pacing opt-in with re-anchor ✓ (:505-526); regression/gap/refusal logic correct ✓; stall budget `>` ⇒ budget tolerated then error ✓ |
| `SourceStream::uri`/`size` | 610-619 | REAL | — | unknown-length contract correct (ended ∧ drained ∧ no-gap) |
| `supports_seek` | 621-626 | REAL | — | returns `true` while actual capability is [retained_from, fetched_upto] — self-documented in comment; same pattern as master.rs |
| `read` | 628-630 | REAL | — | capped by MAX_READ_BYTES ✓ |
| `seek` | 632-657 | REAL | — | backward-within-retained, forward-within-fetched, both refusals typed ✓ |
| `position` | 659-661 | REAL | — | trivial |
| `open_live` | 682-765 | REAL | S3 | F-001 (:720-723); refusal table complete ✓ (:701-715); pacing anchor recorded at open ✓ (:742) |
| tests: `LiveRouter` | 783-850 | REAL | — | real `HttpFetch` port, mutable playlist route, poison-safe locks, budget enforcement |
| tests: `drain`/`read_exactly`/`fast_config` | 855-888 | REAL | — | helpers |
| test `live_open_joins_back_from_the_edge` | 892-919 | REAL | — | pins the CODE's join semantics (see F-001); would fail if the join arithmetic changed |
| test `window_slide_maps_by_absolute_sequence` | 924-966 | REAL | — | byte-exact slide pinning; strong |
| test `endlist_terminates_and_fixes_the_total` | 971-982+ | REAL | — | tail is part3/A04's |

## 5. Test honesty
Tests at :892-966 (mine) and :971+ (A04's) all assert exact bytes/counts and would fail under the mutations I reasoned through (inverted slide filter → refetch count changes; broken stall reset → `stalled_reloads` assertion fails). **Unpinned:** the doc's join semantics (F-001 — the test documents the code, not the doc), empty-window reloads (F-002), segment bodies over the retention budget (the panic below), and `live_edge_back: 0`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "starts live_edge_back segments behind the newest segment (default 3…)" | live.rs:10-13 | FALSE | F-001: keeps `back`, joins `back−1` behind newest |
| "0 = join at the newest segment" | live.rs:130-132 | FALSE | F-001: keeps 0 segments, reloads immediately |
| "A window that moves backwards … refuses typed (window-regression)" | live.rs:46-48 | REAL | :543-550 |
| "the next read that needs the lost bytes fails typed (segment-lost-window)" | live.rs:40-45 | REAL | :554-559 + serve :436-449 (permanent, never bridged) |
| "max_stalled_reloads consecutive stalled reloads fail typed" | live.rs:25-27 | REAL (with F-002 caveat) | :592-603; empty windows can mis-spend the budget |
| "still refused … at open and on every reload" | live.rs:70-77 | REAL | :532-540 |
| network-hls "partial" incl. "6.B1b live window + recorded discontinuities" | docs/feature-matrix.toml:1431-1441 | REAL-UNPROVEN + UNREACHABLE | tests/e2e green (baseline #3); no shipped consumer |

## 7. Dead / duplicated / unreachable code
`open_live`/`HlsLiveStream` are consumed only by `crates/streaming/tests/{hls_live_e2e,net_interrupted,hls_conformance_e2e}.rs` and the in-file tests. The CLI's streaming usage is fetch/ranged/schemes/mockhttp only; `lumen-tauri` has no streaming dependency. Note: even if wired, the module's own doc (:54-58) concedes the container layer refuses unknown-length sources — live playback needs a "later integration" container lane that does not exist.

## 8. Dependencies & flags
`playlists::MediaPlaylist`, `plugin_api::SourceStream`, `crate::clock::WallClock` (pacing port), `crate::fetch`, `super::media` helpers. No feature gates. No unsafe. Workspace build clean (baseline #1), tests green (baseline #3).

## 9. Open questions for the next agent
- **A02 (live.rs part1)** — two defects in your range, cross-flagged from my analysis: (1) the **same S1 panic as master.rs F-001**: `absorb` (:406-420) + `serve` (:427-434) — a segment body over `max_retained_bytes` (or `max_retained_bytes: 0`, which `validate` :187-209 accepts) sets `retained_from = fetched_upto` while `pos < fetched_upto` → `self.pos - self.retained_from` underflows at :431 → slice panic; (2) `reload_wait_ms`'s EXTINF fallback (:480-487) reads `self.pending`, which is **always empty when reload runs** (serve only reloads on a dry window) — the "largest EXTINF" fallback is dead and every TARGETDURATION-less window waits the 1 s floor.
- A04 (part3): the remaining tests (ENDLIST, stall budget, pacing with FakeClock, gap/window-regression e2e) — verify one pins F-002's empty-window shape (none found up to :982).
- Orchestrator: no REQUEST-ORCH-RUN (all findings code-traced deterministic).
