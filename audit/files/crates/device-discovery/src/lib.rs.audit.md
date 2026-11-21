# AUDIT: crates/device-discovery/src/lib.rs (whole file)
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-21 |
| Lines / functions | 156 / 4 (1 struct + `probe` + 3 accessors; plus 7 `pub mod` declarations + re-export block) |
| File verdict | UNREACHABLE (honest typed-fallback scaffold in a dependency-island crate: no manifest in the repo lists `device-discovery` as a dependency) |
| Reachable from shipped entry point? | NO. Proof: `rg -n "device-discovery" --glob 'Cargo.toml' -l .` → 4 files, all comment-only mentions (the crate itself + platform-{linux,macos,windows} header comments saying the edge is issue #42's allowlist follow-up); an awk pass over every `[dependencies]` section in crates/*/Cargo.toml and apps/*/Cargo.toml finds zero entries. Code mentions elsewhere are comments (playback-engine/events.rs:106,777,784) or a different concept (`plugin_api::discovery` has its own `Device`). Nothing in `lumen` or `lumen-tauri` links this crate. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "device-discovery" --glob 'Cargo.toml' -l .` + per-manifest `[dependencies]` awk (island proof) · `rg -ln "device_discovery" apps/ crates/ tools/` (comment-only mentions) · `rg -n "device-discovery" crates/platform-*/Cargo.toml` (comment-only, edges anticipated not declared) · `sed -n '2590,2680p' docs/feature-matrix.toml` (services-discovery rows: sd-directory-watch not_started, sd-upnp blocked) · baseline `orchrun-test-workspace-nofailfast.log.txt:2371-2413` (38 unit tests ok), `:2415-2429` (fallback_paths 5/5 ok — these pin THIS file's `probe()`), `:2431-2441` (ssdp integration 5/5 ok incl. real loopback UDP), volume_monitor 3/3 |

## 1. One-paragraph verdict
This is the honest-scaffold pattern done right: the volume lane ships as a typed `Unavailable` fallback whose every method refuses `DiscoveryError::Unavailable` — never a fake list, never a fabricated event — the SSDP lane runs a real UDP-socket probe and reports Available where the host allows it, and both verdicts land in a capability table that three dedicated integration tests pin row-for-row (exactly three features, volume lanes NotYetImplemented with the follow-up owner named, SSDP Available with the searcher attached). All 51 tests in the crate ran green in the baseline, including real loopback UDP round-trips. The two defects are small: the public `volumes()` accessor hands callers a raw `MutexGuard` (lock lifetime escapes the API), and the same volume verdict is minted twice with two different detail strings that can drift. And the whole crate is another dependency island — 9 modules of tested, capability-honest code that no shipped binary links, which the matrix honestly reflects (sd-directory-watch: not_started, evidence "").

## 2. What actually works (verified)
— This file's `probe()` is directly pinned by baseline-green integration tests: `probe_reports_the_volume_lanes_as_not_yet_implemented` (fallback_paths.rs:16-39 — asserts BOTH the capability rows and that `discovery.volumes().enumerate()` answers the typed Unavailable, "never a fake list"), `probe_reports_every_feature_row_exactly_once` (:41-56 — the report is exactly [NetworkServiceDiscovery, VolumeEnumeration, VolumeNotifications]), `ssdp_lane_probes_available_on_this_host` (:60-72 — Available verdict ⇒ `discovery.ssdp().is_some()`). All green in baseline `:2415-2429`.
— Crate-wide: 38 unit tests ok (`:2371-2413`), ssdp integration 5/5 ok incl. `loopback_roundtrip_collects_parses_and_dedups` (`:2431-2441`), volume_monitor 3/3 ok — the doc's "real loopback UDP round-trips in tests/ssdp.rs" claim (37) is backed by executed baseline evidence.
— The layering claim verified: no `platform-*` crate depends on this crate today (comment-only mentions), matching the doc's "a dependency from here on `platform-*` would point upward (L2→L5) and is forbidden" (27-29) — the seam direction is clean.
— Poison handling in `volumes()` (145-148): `poisoned.into_inner()` recovers the guard — a read seam that must not wedge on a panicked consumer.

## 3. Findings

### F-001 [S4]
Where: crates/device-discovery/src/lib.rs:144-149
Code:
```
    pub fn volumes(&self) -> std::sync::MutexGuard<'_, Box<dyn VolumeSource>> {
        match self.volumes.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
```
What is wrong: the public API returns a `MutexGuard` directly — the lock's lifetime is caller-controlled, nothing prevents a caller from storing the guard (e.g. in a struct) and deadlocking every other consumer indefinitely. The doc (141-143) states the intended single-consumer wiring and says the mutex "keeps the seam sound if they don't" — a guard that can be held forever does not keep anything sound; it just converts a misuse into a hang instead of a race.
Why it matters: today the crate is unwired and the one test consumer drops the guard promptly, so the blast radius is zero; the moment a host actually wires the monitor thread plus a service path, this signature invites the exact deadlock the doc hand-waves away.
What correct looks like: `pub fn with_volumes<R>(&self, f: impl FnOnce(&mut dyn VolumeSource) -> R) -> R` — a scoped closure that guarantees release, or `Arc<Mutex<...>>` shared explicitly with the monitor.
Confidence: HIGH — Verification: read (signature + doc traced; no current caller can deadlock — grep shows no shipped consumers).

### F-002 [S4]
Where: crates/device-discovery/src/lib.rs:106-116
Code:
```
        let volume_reason = DiscoveryError::Unavailable {
            reason: UnavailableReason::NotYetImplemented {
                detail: "no platform adapter implements VolumeSource yet — the \
                         platform-{windows,linux,macos} volume-watcher cards own the \
                         follow-up (doc 01 §2.7 registry edges; issue #42)",
            },
        };
        let volumes: Box<dyn VolumeSource> = Box::new(UnavailableVolumes::because(volume_reason));
        let volume_capability = Capability::Unavailable(UnavailableReason::NotYetImplemented {
            detail: "volume watching waits on the platform-* VolumeSource integrations",
        });
```
What is wrong: the same verdict (volume lane not yet implemented) is minted twice with two DIFFERENT detail strings — the seam error names doc 01 §2.7 and issue #42, the capability row says only "volume watching waits on the platform-* VolumeSource integrations". A user reading the capability table and a user reading the surfaced error get different explanations of the same fact; the strings can drift further apart with any edit.
Why it matters: the crate's whole doctrine is honest capability reporting; two divergent phrasings of one verdict is exactly the kind of drift that doctrine exists to prevent. The integration test only pins `detail.contains("platform")` (fallback_paths.rs:25-28), which both strings satisfy — nothing pins their agreement.
What correct looks like: one `const VOLUME_LANE_DETAIL: &str` used for both the error reason and the capability row.
Confidence: HIGH — Verification: read (both constructions traced; test's substring assert checked).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + `pub mod`/re-exports | 1-70 | REAL | — | claims cross-checked: loopback UDP tests exist and ran (baseline :2431-2441); layering direction verified (no platform-* dep). |
| `Discovery` (struct) | 81-85 | REAL | — | Mutex<Box<dyn VolumeSource>> + Option<SsdpSearcher> + report. |
| `Discovery::probe` | 96-131 | REAL (typed fallback by design) | — | volume lane: compile-time honest Unavailable (no runtime guess); SSDP lane: real socket probe; report rows pinned by 3 integration tests. |
| `Discovery::report` | 133-137 | REAL | — | pure accessor, `#[must_use]`. |
| `Discovery::volumes` | 139-149 | REAL | S4 (F-001) | guard-escaping signature; poison recovery correct. |
| `Discovery::ssdp` | 151-155 | REAL | — | pure accessor, `#[must_use]`. |

## 5. Test honesty
No tests in this file; its behavior is pinned by `tests/fallback_paths.rs` (crate-level integration, not in any single agent's chunk — cited here because it is this file's only coverage):
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| probe_reports_the_volume_lanes_as_not_yet_implemented | YES | pins the capability variant AND that the seam refuses typed (a fake success or a wrong variant fails it). |
| probe_reports_every_feature_row_exactly_once | YES | exact 3-feature sorted vector — an extra/missing/renamed row fails. |
| ssdp_lane_probes_available_on_this_host | YES | Available ⇔ searcher attached; a socket-probe regression fails it (documented host assertion, honest about hermetic-CI futures). |
| volumes_service_over_the_fallback_translates_to_engine_errors | YES | stage/code/source-chain pinned (`discovery.unavailable`, stage Discovery). |
| capability_from_probe_error_collapses_the_reason_class | YES | Io probe → DependencyMissing { what: "host I/O" } exact. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "never a fake success and never a fabricated event" (volume fallback) | lib.rs:26-27 | REAL | pinned by fallback_paths tests (baseline :2415-2429 green). |
| "the std-only native path is feasible (and partly proven: real loopback UDP round-trips in tests/ssdp.rs)" | lib.rs:36-38 | REAL | ssdp integration 5/5 green incl. loopback_roundtrip (baseline :2431-2441). |
| "a dependency from here on `platform-*` would point upward (L2→L5) and is forbidden" | lib.rs:27-29 | REAL | no platform-* manifest depends on this crate (rg + awk). |
| `sd-directory-watch` not_started ×4, evidence "" | docs/feature-matrix.toml:2595-2608 | REAL (honest) | crate exists beyond the row (scaffold + tests) but no shipped capability — matrix conservative direction. |
| `sd-upnp` blocked ×4, blocked_reason cites "rupnp 3.0.0 … async-edge + niche" | docs/feature-matrix.toml:2619-2630 | SHALLOW (stale rationale) | the in-tree direction is a std-only sync SSDP searcher (ssdp.rs, probe-wired here at :123) — the row's rupnp/async rationale predates it; BROWSE (the row's actual claim) is indeed not implemented, so the blocked verdict stands. |

## 7. Dead / duplicated / unreachable code
The crate is a dependency island (header proof) — consistent with its own docs (platform-* follow-up cards own the wiring) and the matrix's not_started/blocked rows. Within the file: the duplicated detail strings (F-002); no dead functions — every accessor is exercised by the fallback_paths tests.

## 8. Dependencies & flags
Internal modules only (capability/context/device/error/events/monitor/ssdp/volume) + `media_types` via the error translation. No feature gates; no unsafe; std-only. 51 tests green in baseline (38 unit + 5 fallback + 5 ssdp + 3 monitor).

## 9. Open questions for the next agent
- A08 (monitor.rs): the monitor thread is the intended second consumer of the `volumes()` seam — F-001's scoped-lock fix should land before that wiring.
- A06 (ssdp.rs): the `SsdpSearcher::probe()` socket probe is this file's only OS-touching call — confirm it binds ephemeral loopback-safe sockets only (no multicast join at probe time).
- Wave-17/hygiene: three dependency islands now confirmed by this wave (transcode 6,527 LOC, device-discovery ~2,700 LOC, DASH/HLS/reconnect inside streaming) — a consolidated dead-code inventory would serve the owner better than per-file notes.
