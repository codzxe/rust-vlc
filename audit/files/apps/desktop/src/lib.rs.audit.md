# AUDIT: apps/desktop/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2025-07-14 |
| Lines / functions | 120 / 0 logic functions (12 `pub mod` + 12 re-export blocks + 1 running doctest) |
| File verdict | REAL module surface with ONE FALSE EVIDENCE CLAIM: the "Measured *here*" cross-language conformance lane cites `prototype/harness.mjs`, which does not exist anywhere in the tree |
| Reachable from shipped entry point? | YES (the lib compiles into `lumen-desktop`; main.rs consumes `host` + `window_config` re-exports); the bulk of the re-exported surface is library-only (see dispatch.rs audit) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `cargo test -p desktop --doc` -> 1 passed, 1 ignored (the ignored one is dispatch.rs's intentional ```ignore tauri shape, verified via `--list`); `rg --files | rg harness` -> no harness.mjs; `ls apps/desktop/prototype/` -> only lumen-frames.mjs; `cargo test -p desktop --lib` -> 65 passed |

## 1. One-paragraph verdict
The crate header is the most honest document in the desktop stack — "No webview is compiled", "ships the seam, not the shell", pending cells "never fabricated" — and the module surface (12 modules, full re-exports) matches reality. But its "Evidence posture (card AC)" section claims as MEASURED, HERE, a cross-language conformance lane run "via `prototype/harness.mjs`, run under Node" — and `harness.mjs` does not exist in the tree (apps/desktop/prototype/ contains ONLY lumen-frames.mjs; a global file search finds no harness.mjs; no CI workflow or script invokes the JS decoder). The runbook doubles down (video-in-webview.md:52: "`node apps/desktop/prototype/harness.mjs` ... also runs headless in CI with Node" — no such CI step) and the compatibility matrix marks "Node (parser only) ... proven (this sandbox)". The Rust-side evidence claims (determinism, golden bytes, chunk alignment, virtual-time latency) are real and run green; the cross-language claim is unreproducible from this tree.

## 2. What actually works (verified)
— verified by `cargo test -p desktop --doc` -> `test result: ok. 1 passed` (the lib.rs mailbox doctest executes real code).
— verified by `cargo test -p desktop --lib` -> 65 passed (the protocol determinism/golden/chunk suites the header claims).
— verified by `ls apps/desktop/prototype/` -> lumen-frames.mjs present (the committed JS decoder the header describes DOES exist; only the runner is missing).

## 3. Findings
F-001 [S2]
Where: apps/desktop/src/lib.rs:47-56 (claim) vs apps/desktop/prototype/ (reality)
Code:
```
//! Measured *here*: protocol determinism (ordering, drop accounting,
//! chunk-alignment independence, golden bytes), cross-language
//! conformance (Rust encoder → the committed JS decoder via
//! `prototype/harness.mjs`, run under Node), and virtual-time
//! end-to-end latency through the real scheduler.
```
What is wrong: `prototype/harness.mjs` does not exist — `ls apps/desktop/prototype/` shows exactly one file (`lumen-frames.mjs`), and a repo-wide search for harness.mjs finds nothing. No CI workflow (.github/workflows/*) and no script runs the JS decoder. The claim "Measured *here*" therefore includes a lane that cannot be executed from this tree, and the closing sentence "never fabricated" sits directly beneath a fabricated-or-deleted evidence lane. Corroborating overclaims: docs/prototypes/video-in-webview.md:52-53 ("`node apps/desktop/prototype/harness.mjs` — the JS parser conformance over the golden stream (also runs headless in CI with Node)") and its matrix row "Node (parser only) | LUF1 v1 conformance green | ... | proven (this sandbox)". Why it matters: the desktop crate's acceptance evidence (card AC) cites a runner that is absent — anyone following the runbook hits file-not-found; the parity/feature audits would count this lane as verified when it cannot run. What correct looks like: commit harness.mjs (or correct the doc to "Rust-side golden conformance only; the JS round-trip harness is pending").
Confidence: HIGH — Verification: read + ls + rg (repo-wide).
Note for the docs auditor: if a wave log cites harness.mjs as landing evidence, this escalates to S0 (evidence cited for a claim it cannot prove).

F-002 [S4]
Where: apps/desktop/src/lib.rs:74-76 vs apps/desktop/Cargo.toml:33-39
Code:
```
//! Engine logic in the shell (doc 01 §2.7 d); bypassing the command
//! layer for UI-facing operations (ARCH-5); `unsafe`; external
//! dependencies (std-only by C5 until ADR-004 changes that).
```
What is wrong: "std-only" is true only at the manifest level (all deps are workspace path crates — verified), but the transitive graph is the ENTIRE player: `lumen-cli` (the whole CLI app: probe, demux hosts, engine wiring) plus renderer-wgpu/video-engine. A "skeleton" crate that links the complete engine stack is not std-only in any link-level sense; and desktop→lumen-cli is an apps→apps dependency (the Cargo.toml comment itself flags the layering, "apps/* → apps/* at the same L6 layer").
Confidence: HIGH — Verification: read + Cargo.toml read.

F-003 [S4]
Where: apps/desktop/src/lib.rs:26-31
Code:
```
//! Beyond the prototype core, [`diagnostics`] (w30.6, the
//! "shell-face" lane) carries the GUI-failure face the desktop
//! binaries need wherever they run: the deterministic crash-log sink
```
What is wrong: The module map at L12-24 lists five modules (wire, bridge, transport, mailbox, host) plus diagnostics — but the crate declares TWELVE `pub mod`s (L78-89: bridge, diagnostics, dispatch, error, files, frame_tap, host, mailbox, shell, transport, window_config, wire; count verified `rg -c '^pub mod'` -> 12). The header's "What this crate is" section omits dispatch/shell/files/window_config/frame_tap/error entirely (they are covered only by "Beyond the prototype core" naming diagnostics alone). The doc under-describes half the crate.
Confidence: HIGH — Verification: read + module list count.

## 4. Per-function table (no functions; module/re-export inventory)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module docs (incl. evidence posture) | 1-77 | REAL with one FALSE claim | S2(F-001) | harness.mjs lane |
| doctest (mailbox transport) | 60-70 | REAL | — | runs green |
| `pub mod` declarations ×12 | 78-89 | REAL | — | all modules exist and compile |
| re-exports (12 blocks) | 91-120 | REAL | S4(F-002) | surface consistent with modules |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| lib.rs doctest (line 60) | YES | constructs a real MailboxTransport, asserts mailbox len 0 — runs (`--doc`: 1 passed) |
| dispatch.rs doctest (line 20) | N/A — intentionally `ignore` | the future tauri shape illustration; correctly marked |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No webview is compiled" | L35 | REAL | main.rs exits 86; no tauri/wry in Cargo.toml |
| "ships the seam, not the shell" | L42-43 | REAL | whole stack library-only (dispatch.rs audit) |
| "cross-language conformance ... via prototype/harness.mjs, run under Node" | L50-52 | FALSE | file absent (F-001); no CI step |
| "those cells are recorded as pending ... never fabricated" | L55-56 | FALSE (self-contradicted by the harness claim) | F-001 |
| "prototype/lumen-frames.mjs speaks it byte-for-byte" | L12-15 | REAL-UNPROVEN | the JS decoder file exists (15,840 bytes); byte-for-byte equivalence untestable — the harness that would prove it is missing |
| "std-only by C5" | L75-76 | REAL (manifest) / misleading (transitive) | F-002 |
| "Node (parser only) ... proven (this sandbox)" | video-in-webview.md matrix | FALSE | no runner in tree (F-001) |

## 7. Dead / duplicated / unreachable code
- Re-exports of the library-only stack (dispatch/shell/bridge/transport/mailbox) are unreachable from the shipped binary — by design (F-002 of dispatch.rs audit).

## 8. Dependencies & flags
Workspace path deps only: commands, lumen-cli, media-types, renderer-wgpu, video-engine (+ dev: testkit, media-clock). No features. Release lane builds clean (baseline).

## 9. Open questions for the next agent
- Was harness.mjs ever committed and deleted (check the docs auditor's wave logs)? If a wave cites it as executed evidence, F-001 escalates to S0.
- Does the UI (ui/src) reimplement the LUF1 decoder, or does it load prototype/lumen-frames.mjs? If the UI has its own copy, the "byte-for-byte" claim needs a third-party check.
