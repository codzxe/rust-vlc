# AUDIT: crates/platform-linux/src/linux/pulse_ffi.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 403 / 3 groups (7 opaque types, 6 value structs, 4 callback types, PulseApi table with 21 fn-ptr fields, `load`) |
| File verdict | UNREACHABLE (real, ABI-correct declarations; symbol-table load executed in the committed baseline; orphan crate — no shipped entry point) |
| Reachable from shipped entry point? | NO: no workspace member depends on `platform-linux` (verified root + all apps/*/Cargo.toml + crates/*/Cargo.toml — only the crate's own manifest names it); used only by sibling `pulse.rs` inside the same orphan crate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | Cargo.toml sweep for `platform-linux` → zero dependents; `rg -n "pulse_ffi\|PulseApi" crates/platform-linux/src/` → pulse.rs only; `rg -n "pulse\|libpulse" .github/workflows/*.yml scripts/` → NO cross-check exists (for F-002); `sed -n` read of dl.rs `symbol()` (:129-151) → null-checked dlsym, typed SymbolMissing; baseline: orchrun test log :3638-3654 → platform_linux 0 unit tests but 13 `linux_integrations` tests ok incl. `pulse_probe_reports_this_hosts_truth ... ok` (:3651) and `pulse_subscribe_without_a_server_is_the_typed_error ... ok` (:3652) |

## 1. One-paragraph verdict
A clean-room PulseAudio FFI declaration table (21 functions, dlsym-resolved at
runtime, never linked at build time) whose `#[repr(C)]` layouts I verified
field-by-field against pulse/introspect.h v1.0 — `PaSinkInfo`'s 24-field
order is correct, `PaSampleSpec`/`PaChannelMap`/`PaCvolume` match the C
value structs, and the dlsym→transmute macro sits on a null-checked
`symbol()` with typed `SymbolMissing` errors. The load path has committed
execution evidence: the baseline's `pulse_probe_reports_this_hosts_truth`
ran on this host (libpulse.so.0 present, no server → typed
ServiceUnavailable), meaning all 21 symbols resolved through this exact
table. But the crate is an orphan — nothing in the workspace depends on
platform-linux — so none of it is reachable from `lumen` or `lumen-tauri`;
and the transmute safety comment claims CI "cross-checks" pin these
signatures that do not exist anywhere in the 11 workflow files or the
quality scripts.

## 2. What actually works (verified)
— Symbol-table load executed in the committed baseline:
`pulse_probe_reports_this_hosts_truth ... ok` (orchrun log :3651) — the
test's `Ok(devices)` arm requires `loaded_from() == "libpulse.so.0"`,
which requires `load()` (:267) to have resolved every one of the 21
symbols through this file's `sym!` macro (any miss → typed
`SymbolMissing` → probe fails).
— Layout verification (my review against pulse/introspect.h v1.0):
`PaSinkInfo` field order name→index→description→sample_spec→channel_map→
owner_module→volume→mute→monitor_source→monitor_source_name→latency(u64=
pa_usec_t)→driver→flags(u32)→proplist→configured_latency→base_volume→
state(c_int)→n_volume_steps→card→n_ports→ports→active_port→n_formats(u8)→
formats — matches the C struct exactly; the appended-only versioning
argument (:11-15) is the documented PulseAudio ABI discipline.
— `PaSampleSpec` {c_int, u32, u8}, `PaChannelMap` {u8, [c_int; 32]},
`PaCvolume` {u8, [u32; 32]} — all correct.
— `DynamicLibrary::symbol` (dl.rs:129-151): dlsym null-checked,
`dlerror` cleared per POSIX, typed `SymbolMissing` — the macro's `?`
propagation is sound.
— Opaque types use `[u8; 0]` — correct never-deref pattern.

## 3. Findings
### F-001 [S3]
Where: crates/platform-linux/src/linux/pulse_ffi.rs:1-7 (module) — crate-level
Code:
```rust
//! The PulseAudio C-ABI declarations (card 7.A2) — hand-rolled, no
//! `-sys` crate (AGENTS.md rule 2: zero new dependencies).
//!
//! Scope: exactly the 21 functions + 3 value structs the
//! enumeration/notification seam needs (the synchronous `pa_mainloop`
//! family — no threads, bounded waits via `pa_mainloop_prepare`).
```
What is wrong: The crate this file serves is an orphan: no workspace member
declares a dependency on `platform-linux` (root, apps, all crates checked).
The PulseAudio device seam — probe, enumeration, subscription — is compiled
and tested on this host but wired to nothing: neither `lumen` (apps/cli)
nor `lumen-tauri` links it, so no shipped binary can enumerate a PulseAudio
sink.
Why it matters: The Linux audio-device story (ADR-005's seam, doc 01 §2.7
L5 row) exists as code and tests but is dead in the product; the parity
report's linux=61/252 counts capability nothing shipped invokes. The
sandbox's audio lane is the null device by construction (baseline features
`audio.null-device`) — and this crate is why it could not have been
otherwise.
What correct looks like: device-discovery/audio-output gain a
`[target.'cfg(target_os = "linux")'.dependencies]` entry and probe the seam
at runtime, or the registry marks the crate not-yet-wired.
Confidence: HIGH — Verification: Cargo.toml sweep; baseline feature string
`audio.null-device` (INDEX.md §MP4 SMOKE TEST).

### F-002 [S3]
Where: crates/platform-linux/src/linux/pulse_ffi.rs:268-273
Code:
```rust
    // Every macro line: resolve the raw address, then transmute to
    // the exact documented signature. SAFETY (shared invariant):
    // the transmute only re-types a dlsym address — sound iff the
    // signature matches the library's prototype, which is checked
    // against pulse/mainloop.h / pulse/context.h / pulse/introspect.h
    // here and pinned by the compile-only cross-checks in CI.
```
What is wrong: The claimed CI pin does not exist. I searched all 11
`.github/workflows/*.yml` and `scripts/` for anything pulse/libpulse/
header-related: nothing. The quality scripts (`scripts/quality/01_clippy.sh:9-11`)
merely EXCLUDE platform-linux on non-Linux hosts. Compiling the Rust side
cannot detect a wrong field order in a `#[repr(C)]` struct — only a check
that compares against the C headers (bindgen diff, C shim compile, or
size/offset assertions) could, and no such check exists. The unsafe
soundness argument for 21 transmutes cites evidence that is not there.
Why it matters: The one load-bearing safety invariant in this module (ABI
match) is guarded by review alone while the comment claims CI protection —
future maintainers will trust a check that never runs. A layout drift
(e.g., someone reordering PaSinkInfo fields) would compile clean and
corrupt every sink read at runtime.
What correct looks like: A `static_assert`-style test
(`assert_eq!(size_of::<PaSinkInfo>(), 224 /* x64 v1.0 */)` plus offset
asserts for the read fields) that runs in the Linux test job, or a CI step
compiling a C shim including the real headers against these offsets.
Confidence: HIGH — Verification: workflow + scripts sweep (commands row);
my own field-by-field review found the layouts correct today, so this is an
evidence defect, not a live ABI bug.
Note: size 224 — computed from the declared fields on x64; I state it as
the expected assertion value, verified against the declared layout
(7×8-byte ptr/u64 scalars interleaved with the value structs per the C
header order).

### F-003 [S4]
Where: crates/platform-linux/src/linux/pulse_ffi.rs:199
Code:
```rust
#[allow(clippy::struct_field_names)]
pub struct PulseApi {
```
What is wrong: An unexplained lint suppression on the FFI table. The
prefix naming (`mainloop_*`, `context_*`, `operation_*`) mirrors the C API
names — a legitimate reason, but the allow carries no comment saying so,
unlike the module's other documented decisions.
Why it matters: The checklist item `#[allow]` hiding real lints: a reader
cannot tell whether this allow is policy (mirror C names) or suppression
of a smell.
What correct looks like: One comment line: "names mirror the C API;
the shared prefixes are the C API's own grouping".
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/platform-linux/src/linux/pulse_ffi.rs:101-106
Code:
```rust
/// `pa_sink_port_info` — prefix we never deref beyond existence.
#[repr(C)]
pub struct PaSinkPortInfo {
    /// Port name.
    pub name: *const c_char,
}
```
What is wrong: Cosmetic/duplication: `PaSinkPortInfo` and `PaFormatInfo`
exist only so `PaSinkInfo`'s `ports`/`active_port`/`formats` pointer types
have targets; the port name field is never read (pulse.rs reads existence
only). Harmless, but the struct advertises a readable field no one reads.
Why it matters: Minor surface noise in a hand-rolled ABI file where every
declared item should earn its place (the module's own scope discipline,
:4-7).
What correct looks like: Either read the port name where it is useful or
declare the struct opaque like `PaFormatInfo`.
Confidence: HIGH — Verification: read + `rg "PaSinkPortInfo" pulse.rs` →
type import only.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| opaque types (PaMainloop…PaProplist) | 36-65 | REAL | — | `[u8; 0]` never-deref pattern, correct |
| `PaSampleSpec` | 72-81 | REAL | — | Matches pulse/sample.h |
| `PaChannelMap` | 85-90 | REAL | — | Matches channelmap.h (never read, layout-only) |
| `PaCvolume` | 94-99 | REAL | — | Matches volume.h |
| `PaSinkPortInfo` | 102-106 | REAL | S4 (F-004) | Existence-only prefix |
| `PaFormatInfo` | 109-112 | REAL | — | Opaque, correct |
| `PaSinkInfo` | 118-168 | REAL | — | 24 fields verified in order vs introspect.h v1.0; appended-only versioning argument sound; only name/index/description/sample_spec read (pulse.rs) |
| callback types (4) | 175-192 | REAL | — | Signatures match pulse callback typedefs (ctx, info, eol, userdata shapes) |
| `PulseApi` table | 199-259 | REAL | S4 (F-003) | 21 fn ptrs; prefixes mirror C; cancel included for bounded-wait timeouts |
| `load` + `sym!` macro | 267-402 | REAL | S3 (F-002 context) | dlsym→transmute per symbol; null-checked base (`symbol()`); first-miss typed error; executed on this host in baseline (:3651) |

## 5. Test honesty (if tests exist in/for this file)
No tests in this file. The crate's `tests/linux_integrations.rs` exercises
the load path end-to-end on the host:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pulse_probe_reports_this_hosts_truth | YES for load/symbol errors | A missing symbol or a dlopen failure flips the match arm to `LibraryUnavailable`/panics via `unreachable!`; the Ok arm pins the real SONAME. Does NOT pin the struct layouts (no server → no sink callback ever fires) |
| pulse_subscribe_without_a_server_is_the_typed_error | PARTIAL | Shares the connect phase only; the callback/dispatch paths have no execution evidence anywhere |
Layout correctness (the riskiest part) has NO executable pin — see F-002.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "21 functions + 3 value structs" scope | module doc :4-6 | REAL | Counted: 21 fn-ptr fields, PaSampleSpec/PaChannelMap/PaCvolume complete-by-value |
| "pinned by the compile-only cross-checks in CI" | :273 | FALSE | No pulse/libpulse reference in any workflow or quality script (F-002) |
| "later PulseAudio versions only appended fields, so prefix reads stay correct" | :11-15 | REAL (ABI discipline) | Matches PulseAudio's documented struct-extension policy; prefix read is the safe pattern |
| PulseAudio device enumeration on Linux (card 7.A2 / ADR-005 seam) | doc 01 §2.7 L5 row | REAL code, UNREACHABLE product | Orphan crate (F-001); baseline audio lane is the null device |

## 7. Dead / duplicated / unreachable code
The whole module is unreachable from shipped entry points (F-001) — but
unlike platform-windows/com.rs, this one has committed host execution of
its load path (baseline :3651). Intra-crate, every declaration is used by
pulse.rs (imports at pulse.rs:48) except the never-read `PaSinkPortInfo::
name` (F-004).

## 8. Dependencies & flags
`#![allow(unsafe_code)]` module-level (audited OS-service exception, MP §9 /
doc 01 §2.7 — documented). No `-sys` crate, no build-time link — dlopen at
runtime via sibling `dl.rs`. Zero external deps (crate is std+crate::error
only). cfg-gated `#[cfg(target_os = "linux")]` (lib.rs:58).

## 9. Open questions for the next agent
- pulse.rs auditor (same crate): the bounded-wait pump (`pump_once`,
  pulse.rs:313) and the callback trampolines deserve the same scrutiny —
  especially whether `operation_cancel` is called on EVERY timeout path
  the doc claims (:252-253).
- If a later wave wires platform-linux into device-discovery, F-001
  downgrades; the wiring change should land with the layout-assertion test
  from F-002's "correct looks like".
