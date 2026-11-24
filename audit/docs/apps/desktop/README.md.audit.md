# CLAIM AUDIT: apps/desktop/README.md
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-24 |
| Claims extracted | 11 |
| REAL 7 · REAL-UNPROVEN 1 · SHALLOW 0 · STUB 0 · PAPER-ONLY 0 · FALSE 3 · UNREACHABLE 0 (1 mixed) | |
| Internal contradictions found | 2 |

## 1. Blunt verdict
The README describes the apps/desktop crate as it was at card 2.D3 — a std-only, 6-file
"variant-B frame-streaming prototype core" whose cross-language conformance story runs through
a Node harness. Three of its concrete claims are false against today's tree: the crate is NOT
std-only (five workspace dependencies including renderer-wgpu and video-engine), the
"conformance harness" it tells you to run (`node prototype/harness.mjs`) does not exist
ANYWHERE in the tree (prototype/ holds exactly one file, lumen-frames.mjs — no browser
reference, no harness), and the "What is here" table lists 6 of the 14 source files, hiding
the entire grown composition root (dispatch/shell/window_config/frame_tap/files/diagnostics/
error/lib). What IS true is verified by execution: the skeleton binary is honest
(`lumen-desktop` → status report + exit 86, executed by me), the ADR-004 pending state and its
evidence record exist, the deliverable doc exists, and the crate's test suite passes (baseline
65/65). The exit-86 lane is the one claim that is exactly as advertised.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Status: the variant-B frame-streaming prototype core — std-only, no webview compiled." | :6-7 | FALSE ("std-only") / REAL ("no webview") | apps/desktop/Cargo.toml:32-44: [dependencies] commands, lumen-cli, media-types, renderer-wgpu, video-engine (read). "no webview": EXECUTED `./target/debug/lumen-desktop` → "no webview host compiled: ADR-004 pending", exit 86 |
| 2 | "The Tauri/wry import is a typed pending state behind ADR-004 (see docs/dependencies/records/tauri-wry-2.x.md ...)" | :7-11 | REAL | record exists (ls); executed binary prints "tauri-ipc pending ADR-004: tauri/wry not imported" |
| 3 | "src/wire.rs | The frame-streaming wire protocol v1 (LUF1 records ...)" | :17 | REAL | file exists; audit/files/apps/desktop/src/wire.rs.audit.md:7 "REAL" |
| 4 | "src/main.rs | The honest skeleton binary (lumen-desktop, exit 86 = webview pending)" | :22 | REAL | EXECUTED `./target/debug/lumen-desktop` → status report, EXIT=86; main.rs:21 `pub const WEBVIEW_PENDING: u8 = 86;` |
| 5 | "prototype/ | The canvas-side reference implementation (browser + Node) + the conformance harness" | :23 | FALSE | `ls apps/desktop/prototype/` → exactly one file: lumen-frames.mjs. No browser dir, no harness |
| 6 | "examples/emit_stream.rs | Emits the golden stream.bin + expected.json the Node harness checks" | :24 | FALSE (consumer half) | example exists (ls); `rg --files -g 'stream.bin' -g 'expected.json'` → no committed goldens; "the Node harness" does not exist (claim 5) |
| 7 | "cargo test -p desktop # protocol + bridge + mailbox + host suites" | :34 | REAL | audit/runs/baseline/cargo-test-workspace.txt:2519-2633: PACKAGE desktop, "65 passed; 0 failed" + main.rs unittests + doc-tests, exit_code=0 (cited, not re-run) |
| 8 | "cargo run -p desktop --example emit_stream -- --out target/proto # golden stream" | :35 | REAL-UNPROVEN | examples/emit_stream.rs exists (ls); not executed (cargo-build budget) |
| 9 | "node prototype/harness.mjs target/proto # cross-language conformance (Rust → JS)" | :36 | FALSE | `rg --files -g 'harness.mjs'` (tree-wide) → zero hits; prototype/ has no harness.mjs — the run-book command cannot run |
| 10 | "cargo run -p desktop # the honest status report (exit 86)" | :37 | REAL | EXECUTED equivalent `./target/debug/lumen-desktop` → report + exit 86 (prebuilt binary; cargo run would rebuild the same main.rs) |
| 11 | "doc 01 §2.7 registry row (L6, 'Phase 2 skeleton → Phase 4 product')" | :3-4 | REAL | docs/01-architecture.md:1045: "**(f) Phase 2 skeleton → Phase 4 product (MP §16).**" — byte-exact quote (the :1120 variant adds "+ token proof"; the README cites the :1045 row) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "std-only" (:7) | apps/desktop/Cargo.toml:32-44 lists 5 workspace [dependencies] incl. renderer-wgpu + video-engine (the 4.H1 comment: "ADOPTS [the CLI host] as a library dependency") | The manifest — read directly; the crate has been an engine-driving composition root since card 4.H1 |
| "prototype/ ... browser + Node + the conformance harness" (:23) and "the Node harness checks" (:24) | apps/desktop/src/lib.rs doctest cites the same harness; its auditor already flagged it: audit/files/apps/desktop/src/lib.rs.audit.md:7 "ONE FALSE EVIDENCE CLAIM: the ... lane cites `prototype/harness.mjs`, which does not exist anywhere in the tree" | The tree — my tree-wide rg agrees: zero harness.mjs |

## 4. Fabrication / inflation indicators
- The "cross-language conformance (Rust → JS)" story — the README's only nontrivial run-book
  step — rests on an artifact that is not in the tree (claims 5, 6, 9). This is the second
  in-tree citation of the fictional harness (lib.rs doctest is the first), so a reader meets
  the same missing file twice and may assume the loss is local rather than systemic.
- The "What is here" table (:15-24) presents a 6-file prototype while the crate ships 14 src
  files (ls: bridge, diagnostics, dispatch, error, files, frame_tap, host, lib, mailbox, main,
  shell, transport, window_config, wire) — an understatement that hides working, audited code
  (dispatch/shell/window_config/frame_tap all audited REAL by earlier waves), the inverse
  of the usual inflation but still a false inventory.
- The README never mentions that a REAL Tauri product shell exists in-tree
  (apps/desktop-tauri — apps/ listing: cli, control-server, desktop, desktop-tauri, web),
  leaving the "webview pending" status unqualified; accurate for THIS crate's release lane
  (desktop-tauri is workspace-excluded per audit/files/Cargo.toml.audit.md), misleading about
  the repo's desktop story as a whole.

## 5. Matrix rows
Not a matrix chunk — no rows. (The README cites no feature-matrix evidence strings.)

## 6. Findings (severity ledger for this doc)
- F-D1 [S3] The run-book's conformance command (`node prototype/harness.mjs`, :36) references
  a file that does not exist anywhere in the tree; prototype/'s claimed "browser + Node +
  harness" content (:23) is one file. A user following the README verbatim fails at step 3 of
  4. Evidence: claims 5/9; executed ls + tree-wide rg. Fix: restore the harness + browser
  reference, or rewrite the row and the run-book to the lumen-frames.mjs reality.
- F-D2 [S3] "std-only" (:7) is false — five workspace dependencies including a wgpu renderer
  and the video engine (Cargo.toml:32-44). The Status paragraph describes the 2.D3-era crate,
  not the 4.H1-era composition root it is now. Fix: update the Status line and say what the
  crate depends on.
- F-D3 [S4] "What is here" table lists 6 of 14 src files; the 8 unlisted files include the
  crate's dispatch/host engine wiring. Fix: complete the table or link to src/.
- F-D4 [S4] The README omits any mention of apps/desktop-tauri's existence (apps/ listing:
  cli, control-server, desktop, desktop-tauri, web), leaving the "webview pending" status
  unqualified for the repo's desktop story as a whole (accurate for THIS crate's release
  lane — desktop-tauri is workspace-excluded, per audit/files/Cargo.toml.audit.md). Claim 11's
  doc-01 quote verified byte-exact (docs/01-architecture.md:1045).

## 7. What actually works (verified)
- EXECUTED `./target/debug/lumen-desktop` → full honest status report (window spec, three
  pending webview hosts, per-OS ADR-003 landing table, frame protocol version) + "no webview
  host compiled: ADR-004 pending" on stderr, EXIT=86 — the README's most concrete promise,
  kept exactly.
- Baseline citation: PACKAGE desktop tests 65 passed / 0 failed / exit 0
  (cargo-test-workspace.txt:2519-2633).
- docs/prototypes/video-in-webview.md and docs/dependencies/records/tauri-wry-2.x.md both
  exist (ls).
