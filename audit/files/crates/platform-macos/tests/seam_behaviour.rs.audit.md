# AUDIT: crates/platform-macos/tests/seam_behaviour.rs
| Field | Value |
|---|---|
| Auditor | W08-A09 |
| Date | 2026-11-23 |
| Lines / functions | 436 / 16 tests |
| File verdict | REAL (test suite: runs, passes, would fail on regression; guards an UNREACHABLE crate) |
| Reachable from shipped entry point? | NO — no crate depends on platform-macos (rg over every Cargo.toml: only its own); release CI builds only `-p lumen-cli -p desktop` (release.yml:193,474,627,763,911) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg (dependents, matrix keys, CI), python3 FNV-1a-64 recompute, sed quote verification |

## 1. One-paragraph verdict
This is an honest, host-runnable acceptance suite: 16 tests, all executed and green in the
orchestrator baseline (cargo-test-workspace.txt:4014-4034, `16 passed; 0 failed; 0 ignored`),
covering typed fallbacks, capability negotiation, the bounded event queue, FNV-1a-64 id
pinning, and the wrong-OS `discover()` wiring. It fabricates nothing — it explicitly refuses
to claim macOS runtime behavior. Its two defects are contextual: the crate it guards is wired
to nothing (dead in every shipped build), and its "stable matrix vocabulary" is not in the
feature matrix.

## 2. What actually works (verified)
- Baseline run: audit/runs/baseline/cargo-test-workspace.txt:4014-4034 -> `running 16 tests ...
  test result: ok. 16 passed; 0 failed; 0 ignored` (platform-macos seam_behaviour).
- FNV-1a-64 pins are genuine: python3 recompute gives
  fnv1a64("a") = 0xaf63dc4c8601ec8c and
  fnv1a64("AppleHDAEngineOutput:0x0,Output,0x1,0x0") = 0x1b44b68bbc0c5287 — both equal the
  literals asserted at seam_behaviour.rs:291-296. The pinning test has real teeth.
- testkit::assert_err is a real assert (testkit/src/lib.rs:100-105: `Ok(_) => unreachable!`).
- `discover_on_a_foreign_host_reports_wrong_os_everywhere` (392-418) executes the real cfg
  wiring on this Linux host and pins the exact typed refusal — would fail if any seam began
  faking success.

## 3. Findings
F-001 [S3]
Where: crates/platform-macos/tests/seam_behaviour.rs:145-156
Code:
```
fn feature_keys_are_the_stable_matrix_vocabulary() {
    assert_eq!(
        Feature::AudioEnumeration.key(),
        "platform.audio.enumeration"
    );
```
What is wrong: the name asserts these keys ARE the matrix vocabulary, but
docs/feature-matrix.toml contains zero occurrences of `platform.audio.enumeration`,
`platform.audio.notifications`, `platform.gpu.enumeration`, or `platform.dialogs`
(rg over the whole repo: matches only in the three platform crates' capability.rs + their
tests). The keys are pinned cross-crate only; the actual matrix row that would consume them
(`afeat-audio-device-switch`, feature-matrix.toml:1785) says `status = { windows =
"not_started", ... }` with `evidence = ""`. Why it matters: the test certifies a wiring that
does not exist; a reader concludes the platform features are matrix-tracked. What correct
looks like: either matrix rows carrying these keys, or a test name saying "cross-lane pinned
keys". Confidence: HIGH — Verification: ran rg.

F-002 [S3]
Where: crates/platform-macos/tests/seam_behaviour.rs:1-15 (whole crate context)
Code:
```
//! This suite runs on a Linux host, which is exactly the "wrong-OS
//! build" case the crate's honesty contract covers: every seam must
//! be the typed fallback and [`MacOsPlatform::discover`] must say
//! `WrongOs` for every feature — the compile-time cfg story
//! verified at runtime from the outside.
```
What is wrong: nothing quoted is false, but the entire suite guards a crate nothing ships:
no Cargo.toml in the repo declares a dependency on platform-macos (rg across
crates/*/Cargo.toml and the root manifest: only platform-macos' own). device-discovery (the
L4 crate below it) deliberately does not depend upward on platform-* (device-discovery
Cargo.toml:12-19), and no L6+ crate depends on it either. All five release-lane CI builds
compile only `-p lumen-cli -p desktop` (release.yml:193,474,627,763,911), so no shipped
binary, on any OS, contains this code. The crate is documented as awaiting issues #41/#42
(platform-macos/Cargo.toml:9-11), so this is disclosed scaffolding, not fraud — but until
wired it is UNREACHABLE code whose green tests prove nothing about any user-visible path.
Confidence: HIGH — Verification: ran rg over manifests + CI.

F-003 [S4]
Where: crates/platform-macos/tests/seam_behaviour.rs:13-15
Code:
```
//! `cargo check --target aarch64-apple-darwin` and
//! `x86_64-apple-darwin` only).
```
What is wrong: the standing "compile-verified" claim has no captured artifact in-tree
(docs/gates/logs/ has no darwin check log; rg `apple-darwin` over docs/gates -> only
phase7-exit.md prose). Sole provenance is the worklog entry (worklog.md:556, task 7.w22b:
"cargo check -p platform-macos --target aarch64-apple-darwin and x86_64-apple-darwin
[--all-targets] clean and warning-free"). Not reproducible here (no darwin rustlib target
installed), and nothing in CI re-checks it — the macOS runners build lumen-cli/desktop
which do not depend on platform-macos, so the FFI can silently stop compiling and no gate
fails. The [VERIFY] labeling keeps it honest in intent. Confidence: HIGH —
Verification: read + rg; the check run itself UNVERIFIED (no darwin target in sandbox).

## 4. Per-function table (tests)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| unavailable_audio_devices_returns_the_typed_reason | 34-46 | REAL | — | fallback stores+returns reason; near-tautological but catches fake-success |
| unavailable_gpu_adapters_returns_the_typed_reason | 48-59 | REAL | — | same pattern |
| unavailable_dialogs_returns_the_typed_reason | 61-74 | REAL | — | same pattern |
| typed_fallbacks_satisfy_the_seam_traits | 79-97 | REAL | — | dyn-trait wiring + FilePick vocabulary distinctness |
| capability_classifies_and_converts_probe_errors | 103-142 | REAL | — | error->reason mapping pinned incl. API name in detail |
| feature_keys_are_the_stable_matrix_vocabulary | 144-156 | MISLEADING NAME | S3 | F-001: keys exist in no matrix |
| platform_report_is_a_table_with_honest_defaults | 158-184 | REAL | — | set() replaces; unprobed = NotYetImplemented |
| event_queue_drains_fifo_and_empties | 191-222 | REAL | — | order pinned |
| event_queue_is_bounded_and_counts_drops | 224-237 | REAL | — | drop accounting pinned |
| event_queue_capacity_is_clamped_to_one | 239-250 | REAL | — | clamp + drop report |
| event_queue_is_thread_safe_for_listener_threads | 252-281 | REAL | — | 4 threads x100 pushes; invariants `pushed==400`, `len<=64`, `dropped+len==400` |
| stable_device_id_is_deterministic_and_pinned | 289-300 | REAL | — | pins verified genuine by my python3 FNV-1a-64 recompute |
| gpu_adapter_info_answers_the_metal_shape | 307-344 | REAL | — | vendor table; metal shape (vendor 0 = "unknown") |
| platform_errors_render_their_details | 350-383 | REAL | — | Display strings pinned |
| discover_on_a_foreign_host_reports_wrong_os_everywhere | 391-418 | REAL | — | strongest test: real cfg wiring exercised |
| file_dialog_request_carries_the_os_neutral_shape | 422-436 | REAL | — | trivial default/constructor shape |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 16 | YES for what they cover | typed-reason equality, queue invariants, FNV pins, WrongOs wiring are all concrete assertions; baseline shows them running (not ignored, not cfg-gated out). What NO test covers: any real macOS FFI path — admitted in header, so not dishonest. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every assertion host-runnable on any machine" | seam_behaviour.rs:2-4 | REAL | baseline: 16/16 ran on Linux host |
| "compile-verified via the two Apple cross-check targets" | seam_behaviour.rs:13-15 | REAL-UNPROVEN | worklog.md:556 records it; no captured log, no CI re-check, not reproducible in sandbox |
| "keys are the stable matrix vocabulary" | seam_behaviour.rs:145 | FALSE (as name) | rg: keys absent from docs/feature-matrix.toml |
| "Real Apple-hardware behavior is `[VERIFY]`" | seam_behaviour.rs:12-13 | HONEST REFUSAL | no runtime macOS test exists or is claimed |

## 7. Dead / duplicated / unreachable code
The whole crate is unreachable from shipped entry points (F-002). The suite is mirrored
near-verbatim in platform-linux and platform-windows (same test names; 15/16 shared with
linux's seam_behaviour) — deliberate cross-lane pinning, but ~3x duplication until the
Lead's L5 unification (worklog.md:563 records this as [PROPOSAL]).

## 8. Dependencies & flags
dev-dep testkit only (Cargo.toml:43-46). No features. No cfg gates in the test file — the
wrong-OS path taken is whatever the host compiles, which is the point.

## 9. Open questions for the next agent
- Who will wire platform-macos into a shipped binary (issues #41/#42)? Until then every
  platform-crate finding is scaffolding-level.
- Does any macOS CI job ever run `cargo check --workspace`? If yes, that would keep the
  FFI compiling; I found none (release.yml builds -p lumen-cli -p desktop only).
