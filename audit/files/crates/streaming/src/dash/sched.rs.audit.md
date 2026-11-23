# AUDIT: crates/streaming/src/dash/sched.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 706 / 7 prod fns + 5 types + 7 tests |
| File verdict | UNREACHABLE (real scheduler; no shipped entry point) |
| Reachable from shipped entry point? | NO — `DashScheduler` referenced only inside `crates/streaming/src/dash/**`; apps/cli has no DASH lane |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `cargo test -p streaming --lib dash::sched` → 7 passed/0 failed; `cargo test -p streaming --lib dash::` → 27 passed/0 failed; rg hunts |

## 1. One-paragraph verdict
The scheduler is real, not a stub: a pure state machine (no I/O, no internal clock) that walks resolved segment plans under ABR control, with a correct Hinnant days-from-civil ISO 8601 parser (I verified the 2026-01-01 = 20,454-day vector by hand), availability arithmetic per ISO/IEC 23009-1 §5.3.9.5 (segment available when its last chunk exists: `AST + PeriodStart + (t+d−PTO)/timescale`), a live-edge join that skips to the newest safely-available segment, and an availability gate that answers `Wait{available_in_s}`. All 7 tests ran green including a dynamic wait-then-join-then-wait timeline. But it is dead code in the shipped product: nothing outside the `dash` module references `DashScheduler`, the CLI has no DASH open lane, and the doc for `next_fetch` contradicts the code for clock-less dynamic manifests. The feature matrix still claims `network-dash` = "partial" for "DASH **playback**" on windows/linux/macos — no shipped binary can play DASH at all.

## 2. What actually works (verified)
— `cargo test -p streaming --lib dash::sched` → `test result: ok. 7 passed; 0 failed` (own run). Hand-verified: `2026-01-01T00:00:00Z` = 20,454·86,400 s (56y + 14 leap days); `segment_availability_epoch(0,0,0,10,10,1)=0` (PTO shifts origin back); live-join at now=10 lands on segment #3 with the 3 s safe distance.

## 3. Findings
F-001 [S2]
Where: docs/feature-matrix.toml:1444-1453 vs crates/streaming/src/dash/sched.rs:257 (pub struct DashScheduler)
Code:
```
id = "network-dash"
vlc_name = "DASH playback"
```
with evidence string:
```
evidence = "crates/streaming/src/dash/** (resolve/fuzz/abr/sched) + crates/playlists/src/mpd.rs (…); streaming 191 lib tests (2 ignored) incl. 27 dash"
```
and status `{ windows = "partial", linux = "partial", macos = "partial", web = "not_started" }` (L1452).
What is wrong: the row is "DASH playback", claimed partial on three desktop platforms, but `rg 'DashScheduler|dash::'` over crates/ and apps/ shows zero consumers outside `crates/streaming/src/dash/**`; apps/cli/src has no DASH/HLS open lane at all. The evidence string proves library-level unit tests, not playback — a user cannot open an MPD in any shipped binary. Why it matters: matrix statuses feed the parity report (already 0/252 verified); "partial playback" without a playback path is inflation. What correct looks like: status not_started (or "library-only" if the schema had it) until a CLI/engine lane opens MPDs. Confidence: HIGH — Verification: ran rg + read of the row.
(Note: the row's own notes field honestly lists pending e2e/DASH-IF items — the status field is what lies.)

F-002 [S3]
Where: crates/streaming/src/dash/sched.rs:341-344 vs 379-387
Code:
```
    /// The next fetch under ABR + availability. `now_epoch` is the
    /// host's observed wall clock (only consulted for dynamic
    /// manifests; `None` means "treat everything as available" —
    /// the static and test posture).
```
vs the dynamic path:
```
        if self.dynamic && !self.joined {
            let now = match now_epoch {
                Some(now) => now,
                None => {
                    return Ok(NextStep::Wait {
                        available_in_s: 0.0,
                    });
                }
```
What is wrong: for a dynamic manifest with `now_epoch = None` the code returns `Wait { available_in_s: 0.0 }` — forever un-joined — which is the exact opposite of the documented "None means treat everything as available". A host following the doc (clock-less, dynamic) receives an infinite sequence of zero-second waits: a busy-loop with no progress and no error. What correct looks like: either the doc tells the truth (dynamic requires a clock; None is a typed error) or None degrades to "join at the last segment of the plan". Confidence: HIGH — Verification: read; no test covers dynamic+None (dynamic_timeline test always passes Some).

F-003 [S4]
Where: crates/streaming/src/dash/sched.rs:124-130
Code:
```
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
```
What is wrong: no month-length validation — `1970-02-31T00:00:00Z` is accepted and silently maps to March 3 (the Hinnant arithmetic just adds day-31 into February), contradicting the doc's "None on any other shape — never a guess" (L93-94). Real manifests never carry Feb 30; blast radius minimal. What correct looks like: per-month day bounds (or days_from_civil round-trip check). Confidence: HIGH — Verification: read (algorithm traced by hand).

F-004 [S4]
Where: crates/streaming/src/dash/sched.rs:614-615
Code:
```
        // Crash the link: eight 0.08 Mbps samples per boundary sink
        // the EWMA below 80% of 800 kbps.
```
What is wrong: `observe_throughput(100, 10_000_000)` is 100 bytes over 10 s = 80 bit/s = 0.00008 Mbps — the comment overstates the sample by 1000×. The test still works (any tiny rate sinks the EWMA), but the stated arithmetic is wrong. Confidence: HIGH — Verification: read + unit math.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SAFE_BEHIND_LIVE_EDGE_S (const) | 38 | REAL | — | 3.0 s, used in join + gate |
| DashSchedError + Display + Error + From | 41-84 | REAL | — | typed dotted codes |
| parse_iso8601_epoch_seconds | 96-142 | REAL | S4 | Hinnant correct (hand-verified); no month-length check (F-003); leap-second clamp documented; `text[19..]` byte-slice is char-boundary-safe (first 19 bytes validated ASCII) |
| split_fraction_and_offset | 147-180 | REAL | — | fraction validated-then-dropped; ±hh:mm bounds checked; no-offset treated as Z (documented decision) |
| segment_availability_epoch | 188-200 | REAL | — | saturating tick math; f64 lossless claim OK for wall-clock seconds |
| live_edge_presentation_seconds | 206-214 | REAL | — | exported, tested, zero non-test callers (dead-in-product, consistent with F-001) |
| ticks_to_seconds | 218-221 | REAL | — | timescale-0→1 documented |
| DashScheduler::new | 286-318 | REAL | — | empty ladder refused; every rung resolved (one broken rung refuses all — documented); AST parsed lazily |
| abr_snapshot / observe_throughput / note_buffer_level / on_buffer_signal | 322-339 | REAL | — | thin forwards to DashAbr |
| DashScheduler::next_fetch | 350-456 | REAL | S3 | boundary ABR + live join + availability gate; F-002 doc contradiction; defensive BadLadder on out-of-range rung; Done on exhausted rung |
| segment_served / segments_served | 460-469 | REAL | — | dwell tick + walk advance |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| iso8601_epoch_vectors | YES | exact epoch values incl. offsets, fraction-drop, leap clamp, refusals |
| availability_is_segment_end_on_the_wall_clock | YES | exact f64 values incl. PTO shift |
| live_edge_computation | YES | exact values for PTO/period/AST shapes |
| static_walk_serves_every_segment_in_order | YES | URL sequence + count (8 = 16 s / 2 s) |
| scripted_crash_steps_the_ladder_down | YES | rung must drop to index 2, numbers stay contiguous |
| dynamic_timeline_waits_then_joins | YES | Wait at now=4, join #3 at now=10, Wait 1.0 s, Fetch #4 at now=11 — precise arithmetic |
| empty_ladder_refuses_typed | YES | BadLadder |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`None` means 'treat everything as available'" | sched.rs:343 | FALSE for dynamic manifests | F-002: Wait{0.0} loop |
| "a mockhttp socket e2e lane is future work (recorded in the feature matrix, not claimed here)" | sched.rs:16-17 | REAL (honest) | no e2e dash test file exists |
| "static MPDs schedule from segment 0, no wall clock needed" | sched.rs:28 | REAL | static test passes None throughout |
| matrix "network-dash" partial (playback) | docs/feature-matrix.toml:1444-1452 | FALSE (no playback path) | F-001 |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (F-001). `live_edge_presentation_seconds` additionally has no caller even inside the crate (test-only).

## 8. Dependencies & flags
No feature gates. Depends on playlists (Mpd model) + crate-internal abr/resolve/buffer. No unsafe.

## 9. Open questions for the next agent
- The matrix `network-hls` row: does it make the same "playback" claim without a CLI lane? (apps/cli has no hls either — same audit shape as F-001.)
