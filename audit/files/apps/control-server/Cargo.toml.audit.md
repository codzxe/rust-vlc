# AUDIT: apps/control-server/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-21 |
| Lines / functions | 48 / config keys: package, lib, bin, dependencies×5, lints |
| File verdict | UNREACHABLE (real, compiling, tested code — but no shipped entry point reaches it: bin not in any packaging payload, product gate off, zero dependents) |
| Reachable from shipped entry point? | NO. Proof: (a) `packaging/packaging.toml` payload comment declares shipped binaries = BIN_CLI + BIN_TAURI only; (b) `grep -rn lumen-control-server packaging/ scripts/ .github/ product.toml` → 0 hits; the name exists only inside apps/control-server's own 3 files; (c) no crate/app's [dependencies] names control-server (only a desktop-tauri comment); (d) product.toml:167 `control-server = false` — and main.rs exits 2 when the flag is off. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | boundaries.toml:631-636 diff (exact match); `rg -l '<crate>::' apps/control-server/src` (all 5 deps used); `grep -rn lumen-control-server` repo-wide (3 self-hits only); packaging.toml payload read; product.toml:167; main.rs exit-contract read; baseline #1 (compiles in workspace build), #3 (e2e in 1681-pass run) |

## 1. One-paragraph verdict
A small, tidy manifest — three internal deps, two externals, a lib+bin pair — matching boundaries.toml exactly, with every dependency genuinely used and an honest exit-code contract in the binary (exit 2 when the product flag is off, main.rs:5-9). The problem is product-level: this app ships nowhere. Its binary is absent from every packaging payload (tar.gz/zip/deb/rpm ship BIN_CLI+BIN_TAURI; AppImage/dmg ship BIN_TAURI only — packaging.toml payload block), nothing embeds its lib, and the product gate that would enable it (`product.toml:167 control-server = false`) is off. It is compiled into every `--workspace` build and its full-stack e2e runs in every test lane — real code, zero shipped reachability.

## 2. What actually works (verified)
— Boundaries match: manifest deps {commands, control-protocol, product-config} == boundaries.toml:633; externals {serde_json, toml} == :635; dev_deps [] == :634 (tests/e2e.rs needs none — it uses the package's own regular deps, legal for integration tests).
— All deps used: commands 3 src files, control_protocol 5, product_config 2, serde_json 1, toml 1.
— Compiles + tests run: baseline #1 (workspace build CLEAN) and #3 (1681 passed includes apps/control-server/tests/e2e.rs — real loopback sockets per its header, doc 10 D-5).
— Honesty of the off-state: e2e.rs:10-11 flips the flag "in a temp copy — the repo's own flag is honestly `false`"; main.rs documents exit 2 for "feature flag off".

## 3. Findings
### F-001 [S3]
Where: apps/control-server/Cargo.toml:1-3,36-38 (whole app vs packaging reality)
Code:
```
# apps/control-server — the optional loopback control surface (card
# 7.J1, issue #50; doc 01 §2.8 registry row: layer L6, "control-
# protocol, commands, engine, contracts"; doc 05 §5.4).
```
What is wrong: The manifest presents the app as "the optional loopback control surface" without disclosing that nothing ships it: the binary `lumen-control-server` is absent from packaging/packaging.toml's payload (BIN_CLI + BIN_TAURI + docs only), absent from all packaging scripts and workflows, has zero dependents, and its enabling product flag is `false` (product.toml:167). Every workspace build and test lane compiles and executes an app that no user can obtain.
Why it matters: Dead product weight at app scale — build time, supply-chain surface (serde_json/toml graph), and a standing illusion in doc 01 §2.7's "optionally embeds the control server" story: the embedder named there (desktop composition) does not exist as a dependency edge. Auditors and users reading this manifest would believe a control server is part of the product; it is part of the repo only.
What correct looks like: Either the manifest header states "repo-only, not in any packaging payload, gate off pending Lead ratification", or the app is moved to a CI/test-only lane, or it ships.
Confidence: HIGH — Verification: read (packaging.toml, product.toml:167, repo-wide grep) + baseline #1/#3.

### F-002 [S4]
Where: apps/control-server/Cargo.toml:44-45
Code:
```
serde_json = "1.0"
toml = "1"
```
What is wrong: Loose version specs, same deviation from the house exact-pin admission style as apps/cli/Cargo.toml:64-65.
Why it matters: Inconsistent pin discipline; doc 11 §3.7 bump-re-review trigger does not fire for these.
What correct looks like: `=1.0.154` / `=1.1.8` (current lock resolutions) or a documented family-wide exception.
Confidence: HIGH — Verification: read (lock cited in file-3 audit).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` | 25-30 | REAL | — | description "token-authenticated, deny-by-default, config-driven" — behavior claims for source audit; gating honesty verified in main.rs. |
| `[lib] name="control_server"` | 32-34 | REAL (repo-internal only) | — | Only consumer found: its own tests/e2e.rs. No app/crate depends on it. |
| `[[bin]] name="lumen-control-server"` | 36-38 | UNREACHABLE | S3 (F-001) | Builds, tested indirectly; packaged nowhere. |
| `commands` | 41 | REAL | — | Used 3 files. |
| `control-protocol` | 42 | REAL | — | Used 5 files. |
| `product-config` | 43 | REAL | — | Used 2 files (schema-validated product model). |
| `serde_json` / `toml` | 44-45 | REAL | S4 (F-002) | Used 1 file each. |
| `[lints] workspace = true` | 47-48 | REAL | — | Verified inherited (file-1 loop-grep). |

## 5. Test honesty
No tests in manifest. tests/e2e.rs drives the FULL app stack (product.toml temp-copy with flag flipped → control-server.toml → resolve_server_config → ControlServer::bind → serve_loop → TestClient over real loopback WS) — mutation-sensitive by construction (any break in bind/dispatch/protocol fails it); included in baseline #3's 1681 passes. Detailed per-test honesty: source agents.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "implementing it now RATIFIES the proposal" | Cargo.toml:5-6 | SHALLOW | Implementation exists, but the ratification gate (product.toml:167 `control-server = false # Phase 5+ [PROPOSAL]`) is still off — the proposal is implemented, not enabled; e2e.rs:10-11 admits the flag "is honestly `false` until the Lead ratifies". |
| "v1 serves the catalog-introspection subset and answers everything else with a typed not-ready — an honest gap, never a silent no-op" | Cargo.toml:14-16 | REAL-UNPROVEN | Dispatch not traced here (source agents); e2e.rs exercises dispatch and asserts typed outcomes (imports CommandOutcome/ProtocolErrorCode). |
| "doc 01 §2.7 'optionally embeds the control server'" | Cargo.toml:12-14 | PAPER-ONLY (embedder side) | No desktop/embedder crate depends on control-server (grep over all manifests; only a desktop-tauri comment mentions its types). |

## 7. Dead / duplicated / unreachable code
The entire app is unreachable from shipped entry points (F-001) while being fully compiled and tested — the precise inverse of dead code: alive in CI, dead in product.

## 8. Dependencies & flags
No `[features]`, no dev-deps, no build-deps. Layer 6 app per boundaries.toml:632. All three internal edges point downward (commands L1, control-protocol L1, product-config lower). Consistent.

## 9. Open questions for the next agent
- If Lead ever flips product.toml:167, the packaging payload must gain the binary — is there a tracking card? None referenced in this manifest.
- The engine-embedding join (playback-engine's EngineSession behind dispatch, :12-14) is named as future work — feature-matrix rows claiming remote control should be checked against this gap by matrix-rows agents.
