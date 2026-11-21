# AUDIT: crates/platform-macos/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 210 / 17 production units (module doc, 7 `pub mod` items, struct, 5 methods, 3 seam builders); 0 in-file tests (crate tests live in tests/seam_behaviour.rs) |
| File verdict | UNREACHABLE (real, well-built adapter; zero production callers; foreign-OS path EXECUTED green, macOS path never executed anywhere) |
| Reachable from shipped entry point? | NO. `rg -n "MacOsPlatform" crates apps --glob '*.rs'` → only `crates/platform-macos/src/lib.rs` and its own `tests/seam_behaviour.rs`. Cargo.lock: `platform-macos` is a leaf — its `dependencies = [audio-output, media-types, testkit]` and no package in the lock depends on it (`grep -n "platform-macos" Cargo.lock` → single self entry, line 571). No `crates/*/Cargo.toml` or `apps/*/Cargo.toml` lists it (`rg -l "platform-(macos|linux|windows) =" crates/*/Cargo.toml apps/*/Cargo.toml` → empty). Only the test suite executes it: 16/16 seam_behaviour tests green in the no-fail-fast baseline (orchrun log lines 3690-3711); lib unittests: 0 tests (line 3684-3688). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "MacOsPlatform" crates apps --glob '*.rs'` (caller map — none); `grep -n "platform-macos" Cargo.lock` + `awk` block read (leaf crate, zero dependents); `rg -l "platform-macos" --glob 'Cargo.toml'` (only its own manifest); `sed -n '97,109p;38,45p;143,153p;155,163p' lib.rs`, `sed -n '700,722p' macos/coreaudio.rs` (quote verification; od -c confirms the `#[must_use` attribute — sed output shows the known ESC-eating `#ust_use` artifact); `rg -n "fn new|fn subscribe|AudioObjectAddPropertyListener" macos/coreaudio.rs` (subscribe is separately fallible — F-001); `sed -n '136,165p' platform-linux/src/lib.rs`, `sed -n '143,172p' platform-windows/src/lib.rs` (pattern mirrored ×3 crates); `git log --format=... -- crates/platform-macos/` (commit 2ca05541 admits first cross-check attempt failed with 8 errors); `rg -n "apple-darwin" .github/workflows/ audit/runs/baseline/` (no committed cross-target run; ci-full macOS lanes "NEVER executed on real runners" per its header); `rg -c "unsafe" src/*.rs src/macos/*.rs` (unsafe confinement claim holds); baseline orchrun log lines 3684-3711 (16/16 tests green) |

## 1. One-paragraph verdict
A clean, honest, competently built macOS adapter that nothing in the
repository ships or calls: no crate or app depends on `platform-macos`,
so `MacOsPlatform::discover` is reachable only from its own acceptance
suite — which did run, 16/16 green on this Linux host (the wrong-OS
degradation path: every seam is the typed fallback, the report says
`WrongOs`). The cfg discipline is genuinely good (unsafe confined to
`src/macos/*` — verified by grep, seam modules have zero unsafe; no
`-sys` deps — verified against Cargo.toml), and the fallback wiring is
tested from the outside. Two real defects: the capability report claims
`AudioNotifications = Available` from an enumeration success alone while
the listener registration (`AudioObjectAddPropertyListener`) is a
separate, independently fallible call that `discover()` never attempts —
a guessed success that contradicts the crate's own "never a guessed
success" contract, mirrored identically in platform-linux and
platform-windows; and the crate's central evidence claim ("FFI surface
compile-verified via `cargo check --target aarch64-apple-darwin` and
`x86_64-apple-darwin`") has zero committed evidence — no such run exists
in `audit/runs/baseline/`, the CI macOS lanes have never executed, and
every committed sandbox build cfg-excludes all 1,887 lines of
`src/macos/*`, so the only corroboration is a commit message, which is
not citable evidence.

## 2. What actually works (verified)
— Wrong-OS degradation contract — verified by executed tests:
`discover_on_a_foreign_host_reports_wrong_os_everywhere` constructs the
platform on Linux, asserts all four Feature rows report
`WrongOs { crate_os: "macos" }`, and every seam refuses with exactly
`PlatformError::NotCompiledForThisOs` (tests/seam_behaviour.rs:392-418);
ran green in the no-fail-fast baseline (orchrun 3690-3711, 16/16).
— Fallback wiring — verified by trace: each `*_seam()` foreign-OS arm
(lib.rs:155-163, 180-188, 204-209) builds the typed `Unavailable*`
carrying the same `PlatformError` the capability row derives from, so
report and seam can never disagree.
— Unsafe confinement — verified: `rg -c "unsafe"` over
`src/{lib,audio,capability,dialog,error,events,gpu}.rs` → only lib.rs:7,
which is prose; all unsafe lives under `src/macos/` as the doc claims
(lib.rs:7-8).
— Std-only claim — verified: Cargo.toml `[dependencies]` =
`audio-output`, `media-types` only; dev-dep `testkit`; no
objc2/coreaudio-sys crates, matching the "hand-declared extern C"
comment (Cargo.toml:14-20).
— Reachability honesty — the crate never claims to be wired: its own
doc positions it as the card 7.A3 adapter awaiting consumers, and the
feature-matrix rows for its lanes honestly say `not_started`
(docs/feature-matrix.toml `platform-aout-coreaudio`, macos =
"not_started", evidence = "").

## 3. Findings

### F-001 [S3]
Where: crates/platform-macos/src/lib.rs:103-107 (and the same class in
crates/platform-linux/src/lib.rs:101 and
crates/platform-windows/src/lib.rs:107)
Code:
```rust
        // The CoreAudio client provides enumeration and listeners
        // together (the listener registers against the same HAL
        // object); one verdict covers both rows.
        report.set(Feature::AudioEnumeration, audio_capability.clone());
        report.set(Feature::AudioNotifications, audio_capability);
```
What is wrong: `audio_seam()` (lib.rs:146-148) probes only
`CoreAudioDevices::new()` + `enumerate()` and returns
`Capability::Available` on `Ok(_)`. Notifications are never probed:
listener registration happens in `subscribe()` as two separate,
independently fallible `AudioObjectAddPropertyListener` calls whose
refusal maps to a typed error (coreaudio.rs:703-720:
`Err(PlatformError::CallFailed { api: "AudioObjectAddPropertyListener(devices)", .. })`).
The comment's rationale ("registers against the same HAL object") is
true of the *target object*, not of the outcome: enumeration can succeed
while listener registration fails. So the report asserts
`AudioNotifications = Available` without ever exercising the
notification path — exactly the "guessed success" the crate's own
header forbids (lib.rs:20-22: "capability negotiation is a first-class
outcome ..., never a guessed success") and capability.rs:6-8 ("computed
from live probes ..., not from compile-time assumptions alone"). The
`Capability::Available` doc's caveat ("individual calls may still fail
with typed errors") papers over this only partially — a failed probe
round would have produced an honest `Unavailable` row, and here no
probe round ran at all. The identical collapse exists in the Linux lane
(platform-linux/src/lib.rs:101, probe = enumeration round only) and the
Windows lane (platform-windows/src/lib.rs:107, probe = `enumerate()`
only) — a systematic 3-crate pattern.
Why it matters: the capability table is the product of this crate — its
entire reason to exist is honest negotiation; the notifications row is
its one unverifiable-in-advance claim. A consumer that trusts the table
to decide whether to offer hotplug UI would be told "available" on
hosts where subscription later fails. Contained at S3 only because no
shipped binary consumes this crate today; it becomes an S2 the day any
host wires `report()` into feature gating.
What correct looks like: probe the listener path during `discover()`
(register against the system object with a dummy queue, then tear
down), or report `AudioNotifications` as
`Unavailable(NotYetImplemented)` until a successful `subscribe()` has
been observed, or split the probe into two rounds and set the two rows
independently.
Confidence: HIGH — Verification: read (lib.rs + coreaudio.rs traced;
no execution possible — no macOS host, and the crate is unreachable
anyway).

### F-002 [S3]
Where: crates/platform-macos/src/lib.rs:38-45 (repeated at
src/macos/mod.rs:9-16, Cargo.toml:22-27, tests/seam_behaviour.rs:3-15)
Code:
```rust
//! **Evidence status (sandbox honesty):** this Linux sandbox has no
//! macOS SDK — the FFI surface is compile-verified via
//! `cargo check --target aarch64-apple-darwin` and
//! `x86_64-apple-darwin` (typechecked declarations, no linking, no
//! execution); runtime behavior on real Apple hardware is
//! `[VERIFY]`.
```
What is wrong: this compile-verification claim has no committed
evidence anywhere. `audit/runs/baseline/INDEX.md` (the complete run
ledger) contains no cross-target check; `rg -n "apple-darwin"
.github/workflows/ audit/runs/baseline/` → no CI lane and no run log.
The only 3-OS workflow (ci-full.yml) states in its own header that
"the windows/macos lanes have NEVER executed on real runners". Every
committed sandbox build (baseline #1, #3-#6) ran on Linux, where
`#[cfg(target_os = "macos")] pub mod macos;` (lib.rs:61-62) excludes
all 1,887 lines of `src/macos/{coreaudio,dialogs,metal,objc}.rs` from
compilation. The sole corroboration is commit `2ca05541`'s message
("cargo check -p platform-macos --target aarch64-apple-darwin and
x86_64-apple-darwin [--all-targets] all clean and warning-free") — and
commit messages are explicitly not citable evidence in this audit.
That same message concedes the earlier claim was false: "The prior
agent never ran the foreign-target checks; the cfg(windows) and
cfg(macos) modules had never been typechecked" — i.e. the tree already
carried one false compile-verified claim (cf71390a) that this commit
had to fix with 8 error corrections, which is precisely why the
current claim needs an artifact.
Why it matters: 1,887 lines of hand-rolled unsafe FFI (39 unsafe lines
in coreaudio.rs alone, plus unsafe `extern "C"` blocks and `unsafe
impl Send`) have zero verifiable compile evidence; the crate's
"sandbox honesty" section is its trust anchor and its strongest stated
fact is unfalsifiable from the repository. This is the exact
evidence-string pattern the xtask feature-audit flags (row-23 class:
evidence citing unverifiable runs).
What correct looks like: a committed run log
(`cargo check -p platform-macos --target aarch64-apple-darwin
--all-targets` + `x86_64-apple-darwin`) under `audit/runs/` or a docs/
evidence file with output, cited by the crate doc; until then the doc
should say "compile-verification claimed, artifact not committed".
Confidence: HIGH — Verification: read + exhaustive search of
audit/runs/baseline/, .github/workflows/, docs/ (worklog archive has
no record either). REQUEST-ORCH-RUN filed for reproduction.

### F-003 [S4]
Where: crates/platform-macos/src/lib.rs:155-163 (twins at 180-188, 204-209)
Code:
```rust
    #[cfg(not(target_os = "macos"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "macos" };
        let capability = Capability::from_probe_error(&reason);
        (
            Box::new(UnavailableAudioDevices::because(reason)),
            capability,
        )
    }
```
What is wrong: the identical 9-line foreign-OS fallback arm is pasted
three times in this crate (audio/gpu/dialog) and the same trio exists
in platform-linux and platform-windows — 9 copies of one trivial
construction across the workspace. The crate doc pre-justifies the
mirroring (lib.rs:47-50, `[PROPOSAL]` unify into a shared L5 crate
later), so this is recorded as hygiene, not a defect.
Why it matters: nine copies means nine places to update when the
fallback vocabulary changes (the 2ca05541 fix had to touch the
`PlatformError` import gating per crate — "macos/lib: PlatformError
import gated likewise" — demonstrating the maintenance cost).
What correct looks like: a generic
`fn foreign_os_fallback<T: SeamTrait>(crate_os: &'static str) -> ...`
per crate, or the Lead's proposed shared L5 vocabulary crate.
Confidence: HIGH — Verification: read (all three crates' arms compared).

### F-004 [S4]
Where: crates/platform-macos/src/lib.rs:97
Code:
```rust
    #[must_use = "probing has side effects on OS handles; reuse the value"]
    pub fn discover() -> MacOsPlatform {
```
What is wrong: on a non-macOS host (the only host this crate has ever
run on) `discover()` constructs three fallbacks and a report — pure,
side-effect-free; the "side effects on OS handles" text is true only
of the macOS arm. The `must_use` rationale text overstates for the
build that all committed evidence exercised.
Why it matters: trivial, but this crate's brand is precision about
what is probed and what is not; a must_use message that asserts
side effects that don't exist on 2 of 3 cfg paths is noise in that
signal.
What correct looks like: "discover() probes the host; call once and
reuse" — true on every path.
Confidence: HIGH — Verification: read (both cfg arms traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| module doc | 1-50 | REAL (mostly) | S3 | F-002: the compile-verified claim is unproven; the rest (seam design, [VERIFY] runtime labeling, [PROPOSAL] mirror note) matches code. |
| `pub mod audio..gpu` (7 decls) | 52-57 | REAL | — | Plain re-exports of seam modules. |
| `pub mod macos` (cfg-gated) | 59-62 | REAL-UNPROVEN | S3 | Never compiled in any committed run (F-002); cfg discipline itself correct. |
| `use` block | 64-72 | REAL | — | `PlatformError` import correctly gated to foreign-OS arms (the 2ca05541 fix). |
| `MacOsPlatform` struct | 82-87 | REAL | — | 3 `Box<dyn>` seams + report; no OS types leak (ARCH-6 holds). |
| `MacOsPlatform::discover` | 97-116 | REAL (foreign-OS arm EXECUTED) | S3 | F-001 notifications collapse; F-004 must_use text. No panics, no unwraps. macOS arm never executed anywhere. |
| `MacOsPlatform::report` | 118-122 | REAL, EXECUTED (via tests) | — | Pure accessor. |
| `MacOsPlatform::audio` | 124-127 | REAL, EXECUTED | — | `&mut *self.audio` — correct dyn reborrow. |
| `MacOsPlatform::gpu` | 129-132 | REAL, EXECUTED | — | Same. |
| `MacOsPlatform::dialogs` | 134-137 | REAL, EXECUTED | — | Same. |
| `audio_seam` | 143-164 | REAL (macOS arm UNPROVEN) | S3 | F-001: probe = enumerate() only; foreign arm EXECUTED. |
| `gpu_seam` | 168-189 | REAL (macOS arm UNPROVEN) | — | Same shape; Metal round via `MetalDevices::new()` + `enumerate()`. |
| `dialog_seam` | 193-210 | REAL (macOS arm UNPROVEN) | — | Same shape; `AppKitPanels::probe()` class-lookup probe. |

## 5. Test honesty (tests/seam_behaviour.rs — the crate's only tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `discover_on_a_foreign_host_reports_wrong_os_everywhere` (392) | YES | Constructs via the public entry, asserts all 4 rows WrongOs AND all 3 seams refuse with the exact typed reason — catches mis-wired cfg arms, wrong reason propagation. EXECUTED green (orchrun 3706). |
| `unavailable_audio_devices_returns_the_typed_reason` (35) | YES | All 3 seam methods + reason() round-trip. EXECUTED green. |
| `unavailable_gpu_adapters_returns_the_typed_reason` (49) | YES | enumerate refuses with the reason. EXECUTED green. |
| `unavailable_dialogs_returns_the_typed_reason` (62) | YES | open_file + save_file refuse. EXECUTED green. |
| `typed_fallbacks_satisfy_the_seam_traits` (80) | YES | `dyn` upcast per seam (the wiring shape discover produces); also pins FilePick vocabulary distinctness. EXECUTED green. |
| `capability_classifies_and_converts_probe_errors` (104) | YES | Pins the error→reason mapping incl. service/call-failure classes. EXECUTED green. |
| `feature_keys_are_the_stable_matrix_vocabulary` (145) | YES | Dotted keys pinned to matrix vocabulary strings. EXECUTED green. |
| `platform_report_is_a_table_with_honest_defaults` (159) | YES | Unprobed → NotYetImplemented default; set() replaces not appends. EXECUTED green. |
| `event_queue_*` (192, 225, 240, 253) | YES | FIFO drain, bounded-drop accounting (pushed = len + dropped), capacity clamp to 1, 4-thread × 100-push concurrency with exact accounting. EXECUTED green. (Queue implementation audited in events.rs file.) |
| `stable_device_id_is_deterministic_and_pinned` (290) | YES | FNV-1a-64 vectors pinned to exact constants. EXECUTED green. |
| `gpu_adapter_info_answers_the_metal_shape` (308) | YES | vendor 0 → "unknown", PCI table classes. EXECUTED green. |
| `platform_errors_render_their_details` (351) | YES | Display strings pinned for 4 error kinds. EXECUTED green. |
| `file_dialog_request_carries_the_os_neutral_shape` (423) | YES | Default shape + filter construction. EXECUTED green. |
Coverage gap (inherent): every test exercises the foreign-OS/fallback
path only; the three real macOS integrations have zero runtime
coverage anywhere and the report's notifications row (F-001) has no
test that could catch it (no test subscribes through discover()'s
verdict on a real integration — impossible without Apple hardware,
which is exactly why the capability inference is the wrong direction).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the FFI surface is compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | lib.rs:38-42, macos/mod.rs:12-14, Cargo.toml:22-24 | REAL-UNPROVEN | No committed artifact (F-002); commit 2ca05541 message claims it; commit messages are not evidence. |
| "the only `unsafe` in this crate lives in the FFI modules, every block safety-commented" | lib.rs:7-8 | REAL | `rg -c "unsafe"` across src/*.rs → lib.rs:7 is prose only; SAFETY comments ≈ unsafe blocks in macos/* (80 SAFETY lines / 69 unsafe blocks). |
| "capability negotiation is a first-class outcome ..., never a guessed success" | lib.rs:20-22 | FALSE for the notifications row | F-001: Available asserted from enumeration success alone. |
| "runtime behavior on real Apple hardware is [VERIFY]" | lib.rs:42-43 | REAL (honest label) | No macOS execution exists anywhere in the evidence corpus. |
| "macOS — CoreAudio devices, Metal layer glue, NSPanel dialogs, compile-only in this sandbox" | commit cf71390a / card 7.A3 (docs/plans/phase7-cards.md:61-70) | REAL-UNPROVEN | Acceptance said "device-enum tests green on each OS runner" — never ran on macOS (ci-full header); the Linux-hosted seam tests are the only green runs. |
| matrix `platform-aout-coreaudio` macos = "not_started", evidence "" | docs/feature-matrix.toml:2379-2388 | REAL (honest) | This crate provides enumeration only, not the AudioUnit output — no over-claim in the matrix. |

## 7. Dead / duplicated / unreachable code
- The entire crate is unreachable from shipped entry points (Field
  row): leaf in Cargo.lock, zero dependents. Its consumers are future
  cards (ADR-005 audio backend, ADR-003 wgpu renderer, desktop-tauri
  dialogs) — none admit it today.
- F-003: fallback-arm triplication (×3 lanes ×3 crates).
- No dead functions found in this file; every item is either executed
  by the test suite or is the cfg-macos arm awaiting a macOS host.

## 8. Dependencies & flags
- Deps: `audio-output` (DeviceDesc/DeviceId seam vocabulary),
  `media-types` (AudioFormat), dev-dep `testkit` — matches
  boundaries.toml `[crates.platform-macos]` (layer 5,
  internal_deps audio-output + media-types, dev testkit,
  external_allowlist = []) — verified against boundaries.toml:482-487.
  Std-only claim holds (no external crates). No feature flags. The
  only cfg switch is `target_os = "macos"` — the crate's whole design.
- `edition = "2024"` — consistent with workspace.

## 9. Open questions for the next agent
- F-001 pattern owners: platform-linux (W03 wave owns platform-*)
  and platform-windows auditors should cross-reference — the
  notifications-collapse is a 3-crate systematic defect; whoever
  audits platform-linux/src/lib.rs should not re-derive it from
  scratch.
- The macos/{coreaudio,metal,dialogs,objc}.rs files (1,887 lines of
  unsafe FFI) are NOT in my assignment list and have never been
  audited line-by-line against their SAFETY invariants; the
  compile-verified question (F-002) makes them the highest-risk
  unaudited surface in this crate — recommend a dedicated file audit
  wave with an orchestrator-run cross-check first.
- REQUEST-ORCH-RUN: cargo check -p platform-macos --target aarch64-apple-darwin --all-targets AND --target x86_64-apple-darwin --all-targets (reproduce the F-002 claim; a failure falsifies the crate doc, a pass with committed log converts REAL-UNPROVEN to REAL)
