# AUDIT: crates/device-discovery/tests/fallback_paths.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Lines / functions | 113 / 5 tests |
| File verdict | REAL — tests REAL honest-degradation behavior; crate UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (test-only file; crate zero dependents) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/fallback_paths-03d593dcc22edc92` -> 5 passed / 0 failed, exit 0 (binary 18:07 > source 17:35); sed verifications |

## 1. One-paragraph verdict
Five real integration tests over the public assembly (`Discovery::probe()`): the volume lanes' NotYetImplemented verdicts are pinned including the follow-up-owner detail and the seam's refusal to fake a list; the report is exactly three rows; the SSDP lane's Available verdict carries the searcher; the fallback translation to engine errors pins stage/code/source-chain; and the Io→DependencyMissing collapse is pinned as intended behavior. All pass. One test asserts a host fact, not a code fact — `ssdp_lane_probes_available_on_this_host` requires a working UDP stack and will fail on any socket-less CI runner (the comment admits this). That is a portability hazard, not fakeness.

## 2. What actually works (verified)
— `./target/debug/deps/fallback_paths-03d593dcc22edc92` -> `5 passed; 0 failed` (0.00s), all five listed.

## 3. Findings

F-001 [S4]
Where: crates/device-discovery/tests/fallback_paths.rs:61-75
Code:
```
fn ssdp_lane_probes_available_on_this_host() {
    // Host assertion (this sandbox has a working UDP stack — the
    // recorded probe); a hermetic-CI future that loses sockets will
    // fail here honestly, saying so.
    let discovery = Discovery::probe();
    let verdict = discovery.report().get(Feature::NetworkServiceDiscovery);
    assert!(
        verdict.is_available(),
        "the native UDP socket probe must pass here; got {verdict:?}"
    );
```
What is wrong: the test asserts the runner's environment (a bindable UDP socket), not a property of the code. On a socket-less container, a CI runner with a locked-down network namespace, or a sandbox where UDP bind returns EPERM, this fails with a message that reads like a product bug ("the native UDP socket probe must pass here"). The honest-degradation design being tested says an unavailable socket should yield a typed Unavailable verdict — the test then rejects exactly that honest outcome on hosts where it occurs. What correct looks like: assert the verdict is either Available-with-searcher or a typed Unavailable whose reason is DependencyMissing — the *consistency* is the code property; raw availability is the host's. Confidence: HIGH — Verification: read + run (passes on this sandbox; failure mode reasoned, not executed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe_reports_the_volume_lanes_as_not_yet_implemented | 19-41 | REAL | — | both lanes + seam refusal pinned (Ok(_) hits unreachable!) |
| probe_reports_every_feature_row_exactly_once | 44-58 | REAL | — | exact 3-row sorted table |
| ssdp_lane_probes_available_on_this_host | 61-75 | REAL | S4 | host-fact assertion (F-001); passes here |
| volumes_service_over_the_fallback_translates_to_engine_errors | 78-97 | REAL | — | stage=Discovery, code="discovery.unavailable", source renders the reason |
| capability_from_probe_error_collapses_the_reason_class | 100-113 | REAL | — | pins the deliberate Io→DependencyMissing collapse (error.rs F-002 is intended-but-mislabeled behavior; this test locks it in) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_reports_the_volume_lanes… | YES | exhaustive verdict match + detail substring + seam Err-variant match |
| probe_reports_every_feature_row… | YES | exact sorted vector equality (a missing/dup row fails) |
| ssdp_lane_probes_available… | YES (on capable hosts) | availability + `ssdp().is_some()` pairing; NO on socket-less hosts (fails spuriously — F-001) |
| volumes_service_over_the_fallback… | YES | Ok branch is unreachable!; exact stage/code/source pins |
| capability_from_probe_error_collapses… | YES | exhaustive match on collapse result |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the volume lanes report the typed NotYetImplemented verdict (a compile-time fact … by layering)" | fallback_paths.rs:6-8 | REAL | lib.rs:100-118 mints the verdict from the layering fact; test pins it |
| "the SSDP lane probes Available (the ephemeral UDP bind works here)" | fallback_paths.rs:8-10 | REAL on this host | test passes; host-limited (F-001) |
| "never a fake list" | fallback_paths.rs:33 | REAL | Ok branch unreachable!; UnavailableVolumes refuses typed |

## 7. Dead / duplicated / unreachable code
None in-file. Crate UNREACHABLE from shipped entry points (zero dependents).

## 8. Dependencies & flags
device_discovery (capability, device, error, volume, Discovery, DiscoveryContext), media_types (implicitly, via err.stage/context — actually `media_types` is a transitive dev-dependency used directly at L90-91). No features, no unsafe, network I/O limited to one UDP bind in the probe.

## 9. Open questions for the next agent
- If the CI matrix ever adds a network-isolated lane, F-001 fires — restructure before then.
- volume.rs's `UnavailableVolumes` and `VolumesService` translation are other agents' files; this suite pins their public behavior — cross-check no stronger claims exist there.
