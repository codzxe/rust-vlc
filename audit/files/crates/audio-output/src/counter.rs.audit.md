# AUDIT: crates/audio-output/src/counter.rs
| Field | Value |
|---|---|
| Auditor | W08-A09 |
| Date | 2026-11-21 |
| Lines / functions | 133 / 7 (+3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI open/play → apps/cli/src/host.rs:926 `open_audio_session` → apps/cli/src/pipeline_host.rs:1598-1599 `NullBackend::open_stream` → null.rs:86 `SampleCounter::new`; advanced at runtime by host.rs:614/726/824/1274 `device.render` → null.rs:301 `render_into` → render.rs:108 `counter.advance`; consumed by pipeline_host.rs:1605-1606 `MediaClock::audio_master(sink.clock_source())` → null.rs:123-125) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "SampleCounter" --glob *.rs` (all constructors/callers: production entry only null.rs:86); `rg -n "with_base"` (callers: counter.rs tests + pump.rs:301 test only); `rg -n "PcmPump|render\(" apps/ crates/` (who drives the device); `sed -n` on counter.rs:42-49,65-70, null.rs:74-129,255-339, render.rs:89-120, pipeline_host.rs:540-660,1560-1635, host.rs:590-634,1815-1844, media-clock/src/source.rs (trait contract); `grep -n "counter::tests" audit/runs/baseline/test-workspace.log.txt` (lines 158-160: all 3 tests ran, ok) |

## 1. One-paragraph verdict
This file is one of the few genuinely real things in the audio-output domain: a 2-field struct (AtomicU64 count + u32 rate) with zero stubs, zero panics, zero allocation, wired into the shipped `lumen` CLI as the audio-master clock's truth source and advanced once per device render. Its unit tests ran and passed in the baseline workspace run. What is NOT real is the story around two of its features: the device-switch offset carry (`with_base`) has no production caller — no device switch can occur in a build whose only backend is the null device — and the "Saturating" doc on `advance` describes a wrap, not a saturation. The counter counts real renders of a **simulated** device: it is the honest clock of a sink that produces no sound.

## 2. What actually works (verified)
— verified by `rg -n "SampleCounter" --glob *.rs` → production construction only at null.rs:86 inside `NullBackend::open_stream`, which pipeline_host.rs:1599 (shipped CLI audio session, called from host.rs:926) invokes; render.rs:108 advances it inside `render_into`, which host.rs:614/726/824/1274 drive during playback.
— verified by `audit/runs/baseline/test-workspace.log.txt:158-160` → `counter::tests::counter_is_monotonic_and_rate_carrying ... ok`, `counter::tests::offset_carry_never_rewinds_across_devices ... ok`, `counter::tests::zero_rate_is_typed ... ok`.
— verified by `audit/runs/baseline/INDEX.md` (avi-firstvertical-open-session.txt: full Idle→Opening→Buffering→Playing→Stopped→Idle lifecycle; sample-aac.mp4 "Buffering → Playing → Stopped (audible output: none — null device)") → the counter-driven clock paces a real playback session in the shipped binary; the audio goes nowhere.

## 3. Findings
### F-001 [S4]
Where: crates/audio-output/src/counter.rs:65-70
Code:
```
    /// Advance by the instants the device just consumed (the render
    /// callback's one counter operation). Saturating — u64 instants
    /// at any rate outlive every playback session ever started.
    pub fn advance(&self, instants: u64) {
        let _ = self.count.fetch_add(instants, Ordering::Relaxed);
    }
```
What is wrong: the doc says "Saturating"; `fetch_add` **wraps** on u64 overflow. `AtomicU64::fetch_add` has no saturating mode. The mitigating clause (u64 outlives every session) is true — u64::MAX at 192 000 Hz ≈ 3 million years — but the stated mechanism is false, and a wrap would be a rewind: the one violation of the module's headline "never rewinds" invariant that the code has no defense against.
Why it matters: the module's entire value proposition is a monotonic truth source the media clock trusts (media-clock/src/source.rs:21-28 documents regression handling as a discontinuity); a doc that misstates the overflow behavior of the only mutating operation invites someone to copy the pattern where it matters. Also `saturating_add` exists and is used one file over (render.rs / host.rs:594-596 use `saturating_add`), so the codebase knows the difference.
What correct looks like: either fix the doc ("Wrapping in theory, unreachable in practice — u64 at any rate exceeds 10^6 years") or make it actually saturate (`fetch_update` with `saturating_add`).
Confidence: HIGH — Verification: read (std semantics of `fetch_add`; no run needed).

### F-002 [S3]
Where: crates/audio-output/src/counter.rs:17-19,52-63
Code:
```
//! counter **never rewinds** (doc 03 §4.9 e): [`SampleCounter::with_base`]
//! carries the previous device's count as the new counter's origin —
//! the offset-carry invariant, structural and tested.
```
```
    pub fn with_base(base: u64, rate_hz: u32) -> Result<SampleCounter, EngineError> {
        let counter = SampleCounter::new(rate_hz)?;
        counter.count.store(base, Ordering::Relaxed);
        Ok(counter)
    }
```
What is wrong: `with_base` has zero production callers. `rg -n "with_base" --glob *.rs` → definition, its own unit tests (counter.rs:103,124), and one test in pump.rs:301. No device-switch path exists anywhere: the shipped backend set is `NullBackend` + `UnsupportedBackend` (lib.rs:152-157 `builtin_backends`, itself caller-free), the CLI opens exactly one null stream per session (pipeline_host.rs:1598-1599), and cpal is gated off (`cpal.rs` `AUDIO_ADR005_ADMITTED = false` with a compile-time assert). The module doc's headline invariant — "across device switches the counter never rewinds… structural and tested" — describes an operation that cannot occur in any shipped binary.
Why it matters: this is seam theater: a tested invariant for a hypothetical future (real OS backends, ADR-005) presented in present-tense module docs as if it were live behavior. A reader auditing device-loss recovery would waste time proving a carry that never happens; the actual shipped behavior on any re-open is a fresh zero-based counter (a rewind the media clock must paper over as a Discontinuity, source.rs:23-28).
What correct looks like: keep `with_base` (it is correct code for the future seam) but mark the doc claim as future/`[PROPOSAL]` like the crate's other not-yet-live claims, and state the v0 reality: re-open starts at zero.
Confidence: HIGH — Verification: read + rg caller sweep (no run; reachability from shipped entry points established by grep of apps/ and crates/).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `SampleCounter::new` | 42-50 | REAL | — | Zero-rate typed refusal (`audio.sample_rate_invalid`) then constructs. No panic. Reachable: null.rs:86 (shipped). NOTE: the zero-rate branch is shadowed in the only production call — null.rs:80 rejects `format.rate == 0` earlier with `format_unsupported` — so the code is reachable but that error arm is not (see §7). |
| `SampleCounter::with_base` | 59-63 | UNREACHABLE (production) | S3 (F-002) | Correct (delegates to `new`, stores base; `Relaxed` store fine pre-share). Callers: tests only. Device-switch story has no production path. |
| `SampleCounter::advance` | 68-70 | REAL | S4 (F-001) | One `fetch_add`, Relaxed — acceptable: no cross-value publication, RMW total order preserves monotonic reads. Doc calls wrap "Saturating". Executed every render in shipped playback (render.rs:108). |
| `SampleCounter::count` | 74-76 | REAL | — | Relaxed load: data-race-free, may be stale, never regresses for a single observer. Read by `MediaClock` under its short lock (source.rs:12-17 contract honored: no I/O, one atomic). |
| `SampleCounter::rate_hz` | 79-81 | REAL | — | Trivial accessor, correct. |
| `impl SampleCounterSource::sample_count` | 85-87 | REAL | — | Delegates to `count`. Trait contract (source.rs:35-39) satisfied. |
| `impl SampleCounterSource::sample_rate_hz` | 89-91 | REAL | — | Delegates to `rate_hz`. `>0` guaranteed by `new`. |
| test `zero_rate_is_typed` | 100-105 | REAL | — | Mutation-sensitive: if the `rate_hz == 0` check were deleted, `.err()` is `None` → `Must::must` panics (testkit/src/lib.rs:65-73). Both `new` and `with_base` covered. |
| test `counter_is_monotonic_and_rate_carrying` | 108-118 | REAL | — | Mutation-sensitive: no-op `advance` ⇒ `count()` stays 0 ⇒ `assert_eq!(…, 2940)` fails. Also pins the trait-object view. |
| test `offset_carry_never_rewinds_across_devices` | 121-131 | REAL | — | Mutation-sensitive: `with_base` not storing base ⇒ `new.count()` = 0 < `old.count()` fails. Tests the invariant; the invariant's production trigger does not exist (F-002). |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `zero_rate_is_typed` | YES | `.err().must()` panics on `Ok` (testkit/src/lib.rs:69-73 `unreachable!`); asserts the exact stable code. |
| `counter_is_monotonic_and_rate_carrying` | YES | Direct arithmetic assertions on `count()`/`advance`; a broken add or wrong-rate accessor fails the `assert_eq`s. |
| `offset_carry_never_rewinds_across_devices` | YES | Base-store removal ⇒ both asserts fail (0 ≥ 2 880 000 false; 1 > 2 880 000 false). |

All three ran and passed in baseline: `audit/runs/baseline/test-workspace.log.txt:158-160`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the device sample counter is the only trusted progress source" — a real counter the clock reads | doc 03 §4.7 e, quoted in counter.rs:2-4 and source.rs:32-34 | REAL (as built) | pipeline_host.rs:1605-1606 wires `sink.clock_source()` (null.rs:123-125 → this struct) into `MediaClock::audio_master`; baseline avi-firstvertical session reaches Playing. Caveat: the "device" is the null device — the counter counts simulated renders. |
| Pause freezes the counter "by construction, not by a special case" | counter.rs:12-15 (doc 03 §4.7 e) | REAL | null.rs:296-300: paused renders return early, so `render_into`/`advance` never run. The counter itself has no pause branch — accurate. |
| Counter "never rewinds… across device switches… structural and tested" | counter.rs:17-19 | SHALLOW | Tested (unit test passes), but no production device switch exists; `with_base` caller-free outside tests (F-002). |
| engine-info feature `audio.null-device` | baseline INDEX.md:45-46 (`lumen-engine-info.txt`) | REAL | The null device is the shipped audio output; this counter is its clock source. |

## 7. Dead / duplicated / unreachable code
- `with_base` — production-dead (tests only): see F-002.
- The zero-rate error arm (`counter.rs:43-45`, code `audio.sample_rate_invalid`) is unreachable through the only production constructor call: null.rs:77-82 rejects `format.rate == 0` first with `audio.format_unsupported` (Device kind), so a zero rate can never reach `SampleCounter::new` from the shipped path. The two layers also disagree on taxonomy for the same condition (Unsupported/`audio.sample_rate_invalid` vs Device/`audio.format_unsupported`). Defense-in-depth, not a bug — but the documented `# Errors` contract of `new` is dead in production.
- No duplication: this is the only `SampleCounter` implementation in the workspace (`media_clock`, `subtitles`, `video-engine` tests use their own local fakes, correctly not reusing this).

## 8. Dependencies & flags
- `media-clock` (trait `SampleCounterSource` — the seam this implements), `media-types` (`EngineError`), `crate::error` (one call, line 44). `testkit` dev-only. Matches Cargo.toml deps exactly; no feature flags gate this file; no `unsafe`; no external crates (C5 honored). Crate compiled in baseline: `test-workspace.log.txt:27 "Compiling audio-output v0.1.0"`.

## 9. Open questions for the next agent
- apps/cli/src/host.rs:613 allocates `vec![0.0_f32; RENDER_QUANTUM * channels]` **inside the render while-loop** of the shipped pump (`advance_time_wall`) — a per-quantum heap allocation on the audio render path, against the crate's own D-1/D-8 zero-alloc discipline (see pipeline_host.rs:611). Auditing apps/cli/src/host.rs should decide whether this violates the zero-alloc ACs asserted in tests/zero_alloc_null_sink.rs.
- playback-engine device-loss recovery (tests/scenarios_b/scenario_12_device_loss.rs): confirm re-open builds a fresh zero-based counter (no `with_base`), i.e. a rewind the clock must report as Discontinuity — the doc-03 §4.9 e "never rewinds" story has no production carrier today.
- `builtin_backends()` (lib.rs:152) and `UnsupportedBackend` have zero callers — the entire `AudioBackend` trait-object layer may be production-dead; the device.rs/stub.rs auditor should settle it.
