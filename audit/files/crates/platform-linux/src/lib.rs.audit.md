# AUDIT: crates/platform-linux/src/lib.rs (whole file, 232 lines)

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 232 / 7 production units |
| File verdict | UNREACHABLE (real, correct composition code; zero consumers — no shipped entry point constructs a `LinuxPlatform`) |
| Reachable from shipped entry point? | NO. `rg -l "platform_linux" --glob '!crates/platform-linux/**' .` → only a docs gate log and the baseline test log: no code outside the crate imports it; no Cargo.toml in apps/ or crates/ (besides its own) declares the dependency; apps/desktop (the self-described composition root) and apps/desktop-tauri dependency lists verified absent. The crate's own suites ARE executed: seam_behaviour + linux_integrations (13/13) + the lib target all pass in `orchrun-test-workspace-nofailfast.log.txt` (platform_linux target at 3638; suite results verified). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read tool over the whole file; `rg -l "platform_linux" --glob '!crates/platform-linux/**' .` (consumer proof); `rg -n "path =" apps/desktop/Cargo.toml` + `rg -n "platform" apps/desktop-tauri/Cargo.toml` (composition-root deps); `rg -c "unsafe" crates/platform-linux/src/*.rs crates/platform-linux/src/linux/*.rs` + `rg -n "unsafe" src/gpu.rs src/linux/drm.rs` (doc claim: unsafe confined to FFI modules — verified: drm/gpu/lib mentions are comments only; real unsafe in pulse/pulse_ffi/dl); `rg -n "fn discover_assembles" -A 25 tests/linux_integrations.rs` (real-host discover test); Read of tests/seam_behaviour.rs head (fallback contracts) |

## 1. One-paragraph verdict
The composition root of a crate nobody composes: `LinuxPlatform::discover` wires three seams with the honest-negotiation discipline the module docs promise — each probe's outcome lands verbatim in the `PlatformReport`, a loadable-but-serverless library is a typed `ServiceNotReachable` rather than an `Available` (the comment at 139-143 is exactly right), non-Linux builds compile every seam into the typed `NotCompiledForThisOs` fallback, and the doc claim that all `unsafe` lives in the FFI modules verified true under grep (the drm/gpu/lib.rs occurrences are comments; the real blocks are pulse/pulse_ffi/dl — their per-block audits belong to A03/A05 this wave). Two S4s: the audio capability deliberately uses ONE verdict for TWO report rows (Enumeration and Notifications share a `Capability` clone — a notifications-only failure is inexpressible, documented but lossy), and the `discover` doc says the probe list includes "the portal-bus reachability check" when in fact no bus connect happens at discovery time (the connect is lazy in `portal_status`/`no_backend_error`; only the env var is read). The crate-wide fact that dominates everything: zero consumers — the entire typed-seam architecture is staged for a composition that does not exist in any shipped binary.

## 2. What actually works (verified)
— Honest seam wiring — verified: `audio_seam` (136-170) runs a FULL enumeration round before saying `Available` (a dead server → typed fallback with `from_probe_error`); `gpu_seam` (175-196) treats an empty sysfs as an honest empty list; `dialog_seam` (203-232) picks the xdg chain only when a command backend resolves, else the typed refusal naming everything tried.
— Non-Linux compilation honesty — verified: every seam builder's `#[cfg(not(target_os = "linux"))` arm returns `NotCompiledForThisOs { crate_os: "linux" }` typed fallbacks; `pub mod linux` itself is cfg-gated (58-59).
— unsafe-confinement doc claim — verified by grep: `unsafe` tokens in src/ outside `linux/` are doc comments only (lib.rs:7/34, gpu.rs:5, drm.rs:2); code occurrences live in linux/{pulse,pulse_ffi,dl}.rs (38/47/11) — those files' audits are A03/A05's this wave.
— `#[must_use = "probing has side effects on OS handles; reuse the value"]` on discover — correct discipline.
— Real-host test — verified: `discover_assembles_this_hosts_report` (linux_integrations 452+) runs `LinuxPlatform::discover()` on the actual sandbox and asserts every unavailable row carries one of the two honest reasons ("anything else is a fabrication") — passing in the baseline.
— Fallback contracts — verified: seam_behaviour.rs drives all three `Unavailable*` fallbacks through their trait methods asserting the exact typed reason returns, never a fake success.

## 3. Findings

### F-001 [S4]
Where: crates/platform-linux/src/lib.rs:98-101
Code:
```rust
        // The PulseAudio client provides enumeration and notifications
        // together (same connection); one verdict covers both rows.
        report.set(Feature::AudioEnumeration, audio_capability.clone());
        report.set(Feature::AudioNotifications, audio_capability);
```
What is wrong: One `Capability` value is written into two semantically distinct report rows. The justification (same PulseAudio connection) is true today, but the report's vocabulary pretends to distinguish the features — a future state where subscription works but enumeration fails (or vice versa) cannot be expressed without changing the wiring, and a consumer reading `AudioNotifications` gets a verdict that was never measured for it.
Why it matters: Capability reports are the crate's entire output contract; conflating rows under a comment is a latent lie-shaped seam. No consumer today (crate is unreachable), so S4.
What correct looks like: Probe both concerns separately (subscribe with a dead-server enumeration already distinguishes), or collapse the two `Feature` variants into one row and document that they are one capability — either honesty works, the half-state does not.
Confidence: HIGH — Verification: read (wiring + the seam_behaviour fallback tests that show the rows exist independently).

### F-002 [S4]
Where: crates/platform-linux/src/lib.rs:83-91 (doc) vs the actual probe set
Code:
```rust
    /// Probes run: PulseAudio client-library load + server connect
    /// (one full enumeration round — `Available` means enumeration
    /// works, not merely that the library is loadable); DRM sysfs
    /// presence; zenity/kdialog discovery plus the portal-bus
    /// reachability check.
```
What is wrong: "plus the portal-bus reachability check" — no bus connect happens inside `discover()`. `XdgDialogs::discover()` (xdg.rs:69-85) only READS `$DBUS_SESSION_BUS_ADDRESS`; the actual `UnixStream::connect` is lazy — first performed by `portal_status()` or, on the no-backend path, `no_backend_error()` (my xdg.rs audit's F-003 context). On a host WITH zenity installed, the discovery-time report never touches the bus at all.
Why it matters: The doc describes a probe that does not run; a reader auditing the report's provenance (or timing the discovery) would expect a socket connect that never happens. Pure doc rot on the file's most-load-bearing doc comment.
What correct looks like: "zenity/kdialog discovery (the portal-bus reachability check is queryable later via `XdgDialogs::portal_status`; it is not part of discovery)".
Confidence: HIGH — Verification: read (XdgDialogs::discover body traced — env read only; connect sites enumerated in the xdg.rs audit).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `LinuxPlatform` (struct) | 75-80 | REAL | — | Three boxed dyn seams + report; no OS types leak (ARCH-6 honored — fields private). |
| `LinuxPlatform::discover` | 93-110 | REAL | S4 (F-001) | Probe + wire + report; `#[must_use]` with side-effect rationale; real-host test. |
| `LinuxPlatform::report` | 114-116 | REAL | — | Pure accessor. |
| `LinuxPlatform::audio` / `gpu` / `dialogs` | 119-131 | REAL | — | `&mut dyn` handouts — callers never branch on which implementation (the doc's 73-74 promise, honored). |
| `audio_seam` | 136-170 | REAL | S4 (F-001, the row conflation) | Full-round probe; typed fallbacks both arms; cfg split correct. |
| `gpu_seam` | 175-196 | REAL | — | Empty-list-is-honest doctrine; the wgpu boundary correctly deferred to renderer-wgpu. |
| `dialog_seam` | 203-232 | REAL | — | Command-backend gate; the honest no-backend detail string built here (test-pinned substrings via the xdg suite). |

## 5. Test honesty (tests for this file live in tests/)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `discover_assembles_this_hosts_report` (linux_integrations 452) | YES | Real discover on the real host; every Unavailable row must carry one of two honest reasons — a fabricated capability fails it. Baseline ok. |
| `unavailable_{audio_devices,gpu_adapters,dialogs}_returns_the_typed_reason` (seam_behaviour 22/36/48) | YES | Exact-reason equality through every trait method — the graceful-degradation contract pinned per seam. |
| `typed_fallbacks_satisfy_the_seam_traits` (seam_behaviour 67) | YES | `dyn` dispatch shape + the `FilePick` success vocabulary pinned. |
| Coverage hole | — | No test asserts the AudioEnumeration vs AudioNotifications row relationship (F-001's conflation is invisible); no non-Linux CI lane compiles the cfg(not(linux)) arms (UNVERIFIED — sandbox is Linux). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the only `unsafe` in this crate lives in the FFI modules ([`linux`]), every block safety-commented" | lib.rs:7-8 | REAL (confinement) / UNVERIFIED (per-block comments) | Confinement grep-verified (drm/gpu/lib mentions are comments); per-block safety comments are pulse/pulse_ffi/dl auditors' (A03/A05) duty |
| "it never leaks OS types upward (ARCH-6)" | lib.rs:9-11 | REAL | Struct fields private; accessors return trait objects; the public API surface (audio/dialog/gpu/events/capability/error) carries no OS types |
| "`Available` means enumeration works end to end (a loadable library with a dead server is the typed ServiceNotReachable verdict)" | lib.rs:85-90, 139-143 | REAL | audio_seam's full-round probe; the sandbox's own libpulse-but-no-server case is the runtime-tested path (linux_integrations pulse tests) |
| "Probes run: … zenity/kdialog discovery plus the portal-bus reachability check" | lib.rs:88-89 | SHALLOW | The bus check does not run at discovery (F-002) |
| "capability negotiation is a first-class outcome … never a guessed success" | lib.rs:20-22 | REAL | Report rows + typed fallbacks + the real-host test's fabrication assertion |
| "The seam vocabulary … deliberately mirrored across the three `platform-*` crates … `[PROPOSAL]`" | lib.rs:44-47 | REAL | Cross-crate mirror visible in platform-windows/macos (this wave's A01/A02/A07/A08 files); the deferral is honest |

## 7. Dead / duplicated / unreachable code
- The whole crate is unreachable from shipped entry points (header proof). Within the crate, nothing is dead: every seam is constructed by discover and exercised by the in-crate suites.
- Anticipated duplication: the three platform-* crates mirror seam vocabulary by documented design (Lead-deferred unification) — flagged as accepted debt, not a finding.

## 8. Dependencies & flags
No external deps (std + crate-internal). One cfg gate (`target_os = "linux"` on `pub mod linux`, 58-59) plus three `#[cfg(not(target_os = "linux"))]` fallback arms. No unsafe, no panics, no feature flags. `[lints] workspace = true` inherited (the 39/40-crate policy per baseline INDEX).

## 9. Open questions for the next agent
1. The zero-consumer fact is now established for the whole crate (xdg.rs audit header + this file): events.rs, drm.rs (my remaining files) inherit it; the capability/events/pulse/dl auditors should cite it rather than re-derive.
2. Matrix/parity auditor: confirm no parity row counts this crate's seams as shipped capability (my dialog-row grep found none; the `platform-aout-*` rows' evidence strings deserve the same check — they cite platform crates that must actually be in a shipped graph to count).
3. Lead/composition owner: F-001's row conflation is cheapest to fix before any host reads the report.
