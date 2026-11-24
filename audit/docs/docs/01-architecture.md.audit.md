# CLAIM AUDIT: docs/01-architecture.md (planning doc 01/15)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 16 |
| REAL 9 / REAL-UNPROVEN 3 / SHALLOW 0 / STUB 0 / PAPER-ONLY 3 / FALSE 1 / UNREACHABLE 0 | |
| Internal contradictions found | 2 (interim-status staleness; summary-table vs actual deps) |

## 1. Blunt verdict
This is the best kind of doc in the repo: a plan whose structural claims are checkable and
mostly checked — layer table, crate registry, lints, error taxonomy, and the CI boundary
gate all exist and match the code I verified. But one claim is flat-out FALSE on audit day
("No `platform-*` crate exists yet" — all three exist, one is 1024 LOC with passing
tests), the crate registry is stale (missing `apps/desktop-tauri`, ADR register missing
ADR-013), and the doc's central promise — ARCH-2 boundary enforcement — is currently
FAILING in the orchestrator baseline (lumen-cli -> streaming edge violation). The doc's
own slogan condemns it: "A boundary without a failing check is not a boundary — it is a
hope." The check exists, it fails, and nothing in the doc acknowledges that.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "No `platform-*` crate exists yet." | :932 | FALSE | `ls crates/` -> platform-linux, platform-macos, platform-windows all present; platform-linux alone = 1024 LOC (audio/capability/dialog/error/events/gpu/lib). Its lib.rs:3 cites "doc 01 §2.7 L5 registry row" as its authority — the code believes the doc, the doc denies the code. Baseline cargo-test-workspace.txt:3943-4003 "### PACKAGE platform-linux ... exit_code=0 FINISHED". |
| 2 | ARCH-1..9 have "a machine check attached" via `cargo xtask check-architecture` | :32-45, :1377 | REAL | tools/xtask/src/cmd/check_architecture.rs exists (loads workspace-root boundaries.toml); baseline full_gate.txt:41-52 runs it: "=== 4. check-architecture === arch: FAILED ... architecture: result=fail edges=fail cycles=pass unsafe-lints=pass static-mut=pass testkit-direction=pass". |
| 3 | ARCH-2 invariant: internal edges only via manifest allowlist (implied "every commit") | :38-39, :1386-1395 | FALSE as repo state | full_gate.txt:43-45: "[EDGE] apps/cli/Cargo.toml [dependencies]: `lumen-cli` depends on `streaming`, which is not in [crates.lumen-cli].internal_deps (doc 01 §5.2 step 3, ARCH-2)"; apps/cli/Cargo.toml:62 `streaming = { path = "../../crates/streaming" }` (sed-verified). The doc's summary row for apps/cli (:253 "engine + commands + contracts") omits streaming — internal contradiction. |
| 4 | "`clippy::await_holding_lock` is denied workspace-wide" | :1340-1341 | REAL | `sed -n '60,67p' Cargo.toml` -> lints table incl. `await_holding_lock = "deny"` (line 66). |
| 5 | "engine core is synchronous: threads + channels. No async runtime in L0–L4" | :1335-1336 | REAL | `rg -c 'async fn|tokio' crates/playback-engine/src/ crates/demux/src/ crates/audio-engine/src/` -> no hits ("no-async-hits"). |
| 6 | PluginKind sketch "mirrors the implemented enum exactly ... 16 kinds" | :344-360 | REAL | crates/plugin-api/src/kind.rs:27-76: 16 variants in identical order (Access, NetworkProtocol, Demuxer, Decoder, Encoder, AudioFilter, VideoFilter, AudioOutput, VideoOutput, SubtitleParser, SubtitleRenderer, PlaylistParser, PlaylistExporter, MetadataProvider, UiPanel, ServiceDiscovery) — rg census run. |
| 7 | Error taxonomy: 7 kinds, stage-tagged | :1270-1285 | REAL | crates/media-types/src/error.rs:13 `pub enum PipelineStage`, :190 `pub enum ErrorKind` with exactly Unsupported/Corrupt/Inaccessible/Network/Decoder/Device/ResourceLimit (sed :190-210). |
| 8 | ui/ "gets the mirror check: an import allowlist ... enforced by workspace lint" | :1429-1433 | REAL | ui/tools/eslint-plugin-local/rules/boundary-imports.js exists + boundary-imports.test.js; eslint.config.js imports it (comment: "three scaffold-local rules ... boundary-imports"). |
| 9 | UI consumes "generated artifacts only" (typed command binding via `tools/xtask schema-gen`) | :1103-1109 | REAL | tools/xtask/src/cmd/schema_gen.rs exists; ui/src/generated/commands.ts exists (123 lines) + ui/src/generated/README.md. |
| 10 | audio-output interim: "null backend, SPSC output ring + render_into ... typed OS-backend stubs (`audio.os_backend_pending`)" | :863-867 | REAL | `rg -n 'os_backend_pending' crates/audio-output/src/` -> stub.rs:72, :133, :153 (stub + tests asserting the code). |
| 11 | "The engine does **not** depend on implementation crates" (playback-engine) | :844-848 | REAL-UNPROVEN | Crates exist with the claimed names (ls crates/ full census); per-crate dep edges not re-derived here (boundaries.toml edge check partially covers; the one failure found is app-side, #3). |
| 12 | Crate registry "complete registry of MP §4's crates/, apps/ and ui/" | :203-255 | FALSE (incomplete) | apps/ contains `desktop-tauri` (ls apps/: cli, control-server, desktop, desktop-tauri, web) — absent from the doc's L6 registry rows (:251-255). Stale. |
| 13 | ADR register rows (ADR-001..012 with statuses) | :1493-1506 | REAL (self-consistent) | docs/adr/ contains ADR-001..006, 010, 011, 012 files — all match; doc admits ADR-007 has no file (:1501 "no `ADR-007-*.md` file exists") — honest. BUT ADR-013-va-api-hw-decoder.md exists in docs/adr/ and is absent from the register table — register stale. |
| 14 | Transcode crate "Phase 7 [PROPOSAL]" row exists in registry | :241 | PAPER-ONLY | crates/transcode exists as directory; content not audited in this pass (assigned elsewhere); the row is a plan row. |
| 15 | Threading roles table (command loop = sole owner of EngineSession; audio RT callback never locks/allocs) | :1321-1331 | REAL-UNPROVEN | Engine crate exists; per-thread deep verification out of scope for a doc-chunk audit; audio-callback no-alloc invariant is enforced by clippy lints partially (#4). Marked for engine-domain agents. |
| 16 | "`ui/` — React + TypeScript workspace ... all controls reflect real engine state" | :1099-1119 | REAL-UNPROVEN | ui/ workspace exists (eslint/vitest evidence in baseline ui-test.txt: 1705 passed per spec §13); "controls reflect real engine state" is a product claim contradicted elsewhere (W11: 143/150 keymap bindings wired to nothing) — but that is a UI-doc claim, not this doc's section; noted as open question. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| :932 "No `platform-*` crate exists yet" | crates/platform-{linux,macos,windows}/ exist; platform-linux/src/lib.rs:3 "doc 01 §2.7 L5 registry row"; baseline :3943-4003 tests run | The crates win; the doc paragraph is stale (written 2026-10-22 "Interim status"; crates landed later, card 7.A2). |
| :253 apps/cli deps "engine + commands + contracts" | apps/cli/Cargo.toml:62 depends on `streaming` | Cargo.toml wins; also the live gate failure (full_gate.txt:43-45) proves the doc's own checker rejects the current dependency set. |

## 4. Fabrication / inflation indicators
None material: this doc self-labels its inventions ([PROPOSAL] xN, [VERIFY] xN) and
discloses its stale ADR-011 status line (:1505 "status line still reads 'proposed' — file
refresh owed"). The one stale-FALSE claim (#1) is dated interim prose, not an inflated
capability claim. No perf or feature-capacity claims here to inflate.

## 5. Matrix rows
N/A — not a matrix chunk. (Rows 209-221 are audited separately:
audit/docs/feature-matrix.toml.rows-209-221.audit.md.)

Findings register (for this doc):
- F-A001 [S3] "No platform-* crate exists yet" FALSE vs three existing crates.
- F-A002 [S2] ARCH-2 boundary currently violated (lumen-cli->streaming), gate red in baseline — doc presents layering as enforced contract; enforcement is failing today.
- F-A003 [S4] Registry staleness: apps/desktop-tauri + ADR-013 missing from doc 01 tables.
