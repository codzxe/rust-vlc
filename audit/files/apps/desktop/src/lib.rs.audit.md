# AUDIT: apps/desktop/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2026-11-21 |
| Lines / functions | 120 / 23 wiring items (12 `pub mod` declarations, 10 `pub use` blocks, 1 doctest) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (the `desktop` lib is the root of the `lumen-desktop` binary (src/main.rs, baseline #1 built it; its unittests + doctests ran) and is consumed by the workspace-excluded tauri wrapper `apps/desktop-tauri` (build UNVERIFIED in sandbox, baseline #2); doctest executed: `test apps/desktop/src/lib.rs - (line 60) ... ok`, orchrun-test-workspace-nofailfast.log.txt:6377-6381) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "status" docs/adr/ADR-004-os-webview.md` (Status: accepted 2026-10-22); `rg -n "accepted 2026-10-22" apps/desktop/Cargo.toml` (crate header corrected, :13); `rg -n "^tauri" apps/desktop-tauri/Cargo.toml` (tauri = "2" at :60 — the import exists); `rg --files -g "harness.mjs"` (NO HITS — the file lib.rs cites does not exist); `ls apps/desktop/prototype/` (only lumen-frames.mjs); `rg -n "harness" .github/workflows/*.yml` (no CI runner for any prototype .mjs); `sed -n '40,70p' docs/prototypes/video-in-webview.md` (run-book :52 claims the harness "also runs headless in CI with Node" — also false); `rg -n "unsafe" apps/desktop/src/` (only doc mentions — the Never claim holds); `rg -n "Doc-tests desktop" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (doctest ok :6377-6381) |

## 1. One-paragraph verdict
A facade: 12 module declarations, 10 re-export blocks, one doctest — all re-exports compile (baseline #1 built the crate; the doctest ran and passed). The code wiring is honest; the documentation is not, in two places. The "What this crate is not" section states "ADR-004 status: planned" — ADR-004 has been ACCEPTED since 2026-10-22, its own header records the triple-contradiction reconciliation (ROADMAP/doc 01/Cargo.toml header all corrected to accepted), and this lib.rs line is a stale fourth site of exactly the contradiction that ADR claims closed — while the shipped wrapper (apps/desktop-tauri/Cargo.toml:60) does import `tauri = "2"`. The "Evidence posture" section claims "Measured *here*: … cross-language conformance (Rust encoder → the committed JS decoder via `prototype/harness.mjs`, run under Node)" — `prototype/harness.mjs` does not exist anywhere in the tree (the prototype dir contains only `lumen-frames.mjs`), and no CI workflow runs Node over it: a claimed measurement with no artifact.

## 2. What actually works (verified)
- Doctest (lines 60-70) executed: `test apps/desktop/src/lib.rs - (line 60) ... ok` — orchrun-test-workspace-nofailfast.log.txt:6377-6381 (Doc-tests desktop: 1 passed; the sibling dispatch.rs doctest is `ignored`, that module's owner's concern).
- All 12 declared modules exist and all 10 re-export lists compile — `cargo build --release --workspace` CLEAN (baseline #1) is the existence proof.
- The "Never: `unsafe`" claim verified: `rg -n "unsafe" apps/desktop/src/` returns only prose mentions (lib.rs:75, diagnostics.rs:14-15 explaining the FFI half lives in the excluded crate).
- The Rust-side half of the evidence posture is real: protocol determinism/latency tests exist under apps/desktop/tests/ (protocol.rs executed in baseline; zero_alloc_transport is the desktop lane's known 1 failure, unrelated to this facade).

## 3. Findings
### F-001 [S3]
Where: apps/desktop/src/lib.rs:35-38
Code:
```
//! **No webview is compiled.** `tauri`/`wry` are allowlist §5
//! "exception-ADR-004" (pre-audited, not imported; ADR-004 status:
//! planned), and this build environment cannot honestly host them:
```
What is wrong: "ADR-004 status: planned" is false. ADR-004's own header: "**Status:** accepted (2026-10-22 — formalized by card 3.DOC-1 …)" and its Supersedes note records that ROADMAP §6 / doc 01 §7 / `apps/desktop/Cargo.toml` header were "all three reconciled to *accepted*" — the crate's Cargo.toml:13 indeed carries "ADR-004 (accepted 2026-10-22 …)". lib.rs was missed in that reconciliation. Additionally "not imported" and "The skeleton therefore ships the seam, not the shell" (:42-43) frame a state that no longer holds project-wide: the wrapper crate imports `tauri = "2"` (apps/desktop-tauri/Cargo.toml:60) and per ADR-004/KNOWN-LIMITS the shell was "BUILT and LAUNCHED on all three OSes" (w28, lumen-tauri.exe).
Why it matters: This is the fourth site of the exact status contradiction ADR-004 exists to close; a reader of the crate root concludes the webview decision is still open when it is accepted and shipped. Doc-drift in the file whose job is to state what the crate is.
What correct looks like: "(pre-audited; imported only by the workspace-excluded wrapper — ADR-004 accepted 2026-10-22; this crate stays std-only)".
Confidence: HIGH — Verification: read (ADR-004 header, Cargo.toml:13, desktop-tauri/Cargo.toml:60 all read directly).

### F-002 [S3]
Where: apps/desktop/src/lib.rs:49-53
Code:
```
//! Measured *here*: protocol determinism (ordering, drop accounting,
//! chunk-alignment independence, golden bytes), cross-language
//! conformance (Rust encoder → the committed JS decoder via
//! `prototype/harness.mjs`, run under Node), and virtual-time
//! end-to-end latency through the real scheduler.
```
What is wrong: The claimed measurement instrument does not exist. `rg --files -g "harness.mjs"` over the whole repo: no hits; `apps/desktop/prototype/` contains only `lumen-frames.mjs`. No `.github/workflows/*.yml` invokes `node` on any prototype file (rg "harness" over workflows: zero). The companion run-book repeats the fiction — docs/prototypes/video-in-webview.md:52 "`node apps/desktop/prototype/harness.mjs` — the JS parser conformance over the golden stream (also runs headless in CI with Node)" and its matrix row "Node (parser only) … proven (this sandbox)".
Why it matters: "Measured *here*" is an evidence claim; the cross-language conformance cell is PAPER-ONLY — no harness, no runner, no recorded output. In a repo whose stated rule is "recorded as pending … never fabricated" (three lines below), a fabricated measurement citation is exactly the defect class this audit exists to flag. The Rust-side determinism/latency halves of the claim are real; the JS half is not.
What correct looks like: Either commit the harness (a ~50-line Node driver over lumen-frames.mjs and the golden stream) and wire it into CI, or reword to "the JS decoder is committed; its conformance harness is owed (pending, not measured)".
Confidence: HIGH — Verification: read + rg (file absence and CI absence both verified).

### F-003 [S4]
Where: apps/desktop/src/lib.rs:72-76
Code:
```
//! # Never
//!
//! Engine logic in the shell (doc 01 §2.7 d); bypassing the command
//! layer for UI-facing operations (ARCH-5); `unsafe`; external
//! dependencies (std-only by C5 until ADR-004 changes that).
```
What is wrong: "external dependencies (std-only…)" is true only at the direct-dependency level: apps/desktop/Cargo.toml depends on five workspace path crates, one of which (renderer-wgpu) necessarily pulls the external wgpu graph. The claim reads as "no external code in this crate's build" which the dependency graph contradicts transitively.
Why it matters: Wording-level: the C5 rule is about this crate's own source and direct manifest; a reader could take "std-only" to describe the compiled artifact.
What correct looks like: "std-only source, workspace-internal direct deps (the wgpu graph enters only via renderer-wgpu)".
Confidence: HIGH — Verification: read (Cargo.toml [dependencies] read in full).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs: "What this crate is" | 5-31 | REAL | — | The wire/bridge/transport/mailbox/host/diagnostics map matches the declared modules. |
| module docs: "What this crate is not" | 33-45 | SHALLOW (stale) | S3 | F-001: ADR-004 status false; seam-not-shell framing outdated. |
| module docs: "Evidence posture" | 47-56 | SHALLOW | S3 | F-002: harness.mjs does not exist; the "never fabricated" sentence sits directly under a fabricated citation. |
| doctest example | 58-70 | REAL | — | Executed ok (no-failfast :6377-6381); uses testkit dev-dep correctly. |
| `pub mod` × 12 | 78-89 | REAL | — | bridge, diagnostics, dispatch, error, files, frame_tap, host, mailbox, shell, transport, window_config, wire — all files exist (build #1). |
| `pub use bridge::{…}` | 91 | REAL | — | Compiles (build #1). |
| `pub use dispatch::{…}` | 92-95 | REAL | — | Compiles. |
| `pub use error::{…}` / `files::{…}` / `host::{…}` | 96-104 | REAL | — | Compiles. |
| `pub use mailbox::{…}` / `shell::{…}` | 105-109 | REAL | — | Compiles. |
| `pub use transport::{…}` | 110-113 | REAL | — | Compiles. |
| `pub use window_config::{…}` | 114-116 | REAL | — | Compiles; the surface my window_config.rs audit covers. |
| `pub use wire::{…}` | 117-120 | REAL | — | Compiles. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| lib.rs doctest (line 60) | YES (weakly) | Asserts `mailbox_len == 0` on a fresh transport — catches a constructor that pre-fills; trivial but real, and it executed. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Evidence |
|---|---|---|---|
| "tauri/wry … ADR-004 status: planned" | lib.rs:36-37 | FALSE | ADR-004 header "Status: accepted (2026-10-22)"; Cargo.toml:13 reconciled; wrapper imports tauri="2" (F-001). |
| "cross-language conformance … via `prototype/harness.mjs`, run under Node" | lib.rs:51-52 | FALSE (PAPER-ONLY) | File absent (rg --files), no CI runner (F-002). |
| "prototype/lumen-frames.mjs speaks it byte-for-byte" | lib.rs:14-15 | REAL-UNPROVEN | The .mjs exists; the byte-for-byte conformance proof is the missing harness (F-002). |
| "NOT measurable here: every per-webview cell … recorded as pending … never fabricated" | lib.rs:53-56 | REAL | Matrix rows in video-in-webview.md are pending/to-run (except the Node row, see F-002). |
| "Never: … `unsafe`" | lib.rs:75 | REAL | rg unsafe in apps/desktop/src: prose only. |
| "external dependencies (std-only by C5)" | lib.rs:76 | SHALLOW | Direct deps all workspace-internal; wgpu enters transitively via renderer-wgpu (F-003). |
| Node row "proven (this sandbox)" | docs/prototypes/video-in-webview.md matrix | FALSE | Depends on the nonexistent harness (cross-file; flagged for that doc's owner). |

## 7. Dead / duplicated / unreachable code
None in the wiring: every `pub use` item is reachable through the facade by the binary, the tests, or the tauri wrapper (window_config surface verified by my window_config.rs audit). No dead modules — all 12 are consumed (build + tests + wrapper).

## 8. Dependencies & flags
[dependencies]: commands, lumen-cli (the composition-root adoption of the CLI's host — ARCH-5 note in Cargo.toml), media-types, renderer-wgpu, video-engine — all workspace path deps. [dev-dependencies]: testkit, media-clock (documented dev-edge justification). `[lints] workspace = true`. No feature gates. The ADR-004 allowlist posture (no tauri/wry here) holds for this crate's manifest.

## 9. Open questions for the next agent
- Who owes `prototype/harness.mjs` (F-002)? The run-book (video-in-webview.md:52) promises it "runs headless in CI with Node" — either the file was deleted post-w30 (check git history, blocked: SESSION-2 no git) or it never existed; either way the lib.rs citation and the doc's "proven (this sandbox)" row need an owner.
- apps/desktop-tauri owners: confirm the wrapper's docs don't inherit lib.rs's "status: planned" framing (app.rs was clean on this point in my spot reads).
