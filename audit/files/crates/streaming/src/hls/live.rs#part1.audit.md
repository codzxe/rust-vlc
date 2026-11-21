# AUDIT: crates/streaming/src/hls/live.rs#part1 (lines 1-494)
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = **lines 1-494**: module docs, config/stats/model types, the `HlsLiveStream` struct + accessors, `fetch_pending`, `absorb`, `serve`, `reload_wait_ms` (ends 494). Boundary taken at the end of `reload_wait_ms` (3 lines past the equal split 1-491, which would cut it mid-body); part2 (W10-A06) = `reload` (499-606) + `SourceStream` impl (609-662) + `open_live` (682-765) + tests start; part3 (W10-A03) = tests. I read 495-766 for context (reload/seek/open traced for the invariant analysis below) but did not audit it. |
| Lines / functions | 494 / 20 (see §4) |
| File verdict | UNREACHABLE (real, socket-tested live session — but zero production callers; the module doc itself admits the demux hosts refuse unknown-length sources) |
| Reachable from shipped entry point? | **NO.** `rg -n "open_live\|HlsLiveStream\|HlsLiveConfig" crates/ apps/ -g '!crates/streaming/src/hls/live.rs'` → `timestamp_map.rs:36` (doc comment) and test files only (`tests/hls_live_e2e.rs`, `tests/net_interrupted.rs`, `tests/hls_conformance_e2e.rs`). The CLI's http(s) lane opens URLs as `RangedResource` and container-probes them (demux_host.rs:441-442) — no HLS session is ever constructed; a `.m3u8` URL is refused at the probe. |
| Findings | S0: 0 S1: 1 S2: 1 S3: 1 S4: 0 |
| Commands run | `rg -n "^(pub )?(fn\|struct\|enum\|impl)" crates/streaming/src/hls/live.rs` (outline); Read 1-770 (part1 + context); `sed -n '406,420p;430,435p;450,452p;391,399p'` (quote verification); `rg -n "DEFAULT_RETAINED_BYTES\|MAX_RETAINED_BYTES\|MAX_READ_BYTES" crates/streaming/src/hls/media.rs` (budgets: 64 MiB default retention, 1 GiB cap, 64 MiB read cap) + `rg -n "DEFAULT_RESPONSE_LIMIT" crates/streaming/src/fetch.rs` (8 MiB); `rg -n "fn absorb" -A 15 crates/streaming/src/hls/media.rs` (VOD twin of F-001 confirmed identical); `rg -n "open_live" crates/ apps/` (reachability); `sed -n '280,400p' crates/streaming/tests/net_interrupted.rs` (retry-after-error untested); `rg -n "hls::live::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (15/15 ok :5029-5043) + `sed -n '5195,5204p'` (hls_live_e2e 2/2 ok); `sed -n '1431,1443p' docs/feature-matrix.toml` (network-hls row). |

## 1. One-paragraph verdict
The live-session state machine is genuinely designed: absolute-sequence window mapping, typed refusals for every dishonest shape (encryption/byterange/MAP arriving mid-live, window regression, segments lost to the slide, unfetched forward seeks), demand-driven reloads with a stall budget, opt-in RFC 8216 §6.3.4 pacing through an injected clock, and 15 unit + 2 socket e2e + fault-injection tests all green in baseline. But the retained-prefix buffer has a real panic: a single segment body larger than `max_retained_bytes` (a *validated, legal* config — nothing requires retention ≥ segment budget) is silently discarded while `fetched_upto` advances past it, and the next read subtracts `retained_from > pos` in unsigned arithmetic — underflow panic in debug, slice-OOB panic in release (F-001, S1; the VOD session in media.rs:326-351 carries the identical code). Second defect: `serve` pops a pending segment *before* fetching it, so a failed fetch permanently drops that segment — a retried read concatenates around the hole with no gap flag (F-002, S3). And the wave's verdict: none of this is reachable from the shipped binary, while the matrix's network-hls row says "partial" on three platforms (F-003, S2).

## 2. What actually works (verified)
— `hls::live::tests` 15/15 `ok` (window slide by absolute sequence, ENDLIST total fixation, discontinuity recording, paced cadences via FakeClock, refusal table, seek honesty, window regression, timestamp-map supersedes-per-reload) — `orchrun-test-workspace-nofailfast.log.txt:5029-5043`.
— `tests/hls_live_e2e.rs` 2/2 `ok` over a real socket: `live_slides_endlist_and_discontinuity_over_a_real_socket`, `stalled_live_source_fails_typed_live_timeout` — same log :5195-5204. `tests/net_interrupted.rs::live_window_edge_survives_wire_delay` byte-exact against the on-disk oracle with scripted wire delays — same run (net_interrupted section).
— Panic-freedom of the *normal* serve path traced: the invariant `retained_from ≤ pos ≤ fetched_upto` holds whenever the last absorbed body was retained or head-dropped with `retained_from = start = pos` (absorb is only reached with `pos == fetched_upto`, since serve fetches only when `pos == fetched_upto` and forward seeks beyond `fetched_upto` are refused at 644-653) — the one violation is F-001's over-budget branch.
— Stall budget traced: 5 consecutive unproductive reloads (default 4 + the `>` check at 594) end in the typed `streaming.hls.live-timeout` — no infinite reload loop in `serve`.

## 3. Findings

### F-001 [S1]
Where: crates/streaming/src/hls/live.rs:406-420 with 430-434
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
```
            if self.pos < self.fetched_upto {
                let at = (self.pos - self.retained_from) as usize;
                let have = self.retained.len() - at;
```
What is wrong: When a single segment body exceeds `max_retained_bytes`, `absorb` discards the body entirely and sets `retained_from = fetched_upto` — *past* the consumer's `pos` (which equals the body's start, since absorb only runs when `pos == fetched_upto`). The next read enters the `pos < fetched_upto` branch (the discarded body's bytes are "fetched"), computes `pos - retained_from` with `retained_from > pos`: u64 underflow — arithmetic overflow panic in debug builds; in release `at` wraps huge and `&self.retained[at..at + take]` panics with slice out-of-range. The config that triggers it is fully legal: `HlsLiveConfig::validate` (187-209) checks only non-zero limits and retention ≤ 1 GiB — e.g. `max_retained_bytes: 1 MiB` with the default `segment_response_limit: 8 MiB` (fetch.rs:28) passes validation, and any segment body between those sizes detonates the first read that demands it.
Why it matters: Panic on valid input from a security-relevant, untrusted-server-facing path — a hostile (or merely unusual) HLS server crashes any consumer that configured a modest retention budget; even before the panic the design is broken because the fetched bytes are silently discarded while `fetched_upto`/stats/boundaries record them as served. The VOD session in `media.rs:326-351` carries the identical `absorb`/`serve` pair, so the same S1 ships twice. No test covers the over-retention-budget body in either file (rg for retention-budget tests: only the response-limit refusal `oversize_segment_refuses_over_budget`, media.rs:978, is pinned).
What correct looks like: Refuse the segment typed when it cannot be retained (`streaming.hls.segment-unretainable`, mirroring the response-over-budget refusal), and/or have `validate` require `max_retained_bytes >= segment_response_limit`; the read loop must never face `pos < retained_from` without a typed error.
Confidence: HIGH — Verification: read + full invariant trace (§2); constants verified (media.rs:97/102, fetch.rs:28); VOD twin compared verbatim.

### F-002 [S3]
Where: crates/streaming/src/hls/live.rs:450-452 with 391-397
Code:
```
            } else if let Some(seg) = self.pending.front().cloned() {
                self.pending.pop_front();
                self.fetch_pending(seg)?;
```
What is wrong: The pending segment is popped *before* the fetch; `next_abs_seq` advances only on success (397). If `fetch_pending` errors (network weather mid-live — exactly what this module exists for), the segment is gone from `pending` but the cursor still expects it. A subsequent `read` (the SourceStream contract does not forbid calling read after an error; the engine's own reconnect ambitions are this crate's neighbor module) finds `pos == fetched_upto`, `gap_pending == false`, and fetches the *next* segment — the failed segment's bytes are silently missing from the concatenation, no typed refusal, no `segments_lost_window` accounting.
Why it matters: The module's own standard is "that is never served silently" (module doc :40-44, the segment-lost-window refusal). This path serves a gap silently — a corrupted TS concatenation (decoder sees a PTS jump) instead of the honest typed error. The fault-injection suite (`net_interrupted.rs`) tests *delays* that heal, never a hard fetch failure followed by a retry — the hole is untested.
What correct looks like: Peek the front, fetch, and pop only on success (or `push_front` the segment back on error / set `gap_pending` and account it in `segments_lost_window`).
Confidence: MEDIUM — Verification: read (control flow traced); exploitability depends on the caller retrying after Err, which no shipped caller does today (the module is unreachable) — but the trait contract (plugin-api/src/access.rs:24-28) imposes no error-fatal rule.
### F-003 [S2]
Where: crates/streaming/src/hls/live.rs:1-6 vs docs/feature-matrix.toml:1431-1439
Code:
```
//! The HLS **live media-playlist session** (card 6.B1b, the card's
//! owned file): a playlist WITHOUT `#EXT-X-ENDLIST` — the live shape
//! B1a refused typed — opens a sliding-window session over the same
//! [`HttpFetch`](crate::fetch::HttpFetch) port and serves the played
//! window's segment concatenation as one continuous
//! [`SourceStream`](plugin_api::SourceStream).
```
```
id = "network-hls"
...
status = { windows = "partial", linux = "partial", macos = "partial", web = "not_started" }
```
What is wrong: The matrix asserts HLS playback is "partial" on windows/linux/macos citing this module's tests as evidence, but `open_live` (and the whole `streaming::hls` tree) has zero production callers — the shipped CLI opens URLs as raw ranged byte streams and container-probes them. A user cannot play any HLS stream, VOD or live, in any shipped binary. The module's own doc admits the integration gap ("live playback through the existing demux hosts is a later integration", :54-58) — the matrix row does not.
Why it matters: Same claim-vs-evidence drift as network-dash (my sched.rs F-003): library tests are presented as platform-level playback availability. "HLS playback" partial on three platforms is false at the product level — it is 0% reachable.
What correct looks like: Platform statuses "not_started" with the library work (which is real and strong) described in notes/evidence, or an explicit "unwired" qualifier — one standard shared with network-dash and network-remote-playlists (which currently says not_started for equally-implemented code).
Confidence: HIGH — Verification: rg call-chain (header) + CLI lane read (demux_host.rs:441-442) + module doc admission at :54-58.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-87 | REAL | — | Reload/window model, unknown-length honesty, refusal table, seek policy — every claim cross-checked against code; the "later integration" admission (54-58) is the honest line the matrix row lacks. |
| `DEFAULT_LIVE_EDGE_BACK` / `DEFAULT_MAX_STALLED_RELOADS` | 106-115 | REAL | — | 3 / 4; both pinned by tests (`live_open_joins_back_from_the_edge`, `stalled_reloads_fail_typed_live_timeout`). |
| `HlsLiveConfig` | 119-149 | REAL | — | All budgets explicit; `pacing_clock` opt-in documented per knob. |
| `Default for HlsLiveConfig` | 151-162 | REAL | — | 8 MiB response limits, 64 MiB retention, edge 3, stalls 4, no clock. |
| `Debug for HlsLiveConfig` | 167-178 | REAL | — | Clock surfaced as armed/not — honest summary. |
| `HlsLiveConfig::validate` | 180-210 | REAL | S1-adjacent (F-001) | Refuses zero budgets / >1 GiB retention; does NOT relate retention to segment budget — the hole F-001 walks through. |
| `HlsLiveStats` | 212-235 | REAL | — | Copy struct; all fields written (requests/reloads/stalls/paced/fetched/lost/discontinuities/bytes). |
| `PendingSegment` | 239-249 | REAL | — | Keyed by abs seq; all fields consumed. |
| `HlsLiveStream` | 253-300 | REAL | — | 17 fields, all used; comment documents the retained-prefix shape `[retained_from, fetched_upto)`. |
| `window`/`media_sequence`/`stats`/`timestamp_map`/`segment_boundaries` | 305-366 | REAL | — | Read-only accessors, `#[must_use]`. |
| `live_edge_wallclock_ms` | 347-357 | REAL | — | LOCAL + Σ EXTINF; anchor assumption honestly marked [VERIFY] (:42-43); negative durations cast to 0 (saturating) — safe. |
| `fetch_pending` | 370-400 | REAL | S3 (F-002) | Status→NetworkError mapping with client/server classification; boundary recorded pre-absorb; `next_abs_seq` on success only. |
| `absorb` | 406-420 | REAL | **S1 (F-001)** | Over-budget single body: discarded + `retained_from` past `pos` → read underflow/OOB. VOD twin at media.rs:326-340 identical. |
| `serve` | 427-468 | REAL | S1 (F-001), S3 (F-002) | Pull loop: retained serve → gap refusal (typed when empty, partial when bytes in hand) → pending fetch → EOF → partial answer → reload. Normal-path invariant traced panic-free (§2). |
| `reload_wait_ms` | 476-494 | REAL | — | TARGETDURATION → largest EXTINF (ceil) → 1s floor (never zero — no busy-loop); stalled halves the wait; saturating mul. |

## 5. Test honesty
Tests live at 767+ (part2/3) and in `tests/`; all green in baseline (:5029-5043, :5195-5204).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `window_slide_maps_by_absolute_sequence` | YES | Byte-exact concatenation across window slides — a refetch or a mis-map breaks equality. |
| `live_open_joins_back_from_the_edge` | YES | Pins the join point (edge-back 3). |
| `slid_out_segments_refuse_typed_at_the_gap` | YES | The typed gap refusal — the very contract F-002 violates on the fetch-error path. |
| `stalled_reloads_fail_typed_live_timeout` | YES | Budget exhaustion → typed error (also e2e: `stalled_live_source_fails_typed_live_timeout`). |
| `paced_*` (2) | YES | FakeClock sleep assertions — cadence is asserted, not waited. |
| `live_seek_is_honest` / `window_regression_refuses_typed` / `encryption_arriving_on_reload_refuses` / `endlist_terminates_and_fixes_the_total` / `discontinuity_is_recorded_across_the_seam` / `timestamp_map_*` (2) / `live_refusal_table_is_typed` / `live_config_bounds_refuse_typed` | YES | Each pins a named typed refusal or record. |
| **Gap**: no test feeds a segment body > `max_retained_bytes` (F-001 untested); no test retries a read after a fetch failure (F-002 untested) — verified by rg over both files' tests. | | |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "live window + recorded discontinuities" (card 6.B1b) | feature-matrix.toml:1440 (network-hls evidence) | REAL (library), UNREACHABLE (product) | 15/15 + e2e 2/2 green; zero production callers (header). |
| network-hls windows/linux/macos = "partial" | feature-matrix.toml:1439 | **Drift (F-003)** | Shipped CLI has no HLS lane at all (demux_host.rs:441-442). |
| "reads then return empty (EndOfStream), and size() becomes the exact drained total" | live.rs:36-39 | REAL | `endlist_terminates_and_fixes_the_total` green; size() gate read (614-619). |
| "that is never served silently" (lost-window gap) | live.rs:40-44 | REAL for the slide path, **SHALLOW for the fetch-error path** | Slide → typed refusal (test green); fetch-error → silent skip (F-002). |
| "never a zero wait, which would degenerate the pacing into a reload busy-loop" | live.rs:473-475 | REAL | `.max(1)` floor at 485/487 read. |

## 7. Dead / duplicated / unreachable code
- Module production-unreachable (header).
- Duplication: `absorb` + the serve retained-branch are copied verbatim from/into the VOD session (media.rs:326-351) — the F-001 bug therefore exists in two files; a shared helper would have made it one fix.

## 8. Dependencies & flags
- `media_types` (typed EngineError minting), `plugin_api::SourceStream`, `playlists::MediaPlaylist`, `crate::{clock, error, fetch, hls::media, hls::timestamp_map}`. No feature gates; no unsafe; no unwrap/expect in production paths (workspace lints).

## 9. Open questions for the next agent
- W10-A09 (hls/media.rs#part1): the VOD session's `absorb`/`serve` (media.rs:326-351) is the F-001 twin — please file/merge the S1 there (same underflow, same legal-config trigger, VOD's `open_vod` also unreachable but same blast radius).
- W10-A06 (hls/live.rs#part2): does `reload`'s pending-rebuild filter (`>= next_abs_seq`, :567) interact with F-002's dropped segment? (A window containing the dropped segment would re-queue it via the filter — partial self-heal on the NEXT reload; the silent skip only holds until a reload re-lists it, making the corruption intermittent and harder to diagnose.)
- Whoever audits `reconnect.rs`: does any reconnect policy retry `read` after an error on a live stream? If yes, F-002 is your entry point.
