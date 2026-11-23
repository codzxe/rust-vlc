# AUDIT: crates/device-discovery/src/volume.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 382 / 18 (12 production fns/impls, 6 tests) |
| File verdict | UNREACHABLE (real, honestly-typed port + fallback; nothing constructs it outside its own tests) |
| Reachable from shipped entry point? | NO — `impl VolumeSource` exists only here (UnavailableVolumes, :199) and in monitor.rs's test Scripted (:295); no `platform-*` crate implements the seam; `VolumesService::new` has zero non-test callers (rg across crates/ and apps/). The catalog's `devices.volume-added`/`-removed` rows therefore have no producer in the shipped binary. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |

## 1. One-paragraph verdict
A cleanly minted port with an honest typed fallback: `VolumeId`/`VolumeKind` (wire spellings tested verbatim), `Volume::to_device` delegating URI construction to the shared `SourceUri::from_file_path` (with tests pinning the Windows drive-root fix and the UNC refusal — including a comment that names the retired FALSE pin it replaced), and `VolumesService` translating one bounded pass into `Device`s. The crate's own docs say the platform adapters that would implement `VolumeSource` have not landed, and the code agrees: the only implementation is `UnavailableVolumes`, and nothing outside the crate's tests constructs the service. Defects are minor: the device-count cap is checked after the full allocation, one bad volume fails the whole pass, and the service's success path is untested.

## 2. What actually works (verified)
— `cargo test -p device-discovery --lib volume` → `9 passed; 0 failed` (6 in this file + 3 monitor tests matching the filter). Wire names, percent-encoding (`file:///media/My%20Volume`, `file:///media/%C3%9Csb`, `file:///E:/`), UNC typed refusal, fallback-typed-errors-only, and the service's error translation (`discovery.unavailable`, stage Discovery) are all exact-match assertions.

## 3. Findings
F-001 [S3]
Where: crates/device-discovery/src/volume.rs:199-203 (and crate lib.rs:22-27)
Code:
```
impl VolumeSource for UnavailableVolumes {
    fn enumerate(&mut self, _ctx: &DiscoveryContext) -> Result<Vec<Volume>, DiscoveryError> {
        Err(self.reason.clone())
    }
}
```
What is wrong: this is the ONLY `VolumeSource` implementation in the tree; no platform crate implements the seam and no production code constructs `VolumesService`. The local-volume discovery feature does not exist beyond the port — the shipped binary has no volume enumeration and no `devices.volume-added` producer. The module doc states this plainly (:12-16), so this is an honestly-annotated gap, not a fabricated claim — but the catalog rows and UI surface it feeds remain dead. What correct looks like: exactly what the doc says — the platform cards landing their adapters; until then this row's honest status is "not shipped".
Confidence: HIGH — Verification: rg `impl VolumeSource|VolumesService::new` across crates/ + apps/ (test-only hits).
F-002 [S4]
Where: crates/device-discovery/src/volume.rs:247-254
Code:
```
        if devices.len() > ctx.max_devices {
            return Err(DiscoveryError::CapacityExceeded {
```
What is wrong: the capacity check runs AFTER the full `Vec` was built and every volume converted — a source that returns a huge list allocates and converts all of it before the cap refuses. The crate doc sells "enumerate passes are device-count clamped" (lib.rs:53-54); this is a clamp-shaped post-hoc refusal, not a bound on work. Blast radius today: nil (no real sources); a real udisks2 adapter would be trusted to bound itself. What correct looks like: take `ctx.max_devices` into the enumeration loop (stop or error at the boundary).
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/device-discovery/src/volume.rs:238-245
Code:
```
            // Per-volume conversion failure fails the pass: a source
            // that reports un-URI-able roots is violating its
            // contract, and silently dropping volumes would hide it.
```
What is wrong: one volume with a non-local root (e.g. a network mount reported by a future adapter) fails the ENTIRE enumeration — all other volumes disappear from the UI. The comment justifies it as contract enforcement; VLC's services-discovery drops the bad entry. Design choice with harsh blast radius once a real adapter lands. What correct looks like: per-volume typed error carrying the offending id, pass continues.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/device-discovery/src/volume.rs:369-381 (test)
Code:
```
        let err = match service.discover(&DiscoveryContext::new()) {
            Err(e) => e,
            Ok(devices) => unreachable!("unavailable lane must not succeed: {devices:?}"),
        };
```
What is wrong: the ONLY test of `VolumesService::discover` exercises the unavailable path; the success path (conversion loop + capacity refusal) has no test at all — it cannot, because no non-failing `VolumeSource` exists outside monitor.rs's test Scripted. Untested success path in the very code a platform adapter will plug into. What correct looks like: a Scripted source here too (the pattern exists 100 lines away in monitor.rs).
Confidence: HIGH — Verification: read + test inventory.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VolumeId::new / as_str / Display | 46-62 | REAL | — | opaque id, log-safe |
| VolumeKind::as_str / Display | 82-96 | REAL | — | catalog wire spelling, tested |
| Volume::to_device | 136-148 | REAL | — | shared constructor, UNC refusal typed |
| VolumeSource::enumerate (trait) | 164-173 | REAL | — | port minted; polling posture documented |
| UnavailableVolumes::because / reason | 188-196 | REAL | — | |
| UnavailableVolumes::enumerate | 200-202 | REAL | S3 | F-001: only impl in tree |
| VolumesService::new | 221-225 | REAL | — | |
| VolumesService::discover | 229-256 | REAL | S4 | F-002 late cap; F-003 all-or-nothing; F-004 success path untested |
| 6 test fns | 272-381 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| kind_wire_names_are_the_catalog_spelling | YES | 4 exact spellings |
| to_device_percent_encodes_space_bearing_roots | YES | exact URI + label + kind |
| to_device_keeps_unreserved_bytes_verbatim | YES | exact URI |
| windows_drive_roots_construct_through_the_shared_constructor | YES | `file:///E:/` + non-ASCII UTF-8 percent-encoding |
| unc_roots_are_a_typed_refusal_not_a_fake_uri | YES | MalformedResponse{protocol:"volume-root"} |
| unavailable_fallback_answers_typed_errors_only | YES | Err both calls |
| volumes_service_translates_the_whole_pass | PARTIAL | F-004: unavailable path only |
No tautologies; no `#[ignore]`.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Until those cards land, UnavailableVolumes is the only implementation" | volume.rs:12-16 | REAL | rg: only impls are here + monitor.rs test |
| "enumerate passes are device-count clamped" | lib.rs:53-54 | SHALLOW | F-002: refusal after the fact, not a clamp on work |
| "the engine host relays … devices.volume-added/-removed" (relay is host wiring) | lib.rs:10-12 | REAL (and confirms nothing relays today) | no host wiring found (rg apps/) |

## 7. Dead / duplicated / unreachable code
Whole lane unreachable from shipped entry (F-001). No dead functions; `DiscoveryContext` usage is pass-through here.

## 8. Dependencies & flags
`media_types` (SourceUri, EngineError), crate-local context/device/error. No features, no OS-specific code (correctly deferred to adapters). Layering L2-minting respected.

## 9. Open questions for the next agent
- monitor.rs (same crate, another agent's file): does `VolumeMonitor::spawn` have any non-test caller? (rg suggests no.)
- Do the `platform-disc-*` matrix rows claim volume enumeration shipped on any OS? (Check their status/evidence against F-001.)
