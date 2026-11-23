# AUDIT: crates/platform-windows/tests/seam_behaviour.rs
| Field | Value |
|---|---|
| Auditor | W08-A08 |
| Date | 2026-11-23 |
| Lines / functions | 448 / 16 tests |
| File verdict | REAL-UNPROVEN (fallback path only; the real Windows FFI has zero runtime tests) |
| Reachable from shipped entry point? | NO — no crate or app depends on platform-windows (rg over crates/, apps/ = only self-references and comments); crate compiles only as a workspace member |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | sed -n (quotes); python3 FNV-1a-64 recompute (matches pinned constants); rg dependents; baseline citation audit/runs/baseline/cargo-test-workspace.txt:4054-4082 |

## 1. One-paragraph verdict
Sixteen honest, well-built tests of the *non-Windows* fallback and shared
vocabulary of platform-windows. The header admits it: on this Linux host every
seam is the typed fallback, so the suite proves the crate refuses honestly when
built for the wrong OS — it proves nothing about WASAPI, DXGI, or the COM
dialogs. The real Windows code (src/windows/, cfg(target_os="windows")) has
zero runtime coverage anywhere; it is compile-checked once, manually, per
worklog.md. The tests themselves are genuine (assert_err panics on Ok; pinned
FNV-1a constants verified by independent recompute), with two tautological
assertions inside otherwise-real tests. The crate the suite guards is not
referenced by any shipped binary, so all of this diligence currently guards
unreachable code.

## 2. What actually works (verified)
- Baseline run: audit/runs/baseline/cargo-test-workspace.txt:4070 "test result:
  ok. 16 passed; 0 failed" for tests/seam_behaviour.rs.
- Pinned FNV-1a-64 constants re-derived independently: `python3` FNV-1a-64("a")
  = 0xaf63dc4c8601ec8c and ("{0.0.0.00000000}.{e32f9c65-1234-4a55-bc86-8ed0d9d1c021}")
  = 0x5b40ed9bb4ce1081 — both match the test's asserted constants (lines 301, 305).
- testkit::assert_err (crates/testkit/src/lib.rs:100-105) panics on Ok — the
  "must fail honestly" assertions are real.

## 3. Findings
F-001 [S2]
Where: crates/platform-windows/tests/seam_behaviour.rs:1-13 (suite scope) vs crates/platform-windows/src/windows/*
Code:
```
//! This suite runs on a Linux host, which is exactly the "wrong-OS
//! build" case the crate's honesty contract covers: every seam must
//! be the typed fallback and [`WindowsPlatform::discover`] must say
//! `WrongOs` for every feature — the compile-time cfg story
//! verified at runtime from the outside. Real Windows behavior is
//! `[VERIFY]` (no Windows SDK in this sandbox; the FFI surface is
//! compile-verified via `cargo check --target
//! x86_64-pc-windows-msvc` only).
```
What is wrong: The only executable test suite for platform-windows never
executes a single line of the real Windows integration (src/windows/wasapi.rs,
dxgi.rs, dialogs.rs, com.rs — the actual product of card 7.A1). Runtime
coverage of the crate's raison d'être is 0%; the cross-compile typecheck is a
one-time manual act recorded in worklog.md, and no workflow in
.github/workflows/ runs `cargo check --target x86_64-pc-windows-msvc`
(audit.yml:24 only mentions platform-windows *clippy* as a future catch).
Why it matters: hand-rolled COM FFI is precisely the code where a wrong
vtable offset or calling convention corrupts memory; "compile-verified" does
not verify any of it. What correct looks like: an automated CI lane
cross-checking the target, plus Windows-runner smoke tests before any Phase-7
wiring — the matrix honestly says windows="not_started" (docs/feature-matrix.toml:2372),
so this is a coverage hole, not a false claim.
Confidence: HIGH — Verification: read (workflows grepped; baseline cited).

F-002 [S3]
Where: crates/platform-windows/tests/seam_behaviour.rs:94-99
Code:
```
    assert_eq!(FilePick::Cancelled, FilePick::Cancelled);
    assert_eq!(
        FilePick::Paths(vec!["C:\\a".into()]),
        FilePick::Paths(vec!["C:\\a".into()])
    );
```
What is wrong: comparing a constant to itself and a constructed value to an
identical construction. These assert only that PartialEq is derived; no
production behavior is exercised. The preceding `is_err()` assertions in the
same test are real; these three lines are padding dressed as coverage.
Why it matters: inflates the assertion count of an "acceptance" suite.
What correct looks like: delete, or test a Display/serialization of FilePick.
Confidence: HIGH — Verification: read.

F-003 [S3]
Where: crates/platform-windows/tests/seam_behaviour.rs:322-329
Code:
```
    let warp = GpuAdapterInfo {
        vendor_id: 0x1414,
        device_id: 0x008C,
        name: "Microsoft Basic Render Driver".to_string(),
        dedicated_video_memory: Some(0),
        is_software: true,
    };
    assert!(warp.is_software);
```
What is wrong: the test constructs the struct with `is_software: true` and
then asserts the field is true. `is_software` is a plain pub field
(crates/platform-windows/src/gpu.rs:35), not computed. The comment "The WARP
software adapter (Microsoft Basic Render Driver)" implies production WARP
detection is being tested; nothing is. The meaningful part of this test is the
`vendor_name()` loop at lines 337-346.
Confidence: HIGH — Verification: read (gpu.rs field confirmed).

F-004 [S4]
Where: crates/platform-windows/tests/seam_behaviour.rs:246
Code:
```
    assert_eq!(queue.dropped() + queue.len() as u64, 3);
```
What is wrong: after `drain()` (line 244), len() is 0, so this reduces to
`dropped() == 3`, already asserted at line 243. The "invariant" framing adds
no discriminating power post-drain. Minor redundancy, no falsehood.
Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/platform-windows/tests/seam_behaviour.rs:15-23 (imports)
Code:
```
use platform_windows::WindowsPlatform;
use platform_windows::audio::{AudioDevices, UnavailableAudioDevices, stable_device_id};
```
What is wrong: nothing in the import set is wrong; this finding records that
the whole suite (and crate) is dead weight in the shipped graph — no crate or
app depends on platform_windows (verified: rg over crates/ and apps/ finds
zero code references outside the crate itself). All 16 green tests guard
unreachable code today. Not a defect of the test; a wiring fact the next
agent must know (Phase-7 target per docs/feature-matrix.toml:2372).
Confidence: HIGH — Verification: ran rg.

## 4. Per-function table (tests)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| unavailable_audio_devices_returns_the_typed_reason | 32-46 | REAL | — | fallback refuses with typed reason |
| unavailable_gpu_adapters_returns_the_typed_reason | 48-59 | REAL | — | same for gpu |
| unavailable_dialogs_returns_the_typed_reason | 61-75 | REAL | — | same for dialogs |
| typed_fallbacks_satisfy_the_seam_traits | 80-100 | SHALLOW | S3 | dyn-trait checks real; FilePick asserts tautological (F-002) |
| capability_classifies_and_converts_probe_errors | 106-150 | REAL | — | real error→capability mapping |
| feature_keys_are_the_stable_matrix_vocabulary | 152-164 | REAL | — | string keys pinned |
| platform_report_is_a_table_with_honest_defaults | 166-194 | REAL | — | NotYetImplemented default verified |
| event_queue_drains_fifo_and_empties | 201-232 | REAL | — | order-sensitive vec compare |
| event_queue_is_bounded_and_counts_drops | 234-247 | REAL | S4 | redundant post-drain assert (F-004) |
| event_queue_capacity_is_clamped_to_one | 249-260 | REAL | — | clamp + drop report |
| event_queue_is_thread_safe_for_callback_threads | 262-291 | REAL | — | 4 threads × 100 pushes; accounting invariant |
| stable_device_id_is_deterministic_and_pinned | 299-313 | REAL | — | FNV-1a constants independently reverified |
| gpu_adapter_info_labels_warp_and_well_known_vendors | 319-347 | SHALLOW | S3 | vendor_name loop real; warp.is_software tautology (F-003) |
| platform_errors_render_their_details | 353-391 | REAL | — | Display strings incl. signed-HRESULT form |
| discover_on_a_foreign_host_reports_wrong_os_everywhere | 399-430 | REAL | — | the core honesty contract, actually executed |
| file_dialog_request_carries_the_os_neutral_shape | 434-448 | REAL | — | Default + FileFilter::new |

## 5. Test honesty
All 16 ran and passed in the orchestrator baseline (cargo-test-workspace.txt
lines 4054-4082). Mutation reasoning: breaking a fallback to return Ok, a
wrong FNV constant, a wrong capacity clamp, a non-FIFO drain, or a lying
discover() would each fail a named test. The suite is honest about what it
cannot test (header quote, F-001). The two tautologies (F-002, F-003) would
pass under any mutation of production code.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every assertion host-runnable on any machine" | file header:1-4 | REAL | baseline 16 passed on Linux |
| "Real Windows behavior is [VERIFY]... compile-verified via cargo check --target x86_64-pc-windows-msvc only" | file header:10-13 | REAL-UNPROVEN | worklog.md records the manual run; no workflow automates it (F-001) |
| WASAPI row windows="not_started", evidence="" | docs/feature-matrix.toml:2372-2375 | REAL (honest) | no runtime Windows code path is claimed |

## 7. Dead / duplicated / unreachable code
The crate under test is entirely unreachable from shipped entry points today
(see F-005). Within the test file: no dead code.

## 8. Dependencies & flags
Deps: platform-windows (audio-output, media-types), dev: testkit. No feature
flags. The cfg(target_os="windows") real lane is never compiled on this host.

## 9. Open questions for the next agent
- Who is supposed to wire platform-windows into lumen-cli/desktop (Phase 7)?
  Until then every line here is prospective.
- Does ci-full.yml's windows lane actually run these tests on a Windows
  runner (where they'd exercise real FFI via discover())? UNVERIFIED —
  workflows disabled in this sandbox per phase2-exit.md §4.1.
