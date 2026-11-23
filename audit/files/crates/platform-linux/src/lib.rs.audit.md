# AUDIT: crates/platform-linux/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W08-A07 |
| Date | 2026-11-23 |
| Lines / functions | 232 / 8 (discover, report, audio, gpu, dialogs + audio_seam, gpu_seam, dialog_seam) |
| File verdict | REAL (executed on this host via linux_integrations tests) — but the crate it fronts is UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — zero dependents of `platform-linux` in the workspace (rg over all Cargo.tomls) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `cargo test -p platform-linux --test linux_integrations` (exit 0, 13 passed); `cargo test -p platform-linux --test seam_behaviour` (exit 0, 15 passed); rg manifest sweep; python hex quote verification |

## 1. One-paragraph verdict
lib.rs is the honest assembly point the crate docs promise: `discover()` runs
live probes (dlopen'd PulseAudio with a full enumeration round, DRM sysfs scan,
PATH search for zenity/kdialog) and wires either the real integration or the
typed fallback carrying the exact reason; the four report rows land verbatim.
The wiring is genuinely executed and cross-checked on this host by
`discover_assembles_this_hosts_report` (13/13 integration tests pass). The two
defects: the whole crate is an orphan — no shipped entry point depends on it,
so `LinuxPlatform::discover` is a public API with zero callers outside its own
tests — and `dialog_seam` contains a compile-time-dead branch guarded by a
`const fn` returning `false`.

## 2. What actually works (verified)
— `cargo test -p platform-linux --test linux_integrations` -> 13 passed
(incl. `discover_assembles_this_hosts_report`, `pulse_probe_reports_this_hosts_truth`),
exit 0.
— The doc claim "Available means enumeration works, not merely that the
library is loadable" matches audio_seam's code (144-153): probe() then a real
enumerate() round before `Capability::Available`.
— Dead-branch guard verified byte-level: `portal_filechooser_implemented` is
`pub const fn ... -> bool { false }` (linux/xdg.rs:52-54).

## 3. Findings
F-001 [S2]
Where: crates/platform-linux/src/lib.rs:93-110 (LinuxPlatform::discover)
Code:
```
    pub fn discover() -> LinuxPlatform {
        let mut report = PlatformReport::new();
        let (audio, audio_capability) = audio_seam();
        let (gpu, gpu_capability) = gpu_seam();
        let (dialogs, dialogs_capability) = dialog_seam();
```
What is wrong: `LinuxPlatform` is the crate's only public entry point and has
no consumer. `rg "platform-linux" --glob Cargo.toml` matches only its own
manifest; apps/cli and apps/desktop dependency lists do not include it. The
release lane never builds this crate. Why it matters: 232 lines of working,
tested platform negotiation that no lumen user can reach — the "L5 platform
layer" is scaffolding without a building. What correct looks like: a
target_os-dispatching consumer (engine host or CLI) or explicit Lead-gated
deferral recorded in the matrix. Confidence: HIGH — Verification: ran (rg,
manifest reads). Confidence on the pattern being workspace-wide: HIGH
(same finding for platform-macos, W08-A07 file 2).
F-002 [S4]
Where: crates/platform-linux/src/lib.rs:210-220
Code:
```
                if linux::xdg::portal_filechooser_implemented() {
                        "portal FileChooser client is implemented; see its probe"
                    } else {
```
(indentation as in file, lines 214-216)
What is wrong: `portal_filechooser_implemented()` is a `const fn` returning
`false` (linux/xdg.rs:52-54), so the "implemented" arm is unreachable today;
the error detail always says "not implemented". The construct is a documented
single-truth switch, but as shipped it is dead code inside a `format!`.
Confidence: HIGH — Verification: read + const-evaluated (const false).
F-003 [S4]
Where: crates/platform-linux/src/lib.rs:98-101
Code:
```
        report.set(Feature::AudioEnumeration, audio_capability.clone());
        report.set(Feature::AudioNotifications, audio_capability);
```
What is wrong: enumeration and notifications are collapsed into one verdict
with the comment "one verdict covers both rows" — on Linux this is accurate
(same PulseAudio connection), but the seam traits expose `subscribe` as a
separate capability that probe() never validates: a build where enumeration
works but subscribe fails (server policy) would report Available for
notifications without ever having subscribed. Latent, low-probability.
Confidence: MEDIUM — Verification: read (probe path never calls subscribe).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| LinuxPlatform::discover | 93-110 | REAL | S2 (reachability) | executed via integration test; orphan crate |
| LinuxPlatform::report | 114-116 | REAL | — | pure accessor |
| LinuxPlatform::audio/gpu/dialogs | 119-131 | REAL | — | &mut dyn accessors, exercised by tests |
| audio_seam | 136-170 | REAL | — | full enumeration round before Available; honest |
| gpu_seam | 175-196 | REAL | — | empty-list-is-honest documented and held |
| dialog_seam | 203-232 | REAL | S4 | F-002 dead branch; F-003 collapsed verdict |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| discover_assembles_this_hosts_report (linux_integrations.rs:452-505) | YES | report/seam consistency asserted both directions; unexpected reasons rejected |
| Host-dependent hazard (not my file, noted): open_file in the discover test | N/A | on a host WITH zenity on PATH, `platform.dialogs().open_file(...)` would launch a modal zenity and block the suite — this sandbox lacks zenity so it passes; flag for the integrations-file auditor |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Available means enumeration works, not merely that the library is loadable" | lib.rs:86-88 | REAL | audio_seam:144-153 does probe+enumerate; test pulse_probe_reports_this_hosts_truth passes |
| "the absent-server path is what is runtime-tested here" | lib.rs:31-33 | REAL | 13/13 integration tests pass on this no-daemon host |
| "one probe, three seams, one honest capability report" | lib.rs:67-68 | REAL | discover_assembles_this_hosts_report drives all three |

## 7. Dead / duplicated / unreachable code
- F-002 dead branch (const-false guard).
- Whole crate unreachable from shipped entry points (F-001).
- cfg(not(target_os="linux")) arms unreachable in any Linux build lane but
  intentionally so (host-honesty design; compiled on other hosts only if the
  crate ever gains a consumer — today none exists, so those arms are dead too).

## 8. Dependencies & flags
audio-output, media-types (path deps); dev-dep testkit. No unsafe in this
file (unsafe lives in linux/ FFI modules behind module-level allow). No
feature flags. std-only crate.

## 9. Open questions for the next agent
- Which card was supposed to consume LinuxPlatform (7.C1 renderer fallback
  chain? engine host?) — if cancelled, the three platform-* crates are
  stranded architecture and the S2 should be escalated at the plan level.
- pulse.rs/drm.rs/xdg.rs are NOT in my assignment; the seam builders trust
  them — the linux_integrations tests cover them (13 pass) but a hostile
  pass over pulse_ffi.rs (raw FFI) is still owed.
