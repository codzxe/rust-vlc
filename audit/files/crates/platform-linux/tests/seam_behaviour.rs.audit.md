# AUDIT: crates/platform-linux/tests/seam_behaviour.rs
| Field | Value |
|---|---|
| Auditor | W08-A07 |
| Date | 2026-11-23 |
| Lines / functions | 475 / 15 tests + 1 helper |
| File verdict | REAL-UNPROVEN→REAL (tests executed, pass; but the code they prove is UNREACHABLE from shipped entry points) — test file itself: REAL |
| Reachable from shipped entry point? | NO — no crate depends on `platform-linux` (see F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `cargo test -p platform-linux --test seam_behaviour` (exit 0, 15 passed); python3 FNV-1a-64 oracle; rg over all Cargo.tomls |

## 1. One-paragraph verdict
This is a real test file, not a tautology farm: 12 of 15 tests drive production
code (`Unavailable*` fallbacks, `Capability::from_probe_error`, `DeviceEventQueue`,
`stable_device_id`, `PlatformError: Display`, `pci_vendor_name`) and would fail
if that code faked success or misaccounted drops. All 15 tests were executed and
pass. The pinned FNV-1a-64 constants were independently re-derived with a Python
oracle and match, so the hash pinning is evidence, not self-reference. The two
real defects: (a) the entire crate these tests vindicate is wired to nothing —
no workspace crate depends on `platform-linux`, so nothing shipped can reach any
of these seams; (b) five assertions in one test compare a value to itself
(compiler-tested tautologies).

## 2. What actually works (verified)
— `~/.cargo/bin/cargo test -p platform-linux --test seam_behaviour` -> "15
passed; 0 failed" exit 0 (compiled in 1.75s).
— python3 FNV-1a-64 of "a" / "alsa_output.pci-0000_00_1f.3.analog-stereo" /
"{0.0.0.00000000}.{e32f9c65-...}" -> 0xaf63dc4c8601ec8c / 0x2e60b52e4138dd9a /
0x5b40ed9bb4ce1081 — all three match the constants asserted at lines 351-360.
— `assert_err` (crates/testkit/src/lib.rs:100-106) panics on `Ok` — the refusal
assertions are real assertions.

## 3. Findings
F-001 [S2]
Where: crates/platform-linux/tests/seam_behaviour.rs:1-14 (whole file's subject)
Code:
```
use platform_linux::audio::{AudioDevices, UnavailableAudioDevices, stable_device_id};
use platform_linux::capability::{Capability, Feature, PlatformReport, UnavailableReason};
```
What is wrong: The seams proven here belong to `platform-linux`, and no crate in
the workspace depends on it. `rg "platform-linux" --glob Cargo.toml` over the
whole tree matches only its own manifest; `apps/cli/Cargo.toml` dependencies
(audio-engine, audio-output, codec-api, codecs, commands, demux, input,
media-clock, media-probe, media-types, playback-engine, plugin-api,
renderer-wgpu, settings-schema, streaming, video-engine) and `apps/desktop`'s
(commands, lumen-cli) do not list it. The release lane (`cargo build --release
--locked -p lumen-cli -p desktop`) never compiles this crate into a shipped
binary. Why it matters: the tests are honest but prove an orphan crate; a user
of lumen cannot trigger any of this code. What correct looks like: a dispatcher
crate (or lumen-cli) selecting platform-* by target_os, or the crate deleted
until wiring lands. Confidence: HIGH — Verification: ran (rg + manifest reads).
F-002 [S3]
Where: crates/platform-linux/tests/seam_behaviour.rs:78-83
Code:
```
    assert_eq!(FilePick::Cancelled, FilePick::Cancelled);
    assert_eq!(
        FilePick::Paths(vec!["/a".into()]),
        FilePick::Paths(vec!["/a".into()])
    );
    assert_ne!(FilePick::Cancelled, FilePick::Paths(Vec::new()));
```
What is wrong: The first two compare a value to a byte-identical construction —
under `#[derive(PartialEq)]` (dialog.rs:59) they cannot fail; they test the
compiler. The third only fails if a hand-written PartialEq equated distinct
variants; none exists. The preceding dyn-dispatch assertions (lines 69-76) are
the real content of this test; these five lines are filler under a comment
claiming they "pin the success shape". Why it matters: inflates the apparent
coverage of the test named `typed_fallbacks_satisfy_the_seam_traits`.
Confidence: HIGH — Verification: read + mutation reasoning (derive cannot fail).
F-003 [S3]
Where: crates/platform-linux/tests/seam_behaviour.rs:309-340
Code:
```
    let handles: Vec<_> = (0..4)
        .map(|thread| {
            let queue = std::sync::Arc::clone(&queue);
            std::thread::spawn(move || {
                for _ in 0..100 {
                    queue.push(PlatformEvent::AudioSetChanged);
                }
                let _ = thread;
            })
```
What is wrong: the concurrency test is a smoke test — 400 uncontended-ish
pushes cannot deterministically expose a race; it verifies the accounting
invariants (pushed==400, dropped+len==400) which real code upholds trivially
under the mutex. The dead `let _ = thread;` (line 321) is a no-op closure
capture. Why it matters: the name promises thread-safety proof it cannot
deliver; a TSan run or interleaved drain-during-push would. What correct looks
like: at least one drain concurrent with pushes asserting no lost/duplicated
accounting. Confidence: MEDIUM — Verification: read + ran (passes).
F-004 [S4]
Where: crates/platform-linux/tests/seam_behaviour.rs:22-60 vs 67-84
Code:
```
    let err = assert_err(devices.enumerate(), "enumerate must fail honestly");
    assert_eq!(err, reason);
```
What is wrong: tests 1-3 and the first half of test 4 assert the same
property (fallback method returns the exact reason) four times over three
types with identical bodies — copy-paste structure. Harmless but bulk.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| unavailable_audio_devices_returns_the_typed_reason | 21-33 | REAL | — | drives real fallback; fails if Ok sneaks in |
| unavailable_gpu_adapters_returns_the_typed_reason | 35-45 | REAL | — | same |
| unavailable_dialogs_returns_the_typed_reason | 47-60 | REAL | — | same, both methods |
| typed_fallbacks_satisfy_the_seam_traits | 66-84 | SHALLOW | S3 | dyn part real; FilePick part tautological |
| capability_classifies_and_converts_probe_errors | 90-157 | REAL | — | exercises from_probe_error incl. symbol/timeouts |
| unavailable_reasons_render_honest_strings | 159-191 | REAL | — | Display strings pinned |
| feature_keys_are_the_stable_matrix_vocabulary | 193-205 | REAL | — | constant keys pinned (weak but real) |
| platform_report_is_a_table_with_honest_defaults | 207-234 | REAL | — | replace-not-append + honest default pinned |
| event_queue_drains_fifo_and_empties | 240-272 | REAL | — | order pinned |
| event_queue_is_bounded_and_counts_drops | 274-292 | REAL | — | drop accounting pinned |
| event_queue_capacity_is_clamped_to_one | 294-306 | REAL | — | clamp pinned |
| event_queue_is_thread_safe_for_callback_threads | 308-340 | SHALLOW | S3 | smoke only; dead `let _ = thread;` |
| stable_device_id_is_deterministic_and_pinned | 349-365 | REAL | — | constants re-derived independently (python3) |
| gpu_adapter_info_labels_well_known_pci_vendors | 371-394 | REAL | — | lookup-table test; would catch a wrong map |
| info_label (helper) | 396-405 | REAL | — | test-local helper |
| platform_errors_render_their_details | 411-475 | REAL | — | 8 Display cases pinned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unavailable_* (3) | YES | fallback returning Ok(vec![]) hits assert_err's unreachable!; wrong reason fails assert_eq |
| typed_fallbacks_satisfy_the_seam_traits | PARTIALLY | dyn assertions yes; lines 78-83 cannot fail (derived PartialEq) |
| capability_classifies... | YES | mapping changes break exact-eq/match arms |
| unavailable_reasons_render... | YES | Display format drift breaks contains() |
| feature_keys_... | YES | key rename breaks it |
| platform_report_is_a_table... | YES | set()-append bug breaks rows==1 |
| event_queue_* (4) | YES | FIFO/bound/clamp/accounting all pinned; thread test only for counter loss |
| stable_device_id_... | YES | any hash change breaks pinned constants (independently derived) |
| gpu_adapter_info_labels... | YES | vendor map change breaks it |
| platform_errors_render... | YES | Display drift breaks it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every assertion host-runnable on any machine" | file header lines 2-5 | REAL | ran full suite on this Linux host, 15/15 pass |
| "cross-checked against an independent implementation — Python" (id minting) | lines 343-346 comment | REAL | re-derived all 3 constants with python3, exact match |

## 7. Dead / duplicated / unreachable code
The test file contains no dead code except `let _ = thread;` (line 321).
The audited crate as a whole is unreachable from shipped entry points (F-001).

## 8. Dependencies & flags
Dev-dependency `testkit` (assert_err) — used, real. No feature gates in this
file. Crate is std-only (no external deps) per its manifest.

## 9. Open questions for the next agent
- Who was supposed to consume `LinuxPlatform` (the 7.C1 fallback chain?
  renderer-wgpu?) — if that consumer card never landed, F-001 is a planning
  failure worth an S2 at the architecture level, not just this file.
- linux_integrations.rs (not assigned to me) covers discover(); confirm its
  host-gating is honest.
