# AUDIT: crates/platform-linux/tests/linux_integrations.rs
| Field | Value |
|---|---|
| Auditor | W08-A06 |
| Date | 2026-11-23 |
| Lines / functions | 502 / 16 (13 tests + 3 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file) — runs via `cargo test -p platform-linux`; verified by direct execution of the prebuilt test binary |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/linux_integrations-ca1702194c57d626` -> 13 passed / 0 failed, exit 0; `rg` in testkit/src/lib.rs |

## 1. One-paragraph verdict
This is a real integration test suite that executes the Linux platform seams against the actual host: DRM sysfs fixtures with injectable roots, real fake-zenity/kdialog shell stubs driven through PATH injection, real D-Bus socket probing with a bound UnixListener, real PulseAudio dlopen probe, and an end-to-end `LinuxPlatform::discover` consistency check. It is unusually honest about host variance (docstring lines 8-11 declare "honesty, not hardware presence"). Two tests assert nothing of substance (`xdg_discovery_uses_the_real_environment`) or have a dead empty success arm (`pulse_subscribe_without_a_server_is_the_typed_error`), and the live-server and real-GPU paths are admitted as `[VERIFY]` gaps rather than faked. No tautological fixture-baking, no mocked-out thing-under-test: the stubs replace zenity/kdialog, which are legitimately the test doubles, not the code under test.

## 2. What actually works (verified)
- Ran the prebuilt binary directly: `./target/debug/deps/linux_integrations-ca1702194c57d626` -> `test result: ok. 13 passed; 0 failed; 0 ignored` exit 0 (matches baseline cargo-test-workspace.txt:3961-3980).
- The zenity/kdialog stub path genuinely execs a real `/bin/sh` script (write_script, lines 85-93) and parses its stdout — a real subprocess round trip.

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/tests/linux_integrations.rs:369-376
Code:
```rust
fn xdg_discovery_uses_the_real_environment() {
    // $PATH-driven discovery (this host has no zenity/kdialog — the
    // discovery must say exactly that, not guess).
    let dialogs = XdgDialogs::discover();
    let _ = dialogs.zenity();
    let _ = dialogs.kdialog();
}
```
What is wrong: This test asserts nothing. Both probe results are discarded with `let _ =`; the comment claims "the discovery must say exactly that" but no assertion ever checks what discovery said. It can only fail if `discover()` panics. Why it matters: a regression that makes PATH discovery return `Some("zenity")` on a zenity-less host (or vice versa) passes this test silently. What correct looks like: assert `dialogs.zenity().is_none() || dialogs.zenity().is_some_and(|p| p.is_file())` — or at minimum pin the on-host expectation. Confidence: HIGH — Verification: ran (passes trivially; mutation reasoning).
F-002 [S3]
Where: crates/platform-linux/tests/linux_integrations.rs:432-440
Code:
```rust
    match devices.subscribe(queue) {
        Ok(()) => {
            // A live server ([VERIFY] here): subscription succeeded;
            // the queue is the delivery sink.
        }
```
What is wrong: The success arm is empty. On a host with a live PulseAudio server this test verifies nothing about the subscription — the comment promises "the queue is the delivery sink" but no event is ever produced or observed on `queue`. Why it matters: the notification path's happy case is untested anywhere in this suite; a subscription that silently drops all callbacks passes. What correct looks like: on a live server, trigger a sink change (or at least assert the subscription context stays alive) and observe `queue`. Confidence: HIGH — Verification: read; the `[VERIFY]` marker is honest but the arm remains a no-op.
F-003 [S4]
Where: crates/platform-linux/tests/linux_integrations.rs:197-201
Code:
```rust
    let adapters = scanner.enumerate().must();
    for adapter in &adapters {
        assert!(!adapter.name.is_empty());
        assert!(!adapter.name.contains('\u{0}'));
    }
```
What is wrong: On this host (no DRM nodes) `adapters` is empty, so both loop assertions are dead code; the only live assertions are the `sys_root()` equality and absence of panic. The test name promises "honestly" but honesty of a non-empty result is never checked on a GPU host here. Confidence: HIGH — Verification: read + host fact (sandbox has no /sys/class/drm cards — see capability.rs audit); minor because the injectable-root tests (lines 100-188) do pin the populated path.
F-004 [S4]
Where: crates/platform-linux/tests/linux_integrations.rs:389-419
Code:
```rust
    match PulseAudioDevices::probe() {
        Ok(devices) => { ... }
        Err(PlatformError::LibraryUnavailable { names, .. }) => { ... }
        Err(other) => {
            unreachable!("probe yields Library/Service truth, got {other:?}");
        }
    }
```
What is wrong: The test accepts every outcome the API can produce (success, either typed error), so on any single host it verifies only one branch of the taxonomy. Mitigated: the branch it does hit is pinned hard (SONAME, service string), and the `[VERIFY]` markers (lines 388, 399) admit the gap. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_file_retrying_etxtbsy | 47-61 | REAL | — | Bounded ETXTBSY retry, 4 attempts, 150ms*attempt backoff; only retries the SpawnFailed+"Text file busy" face |
| temp_root | 65-72 | REAL | — | Per-tag+pid temp dir; cleans prior runs |
| write_script | 85-93 | REAL | — | Staging+rename exec stubs, 0o755; correct overlayfs mitigation |
| drm_scanner_reads_a_realistic_sysfs_tree | 100-150 | REAL | — | Pins names, vendor/device ids, software classification, None VRAM; would fail if scanner broken |
| drm_scanner_is_honest_on_empty_and_absent_trees | 153-171 | REAL | — | Pins empty-not-error for absent sysfs |
| drm_scanner_handles_degenerate_nodes | 174-188 | REAL | — | Pins ids=0 / "unknown", never guessed |
| drm_scanner_reads_the_real_host_sysfs_honestly | 191-202 | SHALLOW | S4 | Loop assertions dead on no-GPU host (F-003) |
| xdg_dialogs_run_the_zenity_fallback | 209-240 | REAL | — | Real subprocess, multi-path parse pinned |
| xdg_dialogs_cancel_is_not_an_error | 243-263 | REAL | — | Exit-1 = Cancelled for zenity and kdialog |
| xdg_dialogs_kdialog_fallback_parses_output | 266-283 | REAL | — | kdialog stdout parse pinned |
| xdg_dialogs_without_any_backend_refuse_with_the_typed_error | 286-323 | REAL | — | Pins ServiceUnavailable wording + CallFailed{api,code} |
| xdg_portal_probe_reports_bus_truth | 326-367 | REAL | — | Four bus states incl. bound socket + abstract refusal |
| xdg_discovery_uses_the_real_environment | 370-376 | STUB (as a test) | S3 | Asserts nothing (F-001) |
| pulse_probe_reports_this_hosts_truth | 383-420 | REAL | S4 | Pins the branch this host hits (F-004) |
| pulse_subscribe_without_a_server_is_the_typed_error | 423-444 | SHALLOW | S3 | Empty Ok arm (F-002) |
| discover_assembles_this_hosts_report | 452-502 | REAL | — | Cross-checks report() vs live seam calls; consistency invariant |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| drm_scanner_reads_a_realistic_sysfs_tree | YES | Exact name strings + ids + ordering pinned |
| drm_scanner_is_honest_on_empty_and_absent_trees | YES (partly) | Only pins Ok(empty); a scanner that errors on missing root would fail — good |
| drm_scanner_handles_degenerate_nodes | YES | vendor 0 / "unknown" pinned |
| drm_scanner_reads_the_real_host_sysfs_honestly | NO (on this host) | Empty list ⇒ loop body dead; only sys_root pinned |
| xdg_dialogs_run_the_zenity_fallback | YES | Two paths parsed and compared |
| xdg_dialogs_cancel_is_not_an_error | YES | Exit-1→Cancelled mapping pinned |
| xdg_dialogs_kdialog_fallback_parses_output | YES | Output parse pinned |
| xdg_dialogs_without_any_backend_refuse... | YES | Error type + detail substrings + crash code pinned |
| xdg_portal_probe_reports_bus_truth | YES | All four states pinned with real sockets |
| xdg_discovery_uses_the_real_environment | NO | Results discarded |
| pulse_probe_reports_this_hosts_truth | YES (weakly) | Only the error taxonomy + strings pinned |
| pulse_subscribe_without_a_server... | YES (this host) | ServiceUnavailable arm asserted; Ok arm no-op |
| discover_assembles_this_hosts_report | YES | report()/seam consistency enforced both directions |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the sanctioned test double" (zenity/kdialog fakes) | lines 3-4 | REAL | Real /bin/sh stubs executed (ran: 13 passed) |
| "outcomes vary with the host ... every assertion accepts exactly the outcomes the integrations can truthfully produce" | lines 8-11 | REAL | Verified by reading all 13 tests; two take this license too far (F-001, F-002) |

## 7. Dead / duplicated / unreachable code
- Lines 197-201 loop body is unreachable on this host (no DRM adapters).
- Lines 433-436 (Ok arm) unreachable on this host (no PA server).

## 8. Dependencies & flags
- dev-dep testkit (Must/assert_err) — real helpers, verified at crates/testkit/src/lib.rs:44-105.
- No cfg gating, no #[ignore], no feature flags in this file. All 13 tests run (execution output above).

## 9. Open questions for the next agent
- Does `pulse.rs` subscribe() on a live server actually route events to the DeviceEventQueue? UNVERIFIABLE in this sandbox (no PA server) — flagged [VERIFY] by the test itself.
- Confirm `portal_filechooser_implemented()` returning false is matched by xdg.rs reality (audited next).
