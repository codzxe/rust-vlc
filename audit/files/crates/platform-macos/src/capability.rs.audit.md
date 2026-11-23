# AUDIT: crates/platform-macos/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W08-A08 |
| Date | 2026-11-23 |
| Lines / functions | 200 / 10 (2 enum impls + Display + Feature::key + PlatformReport ×4) |
| File verdict | REAL (typed vocabulary, all functions exercised by the crate's seam suite) |
| Reachable from shipped entry point? | NO — no crate or app depends on platform-macos (rg over crates/, apps/: only its own tests reference platform_macos); workspace-member compile only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | sed -n (quotes); diff macos↔linux↔windows capability.rs (exit 1 each: ONE line differs — the card id); rg key consumers; baseline citation cargo-test-workspace.txt:4014-4034 (16/16 ok) |

## 1. One-paragraph verdict
A small, real, honest vocabulary module: Capability/UnavailableReason/Feature/
PlatformReport with typed error→reason mapping and an honest
NotYetImplemented default for unprobed features. Every function does what its
name says, the crate's 16-test seam suite exercises it end-to-end
(capability_classifies_and_converts_probe_errors, platform_report_is_a_table_
with_honest_defaults, feature_keys_are_the_stable_matrix_vocabulary — all
green in baseline), and lib.rs's discover() really feeds it live probe
outcomes on macOS / typed WrongOs elsewhere. Two structural defects: it is a
byte-identical 3-way copy-paste (only the card-id comment line differs from
platform-linux and platform-windows — verified by diff), and the "stable
dotted key (feature-matrix vocabulary)" claim has zero consumers — none of
the four keys appears in docs/feature-matrix.toml; they exist only in the
three crates' own self-referential tests. The crate it serves is unreachable
from any shipped binary.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/cargo-test-workspace.txt:4014-4034 — platform
  -macos seam_behaviour 16/16 ok, including the three capability tests.
- from_error mapping verified by read against error.rs variants; the
  catch-all arm (lines 101-104) collapses unknown errors into
  ServiceNotReachable with the Display string — lossy but typed.
- discover() wiring (lib.rs:99-114 + audio_seam/gpu_seam/dialog_seam): on
  macOS each capability is set from a live probe outcome; on this host
  everything is WrongOs — the vocabulary is genuinely load-bearing.

## 3. Findings
F-001 [S3]
Where: crates/platform-macos/src/capability.rs (whole file) vs platform-linux/src/capability.rs and platform-windows/src/capability.rs
Code:
```
//! Capability negotiation: what this build can actually do, honestly
//! (card 7.A3; MP §13 "no invented crate capabilities").
```
What is wrong: `diff` of this file against platform-linux/src/capability.rs
and platform-windows/src/capability.rs shows exactly ONE differing line — the
card id ("7.A3" vs "7.A2" vs "7.A1"). 200 lines × 3 crates of byte-identical
copy-paste. The crate docs defend this as intentional "mirroring so the
lanes cannot drift", but copy-paste is the mechanism by which lanes drift: a
future fix (e.g. a new UnavailableReason variant or a from_error correction)
must be hand-replicated three times, and the only guard is each crate's own
test suite. What correct looks like: a shared platform-common crate (the
audio-output seam already plays this role for DeviceDesc/DeviceId), or an
xtask check that the three files stay identical.
Confidence: HIGH — Verification: ran diff (both exits 1, one hunk, line 2).

F-002 [S3]
Where: crates/platform-macos/src/capability.rs:142-151
Code:
```
    /// The stable dotted key (feature-matrix vocabulary).
    #[must_use = "pure accessor"]
    pub fn key(self) -> &'static str {
        match self {
            Feature::AudioEnumeration => "platform.audio.enumeration",
            Feature::AudioNotifications => "platform.audio.notifications",
            Feature::GpuEnumeration => "platform.gpu.enumeration",
            Feature::FileDialogs => "platform.dialogs",
        }
    }
```
What is wrong: none of these four dotted keys appears anywhere in
docs/feature-matrix.toml (matrix rows use kebab ids like `platform-aout-cpal`);
the keys are read only by the three platform crates' own tests, which assert
each key equals the literal the function returns — a self-referential pin
proving no linkage. "Feature-matrix vocabulary" is a claim about a consumer
that does not exist. (Corroborated: the wave-7 auditor of
crates/platform-linux/src/capability.rs found the identical issue —
audit/files/crates/platform-linux/src/capability.rs.audit.md.) What correct
looks like: wire the keys into matrix evidence/notes or drop the claim.
Confidence: HIGH — Verification: ran rg over docs/ + all crates (only
self-tests and device-discovery's separate Feature enum match).

F-003 [S4]
Where: crates/platform-macos/src/capability.rs:6-8
Code:
```
//! it never fakes success. The report is computed from live probes
//! (library presence, service reachability), not from compile-time
//! assumptions alone.
```
What is wrong: for the macOS lane "library presence" probing does not exist —
CoreAudio/Metal/objc are link-time frameworks, and the seams probe
services/classes (CoreAudioDevices::enumerate, AppKitPanels::probe), not
libraries (library probing is the Linux dlopen lane's model). The module
itself performs no probes at all; the sentence describes discover(), not
this file. Minor doc overstatement, no behavioral falsehood.
Confidence: HIGH — Verification: read (lib.rs seam builders + Cargo.toml
"link-time frameworks" note).

## 4. Per-function table (file is 200 LOC — table mandatory)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Capability::is_available | 26-29 | REAL | — | matches! Available |
| Capability::from_probe_error | 33-36 | REAL | — | delegates to from_error |
| UnavailableReason::from_error | 76-106 | REAL | — | 4 typed arms + stringified catch-all |
| UnavailableReason::Display | 110-127 | REAL | — | human forms; pinned by seam test |
| Feature::key | 145-152 | REAL | S3 | no consumer of the "matrix vocabulary" (F-002) |
| PlatformReport::new | 165-167 | REAL | — | empty default |
| PlatformReport::set | 170-180 | REAL | — | replace-in-place, linear over ≤4 rows |
| PlatformReport::get | 185-194 | REAL | — | honest NotYetImplemented default |
| PlatformReport::iter | 197-199 | REAL | — | row iterator |

## 5. Test honesty
No tests in this file; the crate's tests/seam_behaviour.rs covers all of it
(baseline 16/16 ok). Would those tests fail if this code broke? Yes for
from_error mappings, Display strings, key literals, get/set/replace
semantics, and the NotYetImplemented default. No for anything macOS-specific
(all real-lane code is cfg'd out on this host).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "never fakes success" | header:4-5 | REAL | fallbacks return typed errors (seam tests) |
| "computed from live probes (library presence, service reachability)" | header:6-8 | SHALLOW (macOS lane) | service/class probes only; no library-presence probe exists in this lane (F-003) |
| "The stable dotted key (feature-matrix vocabulary)" | :143 | FALSE (no consumer) | rg: keys appear only in the 3 crates' self-tests (F-002) |
| "unprobed features are honestly NotYetImplemented" | :182-183 | REAL | get() default; pinned by platform_report_is_a_table_with_honest_defaults |

## 7. Dead / duplicated / unreachable code
Duplication: the entire file ×3 (F-001). Unreachable: the crate has no
dependents; nothing shipped can call any of this today (Phase-7 target —
docs/feature-matrix.toml platform-aout-coreaudio row is "not_started").

## 8. Dependencies & flags
Deps: crate::error only. No features. The macOS real lane (src/macos/) is
cfg(target_os="macos") and never compiled in this sandbox or in the shipped
Linux release lane.

## 9. Open questions for the next agent
- Should the three platform crates' shared vocabulary collapse into one crate
  before Phase 7 wires them in? Post-wiring, the ×3 copy-paste becomes ×3
  runtime divergence risk.
- Does any planned UI "capability table" consume PlatformReport::iter()? No
  consumer exists today (rg-verified).
