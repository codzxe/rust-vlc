# AUDIT: plugins/extensions/ui-panel/manifest.toml
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-24 |
| Lines / functions | 27 / n-a (data artifact + sibling plugin.wasm, 764 bytes, sha256 acfe368e…1e4d37) |
| File verdict | REAL — committed example artifact, drift-checked and exercised end-to-end by the conformance kit |
| Reachable from shipped entry point? | NO as a shipped artifact (plugin-host-wasm lane); consumed by `cargo test -p plugin-host-wasm --test plugin_examples_conformance` (ran green myself) and the `lumenext`/`lumenext-examples` dev bins. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | cargo test -p plugin-host-wasm --offline --test plugin_examples_conformance → 5 passed, exit 0; cmp/od/sha256sum on the wasm blobs (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
A clean example-extension manifest whose claims are all backed by executed proof: the sibling plugin.wasm is a genuine wasm MVP module (magic `00 61 73 6d 01 00 00 00` od-verified), byte-distinct from the metadata-provider example (differ at char 10; different sha256s — not a copy-paste), byte-identical to what the generator regenerates (my run of `committed_example_modules_match_regeneration`: ok), and the committed pair installs, loads, spawns under production limits and registers exactly one panel whose descriptor carries `"surface":"side-panel"` — the manifest's `surfaces = ["side-panel"]` capability claim exercised, not just declared (my run of `ui_panel_example_passes_conformance`: ok). The provenance hash ships the documented placeholder form; the placeholder is required and honest — `lumenext package`/the kit stamps the real digest before packaging, and a direct install of the placeholder is refused typed (`is_full()` gate in install.rs:494-501). One divergence from the doc 06 §2.3(d) example it claims to take its "shape" from: no `priority` field (optional; extension default 50) and no `fs-sandboxed-read` permission — both legitimate, neither contradicting the "shape" wording.

## 2. What actually works (verified)
— `cargo test -p plugin-host-wasm --offline --test plugin_examples_conformance` (my run): all 5 pass — `committed_example_modules_match_regeneration`, `ui_panel_example_passes_conformance`, `metadata_provider_example_passes_conformance`, `examples_refuse_future_api_major_with_the_typed_error`, `lumenext_cli_round_trip_packages_and_inspects_both_examples` (exit 0).
— od: plugin.wasm magic `00 61 73 6d 01 00 00 00`; `file`: WebAssembly (wasm) binary module version 0x1 (MVP); 764 bytes; sha256 acfe368e199b9fc774a9fec9b0e074646e718e45fc55352414b819c7451e4d37.
— cmp: ui-panel/plugin.wasm ≠ metadata-provider/plugin.wasm (differ at char 10) — distinct modules, no duplicated blob.
— Baseline: cargo-test-workspace.txt:4911-4917 (conformance 5 passed, exit_code=0).

## 3. Findings
F-001 [S4]
Where: plugins/extensions/ui-panel/manifest.toml:13 (absence) vs docs/06-plugin-architecture.md §2.3(d)
Code:
```
id = "ext.example.panel"
kind = "ui-panel"
version = "0.1.0"
api_version = 1
origin = "extension"
```
What is wrong: the header (line 2) claims "doc 06 §2.3 (d) shape", but the doc (d) example also carries `priority = 50`, `fs-sandboxed-read = true`, an empty `[dependencies]`, and a `[config_schema]`-less comment block; this manifest omits priority and fs-sandboxed-read entirely. The omission is legal (priority is optional — parse succeeds; the example needs no fs read — it registers a panel only), and "shape" ≠ "verbatim", but a reader diffing the two will hit unexplained deltas. What correct looks like: either add `priority = 50` for shape parity or name the deltas in the header comment. Confidence: HIGH — Verification: read + diff vs doc block.

## 4. Per-function table
Data file — keys audited instead: `id` (ext.example.panel, plugin-api id grammar, parses), `kind` (ui-panel — extension-legal in v1: asserted by conformance spawn + doc06 fixture d `extension_allowed_in_v1`), `version` (0.1.0 semver), `api_version` (1 — negotiated at init, asserted `init` returns 1), `origin` (extension — required for install, no origin forgery), `[capabilities] surfaces` (["side-panel"] — exercised in the registered panel descriptor), `[permissions] ui-surface` (true — granted() feeds the consent; conformance installs with exactly this consent), `[provenance]` (publisher/source/hash-placeholder/license — placeholder REQUIRED, stamped at package time, typed refusal on direct install).

## 5. Test honesty
| Test | Would it fail if the artifact were broken? | Reason |
|---|---|---|
| committed_example_modules_match_regeneration | YES | runs the real generator binary with `--check`; a hand-edited or stale plugin.wasm fails |
| ui_panel_example_passes_conformance | YES | installs+spawns the committed pair; asserts init handshake, manifest-report identity, exactly one panel, descriptor contains panel_id/surface verbatim — a manifest claim the module cannot demonstrate fails the suite (§7.2 honesty by construction) |
| lumenext_cli_round_trip… | YES | packages and inspects through the real dev CLI |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the ui-panel example extension (… doc 06 §2.3 (d) shape …)" | manifest.toml:2-3 | REAL (shape, not verbatim — see F-001) | structural fields match the (d) example; deltas: priority/fs-read absent |
| "The `[provenance] hash` ships the doc 06 §2.3 placeholder form; `lumenext package` stamps the real digest" | manifest.toml:5-9 | REAL | stamp_manifest_hash exercised by the kit (`Example::lumenext`), conformance green; direct install of the placeholder refused typed (install.rs is_full gate) |
| "any diverging full digest is the tamper refusal" | manifest.toml:8-9 | REAL | install.rs:503-508 HashMismatch + package.rs stamp tests (baseline ok) |

## 7. Dead / duplicated / unreachable code
None. Both committed examples are consumed by the conformance kit and dev CLI; wasm blobs verified distinct.

## 8. Dependencies & flags
None (data). Sibling plugin.wasm depends on lumen:v1.{log,ui} imports (proven by the spawned instance answering init/on_command in the kit).

## 9. Open questions for the next agent
- lumenext-examples.rs (generator) and package.rs `stamp_manifest_hash` belong to other agents: the placeholder grammar ("sha256:…(64 hex)" with U+2026) is load-bearing for the stamp regex — any loosening there should re-run this kit.
