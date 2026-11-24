# AUDIT: plugins/extensions/ui-panel/plugin.wasm (binary)
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-24 |
| Size / type | 764 bytes / `file`: "WebAssembly (wasm) binary module version 0x1 (MVP)" — magic `0061736d 01000000` verified |
| sha256 | acfe368e199b9fc774a9fec9b0e074646e718e45fc55352414b819c7451e4d37 |
| File verdict | REAL (valid, conformance-proven, regeneration-pinned example module — example/test asset, not a shipped product surface) |
| Reachable from shipped entry point? | NO — product.toml:166 `wasm-extensions = false` (Phase 7 gate); the module is consumed by the plugin-host-wasm conformance suite only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; python3 wasm section parser (sections/imports/exports/data below); python3 sha256; `rg` over feature-matrix.toml / product.toml / conformance tests; baseline cite cargo-test-workspace.txt:4974,4977 |

## 1. One-paragraph verdict
A genuine 764-byte wasm module, not a placeholder blob: imports `lumen:v1.host::{api_version, report_manifest}`, `lumen:v1.log::log_write`, `lumen:v1.ui::ui_register_panel`; exports `init`, `on_command`, `memory`; three data segments carrying the self-reported manifest, the log line `now-playing panel example ready`, and the panel descriptor `{"panel_id":"main","title_key":"panel.example.now-playing.title","surface":"side-panel"}` — exactly matching the emitter documented in crates/plugin-host-wasm/src/bin/lumenext-examples.rs:449-470. The committed bytes are regeneration-pinned (`committed_example_modules_match_regeneration … ok`, baseline :4974) and the module runs through the real host (`ui_panel_example_passes_conformance … ok`, baseline :4977). Nothing overclaims it: the feature matrix's `plugin-wasm-extensions` row is not_started on all platforms with empty evidence, and product.toml gates extensions off.

## 2. What actually works (verified)
— verified by python3 parse: valid MVP module, sections 1/2/3/5/7/10/11 present, imports/exports/data as inventoried above; strings match lumenext-examples.rs:459-462 byte-for-byte. Baseline :4974 (regeneration match) + :4977 (conformance through the real host) both ok.

## 3. Findings
None. Checked: magic/version, section structure, import surface (deny-by-default `lumen:v1.*` capability imports only), export surface (no start section — no auto-exec; `init` is explicit, matching the `plugin_no_auto_exec` posture), data segment contents (the embedded manifest carries the empty-string digest `e3b0c442…` — the documented self-report convention, parse+identity only, never byte-equality), the sibling manifest.toml's placeholder `sha256:…(64 hex)` (the documented developer flow — `lumenext package` stamps the real digest at packaging time, package.rs:359 stamp_manifest_hash).

## 4. Doc/matrix citations
| Citation | Source | Verdict |
|---|---|---|
| plugin-wasm-extensions row: not_started everywhere, evidence = "" | docs/feature-matrix.toml (plugin-feature rows, ~:2321) | HONEST — nothing claims this binary as product evidence |
| `plugins/extensions/{metadata-provider,ui-panel}` artifacts are conformance fixtures | tests/plugin_examples_conformance.rs:3 | REAL — both example tests pass in baseline |
| Example README regeneration flow | plugins/extensions/README.md:41-45 | REAL — regeneration test passes |

## 5. Notes for the next agent
The ui-panel example exercises `lumen:v1.ui::ui_register_panel` — the only in-tree consumer of that host import outside the sandbox tests; if the plugin host ever ships, this module is the reference fixture for the ui-surface capability.
