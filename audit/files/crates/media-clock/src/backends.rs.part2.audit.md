# AUDIT: crates/media-clock/src/backends.rs#part2 (lines 530-1058)
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 529 (chunk of 2118-line file) / 22 (EventRing 2, plan_fallback, ClockBackends 7, MediaClock 9, Source 1, switch_locked, 2 test helper types) |
| File verdict | UNREACHABLE (chunk: the card-3.B2 multi-backend API — real, heavily tested, zero shipped callers) |
| Reachable from shipped entry point? | NO for every public item in this chunk (proof below). The shipped hosts construct clocks via 3.B1 only: `apps/cli/src/pipeline_host.rs:1605 MediaClock::audio_master(...)`, `:1630 MediaClock::system_monotonic(...)`, `apps/web/src/host.rs:1737 MediaClock::audio_master(...)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | For-loop `rg -n "<symbol>" crates/ apps/ --type rust` (excluding media-clock) over all 12 public symbols (`on_master_lost`, `switch_master`, `observe_video`, `mark_ended`, `seek_back`, `is_ended`, `take_master_change`, `master_changes_lost`, `with_backends`, `ClockBackends`, `MasterLoss`, `VideoSteer`, `MasterChanged`, `MasterStream`) → **zero non-media-clock hits** (near-name matches `observe_video_end`, streaming test fns, testkit `is_ended` are unrelated); `rg -n "MediaClock::[a-z_]+" apps/ --type rust -o` → only `audio_master`, `system_monotonic`, `resume`; baseline: `sed -n 3204,3236p` + `rg backends::tests` → 30+ tests ok; doctest `backends::ClockBackends (line 582)` ok at orchrun :6429 |

## 1. One-paragraph verdict
This chunk is the entire card-3.B2 multi-backend clock surface — the fallback
planner, the `ClockBackends` builder, master-loss demotion, engine switches,
video steering, the Ended latch with `seek_back`, and the bounded event ring —
and every public item in it is unreachable from the shipped `lumen` binary:
all three hosts build 3.B1 single-master clocks (`audio_master` /
`system_monotonic`) and never call any 3.B2 method. The code itself is real
and unusually well-tested (30+ in-file tests plus the builder doctest, all ok
in the no-fail-fast baseline), and the internal logic I traced (ring
arithmetic, demotion order, switch seam, ended latch) is correct — but in the
product it is dead weight behind a doc (03 §4.7 d/f) that claims the player
"emits `ClockMasterChanged`" when it falls back to system time, while the
shipped engine has no such wire event at all (playback-engine event.rs:46-47
defers it to "Phase 3+").

## 2. What actually works (verified)
— All backends tests executed ok: baseline
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3207-3236`
(30 test results incl. `master_loss_chain_audio_video_free_runs_with_events`,
`ended_latch_freezes_and_seek_back_re_arms_paused_not_free_running`,
`fallback_plan_is_deterministic_over_the_full_matrix`).
— Doctest for this chunk's builder ran ok: `backends::ClockBackends (line
582) ... ok` (orchrun :6425-6429).
— EventRing FIFO arithmetic verified by hand: push wraps head, pop computes
`(head + CAP − len) % CAP` — correct oldest-first; overflow counts `lost`
(bounded, no growth).
— `plan_fallback` verified against `DEMOTION_ORDER` [Audio, External, Video,
Free] (backends.rs:212-217) and `WiredBackends::wired` (:449-459): skips lost
kinds, never invents a master.
— `switch_locked` traced: carried clamped position → source swap inside lock
→ re-anchor or `awaiting_anchor` → freeze carry → epoch bump → event push.
— Mutation check: `fallback_plan_is_deterministic_over_the_full_matrix`
exhausts the wired×lost×current grid, so reordering DEMOTION_ORDER fails it;
`switch_storm_without_drain_surfaces_the_lost_count` pins the ring's lost
receipt.

## 3. Findings
### F-001 [S3]
Where: crates/media-clock/src/backends.rs:736 (and 689, 769, 804, 857, 881, 915, 922, 929)
Code:
```rust
    /// Idempotent per stream: a second report of the same loss is a
    /// `Recorded` no-op.
    pub fn on_master_lost(&self, stream: MasterStream) -> MasterLoss {
```
What is wrong: The whole card-3.B2 public surface this chunk implements —
`with_backends`, `on_master_lost`, `switch_master`, `observe_video`,
`mark_ended`, `seek_back`, `is_ended`, `take_master_change`,
`master_changes_lost`, the `ClockBackends` builder — has ZERO callers outside
`crates/media-clock` (verified symbol-by-symbol with rg across `crates/` and
`apps/`). The shipped hosts pick a single master at open:
`pipeline_host.rs:1605` (audio) or `:1630` (wall); apps/web `host.rs:1737`
(audio). No shipped entry point can ever demote a master, steer video, latch
Ended via this API, or drain a master-change event.
Why it matters: ~530 lines of lock-protected state machinery (the event ring,
loss bitset, demotion planner, switch seam) ship dead in the product while
the project's docs sell master-fallback as designed behavior; maintenance
cost and audit surface with zero product reach.
What correct looks like: Either wire the engine to `with_backends` (the
7.C1/3.B2 plan) or gate the surface behind a not-yet-wired feature so the
shipped binary provably excludes it.
Confidence: HIGH — Verification: 12-symbol rg sweep (commands row) + the
three host constructor sites read.

### F-002 [S3]
Where: crates/media-clock/src/backends.rs:721-735 (doc claim) vs crates/playback-engine/src/event.rs:44-48
Code:
```rust
    /// - nothing left wired → [`MasterLoss::NoFallback`]: the clock
    ///   stays on the dead master and its position holds — frozen
    ///   truth, never a fabricated seamless clock.
    ///
    /// Idempotent per stream: a second report of the same loss is a
    ///   `Recorded` no-op.
```
(contradicted in product by:)
```rust
//! Doc 03 §6 step 1's engine-side `SeekAccepted` has no wire event
//! and no variant here: the class-A `Accepted` receipt IS the accept
//! evidence (doc 05 §1.4 C3 — completion arrives as the terminal
//! events above). Device events and `ClockMasterChanged` remain
//! Phase 3+ (`2.C3`/`7.C1`, `3.B2`). Event *completeness* — every
```
What is wrong: Doc 03 §4.7(d) says a missing audio track means "master is
`SystemMonotonic`, and the UI is told (truthful state, not a silent
approximation)" and §4.7(f) promises "audio-less media keeps playing on
system time and emits `ClockMasterChanged`". In the shipped product the
first half is real (host picks `wall_clock()` at open,
pipeline_host.rs:1626-1637) but the second half is PAPER-ONLY: the engine
has no `ClockMasterChanged` event (event.rs:46-47 defers it to Phase 3+),
and the API that would produce the records (`take_master_change`) is
unwired (F-001). The UI is NOT told; the fallback is exactly the "silent
approximation" the doc forbids.
Why it matters: A doc-guaranteed observability behavior does not exist in
the shipped binary; users playing video-only media get no master-change
signal anywhere.
What correct looks like: Engine forwards `take_master_change` drains as a
wire event at open-time master selection, or the doc marks the emission
Phase-3.
Confidence: HIGH — Verification: event.rs read; rg for ClockMasterChanged
outside media-clock → only the event.rs comment.

### F-003 [S4]
Where: crates/media-clock/src/backends.rs:979-984
Code:
```rust
    let Some(next_source) = core.backend.wired.source_for(to) else {
        // Unreachable by construction: every caller validated the
        // target against the same wired set (plan_fallback /
        // switch_master). Defensive: refuse rather than invent.
        return (from, from);
    };
```
What is wrong: The defensive arm returns `(from, from)`, which
`on_master_lost` would surface as `MasterLoss::Demoted { from, to: from }` —
a demotion-to-itself lie — and `switch_master` as
`MasterSwitch::Switched { from, to: from }`. Both callers validate against
the same `wired` set under the same lock, so it is genuinely unreachable;
but if it ever fired, the lie propagates to callers with no counter or
diagnostic.
Why it matters: A "refuse" that reports success-shaped data contradicts the
crate's typed-truth discipline.
What correct looks like: Return an Option/Result the callers turn into a
typed refusal, or at least a `Recorded`-style no-op label.
Confidence: HIGH — Verification: read of both call paths (746-751, 787-788).

### F-004 [S4]
Where: crates/media-clock/src/backends.rs:934-956 (vs clock.rs:259-277)
Code:
```rust
impl Source {
    /// The planner vocabulary for this source (card 3.B2).
    pub(crate) fn master_kind(&self) -> MasterKind {
        match self {
            Source::Samples(_) => MasterKind::Audio,
            Source::Monotonic {
                kind: MonotonicKind::System,
                ..
            } => MasterKind::System,
```
What is wrong: `Source::master_kind` duplicates `Source::master`
(clock.rs:259-277) — two parallel five-arm matches over the same enum that
must be kept in sync by hand (one maps to `MasterKind`, the other to
`ClockMaster`).
Why it matters: Adding a `MonotonicKind` variant silently desynchronizes the
planner vocabulary from the public label vocabulary.
What correct looks like: One canonical mapping plus a `From`/conversion
between the vocabularies.
Confidence: HIGH — Verification: both matches read.

### F-005 [S4]
Where: crates/media-clock/src/backends.rs:734-735 (doc) with 746-755
Code:
```rust
    /// Idempotent per stream: a second report of the same loss is a
    /// `Recorded` no-op.
```
What is wrong: In the `NoFallback` corner the claim is false: when the
current master is the lost one and nothing remains wired, a second report of
the same stream re-enters `plan_fallback`, gets `None` again, and returns
`MasterLoss::NoFallback` — not `Recorded`. The state effect is idempotent,
the return value is not. The test
`repeated_loss_reports_are_idempotent` (backends.rs tests) only covers the
fallback-exists case (asserts `Recorded { VideoOutput }`); the NoFallback
re-report corner is untested.
Why it matters: Doc-comment overpromise plus a test that would not catch it.
What correct looks like: Doc says "a second report is a no-op (Recorded when
a fallback exists, NoFallback again otherwise)" + a corner test.
Confidence: HIGH — Verification: read of 736-756 and the test body.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `EventRing::push` | 532-540 | REAL (unreachable) | — | Bounded; drops NEWEST on full, counts `lost` (receipt) — documented D-2 policy |
| `EventRing::pop` | 543-551 | REAL (unreachable) | — | FIFO index math verified correct; alloc-free |
| `plan_fallback` | 559-572 | REAL (unreachable) | — | Pure; skips lost kinds along DEMOTION_ORDER; never invents; exhaustively tested (full-matrix test) |
| `ClockBackends::new` | 623-632 | REAL (unreachable) | — | Wall required; defaults snap 40 ms (:100), InitialMaster::Auto |
| `ClockBackends::audio` / `external` | 636-646 | REAL (unreachable) | — | Builder setters |
| `ClockBackends::video` / `video_snap_ns` | 653-662 | REAL (unreachable) | — | `video_snap_ns` accepts any u64 unvalidated (0 ⇒ every off-bound frame jumps with epoch bump — caller's choice, no typed guard) |
| `ClockBackends::initial` | 665-668 | REAL (unreachable) | — | Auto chain [Audio, Video, System] skips External by design (documented 679-681) |
| `MediaClock::with_backends` | 689-719 | REAL (unreachable) | — | Auto/initial resolution, typed `backend-not-wired` refusal; builds via `BackendCore::multi` |
| `MediaClock::on_master_lost` | 736-756 | REAL (unreachable) | S3 (F-001, F-002 context) | Records loss either way; demotes only when current stream lost; NoFallback holds position (frozen truth) |
| `MediaClock::switch_master` | 769-789 | REAL (unreachable) | — | NotWired/Lost typed refusals; Already idempotent; System/Free not choosable (documented) |
| `MediaClock::observe_video` | 804-843 | REAL (unreachable) | — | NotMaster/Paused/Ended early-outs; anchor adopted before bound check (snap invisible via monotone clamp; jump = epoch bump + last_ns=pts); i128 error math |
| `MediaClock::mark_ended` | 857-867 | REAL (unreachable) | — | Freezes at current reading; ended subsumes pause; idempotent (tested) |
| `MediaClock::seek_back` | 881-912 | REAL (unreachable) | — | NotEnded typed refusal with current position; re-arms paused-at-target; awaiting_anchor on unreadable source; epoch bump; overflow keeps latch (tested) |
| `MediaClock::is_ended` | 915-917 | REAL (unreachable) | — | Trivial |
| `MediaClock::take_master_change` | 922-924 | REAL (unreachable) | S3 (F-001) | Would feed doc 03 §4.6 `ClockMasterChanged` — no consumer exists |
| `MediaClock::master_changes_lost` | 929-931 | REAL (unreachable) | — | Honest receipt |
| `Source::master_kind` | 936-956 | REAL (unreachable) | S4 (F-004) | Duplicate of `Source::master` |
| `switch_locked` | 967-1012 | REAL (unreachable) | S4 (F-003) | One-lock seam traced correct; defensive arm misleading on return |
| test helpers `FakeCounter` | 1024-1049 | REAL | — | Deterministic; `sample_rate_hz` truncating try_from(0) fallback — test-only |
| test helpers `FakeMonotonic` | 1052-1068+ | REAL | — | Deterministic wall |

## 5. Test honesty (if tests exist in/for this file)
Test fns live past line 1081 (part3's range); this chunk holds the fakes and
the builder doctest. From the baseline all 30+ ran ok (:3207-3236). Spot
checks:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| repeated_loss_reports_are_idempotent | YES for its case | Exact `Recorded{VideoOutput}` + master equality; but does NOT cover the NoFallback re-report corner (F-005) |
| single_master_clock_reports_no_fallback_and_holds | YES | Held position `1_000_000_000` ticks exact |
| fallback_plan_is_deterministic_over_the_full_matrix | YES | Grid exhaustion — any order/bit bug flips |
| switch_storm_without_drain_surfaces_the_lost_count | YES | Lost-count receipt pinned numerically |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Missing audio track → master is SystemMonotonic, and the UI is told" | doc 03 §4.7(d) | HALF-REAL: fallback real, UI-telling PAPER-ONLY | wall_clock at open (pipeline_host.rs:1626-1637) real; no ClockMasterChanged event exists (event.rs:46-48) — F-002 |
| "audio-less media keeps playing on system time and emits ClockMasterChanged" | doc 03 §4.7(f) | HALF-FALSE (emission half) | Same proof; baseline AVI-with-audio and mp4-aac lanes never surface any master event |
| "the clock... reports master changes (ClockMasterChanged) as events" | doc 03 §4.7(d) | PAPER-ONLY in product | take_master_change has zero callers (F-001) |
| Demotion chain audio → external → video → free wall, position carried, epoch bumped | doc 03 card 3.B2 / backends.rs:724-727 | REAL (library), UNREACHABLE (product) | 30+ passing tests; zero shipped callers |

## 7. Dead / duplicated / unreachable code
- The entire chunk's public surface: unreachable from shipped entry points
  (F-001) — the headline finding.
- `Source::master_kind` duplicates `Source::master` (F-004).

## 8. Dependencies & flags
No new deps in this chunk (`std` sync/atomics only). No feature gates, no
unsafe. Lock discipline: every public method takes `lock_core()` once and
mutates under it — single-lock seams verified by reading (`switch_locked`
documents the atomicity argument at 959-966).

## 9. Open questions for the next agent
- part3/part4 auditors: do the in-file tests (1059+) or any later section
  wire 3.B2 into a host-visible path? (My sweep says no, but the test module
  is large.)
- Wave agent for apps/desktop: does the tauri shell construct its own clock
  (mailbox/frame_tap suggest it reuses the CLI pipeline host's)? If it, too,
  is 3.B1-only, F-001 stands for all three shipped binaries.
