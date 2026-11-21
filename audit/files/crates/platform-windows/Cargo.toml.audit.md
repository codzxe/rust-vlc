# AUDIT: crates/platform-windows/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-21 |
| Lines / functions | 43 / 10 config keys (package×6, lints, dependencies×2, dev-dependencies×1) |
| File verdict | REAL (one evidence claim UNVERIFIED — F-001) |
| Reachable from shipped entry point? | NO. Zero dependents: Grep `platform-windows` in every Cargo.toml → only a comment in crates/platform-linux/Cargo.toml:5. Nothing in apps/ or tests/ references it. Compiled+seam-tested as a workspace member (baseline #1 build exit 0; #3 test run) but no shipped binary links it |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | Grep `platform-windows` in **/Cargo.toml (zero dependents) · Grep `msvc`/`windows-msvc` in .github/workflows/, scripts/, tools/xtask/src/ (zero hits — cross-check lane does not exist) · sed -n 80,170p .github/workflows/ci-full.yml (3-OS matrix: check/clippy/test on ubuntu/windows/macos-latest) · grep `x86_64-pc-windows-msvc` in docs/ (only: cpal record, feasibility doc, matrix row 416 note, phase7-exit.md:162 OF-5 fabrication record; worklog: ZERO) · grep dep usage (audio.rs:16, events.rs:23 audio_output; wasapi.rs:48 media_types) · grep `allow(unsafe_code)` (4 FFI modules) · sed -n 85,100p com.rs (invariant docs verified) · sed -n boundaries.toml [crates.platform-windows] (exact match) |

## 1. One-paragraph verdict
A disciplined manifest: two production deps both used (audio-output in the
seam vocabulary, media-types in exactly one FFI site), zero external crates
(no windows-s — hand-rolled COM FFI behind four module-level
`#![allow(unsafe_code)]` exceptions with per-block invariant comments,
spot-verified), boundaries.toml agreement exact, dev-dep used. Two defects:
the "Sandbox honesty" block claims the FFI surface "is compile-verified via
`cargo check --target x86_64-pc-windows-msvc`" — a command that no workflow,
script, xtask, or worklog entry anywhere in the repo records executing, in a
repo with a documented precedent (OF-5, issue #51) of this exact claim shape
being fabricated; and the crate is dead weight from every shipped entry
point's perspective (zero dependents, phase-7 parked).

## 2. What actually works (verified)
— Dep usage: `audio-output` (audio.rs:16 `use audio_output::{DeviceDesc,
DeviceId};`, events.rs:23), `media-types` (wasapi.rs:48
`use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};`).
— Dev-dep used: testkit (tests/seam_behaviour.rs:25 `use testkit::assert_err;`,
16 tests in the file).
— Boundaries agreement: boundaries.toml `[crates.platform-windows]` →
internal_deps = ["audio-output", "media-types"], dev_deps = ["testkit"] —
identical to this manifest.
— C5 claim TRUE: zero external deps, no windows-s crate (sed -n '36,43p').
— Invariant documentation claim TRUE at spot-check: com.rs:85-87 documents
the extern-block invariants ("CoInitializeEx/CoUninitialize are paired per
thread; handles returned by CoCreateInstance are released exactly once")
immediately before `#[link(name = "ole32")] unsafe extern "system"`.
— Crate compiles on Linux host (extern declarations need no linking):
baseline #1 `cargo build --release --locked --workspace` exit 0; seam tests
run in baseline #3 (1681 passed workspace-wide).

## 3. Findings
### F-001 [S2] — claimed cross-target verification has no execution record anywhere; identical claim shape was fabricated before (OF-5)
Where: crates/platform-windows/Cargo.toml:20-24
Code:
```toml
# Sandbox honesty: this Linux sandbox has no Windows SDK, so the
# FFI surface is compile-verified via `cargo check --target
# x86_64-pc-windows-msvc` (typechecked declarations, no linking, no
# runtime); the host-runnable tests cover the typed seams, capability
```
What is wrong: The manifest asserts a specific verification event. Grep for
`x86_64-pc-windows-msvc` / `windows-msvc` across .github/workflows/,
scripts/, tools/xtask/src/, docs/worklog/ → ZERO execution records. The only
committed occurrences are: the crate's own files, docs/dependencies/records/
audio-output-cpal.md:59 (a different crate's record), a feasibility doc, the
feature-matrix note, and docs/gates/phase7-exit.md:156-171 — which documents
OF-5: a previously committed claim of "compile-verified via cargo check
--target aarch64-apple-darwin + x86_64-pc-windows-msvc" that was FALSE (no
MF code ever existed), retracted in issue #51. No lane in any workflow runs
a windows cross-check; ci-full.yml does compile the crate on real
windows-latest runners (check/clippy/test, 3-OS matrix, lines 80-170), which
would subsume the claim — but no committed run output proves ci-full ever
executed (baseline INDEX contains no CI-run evidence).
Why it matters: "Compile-verified" is the sole quality claim separating this
hand-rolled FFI (177 unsafe occurrences across the 4 windows/ modules) from
unreviewed bytecode. An unevidenced claim of the exact shape this repo has
already once fabricated is precisely what a devil audit must flag: until a
run is cited, the FFI surface's type correctness rests on nothing citable.
What correct looks like: Add the cross-check as a CI step
(`cargo check -p platform-windows --target x86_64-pc-windows-msvc`) or cite
the ci-full windows-latest run URL/output in the comment; until then mark
the claim `[VERIFY]` like the neighboring "Real-device behavior" sentence
does.
Confidence: HIGH — Verification: read + exhaustive grep (no record exists);
UNVERIFIED that the check ever ran (that is the finding).

### F-002 [S3] — crate is unreachable from every shipped entry point
Where: crates/platform-windows/Cargo.toml:36-38 (dependency edges nothing consumes)
Code:
```toml
[dependencies]
audio-output = { path = "../audio-output" }
media-types = { path = "../media-types" }
```
What is wrong: No manifest in the repo depends on platform-windows (grep:
only a prose mention in platform-linux's header). The `lumen` CLI,
desktop-tauri, and web app never link it. Its WASAPI/DXGI/dialog surfaces
are future phase-7 adapters (manifest lines 8-11: codec-api = issue #41,
renderer-wgpu lead-gated, device-discovery = issue #42 — none landed).
Why it matters: Real FFI code (4 modules, 177 unsafe sites) compiles and
passes seam tests in every workspace lane while being executable by zero
users. Any doc claiming Windows device enumeration/dialogs work in the
shipped binary is FALSE on reachability alone.
What correct looks like: This is an accepted phase-7 posture (registry row
records the future deps); keep it parked but ensure no matrix row cites this
crate as live (currently none does — see §6).
Confidence: HIGH — Verification: repo-wide dependent grep (zero).

## 4. Per-key table
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `package.name = "platform-windows"` | 27 | REAL | — | Matches boundaries.toml [crates.platform-windows] |
| `package.version = "0.1.0"` | 28 | REAL | — | Repo convention |
| `package.edition = "2024"` | 29 | REAL | — | Workspace-consistent |
| `package.description` | 30 | REAL | — | WASAPI enum + notifications, DXGI, dialogs — all present in src (audio.rs/events.rs/gpu.rs/dialog.rs); "typed Unavailable fallback" matches error.rs surface (seam tests assert_err) |
| `package.publish = false` | 31 | REAL | — | — |
| `[lints] workspace = true` | 33-34 | REAL | — | Inherits deny tables; unsafe exception via 4 module-level `#![allow(unsafe_code)]` (com.rs:26, dialogs.rs:26, dxgi.rs:17, wasapi.rs:38) — the documented MP §9 OS-service exception |
| `audio-output` dep | 37 | REAL | — | Used (audio.rs:16, events.rs:23); matches boundaries |
| `media-types` dep | 38 | REAL | — | Used in exactly one site (wasapi.rs:48) — narrow but real; matches boundaries |
| dev-dep `testkit` | 43 | REAL | — | Used (seam_behaviour.rs:25) |
| (absent `[features]`) | — | REAL | — | No features declared → no never-enabled-feature exposure |

## 5. Test honesty
Manifest has no tests. tests/seam_behaviour.rs (16 tests) runs in baseline #3
— but it tests the typed seams/fallbacks on a Linux host, NOT the FFI
(the manifest admits this). Would a test fail if an FFI signature were
wrong? No — nothing in the recorded evidence links against ole32 on Windows;
that is exactly the gap F-001 documents.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the FFI surface is compile-verified via cargo check --target x86_64-pc-windows-msvc" | Cargo.toml:21-23 | UNVERIFIED / no record | Zero execution records repo-wide; OF-5 precedent of same-shape claim being false (phase7-exit.md:156-171) — F-001 |
| "NO external dependencies, NO windows-s crate" | Cargo.toml:14 | REAL | [dependencies] has only 2 path deps (sed -n 36,38p) |
| "documented invariants per block (the MP §9 audited OS-service exception)" | Cargo.toml:17-18 | REAL (spot-checked) | com.rs:85-87 invariant comment precedes the extern block; 4 modules carry `#![allow(unsafe_code)]` |
| "the host-runnable tests cover the typed seams, capability negotiation and fallbacks" | Cargo.toml:23-24 | REAL | seam_behaviour.rs 16 tests ran in baseline #3 |
| "Real-device behavior is [VERIFY]" | Cargo.toml:24 | REAL (honest) | Consistent with F-001's gap |

## 7. Dead / duplicated / unreachable code
Whole crate: UNREACHABLE from shipped binaries (F-002) — deliberately parked
phase-7 adapter. No unused or duplicated deps in this manifest.

## 8. Dependencies & flags
2 internal prod deps + 1 internal dev dep; zero external; zero features.
Matches boundaries.toml [crates.platform-windows] verbatim. Registry row
(doc 01 §2.7) names 5 future deps (media-types, codec-api, audio-output,
renderer-wgpu, device-discovery); manifest is the honest 2-dep allowlist
subset, consistent with boundaries.

## 9. Open questions for the next agent
- Whoever audits src/windows/*.rs: verify EVERY unsafe block carries an
invariant comment (manifest claims "per block"; I spot-checked one).
- Orchestrator: REQUEST-ORCH-RUN — `cargo check -p platform-windows
  --target x86_64-pc-windows-msvc` to convert F-001's unevidenced claim into
  evidence (or refute it).
- Check whether ci-full.yml's windows-latest lane has ever run (no committed
  run output exists) — relevant to F-001's mitigation claim.
