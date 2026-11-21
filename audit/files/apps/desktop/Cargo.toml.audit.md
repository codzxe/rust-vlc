# AUDIT: apps/desktop/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-21 |
| Lines / functions | 54 / 12 manifest keys (package, bin, 5 deps, 2 dev-deps, lints) |
| File verdict | REAL (skeleton manifest whose own dep table is honest; its header narrative is stale on two counts: ADR-004 "pending"/"NOT imported" vs the existing sibling admission crate, and an "evidence core" whose transport claim fails in the baseline test run) |
| Reachable from shipped entry point? | YES (call chain: `apps/desktop-tauri/Cargo.toml:53` `desktop = { path = "../desktop" }` → lib; and the workspace bin `lumen-desktop` builds in baseline #1. Caveat: lumen-tauri shell itself is UNVERIFIED-BUILD — baseline #2 exit 101, missing gdk/gtk dev libs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n 'tauri\|wry' apps/desktop-tauri/Cargo.toml` (sibling IS the tauri admission crate) · `rg -n 'lumen_cli' apps/desktop/src` (used: frame_tap.rs:34, shell.rs:139, dispatch.rs:67-92) · `rg -ln 'testkit\|media_clock' apps/desktop/tests` (all 4 test files use them) · `sed -n '522,560p' boundaries.toml` ([crates.desktop] mirrors manifest exactly) · `sed -n '1,90p' apps/desktop/src/main.rs` (bin always exits 86) · `git ls-files \| rg prototype` (only `apps/desktop/prototype/lumen-frames.mjs`; no harness.mjs) · baseline cites INDEX #1/#2/#3 |

## 1. One-paragraph verdict
A small, structurally clean skeleton manifest: five internal path deps, zero externals (matching `boundaries.toml:538-542` `[crates.desktop] external_allowlist = []`), a `lumen-cli` library adoption that is genuinely used (not a paper dep), and a bin whose honest-skeleton behavior (always exit 86, never 0) matches its docs. The defects are narrative staleness: (1) the header still sells "NO tauri/wry import … pending the admission criteria in ADR-004 … NOT imported" while the sibling `apps/desktop-tauri` — "the ADR-004 admission crate (issue #67, 2026-11-13)" — exists in-tree and imports tauri/wry behind its own lockfile; (2) the "evidence core" story is contradicted by the executed baseline, where this crate's own `zero_alloc_transport.rs` is the single failing workspace test (608 bytes allocated against a zero-allocation contract); (3) the claimed prototype conformance harness (`prototype/harness.mjs`, cited by the shipped reference implementation itself) is not in the tree.

## 2. What actually works (verified)
- Dep table ↔ boundaries.toml exact match: `boundaries.toml:538-542` `internal_deps = ["commands", "lumen-cli", "media-types", "renderer-wgpu", "video-engine"]`, `dev_deps = ["testkit", "media-clock"]`, `external_allowlist = []` — identical to manifest lines 33–54. Verified by `sed -n '522,560p' boundaries.toml`.
- `lumen-cli` as library is real, not a paper claim: `apps/desktop/src/dispatch.rs:67` `host: lumen_cli::host::CliHost`, `:84` `host: lumen_cli::host::CliHost::new()`, `shell.rs:139` `pub fn engine_host(&mut self) -> &mut lumen_cli::host::CliHost`, `frame_tap.rs:34` `use lumen_cli::pipeline_host::PresentTap;` — verified by `rg -n 'lumen_cli' apps/desktop/src`.
- Both dev-deps used: `rg -ln 'testkit|media_clock' apps/desktop/tests` → pipeline.rs, zero_alloc_transport.rs, common/mod.rs, protocol.rs.
- The bin is an honest skeleton: `apps/desktop/src/main.rs:86` `ExitCode::from(WEBVIEW_PENDING)` on every path (and `:56` early return on window-config parse failure); doc says "Exit codes: `0` never" — code matches doc.
- Std-only claim holds for this crate: no external dependency, no build.rs, `[lints] workspace = true` inherited (workspace member via `apps/*` glob, root Cargo.toml line 37).
- Baseline #1 (`cargo build --release --locked --workspace`, exit 0) compiled this crate and its bin.

## 3. Findings

### F-001 [S3]
Where: apps/desktop/Cargo.toml:11-16
Code:
```
# NO tauri/wry import (the honest deferral, records/tauri-wry-2.x.md):
# the webview host is a typed seam pending the admission criteria in
# ADR-004 (accepted 2026-10-22; docs/adr/ADR-004-os-webview.md) and a
# desktop build environment — this sandbox has no webkit2gtk-4.1 dev
# packages, no display server, and tauri/wry are allowlist §5
# "exception-ADR-004" (pre-audited, NOT imported). Std-only by C5.
```
What is wrong: The narrative is stale for the current tree. `apps/desktop-tauri/Cargo.toml:1` is "the ADR-004 admission crate (issue #67, 2026-11-13)" and imports tauri/wry (its own lockfile carries the tauri closure); the root workspace manifest documents the ADR-004 admission and the exclusion. "NOT imported" and "pending the admission criteria in ADR-004" describe a pre-2026-11-13 state that no longer exists; a reader of this manifest alone concludes the project has no webview host code at all.
Why it matters: This is the project's self-description of its desktop story; auditors and the owner will mis-trace the desktop surface. The real state is worse than either story: the tauri shell exists but is UNVERIFIED-BUILD (baseline #2, exit 101, missing gdk-3.0/gtk pkg-config) — a manifest that still says "nothing imported" hides a broken sibling build.
What correct looks like: Header updated to: webview host lives in workspace-excluded `apps/desktop-tauri` (ADR-004 admission, issue #67), its build is sandbox-blocked (baseline #2), this crate remains the std-only composition core.
Confidence: HIGH — Verification: read (rg of desktop-tauri manifest + root Cargo.toml ADR-004 comment) + baseline cite #2.

### F-002 [S3]
Where: apps/desktop/Cargo.toml:5-9
Code:
```
# v0 is the ADR-003 variant-B evidence core: the frame-streaming wire
# protocol (src/wire.rs), the Presenter-side canvas bridge
# (src/bridge.rs), the bounded latest-wins mailbox (src/mailbox.rs)
# and the command-layer host seam (src/host.rs). The canvas-side
# reference implementation lives in prototype/ (browser + Node).
```
What is wrong: Two drifts. (a) The "evidence core" is the crate whose single test fails in the executed baseline: INDEX #3 — `cargo test --workspace --locked` → "1681 passed / 1 failed … fail: `apps/desktop/tests/zero_alloc_transport.rs:98` zero-alloc claim (608 bytes allocated)". The steady-state frame path allocates where the crate's contract (transport.rs:174 "zero-allocation shape", wire.rs:1082 "the measured zero-allocation proof") says it must not. (b) The prototype evidence chain is incomplete: `git ls-files | rg prototype` shows only `apps/desktop/prototype/lumen-frames.mjs`; that file's own header cites "the conformance proof is prototype/harness.mjs over the Rust-emitted golden stream" — `harness.mjs` does not exist anywhere in the tree.
Why it matters: (a) means the crate's headline claim (bounded, allocation-free frame path) is falsified by the project's own executed evidence — the "evidence core" name is unearned until the 608-byte allocation is fixed or the contract restated; (b) means the claimed browser/Node conformance proof cannot be re-run by anyone.
What correct looks like: Fix the allocation (or restate the contract and test), commit the harness, then the header is true again.
Confidence: HIGH — Verification: read + baseline cite #3 + git ls-files.

### F-003 [S4]
Where: apps/desktop/Cargo.toml:9 (cross-ref: apps/desktop/prototype/lumen-frames.mjs:7)
Code:
```
# reference implementation lives in prototype/ (browser + Node).
```
and (from lumen-frames.mjs:5-7)
```
// This module mirrors apps/desktop/src/wire.rs BYTE FOR BYTE (the
// conformance proof is prototype/harness.mjs over the Rust-emitted
// golden stream).
```
What is wrong: "prototype/ (browser + Node)" resolves to a single 15,840-byte file (`lumen-frames.mjs`); the referenced harness and any separate browser artifact are absent.
Why it matters: The manifest points auditors at an evidence surface that is half-missing; "browser + Node" implies two runnable halves.
What correct looks like: Either commit the harness/browser demo or say "a single dependency-free .mjs parser/presenter (Node + browsers), conformance harness not yet committed".
Confidence: HIGH — Verification: read + git ls-files.

### F-004 [S4]
Where: apps/desktop/Cargo.toml:20,22
Code:
```
version = "0.1.3"
description = "Lumen desktop composition-root skeleton: the ADR-003 variant-B frame-streaming prototype core (wire protocol, canvas bridge, latest-wins mailbox, command-layer host seam)."
```
What is wrong: Nothing in the description or version marks that the shipped bin is a non-functional skeleton (always exits 86) and that the real window host lives in a different crate. A user building `lumen-desktop` gets an exit-86 report tool, which the description does not disclose.
Why it matters: Packaging surfaces (`cargo run --bin lumen-desktop`) will look like a working desktop player from the manifest metadata; it is a status printer.
What correct looks like: Description should carry "binary reports the ADR-004 pending state (exit 86); the webview host is apps/desktop-tauri".
Confidence: HIGH — Verification: read + main.rs exit-path read.

## 4. Per-function table (manifest keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package] name=desktop` | 18-21 | REAL | — | workspace member via `apps/*` glob |
| `[package] description` | 22 | drift | S4 | F-004: skeleton status undisclosed |
| `[package] publish=false` | 23 | REAL | — | correct |
| `[lints] workspace=true` | 25-26 | REAL | — | inherited; member of workspace (baseline: 39/40 inherit; exception is desktop-tauri only) |
| `[[bin]] lumen-desktop` | 28-30 | REAL | — | honest skeleton: exits 86 on every path (main.rs:56,86) |
| dep `commands` | 33 | REAL | — | in boundaries internal_deps; used via lumen-cli surface (dispatch.rs) |
| dep `lumen-cli` | 34-40 | REAL | — | ARCH-5 adoption genuinely used (dispatch.rs:67,84; shell.rs:139) — verified |
| dep `media-types` | 41 | REAL | — | L0 vocabulary (bridge speaks frame/error types) |
| dep `renderer-wgpu` | 42 | REAL | — | boundaries-mirrored |
| dep `video-engine` | 43 | REAL | — | boundaries-mirrored |
| dev-dep `testkit` | 45-48 | REAL | — | used in all 4 test files (rg verified) |
| dev-dep `media-clock` | 49-54 | REAL | — | TEST-ONLY edge, used in pipeline.rs harness (rg verified); mirrors boundaries dev_deps |

## 5. Test honesty
No tests in this file; the crate's suite is the manifest's downstream evidence. Mutation reasoning for the two headline tests (read, not run): `zero_alloc_transport.rs` asserts allocation counts via a guard — it FAILED in baseline #3 (608 bytes), i.e. it is a real, non-tautological test that currently catches a real contract violation — the violation, not the test, is the defect (owner: transport/mailbox/bridge auditor). `protocol.rs`/`pipeline.rs` use the real `CliHost` via lumen-cli (composition under test is real, no mock of the subject).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO tauri/wry import … NOT imported" | manifest lines 11-16 | FALSE (stale, for the project state) | apps/desktop-tauri/Cargo.toml:1,27,53 imports tauri/wry and reuses this crate (F-001) |
| "the webview host is a typed seam pending … ADR-004" | manifest lines 12-14 | FALSE (stale) | ADR-004 admission crate landed 2026-11-13 (root Cargo.toml comment; desktop-tauri header) |
| "Std-only by C5" (this crate) | manifest line 16 | REAL | zero externals; boundaries external_allowlist = [] |
| "ADOPTS [lumen-cli] as a library dependency" | manifest lines 34-40 | REAL | rg: dispatch.rs:67,84; shell.rs:139; frame_tap.rs:34 |
| "canvas-side reference implementation lives in prototype/ (browser + Node)" | manifest line 9 | SHALLOW | one .mjs file exists; cited harness.mjs missing (F-002b/F-003) |
| "bounded latest-wins mailbox" (evidence core) | manifest lines 5-8 | REAL-UNPROVEN | bounded-ness not re-verified here; the adjacent zero-alloc claim FAILED baseline #3 (F-002a) |

## 7. Dead / duplicated / unreachable code
No dead manifest keys. The `lumen-desktop` bin is reachable (built in baseline #1) but cannot succeed by design (exit 86 always) — a status tool, not a player; UNREACHABLE-as-a-player is the honest framing, disclosed in main.rs but not in this manifest's description (F-004).

## 8. Dependencies & flags
All five deps + two dev-deps are internal path edges, mirrored exactly in boundaries.toml:538-542 (verified). No feature flags. The crate inherits workspace lints. The one structural risk — `lumen-cli` as an app→app library dependency — is declared legal at L6 ("apps/* → apps/*", ARCH-5) and is actually exercised.

## 9. Open questions for the next agent
1. transport.rs/mailbox.rs/bridge.rs auditor: pin the 608-byte allocation caught by baseline #3 (zero_alloc_transport.rs:98) to a concrete line — it falsifies the "evidence core" name in this manifest.
2. apps/desktop-tauri auditor: confirm which of the two `[[bin]]`s (`lumen-desktop-tauri` vs `lumen-tauri`, lines 24/31 of its manifest) is the shipped one, and that the shell build failure is purely sandbox-missing-libs (baseline #2) — not a repo defect.
3. Who deleted/never-committed `prototype/harness.mjs`? The reference implementation cites it as THE conformance proof.
