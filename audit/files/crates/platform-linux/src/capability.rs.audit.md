# AUDIT: crates/platform-linux/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W08-A06 |
| Date | 2026-11-23 |
| Lines / functions | 200 / 9 (Capability:2, UnavailableReason:2, Feature:1, PlatformReport:4) |
| File verdict | REAL (code) / UNREACHABLE (from shipped entry point) |
| Reachable from shipped entry point? | NO — platform-linux is an orphan crate (see xdg.rs audit §7): no workspace Cargo.toml depends on it; only callers are its own tests and `LinuxPlatform::discover` (lib.rs:96-109), itself test-only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg "platform\.(audio|gpu|dialogs)" docs/ apps/ crates/` (keys exist only in the 3 platform crates' own tests); `rg 'id = "platform' docs/feature-matrix.toml` (no matching matrix keys); host probes: `ls /sys/class/drm` (only `version`), `ls /dev/dri` (absent), `ldconfig -p | rg libpulse` (libpulse.so.0 present), `ps aux | rg pulse` (no daemon), `ls /run/user/1001/pulse /run/pulse` (absent), `command -v zenity kdialog` (absent); prebuilt test runs (see below) |

## 1. One-paragraph verdict
The capability vocabulary itself is real and well-built: typed reasons, live-probe-derived verdicts, unprobed features default to a typed `NotYetImplemented`, never a guessed `Available`. On THIS sandbox the assembled report is honest for audio and dialogs and debatable for GPU: audio = Unavailable(ServiceNotReachable "PulseAudio server") because libpulse.so.0 loads but no daemon exists; dialogs = Unavailable(ServiceNotReachable, no zenity/kdialog/portal); GPU = **Available with zero adapters**, because the DRM scanner treats an empty sysfs as a successful enumeration (documented at lib.rs:172-174) — "the enumeration feature works" is true, but any consumer reading only the capability bit will believe this GPU-less container has GPU support. The one outright false claim in this file is the doc on `Feature::key()` — "feature-matrix vocabulary": the dotted keys exist nowhere in docs/feature-matrix.toml (which uses `platform-aout-*`/`platform-device-*` kebab ids); the keys live only inside the platform crates, and the test that claims to pin "the stable matrix vocabulary" merely asserts the constants equal themselves.

## 2. What actually works (verified)
- Host-state probes (commands above): no DRM cards, no /dev/dri, libpulse.so.0 present (0.24.3), no pulse daemon, no zenity/kdialog, DBUS unset — so the honest report on this host is exactly: audio rows ServiceNotReachable, GPU row Available-with-empty-list, dialogs row ServiceNotReachable.
- Prebuilt runs: `linux_integrations-ca1702194c57d626` (13/13) exercises report-vs-seam consistency (`discover_assembles_this_hosts_report`); `seam_behaviour` 15/15 includes `capability_classifies_and_converts_probe_errors` and `platform_report_is_a_table_with_honest_defaults` (baseline cargo-test-workspace.txt:3983-3998).

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/src/capability.rs:142-152
Code:
```rust
impl Feature {
    /// The stable dotted key (feature-matrix vocabulary).
    #[must_use = "pure accessor"]
    pub fn key(self) -> &'static str {
        match self {
            Feature::AudioEnumeration => "platform.audio.enumeration",
```
What is wrong: The doc claims these dotted keys are "feature-matrix vocabulary". They are not: `rg` over docs/feature-matrix.toml shows zero occurrences of `platform.audio.enumeration`, `platform.audio.notifications`, `platform.gpu.enumeration`, or `platform.dialogs`; matrix platform rows use kebab ids (`platform-aout-cpal`, `platform-device-capture`, ...). The keys appear ONLY in the three platform crates' own tests, and the test named `feature_keys_are_the_stable_matrix_vocabulary` (tests/seam_behaviour.rs:192-204) asserts each key equals the literal it returns — a self-referential pin that proves no matrix linkage. Why it matters: a "stable vocabulary" claim with no consumer is how future code believes it is wired into the matrix when it is not. What correct looks like: either add these keys to the matrix rows' evidence/notes, or delete the claim. Confidence: HIGH — Verification: ran rg over docs/ + all crates.
F-002 [S3]
Where: crates/platform-linux/src/lib.rs:175-185 (the consumer of this file's verdicts; filed here because the hostile question is about reported capabilities)
Code:
```rust
fn gpu_seam() -> (Box<dyn GpuAdapters>, Capability) {
    #[cfg(target_os = "linux")]
    {
        let mut scanner = linux::drm::DrmAdapterScanner::new();
        match scanner.enumerate() {
            Ok(_) => (Box::new(scanner), Capability::Available),
```
What is wrong: On this GPU-less sandbox (`/sys/class/drm` contains only `version`, `/dev/dri` absent) `enumerate()` returns Ok(empty) and GpuEnumeration reports **Available** with an empty adapter list. The doc (lib.rs:172-174: "a kernel without DRM enumerates as an honest empty list") makes this a deliberate, documented choice — "Available" means the enumeration feature works, not that hardware exists — but the capability bit alone reads as "GPU support present" on a machine with no GPU. Why it matters: any UI or fallback-chain consumer (the 7.C1 input) that branches on `Capability::is_available()` without counting adapters will take the GPU path on GPU-less containers. What correct looks like: either a distinct verdict (e.g. AvailableWithNoDevices) or the doc of `Capability::Available` explicitly warning consumers to check the enumerated list. Confidence: HIGH — Verification: host facts verified by commands above; the code path is executed every `discover()` (test run).
F-003 [S4]
Where: crates/platform-linux/src/capability.rs:101-104
Code:
```rust
            other => UnavailableReason::ServiceNotReachable {
                service: "platform integration",
                detail: other.to_string(),
            },
```
What is wrong: The catch-all maps every non-probe error class (CallFailed, SpawnFailed, Timeout) to `ServiceNotReachable` with the generic service "platform integration". A probe that crashed (CallFailed) would be reported as "not reachable" — a semantic lie, though in practice this crate's seams only feed probe-shaped errors (NotCompiledForThisOs/LibraryUnavailable/ServiceUnavailable/SymbolMissing) into `from_probe_error`. Confidence: MEDIUM — Verification: read; all current call sites checked (lib.rs:150,156,182,220 + windows analogues).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Capability::is_available | 24-29 | REAL | — | Trivial, correct |
| Capability::from_probe_error | 31-36 | REAL | — | Delegates to reason converter |
| UnavailableReason::from_error | 73-107 | REAL | S4 | Catch-all overmaps error classes (F-003) |
| UnavailableReason::Display | 109-127 | REAL | — | Rendered strings tested (`unavailable_reasons_render_honest_strings`) |
| Feature (enum) + key | 129-153 | REAL (code) / FALSE (doc claim) | S3 | Vocabulary claim untrue (F-001) |
| PlatformReport::new | 162-167 | REAL | — | Default empty |
| PlatformReport::set | 169-180 | REAL | — | Replace-in-place; linear scan over ≤4 rows |
| PlatformReport::get | 182-194 | REAL | — | Unprobed → typed NotYetImplemented; tested |
| PlatformReport::iter | 196-200 | REAL | — | Borrowed iteration |

## 5. Test honesty
Covered by the executed seam_behaviour + linux_integrations runs: `capability_classifies_and_converts_probe_errors` genuinely pins the error→reason mapping; `platform_report_is_a_table_with_honest_defaults` pins the unprobed default; `discover_assembles_this_hosts_report` pins report/seam consistency on this host. `feature_keys_are_the_stable_matrix_vocabulary` would NOT fail if the keys drifted from any external vocabulary — it compares the function to its own literals (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The report is computed from live probes (library presence, service reachability), not from compile-time assumptions alone" | capability.rs:6-8 | REAL | lib.rs audio_seam runs a full enumeration round (144-153); dialog_seam probes PATH + bus (203-231); gpu_seam scans sysfs (175-185) |
| "unprobed features are honestly `NotYetImplemented` ... never assumed available" | capability.rs:182-183 | REAL | get() lines 185-194; executed test `platform_report_is_a_table_with_honest_defaults` |
| "The stable dotted key (feature-matrix vocabulary)" | capability.rs:143 | FALSE | rg: keys absent from docs/feature-matrix.toml; present only in the 3 crates' own tests (F-001) |

## 7. Dead / duplicated / unreachable code
- Orphan crate: entire file unreachable from shipped entry points (see xdg.rs audit).
- `Feature::key()` has no non-test caller: rg shows key() invoked only in the three seam_behaviour test files. Dead in production even within the crate.

## 8. Dependencies & flags
- Depends only on crate::error (PlatformError). No std feature gates. Mirrored near-verbatim in platform-windows/src/capability.rs and platform-macos (documented duplication, cross-ref platform-windows lib.rs audit F-003).

## 9. Open questions for the next agent
- If the Lead unifies the platform crates (the [PROPOSAL] in all three lib.rs docs), does `Feature::key()` get real matrix consumers, or is it deleted? Currently it is vocabulary for a matrix that does not reference it.
