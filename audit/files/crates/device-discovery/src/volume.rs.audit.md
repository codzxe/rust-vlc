# AUDIT: crates/device-discovery/src/volume.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-11-21 |
| Lines / functions | 382 / 12 production items + 7 tests |
| File verdict | REAL (honest typed-fallback seam, correct URI construction, real tests) — UNREACHABLE: the crate is not linked into ANY shipped binary |
| Reachable from shipped entry point? | NO — stronger than the transcode/playlists cases: device-discovery is not even compiled into a shipped binary. Proof: only platform-{linux,macos,windows} declare it (grep all Cargo.tomls); no app depends on any platform-* crate (apps/cli deps read: no platform edge; apps/desktop deps = commands, lumen-cli, media-types, renderer-wgpu, video-engine; commands = serde only; desktop-tauri = desktop + commands); `rg -l device_discovery apps/ crates/commands/ crates/playback-engine/` → zero. Workspace CI builds/tests it; no user executable contains it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | read 1-382 in full · grep dependents of device-discovery + platform-* across all Cargo.tomls (dead chain established) · rg VolumeEvent/VolumeMonitor (events.rs/monitor.rs own them — volume.rs's doc link broken, F-003) · sed -n 22,27p / 205,213p / 237,246p (quote verification) · rg DeviceKind (device.rs:59-65 — `Volume(VolumeKind)` documents network mounts as a volume kind, F-002 evidence) · baseline #3 (all 7 tests pass) |

## 1. One-paragraph verdict
An honestly-declared seam module: the only implementation of
`VolumeSource` is the typed fallback `UnavailableVolumes`, which never
fakes a volume or an event, and the module docs say exactly that. The
URI-construction path (`to_device` via the shared
`SourceUri::from_file_path`) is correct and pinned by tests that encode
the wave-30 Windows fix — including a test comment that openly documents
the RETIRED test which had asserted a FALSE round-trip belief (good
honesty artifact). Two design-level defects: the vocabulary promises
network mounts as first-class volumes (`VolumeKind::Network`,
`DeviceKind::Volume` docs) while a UNC-rooted network volume not only
fails `to_device` but fails the ENTIRE enumeration pass — one network
volume poisons every local volume in the list; and two comment defects
(a broken intra-doc link, a false lock-scope claim). None of it is
linked into any shipped binary: the platform adapters that would
implement the seam have no app consumer either.

## 2. What actually works (verified)
— Typed fallback honesty: `UnavailableVolumes::enumerate` returns the
constructed reason verbatim (199-203); the test asserts repeated calls
stay typed errors, never fake success.
— `to_device` URI construction: space-bearing root →
`file:///media/My%20Volume` (289-293), unreserved bytes verbatim (305),
Windows `E:\` → `file:///E:/` (322 — the wave-30 fix), non-ASCII →
UTF-8 percent-encoding (328), UNC root → typed MalformedResponse with
protocol "volume-root" (338-348). All pinned by tests that would fail
on any encoder regression.
— Wire vocabulary: `VolumeKind::as_str` matches the catalog's kebab-case
spellings exactly (273-278).
— Poison recovery in `discover` (230-233) mirrors the crate's queue
discipline.
— Capacity check against `ctx.max_devices` (247-254) — after-the-fact
(the source's own pass bounds are its contract), typed
CapacityExceeded.
— All 7 tests pass (baseline #3); each is a real behavioral pin (table
below).

## 3. Findings

### F-001 [S3]
Where: crates/device-discovery/src/volume.rs:64-77 (VolumeKind::Network) + 126-129 (UNC refusal rationale) + 238-246 (whole-pass failure), device.rs:59-62
Code:
```rust
        for volume in &volumes {
            // Per-volume conversion failure fails the pass: a source
            // that reports un-URI-able roots is violating its
            // contract, and silently dropping volumes would hide it.
            let device = volume
                .to_device()
                .map_err(DiscoveryError::into_engine_error)?;
```
and the model that promises otherwise (device.rs:59-62):
```rust
pub enum DeviceKind {
    /// A local volume (fixed disk, removable medium, optical disc,
    /// network mount).
    Volume(VolumeKind),
```
What is wrong: The vocabulary treats network mounts as first-class
volumes (`VolumeKind::Network`; `DeviceKind::Volume`'s doc lists
"network mount"), but `to_device` refuses UNC roots
(`\\server\share` — the natural Windows spelling of exactly such a
mount) as "not a local-file source" (126-129), and `discover` fails the
WHOLE pass on that single refusal. One network volume hides every local
volume in the enumeration. Worse, the refusal principle is form-based,
not network-based: a drive-letter-mapped network root (`Z:\` → SMB)
constructs `file:///Z:/` without complaint — so "a network mount is not
a local-file source" is true only for UNC spellings, and a realistic
Windows enumeration containing one unmapped network share returns
zero devices with a MalformedResponse instead of the local disks.
Why it matters: When the platform cards land, this is the difference
between "browse my files" working and failing on machines with any
network mount; the all-or-nothing choice (documented, with its
reasoning, at 239-241) maximizes the blast radius of one integration
oddity.
What correct looks like: Either exclude network volumes from the
Device pass explicitly (filter by kind, warn per-volume), or convert
per-volume failures into per-volume warnings with the pass surviving —
and align the doc rationale with the actual form-based rule.
Confidence: HIGH — Verification: read (all three code sites + the
cross-crate DeviceKind doc verified by sed/rg).

### F-002 [S3]
Where: crates/device-discovery/src/volume.rs:1-16 (the whole module), provenance crates/device-discovery/Cargo.toml
Code:
```rust
//! Until those cards land,
//! [`UnavailableVolumes`] is the only implementation — the typed
//! fallback whose every method names the exact reason. It never
//! fabricates a volume and never fabricates an event; the UI's honest
//! empty state is the truth.
```
What is wrong: Nothing in the code — but the honest empty state is also
the only state any user could ever see, because no shipped binary links
this crate: its dependents are the three platform-* crates, which no
app depends on (proof in the header). The seam, the fallback, the
service wrapper, the monitor (another file) — all workspace-test-only.
Why it matters: Same class as the transcode/playlists deadness
(W01-A03 F-002; my earlier F-002s) but total: those crates at least
ride into `lumen` via streaming; this one is not in any shipped
dependency tree at all. Any doc claiming device discovery / volume
browsing works is PAPER-ONLY for users.
What correct looks like: The platform-adapter cards (7.D1's follow-ups)
wired into an app, or the layer recorded as not-yet-hosted.
Confidence: HIGH — Verification: read (dependency chain grepped to
every app root).

### F-003 [S4]
Where: crates/device-discovery/src/volume.rs:24
Code:
```rust
//! [`VolumeEvent`] maps 1:1 onto those payloads — `VolumeKind::as_str`
```
What is wrong: `VolumeEvent` is not in scope in this module (imports at
29-35: fmt, media_types, crate::context/device/error only; the type
lives in `crate::events`). The intra-doc link is broken — rustdoc
renders it as plain text and emits a broken_intra_doc_links warning.
Why it matters: Doc navigation rot; the link is the module's one pointer
from the wire-mapping contract to the event type that fulfills it.
What correct looks like: `[`crate::events::VolumeEvent`]` or a use.
Confidence: HIGH — Verification: read (imports + rg of the type's true
home).

### F-004 [S4]
Where: crates/device-discovery/src/volume.rs:205-212 (comment) vs 229-236 (the code)
Code:
```rust
/// Wraps the source in a mutex so the trait's `&self` discovery
/// method can share a `&mut self` seam with the monitor thread (a
/// host wires exactly one consumer in practice; the lock keeps the
/// seam sound if they don't, and no method holds it across a call
/// out).
```
What is wrong: `discover` locks the source mutex (230-233) and holds it
across `source.enumerate(ctx)` (234-236) — a call out into the boxed
integration's code. A slow/blocking OS integration therefore blocks the
monitor thread's poll on the same mutex; the comment's claim is false
as written.
Why it matters: Lock-scope comments are load-bearing for the next
maintainer; someone trusting "no method holds it across a call out"
will add a second lock-user and be surprised by the contention/deadlock
shape the comment promised cannot happen.
What correct looks like: Delete the clause, or restructure to collect
under the lock and call out without it (not actually necessary while
the only implementation is the non-blocking fallback — but then say
that).
Confidence: HIGH — Verification: read (code paths compared).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-27 | REAL | S4 | Honest about fallback-only status; F-003 broken link. |
| `VolumeId` (+new/as_str/Display) | 40-62 | REAL | — | Opaque id; "carries no user path by construction" — the platform integration's minting discipline, not enforced here (trust boundary documented). |
| `VolumeKind` (+as_str/Display) | 67-96 | REAL | — | Wire spellings pinned. |
| `Volume` struct | 99-110 | REAL | — | label documented untrusted (V7). |
| `Volume::to_device` | 136-148 | REAL | S3 | Correct shared-constructor use; F-001's refusal shape (UNC typed refusal) + rationale conflation. |
| `VolumeSource` trait | 164-173 | REAL | — | Polling posture documented; `&mut self` seam. |
| `UnavailableVolumes` (+because/reason/enumerate) | 180-203 | REAL | — | The honest typed fallback — never fakes success (tested). |
| `VolumesService` struct + new | 213-226 | REAL | — | Mutex-wrapped seam. |
| `VolumesService::discover` | 228-257 | REAL | S3/S4 | Poison recovery; per-volume failure fails pass (F-001); F-004 lock-scope comment; capacity check after the fact (typed). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| kind_wire_names_are_the_catalog_spelling (272) | YES | Exact four-way equality. |
| to_device_percent_encodes_space_bearing_roots (280) | YES | raw() URI + label + kind asserted. |
| to_device_keeps_unreserved_bytes_verbatim (298) | YES | Verbatim path pinned. |
| windows_drive_roots_construct_through_the_shared_constructor (308) | YES | E:\ → file:///E:/ and non-ASCII → %C3%9C pinned; the comment documents the RETIRED false test (honesty artifact). |
| unc_roots_are_a_typed_refusal_not_a_fake_uri (331) | YES | Typed MalformedResponse + protocol asserted. |
| unavailable_fallback_answers_typed_errors_only (351) | YES | Never-fake-success pinned, incl. repeatability. |
| volumes_service_translates_the_whole_pass (368) | YES | Stage + dotted code asserted through the engine bridge. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "UnavailableVolumes is the only implementation … never fabricates" | volume.rs:13-16 | REAL | Only impl in crate (rg); fallback tested. |
| "the engine-side relay … is a mechanical field copy, never a re-derivation" | volume.rs:24-27 | REAL-UNPROVEN | No relay exists yet (crate unlinked); the mapping claim is about future wiring. |
| "a network mount is not a local-file source" | volume.rs:126-129 | FALSE (form-based, not network-based) | F-001: drive-letter network roots construct fine. |
| "no method holds it across a call out" | volume.rs:209-212 | FALSE | F-004: discover holds the lock across enumerate. |
| Wire vocabulary fixed/removable/optical/network | doc 05 §6.2 via volume.rs:20-23 | REAL | as_str pinned by test. |

## 7. Dead / duplicated / unreachable code
The entire crate is outside every shipped dependency tree (F-002) —
deader than transcode/playlists (which at least link into lumen via
streaming). No internal dead code; all items used by monitor.rs/events
or tests.

## 8. Dependencies & flags
media-types (EngineError, SourceUri) + crate-internal
(context/device/error). No unsafe, no feature gates, no I/O. std Mutex
poison-recovery consistent with the crate's queue discipline.

## 9. Open questions for the next agent
- monitor.rs/events.rs auditors: `diff_volumes` is the event minting —
  confirm an id-based diff cannot flap when a volume's label changes
  (same id, new label → which event?), and whether VolumeEvent's wire
  mapping matches this file's as_str spellings.
- When the platform cards land: F-001's all-or-nothing pass decision
  needs a revisit against a real Windows enumeration with a mix of
  local + UNC network volumes.
