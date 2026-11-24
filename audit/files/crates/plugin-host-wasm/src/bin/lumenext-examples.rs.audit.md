# AUDIT: crates/plugin-host-wasm/src/bin/lumenext-examples.rs
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2025-06-27 |
| Lines / functions | 625 / 25 |
| File verdict | REAL (dev tool; not shipped) |
| Reachable from shipped entry point? | NO — plugin-host-wasm has zero workspace dependents (rg over all Cargo.toml); release lane builds only `-p lumen-cli -p desktop`; product.toml:166 `wasm-extensions = false`. Reachable from the TEST lane: plugin_examples_conformance.rs:167 runs the binary via `CARGO_BIN_EXE_lumenext-examples` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `file plugins/extensions/*/plugin.wasm` -> "WebAssembly (wasm) binary module version 0x1 (MVP)"; python3 independent section/exports parse (below); `rg -n "stamp_manifest_hash" crates/plugin-host-wasm/src/package.rs` -> :359; `rg "plugin-host-wasm" --glob Cargo.toml` -> no dependents; baseline citation cargo-test-workspace.txt:4971-4981 (5/5 conformance ok) |

## 1. One-paragraph verdict
A genuine, working, std-only wasm binary emitter + drift-gate tool. It hand-encodes sections in canonical order (1,2,3,5,7,10,11), LEB128 correct, function-index arithmetic (`imports.len() + funcs.len() - 1`, lumenext-examples.rs:153) correct, and the two generated guests really load and pass conformance in the real wasmi host (baseline: `metadata_provider_example_passes_conformance ... ok`, `ui_panel_example_passes_conformance ... ok`). The committed blobs are valid wasm (`file` confirms) and my independent Python parse of the committed binaries shows exactly the exports and data the generator claims to emit. It is a maintainer/CI tool, not shipped: no crate depends on plugin-host-wasm, and the extensions feature is parked (`wasm-extensions = false`, Phase 7). Defects are hygiene-grade: a duplicated emitter twin of tests/common/mod.rs, a self-reported manifest with a knowingly-false provenance hash, and duplicated drift output.

## 2. What actually works (verified)
— Baseline (executed by orchestrator) audit/runs/baseline/cargo-test-workspace.txt:4971-4981: `committed_example_modules_match_regeneration ... ok`, `examples_refuse_future_api_major_with_the_typed_error ... ok`, `metadata_provider_example_passes_conformance ... ok`, `ui_panel_example_passes_conformance ... ok`, `lumenext_cli_round_trip_packages_and_inspects_both_examples ... ok` — 5/5. This proves: generator output == committed bytes (drift gate), and the generated modules load and run under the production host.
— My independent parse (python3, this session): metadata-provider/plugin.wasm exports `init`(func 6), `extract`(func 7), `memory`(mem 0) — exactly the 6-imports+2-funcs layout the generator builds; ui-panel/plugin.wasm exports `init`(func 4), `on_command`(func 5), `memory` — exactly 4 imports + 2 funcs; data segments contain `ext.example.*`, `provides`/`surfaces`, `fs-sandboxed-read`/`ui-surface` markers. Structure matches the code, not just the docs.

## 3. Findings
F-001 [S4]
Where: crates/plugin-host-wasm/src/bin/lumenext-examples.rs:41-44 and tests/common/mod.rs:95-142
Code:
```
// ---------------------------------------------------------------------------
// The wasm binary emitter (public-spec format, std-only — the trimmed
// twin of the fixture emitter in tests/common/mod.rs)
// ---------------------------------------------------------------------------
```
What is wrong: Two hand-rolled wasm emitters exist in one crate — this one and the integration-fixture one (`fn uleb` tests/common/mod.rs:95, `struct Wasm` :142, `finish` :238). A format bug fixed in one (e.g. a LEB128 edge) silently misses the other. Why it matters: the drift gate pins committed bytes to THIS emitter; the conformance fixtures are built by THAT emitter; divergence between them is invisible to CI. What correct looks like: one shared emitter module (test-util crate or `tests/common` re-exported to the bin). Confidence: HIGH — Verification: read + rg (both emitters located). Acknowledged in the comment, so hygiene not fraud.

F-002 [S4]
Where: crates/plugin-host-wasm/src/bin/lumenext-examples.rs:344-356
Code:
```
fn report_manifest(id: &str, kind: &str, capabilities: &str, perms: &str) -> String {
    // The example provenance placeholder URL (RFC 2606 example.com) —
    // fixture payload carried inside the guest's data segment, not a
    // config endpoint the product ever contacts.
    let source = format!("https://example.com/{id}-0.1.0.lumenext"); // lumen-allow-literal: doc 06 §2.3 example provenance placeholder (RFC 2606 example.com) — fixture payload, not a config endpoint
```
What is wrong: The guest's self-reported manifest embeds `hash = "sha256:e3b0c44…"` (the digest of the empty string, L341-343 admits it) and a placeholder source URL. The self-reported manifest's `[provenance]` block is therefore false data that the host must deliberately never validate against the real package digest. Why it matters: the honesty of the F1 handshake depends on the host ignoring fields it cannot trust; if a future host version starts cross-checking the guest-reported hash, these shipped example modules fail to load with no drift-gate warning (bytes are pinned as "correct"). What correct looks like: omit `[provenance]` from the guest-embedded manifest entirely (it is metadata about the PACKAGE, not the guest), or assert in conformance that the host ignores it. Confidence: MEDIUM — Verification: read; the README (plugins/extensions/README.md:16-18) confirms the placeholder convention and that packaging stamps the real digest (`stamp_manifest_hash` package.rs:359 exists — verified).

F-003 [S4]
Where: crates/plugin-host-wasm/src/bin/lumenext-examples.rs:594-599
Code:
```
if check {
    eprintln!(
        "drift: {shown} ({} bytes committed, {} regenerated)",
        current.map_or(0, |c| c.len()),
        bytes.len()
    );
```
What is wrong: Drift is reported twice — once per module here, then again by `main` via `eprintln!("lumenext-examples: {err}")` (L552) with the aggregate `Failure::Drift`. Also `current.map_or(0, …)` prints "0 bytes committed" for a MISSING file, which reads as "empty file" not "absent". Why it matters: CI log consumers see two drift formats; the missing-file case is misdescribed. What correct looks like: one report format; print "missing" when `current.is_none()`. Confidence: HIGH — Verification: read (both print sites in file).

## 4. Per-function table (mandatory, file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `uleb` | 57-67 | REAL | — | unsigned LEB128, correct |
| `sleb` | 70-81 | REAL | — | sign-extend termination condition correct |
| `section` | 83-87 | REAL | — | id + uleb len + body |
| `name` | 89-92 | REAL | — | len-prefixed utf8 |
| `Body`/`Wasm` structs | 96-110 | REAL | — | plain builders |
| `Wasm::new` | 113-123 | REAL | — | mem always Some((1,16)) |
| `Wasm::ty` | 125-135 | REAL | — | dedupe by linear scan, fine at this size |
| `Wasm::import` | 138-143 | REAL | — | returns import-space func idx |
| `Wasm::func` | 146-154 | REAL | — | idx = imports+funcs-1, correct |
| `Wasm::export_func` | 156-158 | REAL | — | kind 0x00 |
| `Wasm::data` | 160-162 | REAL | — | active segment, offset emitted later |
| `Wasm::finish` | 166-251 | REAL | — | canonical section order 1,2,3,5,7,10,11; memory export auto-add; body `end` append; data active form `0x00 0x41 sleb 0x0B` — all correct (verified against committed blobs via python3 parse) |
| `code` | 257-263 | REAL | — | concat |
| `i32c`/`call`/`dropv`/`lget`/`lset` | 266-296 | REAL | — | opcodes 0x41/0x10/0x1A/0x20/0x21 correct |
| `i32load` | 300-302 | REAL | — | 0x28 align 2 offset 0 |
| `eqz`/`ifv`/`endv` | 305-317 | REAL | — | 0x45 / 0x04 0x40 / 0x0B |
| layout consts | 324-337 | REAL | — | REPORT_AT 0, MSG_AT 1024, HANDLE_AT 2048, OUT_LEN_AT 2052, RESULT_AT 4096, READ_CAP 4096 — no overlap (manifest ~397B < 1024); README documents same offsets |
| `report_manifest` | 344-356 | REAL | S4 | F-002 placeholder provenance |
| `metadata_module` | 371-446 | REAL | — | init + extract bodies; local indices checked (params 0,1 + 1 declared = status at 2); status propagation correct |
| `panel_module` | 460-512 | REAL | — | init + on_command; on_command tail-calls log_write result as return — correct |
| `Failure::code` | 529-534 | REAL | — | Usage/Drift=1, Io=2; matches doc L32-33 |
| `Display for Failure` | 537-545 | REAL | — | prefixed messages |
| `main` | 547-556 | REAL | — | exit-code mapping |
| `run` | 558-625 | REAL | S4 | F-003 duplicate drift output; missing-file counted as drift (correct behavior, wrong label) |

## 5. Test honesty (if tests exist)
No tests inside this file (baseline line 4953-4959: `running 0 tests`). The file is tested EXTERNALLY by plugin_examples_conformance.rs (5/5 ok, baseline 4971-4981), which runs the real binary (`--check` drift gate), loads the generated modules in the real host, and drives the full protocols. That is a strong external harness — the tool's outputs are pinned and executed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "emits the committed plugin.wasm binaries … deterministically" | L2-3 | REAL | baseline `committed_example_modules_match_regeneration ... ok` (drift gate executed) |
| "`--check` regenerates and diffs them, so a stray edit … is a loud CI failure" | L10-12 | REAL | run() L594-617 compares bytes, exit 1 on drift; conformance test invokes it (plugin_examples_conformance.rs:167) |
| "no `wat`/`wasm-tools` dependency" | L5-6 | REAL | Cargo.toml [dev-dependencies] has no wasm-tools; bin uses only std |
| "the examples are real loadable modules" | L8-9 | REAL | baseline `metadata_provider_example_passes_conformance ... ok`, `ui_panel_example_passes_conformance ... ok` |
| "the INSTALLED manifest's digest is the real one, enforced at install and every load" | L341-343 | REAL | `stamp_manifest_hash` exists (package.rs:359, rg verified); enforcement proven by executed tests: install refuses HashMismatch (plugin_install_lifecycle.rs:92-100) and load re-verifies InstalledHashMismatch on tamper (:397-408), both green in baseline :4990-4996 |
| README: "packaging stamps the real digest … `stamp_manifest_hash` in package.rs" | plugins/extensions/README.md:16-18 | REAL | rg verified :359 + tests :483-516 |

## 7. Dead / duplicated / unreachable code
Duplication: the emitter twin (F-001). No dead code found. Unreachable from shipped lane by design (dev tool; parked feature).

## 8. Dependencies & flags
std-only (fmt, fs, path, process). No feature gates. No unsafe. Built as workspace `[[bin]]` of plugin-host-wasm (Cargo.toml:76-83). wasmi is a lib dependency of the package but this bin does not use it.

## 9. Open questions for the next agent
- If a future phase enables wasm-extensions (product.toml:166), will the examples' placeholder provenance (F-002) survive a host that starts validating guest-reported manifests? Owner: plugin-host lib auditor.
