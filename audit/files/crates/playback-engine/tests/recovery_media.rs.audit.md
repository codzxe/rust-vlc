# AUDIT: crates/playback-engine/tests/recovery_media.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 689 / 25 (7 #[test]s, 18 fakes/builders/helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test crate) — runs under `cargo test -p playback-engine`; I ran the prebuilt binary (see Commands). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/recovery_media-2d1b70ceb0ed612e` → 7 passed, 0 failed, finished in 0.01s, exit 0. |

## 1. One-paragraph verdict
Genuine regression suite with teeth: every test pins exact observable counts (delivered frames, damage totals, budget figures) against the REAL engine, and every pin is the kind that breaks the moment the behavior regresses. The G3 lane proves audio per-frame damage is skip-and-count (38 delivered of 40, damage==2, never silence), that runs reset on success (5 total across two runs), that budget exhaustion is the typed `decode.concealment.budget` terminal with `attempted: 1001` and the last corrupt error on the source chain, and that non-per-frame faults stay fatal. The G1 lane pins the `NoDecodableTrack` enrichment (container + every codec id, "(none reported)" instead of fabrication, un-enriched back-compat). All 7 executed by me and pass. The only defect is housekeeping: the file duplicates the shared fakes that five sibling suites already import from `tests/common`, justified by a header comment that claims the hoist is still "planned" when it has already landed.

## 2. What actually works (verified)
- `./target/debug/deps/recovery_media-2d1b70ceb0ed612e` → `test result: ok. 7 passed; 0 failed` — all four G3 tests and all three G1 tests.
- Cross-checked the exhaustion pin against the production constant: the test asserts `budget: u64::from(playback_engine::VIDEO_CONCEALMENT_BUDGET)` (1000) and `attempted: 1001` (recovery_media.rs:636-643), which matches pipeline.rs:2381 `if limb.conceal_failures > VIDEO_CONCEALMENT_BUDGET` — the terminal fires on the 1001st consecutive failure. The test and the code agree on the off-by-one.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/recovery_media.rs:19-21 (vs tests/session_flow.rs:53, tests/model_walk.rs:54, tests/reopen_alloc.rs:19, tests/recovery_env.rs:112, tests/hotplug.rs:46)
Code:
```
//! The fakes are deliberately local (the session_flow.rs fakes are
//! that suite's; 3.C1's planned hoist to `tests/common` will unify
//! them — this file keeps its own so the two waves cannot collide).
```
What is wrong: the comment is false about the current repo — the hoist it calls "planned" already landed; five sibling suites declare `mod common` and consume the unified fakes. This file re-declares `Collector`, `FakeDemuxer`, `shallow_clone`, `base`, `at_micros`, `packet`, `track`, `audio_frame`, `tiny_queues`, `tiny_budget`, `virtual_clock`, `frozen_throttle`, `poll_until`, `take_events` (~250 lines, 225-400) as private duplicates — and `FaultDecoder`/`FaultProvider` are local extensions that could compose on the shared base. The stated rationale ("so the two waves cannot collide") is void: the collision risk it guards against was resolved by the hoist. Why it matters: the next fake fix (e.g. a poison-recovery or stale-guard change) must now be applied twice, and a reader is told the duplication is load-bearing when it is not. What correct looks like: `mod common;` + a local `FaultDecoder`/`FaultProvider` (the genuinely fault-armed part) on top, comment deleted. Confidence: HIGH — Verification: read + rg (`mod common` in 5 sibling files). The duplication itself verified benign today (fakes are behaviorally identical to common/mod.rs's).

F-002 [S4]
Where: crates/playback-engine/tests/recovery_media.rs:111-118, 132-141
Code:
```
struct FaultDecoder {
    generation: GenerationId,
    /// Decode failures for these packet ordinals (0-based, in decode
    /// order for this limb).
    fail_indices: Vec<u64>,
    /// Fail EVERY packet (the budget-exhaustion shape).
    fail_all: Option<EngineError>,
    seen: u64,
}
```
What is wrong (minor, documented-by-design but worth recording): `fail_indices` are ordinals of decode CALLS (`self.seen` counts every `decode`, including stale-packet refusals), not packet pts or script indices — the two `audio_per_frame_damage` tests pass only because no stale packets occur in these fixtures. A future fixture that mixes seeks with armed faults would see its fault placement shift silently. Naming the field `fail_decode_ordinals` (or keying on pts) would remove the trap. Confidence: HIGH — Verification: read (decode-order counting at 132-133 verified).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SharedNow`/`MonotonicSource::now_ns` | 52-58 | REAL | — | Virtual time, no sleeps. Duplicate of common (F-001). |
| `Collector`/`FrameConsumer::accept` | 63-76 | REAL | — | Poison-recovery lock; the G3 "never fabricated" proof surface. Duplicate. |
| `FakeDemuxer` (`start`/`next_event`/`seek`) | 80-104 | REAL | — | Fixed script → EndOfStream; unseekable (consistent with suite scope — no seek tests here). Duplicate. |
| `FaultDecoder` (`configure`/`decode`/`flush`) | 111-152 | REAL | S4 | The fault seam: per-ordinal or fail-all errors, fresh error per call, stale-packet guard, one frame per good packet. F-002 ordinal semantics. |
| `fault_error` | 157-168 | REAL | — | `Corrupt { Truncated }` — a per-frame kind for both lanes (matches pipeline.rs `per_frame_damage` arms). |
| `shallow_clone` | 172-180 | REAL | — | Duplicate of common. |
| `non_per_frame_error` | 184-190 | REAL | — | The control fault (`Unsupported`) — proves the skip lane is class-scoped. |
| `FaultProvider::decoder_for` | 198-211 | REAL | — | Arms each decoder with the test's fault plan. |
| Builders (`base`,`at_micros`,`packet`,`track`,`audio_frame`,`tiny_queues`,`tiny_budget`,`virtual_clock`,`frozen_throttle`) | 217-306 | REAL | — | Byte-identical in substance to tests/common (F-001). |
| `open_with` / `open_faulty_audio` | 315-357 | REAL | — | Real `Engine::open` with container declaration, tiny queues, frozen throttle. |
| `poll_until` / `take_events` / `first_fatal` | 362-400 | REAL | — | Bounded hang-proof driver; event filter; fatal finder. |
| `flushed_decode_damage` | 538-545 | REAL | — | Finds the coalesced `RecoveredDamage{Decode}` count. |
| `no_decodable_track_names_container_and_codecs` | 413-460 | REAL | — | Executed PASS: container in kind + every codec in source leaf + Display. |
| `no_decodable_track_with_empty_track_list_names_container_only` | 467-497 | REAL | — | Executed PASS: "(none reported)", no invented fourccs. |
| `no_decodable_track_without_container_stays_unenriched` | 502-528 | REAL | — | Executed PASS: `format: None` back-compat, codec list still present. |
| `audio_per_frame_damage_is_skipped_and_reported` | 554-592 | REAL | — | Executed PASS: no fatal, damage==2, 38 delivered frames each at the next good pts (per-frame pts assert at 581-591 — the "gap is honest" proof). |
| `audio_damage_runs_reset_on_success` | 599-620 | REAL | — | Executed PASS: two runs (3+2), damage==5, 35 delivered. |
| `audio_damage_budget_exhaustion_is_a_typed_terminal` | 628-659 | REAL | — | Executed PASS: `decode.concealment.budget`, `ResourceLimit{RecoveryAttempts, 1000, 1001}`, source carries "truncated", flush==1000. |
| `audio_non_per_frame_damage_stays_fatal` | 666-689 | REAL | — | Executed PASS: fatal `decode.codec.unsupported`, no damage accounting, zero frames. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| no_decodable_track_names_container_and_codecs | YES | Asserts the exact `ErrorKind::Unsupported{format}` payload, the context code, and the source-leaf text — any regression to the un-enriched shape fails. |
| no_decodable_track_with_empty_track_list_names_container_only | YES | "(none reported)" is asserted verbatim. |
| no_decodable_track_without_container_stays_unenriched | YES | Pins `format: None` + source presence — over-enrichment (fabricating a container) fails. |
| audio_per_frame_damage_is_skipped_and_reported | YES | If the skip lane regressed to fatal, `first_fatal().is_none()` fails; if it synthesized silence, `delivered.len()==38` fails; if pts were invented, the per-frame pts loop fails. Triple-locked. |
| audio_damage_runs_reset_on_success | YES | A cumulative (not consecutive) budget would make 5 < 1000 pass but a wrong reset would change damage/delivered counts — both asserted exactly. |
| audio_damage_budget_exhaustion_is_a_typed_terminal | YES | Asserts the full typed shape incl. `attempted: 1001` — an off-by-one in either direction fails. |
| audio_non_per_frame_damage_stays_fatal | YES | If the skip lane swallowed everything, the session would `Ended` and the Error poll_until would trip its bound assert. |
No `#[ignore]`, no tautologies, no mock-of-the-unit (fakes sit at plugin seams; engine/session/pipeline are production).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one named test per defect in the worklog's W2-19 §C2.0 gap list" (G1, G3) | recovery_media.rs:1-17 | REAL | 3 G1 tests + 4 G3 tests present; all executed. |
| "audio ... drop the packet, count it, NEVER synthesize silence" | 13-17 | REAL | The 38-of-40 + per-frame-pts asserts; also matches production pipeline.rs:2414-2422 comments/behavior. |
| "3.C1's planned hoist to `tests/common` will unify them" | 19-21 | FALSE | The hoist already happened — 5 sibling files use `mod common` (F-001). The claim describes a past plan as future work. |
| "PCM cannot produce per-frame damage from real bytes; any bytes are valid PCM" (justifying scripted faults) | 106-110 | REAL | Consistent with the production docs at pipeline.rs:664-667 ("PCM audio accepts any bytes, so the audio lane is exercised by scripted decoder faults, never by real bytes"). |

## 7. Dead / duplicated / unreachable code
- The entire local fake/builder block (52-306) duplicates `tests/common/mod.rs` — see F-001. `FakeDemuxer::seek`'s unseekable arm is never exercised in this suite (no seek tests) — harmless dead arm, present in the shared fake too.
- No `#[ignore]`, no `todo!`/`unimplemented!`.

## 8. Dependencies & flags
- Test-only: `playback_engine`, `plugin_api`, `media_types`, `media_clock`, `packet_queue`, `codec_api`, `testkit` — all workspace crates, no external deps, no feature gates. Runs in the default test lane (baseline + my run).

## 9. Open questions for the next agent
- None blocking. The G2 defect (named in the header's gap-list context as part of W2-19 §C2.0) is NOT covered in this file — confirm which suite owns it (recovery_env.rs presumably, which uses `mod common`).
