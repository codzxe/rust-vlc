# AUDIT: crates/streaming/src/dash/sched.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-21 |
| Lines / functions | 705 / 25 (18 prod items + 3 test helpers + 7 tests, see §4) |
| File verdict | UNREACHABLE (real, tested, panic-free state machine — but no shipped entry point reaches it) |
| Reachable from shipped entry point? | **NO.** `rg -ln "dash::|DashScheduler|DashSession|dash_session" apps/ crates/` → only `crates/streaming/src/dash/sched.rs` itself and the re-export in `crates/streaming/src/dash/mod.rs:36-39`. The shipped CLI opens http(s) URLs as raw ranged byte streams (`apps/cli/src/demux_host.rs:490-493` — `streaming::ranged::RangedResource` / `SchemeRegistry::open_http`), never as an MPD session; no app (`apps/cli`, `apps/desktop`, `apps/desktop-tauri`, `apps/control-server`, `apps/web`) references the dash module. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg -n "DashScheduler\|dash::sched\|next_fetch\|SAFE_BEHIND_LIVE_EDGE" apps/ crates/` (callers: none); `rg -n "dash\|Dash" apps/cli/src/` (no output — CLI has zero DASH references); `rg -ln "dash::\|DashScheduler\|DashSession\|dash_session" apps/ crates/` (2 files: sched.rs + mod.rs); `sed -n '96,100p;95p;124,130p;341,345p;379,387p'` (quote verification — NOTE: sandbox `sed` mangles lines beginning `#[m…` into `#ust_use]`; those verified with the Read tool); `sed -n '110,340p' crates/streaming/src/dash/abr.rs` (decide/current/commit semantics — current index provably stays within ladder bounds); `rg -n "dash::sched" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (7/7 tests green); `rg -n -i "dash" docs/feature-matrix.toml` (row `network-dash`, lines 1444-1454). |

## 1. One-paragraph verdict
The scheduler is a genuinely implemented pure state machine: correct Hinnant civil-date arithmetic, char-boundary-safe ISO 8601 parsing (I traced every slice index — `&text[19..]` is only reached when bytes 17-18 are ASCII digits, so 19 is always a boundary), saturating tick math, defensive `get()`/exhaust checks instead of panics, and honest typed refusals (`BadLadder`, `BadAvailabilityTime`). Its 7 tests are real behavioral tests (assert concrete URLs, counts, wait durations), all green in the baseline orchestrator run (`orchrun-test-workspace-nofailfast.log.txt:5010-5016`). But the hostile question answers itself: **none of this is reachable from any shipped binary** — no production host exists for `DashScheduler`; the CLI treats every URL as a raw ranged resource. The feature matrix nevertheless claims `network-dash` = "partial" on windows/linux/macos (F-003). Two localized defects: the `next_fetch` doc contradicts the dynamic+`None` behavior (F-001), and the date parser accepts impossible days (F-002).

## 2. What actually works (verified)
— `dash::sched::tests::*` (7/7: `iso8601_epoch_vectors`, `availability_is_segment_end_on_the_wall_clock`, `live_edge_computation`, `static_walk_serves_every_segment_in_order`, `scripted_crash_steps_the_ladder_down`, `dynamic_timeline_waits_then_joins`, `empty_ladder_refuses_typed`) all `ok` — `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5010-5016` (orchestrator rerun of baseline test suite #3/#4).
— Panic-freedom traced by reading: every `self.plans[...]` index (393, 412, 431, 448) is guarded by `self.plans.get(self.abr.current())` at 361-364 plus the exhaust check 366-371; `abr.current()` provably stays in `0..plans.len()` (starts from `select_representation_index` over the same `set.representations` order, and `DashAbr::commit` only moves within sorted ladder indices — abr.rs:337-345).
— Live-join arithmetic hand-checked: 10-segment timeline `t=0,d=2,r=9`, AST=epoch → at now=10 the newest safely-available (avail+3≤now) segment is #3, matching the test's `live-3.m4s` assertion at 667-673.

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/dash/sched.rs:341-345 and 379-387
Code:
```
    /// The next fetch under ABR + availability. `now_epoch` is the
    /// host's observed wall clock (only consulted for dynamic
    /// manifests; `None` means "treat everything as available" —
    /// the static and test posture).
```
```
        if self.dynamic && !self.joined {
            let now = match now_epoch {
                Some(now) => now,
                None => {
                    return Ok(NextStep::Wait {
                        available_in_s: 0.0,
                    });
                }
            };
```
What is wrong: The doc promises `None` means "treat everything as available"; the code makes `None` mean "nothing is ever available" for dynamic manifests — it returns `Wait { available_in_s: 0.0 }` on every call, forever (the join never completes and the gating block is skipped because `now_epoch` is `None`).
Why it matters: A host that follows the doc (pass `None` to mean "no clock, fetch anyway") busy-loops `next_fetch` at 0-second intervals on a dynamic MPD — an infinite spin with no progress and no error. The two postures are opposite behaviors; one of them is a lie.
What correct looks like: Either `None` skips the join/gating for dynamic manifests (matching the doc), or the doc says plainly: "`None` on a dynamic manifest answers `Wait { 0.0 }` — the host must supply a wall clock."
Confidence: HIGH — Verification: read (both blocks traced; no state change on the `None` path — `joined` can never become true).

### F-002 [S3]
Where: crates/streaming/src/dash/sched.rs:124-130
Code:
```
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let second = second.min(59); // leap second clamp
```
What is wrong: Day is validated only against 1..=31 — impossible dates are accepted and silently normalized by the civil-days algorithm: `2026-02-30T00:00:00Z` parses to the epoch of **March 2** (Hinnant `doy` for day 30 in month 2 rolls over the month), `2026-04-31` → May 1. There is no day-in-month or leap-year check.
Why it matters: `availabilityStartTime` drives the live-edge join; a server manifest with a typo'd date produces a live edge shifted by days with no diagnostic — the scheduler will `Wait`/skip to the wrong segment with a confident `Ok`. Real date parsers (RFC 3339) reject these shapes; "never a guess" (the module's own standard, line 93) is violated.
What correct looks like: Validate day ≤ days_in_month(year, month) (leap-aware) and return `None` otherwise, next to the existing month/hour checks.
Confidence: HIGH — Verification: read + hand-executed the algorithm (`2026-02-30` → y=2025, mp=11, doy=366 → days=20514 = 2026-03-02).

### F-003 [S2]
Where: crates/streaming/src/dash/sched.rs:1-17 (module) vs docs/feature-matrix.toml:1444-1451
Code:
```
//! DASH **segment scheduling + live availability-time sync** (card
//! 6.C2): the segment walk over 6.C1b [`RepresentationPlan`]s under
//! 6.C2 [`DashAbr`] control, and the wall-clock arithmetic that
//! makes *dynamic* MPDs schedulable — ISO 8601
```
```
id = "network-dash"
...
status = { windows = "partial", linux = "partial", macos = "partial", web = "not_started" }
```
What is wrong: The feature matrix asserts DASH playback is "partial" on windows/linux/macos, citing this module's tests as evidence — but `DashScheduler` (and the entire `streaming::dash` tree) has zero production callers. No shipped entry point (`lumen` CLI, desktop, tauri shell, control-server, web) constructs a DASH session; the CLI's URL lane (`apps/cli/src/demux_host.rs:490-493`) opens only raw ranged byte streams.
Why it matters: A reader of the matrix believes MPEG-DASH playback is partially available in the product on three platforms. In the shipped binary it is available on **zero** platforms — a `.mpd` URL handed to `lumen` is probed as a container and fails; nothing parses it. Library-level "partial" is presented as platform-level "partial". This is the exact network claim-vs-evidence gap this wave was told to hunt.
What correct looks like: Either the row's platform statuses are "not_started" (with the library work noted in `notes`, which are honest), or the row gains an explicit "unwired: no entry point reaches the scheduler" qualifier — as it stands the evidence string ("streaming 191 lib tests incl. 27 dash") proves the library, not the feature.
Confidence: HIGH — Verification: rg call-chain (see header) + baseline INDEX.md #11 (CLI links `streaming` only for the ranged lane) + feature-matrix row read.

### F-004 [S4]
Where: crates/streaming/src/dash/sched.rs:412-424 and 428-447
Code:
```
            let rung = &self.plans[self.abr.current()];
            let seg = &rung.segments[self.next];
            let avail = segment_availability_epoch(
                ast,
                self.period_start_s,
                seg.time,
                seg.duration,
                rung.presentation_time_offset,
                rung.timescale,
            );
            if now as f64 >= avail + self.safe_behind_live_edge_s {
                self.joined = true;
            }
```
What is wrong: The positioned segment's availability is computed in the join block, then recomputed verbatim in the gating block two statements later (the `if self.dynamic && let (Some(now), Some(ast))` arm at 428-447 runs on the same `self.next` and yields the same `avail`). The join block's `joined = true` write is the only extra effect.
Why it matters: Duplication invites drift — if the safe-distance rule changes in one block (e.g., a different join margin), the two checks silently disagree and the state machine hangs between "joined" and "Wait". It is also wasted work per poll in a hot live loop.
What correct looks like: Compute `avail`/`ready_at` once per `next_fetch`, set `joined` from it, then gate on the same value.
Confidence: HIGH — Verification: read (both blocks side by side; no intervening mutation of `self.next`, `abr.current()`, or the rung).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `SAFE_BEHIND_LIVE_EDGE_S` (const) | 35-38 | REAL | — | 3.0 s; used at 315, 403, 422, 441. Unreachable in shipped binary (whole module). |
| `DashSchedError` (enum) | 40-54 | REAL | — | 4 typed variants; no data loss. |
| `Display for DashSchedError` | 56-73 | REAL | — | Stable `streaming.dash.*` strings; matches module docs. |
| `Error for DashSchedError` | 75 | REAL | — | Empty impl, house style. |
| `From<DashAbrError>` | 77-84 | REAL | — | `BadRepresentation→BadLadder` mapping is semantically lossy but documented at both ends. |
| `parse_iso8601_epoch_seconds` | 88-142 | REAL | S3 (F-002) | Char-boundary-safe (traced: 17..19 digits ⇒ 19 is a boundary); leap-second clamp; day-in-month NOT validated; Hinnant algorithm verified against the 2026 test vector. |
| `split_fraction_and_offset` | 144-180 | REAL | — | Fraction validated-then-dropped (documented); `+hh:mm`/`-hh:mm` bounds-checked; `tail[0..2]`/`tail[3..5]` panic-safety traced (multibyte straddles impossible given `bytes[2]==b':'` and len==5). No-offset ⇒ treated as UTC — recorded decision at 158-162. |
| `segment_availability_epoch` | 182-200 | REAL | — | Saturating adds; `as f64` lossless for epoch-scale i64; spec arithmetic (§5.3.9.5) as documented. |
| `live_edge_presentation_seconds` | 202-214 | REAL | — | Pure; negative (pre-AST clock) yields negative edge — caller's concern. |
| `ticks_to_seconds` | 216-221 | REAL | — | Timescale 0 → 1 documented as parser-default-only. |
| `FetchStep` (struct) | 225-237 | REAL | — | POD; `url: String` clone per step (fine at 1 fetch/segment). |
| `NextStep` (enum) | 239-253 | REAL | — | `Wait{available_in_s}` contract abused by F-001's `0.0`. |
| `DashScheduler` (struct) | 255-274 | REAL | — | All fields used. |
| `DashScheduler::new` | 276-318 | REAL | — | Resolves every rung (one broken rung refuses the ladder — honest); `period.start.unwrap_or(0.0)` documented default. |
| `abr_snapshot` | 320-324 | REAL | — | Delegates; `#[must_use]`. |
| `observe_throughput` | 327-329 | REAL | — | Delegates to abr EWMA. |
| `note_buffer_level` | 332-334 | REAL | — | Delegates; abr rejects non-finite/negative. |
| `on_buffer_signal` | 336-339 | REAL | — | `crate::buffer::BufferSignal` exists (lib.rs:29). |
| `DashScheduler::next_fetch` | 341-456 | REAL | S3 (F-001), S4 (F-004) | Core walk; `let _ = self.abr.decide(&ladder)` at 359 discards the switch event — benign (abr commits internally, verified abr.rs:337-345) but unsignaled; live-join loop + gating traced panic-free. |
| `segment_served` | 458-463 | REAL | — | `next += 1` unbounded-safe (usize). |
| `segments_served` | 465-469 | REAL | — | Returns `next` (join-skipped segments not counted — correct semantics). |
| tests: `parse`/`static_ladder_mpd`/`scheduler` (helpers) | 479-484, 549-573 | REAL | — | `unreachable!` on parse failure = test fails loudly, correct posture. |
| tests: 7 (see §5) | 488-704 | REAL | — | All green in baseline; honest assertions. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `iso8601_epoch_vectors` | YES | 11 concrete vectors incl. offsets, fraction-drop, leap-second clamp, refusals; mutating the offset sign or the era math flips at least one `assert_eq!`. |
| `availability_is_segment_end_on_the_wall_clock` | YES | Exact f64 equality on three availability shapes (PTO shift, timescale 90000). |
| `live_edge_computation` | YES | Four exact vectors incl. PTO and period-start signs. |
| `static_walk_serves_every_segment_in_order` | YES | Full 8-segment walk; asserts every URL and the count — a `segment_served` that forgets `next += 1` hangs the loop (test timeout = fail), a wrong start rung breaks the `hi-N.m4s` expectation. |
| `scripted_crash_steps_the_ladder_down` | YES | Drives EWMA crash → asserts down-step to rung 2 AND contiguous numbers 3..=8; breaking hysteresis/dwell breaks either assertion. |
| `dynamic_timeline_waits_then_joins` | YES | Three-phase timeline (Wait at 4, join at #3, Wait 1.0s for #4, Fetch at 11) — each phase pins the availability math to exact values. |
| `empty_ladder_refuses_typed` | YES | Pins the `BadLadder` typed refusal. |
Execution evidence: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5010-5016` — 7/7 `ok`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "segment scheduling and ISO 8601 availabilityStartTime live-edge join landed" | docs/feature-matrix.toml:1454 (network-dash notes) | REAL (library-level) | Code exists (this file), tests green (orchrun log :5010-5016). |
| network-dash status windows/linux/macos = "partial" | docs/feature-matrix.toml:1451 | **UNREACHABLE presented as partial** (F-003) | Zero production callers (rg, header); CLI URL lane is ranged-bytes only (demux_host.rs:490-493). |
| "The scheduler is a pure state machine: no I/O and no clock of its own" | module doc, sched.rs:11-13 | REAL | Read: no I/O, `now_epoch` supplied by caller. |
| "`None` means 'treat everything as available'" | sched.rs:343-344 | FALSE for dynamic manifests (F-001) | Code at 379-387 returns `Wait{0.0}` forever. |
| "mockhttp socket e2e lane is future work (recorded in the feature matrix, not claimed here)" | sched.rs:16-17 | REAL | matrix notes :1454 say "mockhttp socket e2e + DASH-IF vectors pending". |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped entry points (F-003) — it compiles into the `streaming` crate (and thus into the CLI binary via the ranged lane's dependency on the crate), but no code path constructs a `DashScheduler`.
- F-004 duplication: availability computed twice per `next_fetch` on the dynamic path.

## 8. Dependencies & flags
- `playlists::{AdaptationSet, Mpd}` (crate dep, workspace-internal); `super::abr`, `super::resolve` (same tree); `crate::buffer::BufferSignal`. No feature flags gate this module — it is unconditional code in the `streaming` crate (no `#[cfg]` outside tests), so the unreachability is wiring, not flags. No unsafe, no unwrap/expect in production code (workspace lints).

## 9. Open questions for the next agent
- `crates/streaming/src/dash/mod.rs` (W10-A08) re-exports this module — confirm no other crate consumes `streaming::dash` via a path my grep patterns missed (I searched `dash::`, `DashScheduler`, `DashSession`, `dash_session`; a `use streaming::dash` alias would still contain `dash::`).
- Whoever audits the CLI open lane: does `lumen open <url>.mpd` produce a typed refusal naming DASH, or a generic container-probe failure? (demux_host.rs:433 suggests a typed refusal for unimplemented lanes — check whether `.mpd` maps to one.)
