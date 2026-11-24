# CLAIM AUDIT: docs/06-plugin-architecture.md (planning doc, 1087 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 14 |
| REAL 5 / REAL-UNPROVEN 2 / SHALLOW 0 / STUB 0 / PAPER-ONLY 5 / FALSE 2 / UNREACHABLE 0 | |
| Internal contradictions found | 2 |

## 1. Blunt verdict
The most honest planning doc audited so far: §3 carries a [LEAD-ATTENTION] blockquote that
volunteers "Nothing in this section or §4 is implemented ... no `PluginRegistry` type
exists ... `plugins/builtin/` does not exist" — and every one of those self-indictments
checks out byte-for-byte against the tree. §5 (WASM host) is not just spec: it is
implemented (5901 LOC, wasmi 2.0.0 per ADR-002, baseline test exit 0). The two FALSE
claims are enforcement promises: the §6 forbidden-list "token scan ... must return zero
hits" would fail its own scan today (live `dlopen` loaders exist in codecs/vaapi and
comments in renderer-wgpu/platform-macos, and no scan runner exists at all), and §7's
"The kit is implemented in `crates/testkit` (`ConformanceRunner`...)" — no ConformanceRunner
exists anywhere in crates/ (rg → 0), schemas/plugin.schema.json does not exist. The doc's
§5.1 runtime-candidate table is also stale (wasmi already accepted by ADR-002).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Nothing in this section or §4 is implemented ... no `PluginRegistry` type exists in `crates/plugin-api`" | :381-386 | REAL (disclosure accurate) | `rg -l 'PluginRegistry' crates/` -> crates/input/src/lib.rs only (doc comment, sed :53 verified). `ls plugins/` -> extensions/ only, no builtin/ (rg'd, ls'd). |
| 2 | 16 kinds; "`PluginKind` (`crates/plugin-api/src/kind.rs`) mirrors this section's table exactly" | :43,104-108 | REAL | kind.rs:27-76 — 16 variants in table order (census run for doc 01 audit; re-verified). |
| 3 | §5 WASM host spec (linear memory, no host pointers, lumen:v1.* imports, limits, .lumenext, crash isolation) | :633-791 | REAL | crates/plugin-host-wasm/src = 5901 LOC: instance/api/imports/limits/fsjail/http/package/install/sha256/state/supervisor; lib.rs:1-40 documents exactly these; Cargo.toml:57 `wasmi = { version = "2.0.0", ... }`. Baseline cargo-test-workspace.txt:4884-5022 "### PACKAGE plugin-host-wasm ... exit_code=0 FINISHED". |
| 4 | §9.4/§9.5 test plan (sandbox security, crash isolation, install lifecycle, examples conformance) | :1000-1022 | REAL (names differ slightly) | crates/plugin-host-wasm/tests/: plugin_sandbox_security.rs, plugin_crash_isolation.rs, plugin_install_lifecycle.rs, plugin_examples_conformance.rs, plugin_hello_world.rs (ls). Exit 0 in baseline (same cite as #3). |
| 5 | Example extensions "in `plugins/extensions/`" consumed by tests | :826-828 | REAL | plugins/extensions/{metadata-provider,ui-panel}/manifest.toml + plugin.wasm; `file` -> "WebAssembly (wasm) binary module version 0x1 (MVP)"; plugin_examples_conformance.rs:3 references them. |
| 6 | "`fuzz/registry.toml` `[PROPOSAL]` maps plugin id → target name; `tools/feature-audit` fails CI if any parser-kind plugin lacks an entry" | :921-924 | PAPER-ONLY | `ls fuzz/registry.toml` -> "no-fuzz-registry.toml"; fuzz/ has corpus + fuzz_targets only. Doc's own status note (:543-547) concedes the bijection check "has no runner today". |
| 7 | Forbidden-list row: "token scan for `dlopen(`, `LoadLibrary`, `libloading` across `crates/ apps/ tools/` must return zero hits (allowlist: empty)" | :799 | FALSE (as a repo-state claim) | `rg -n 'dlopen' crates/` -> codecs/src/video/h264/hw/vaapi/mod.rs:10,33 (runtime dlopen loader of libva.so.2 — ADR-013), renderer-wgpu/src/lib.rs:31 (Vulkan loader comment), platform-macos error.rs:22. And NO scan runner exists: `rg -l 'dlopen|LoadLibrary' scripts/ tools/` -> 0. The scan as specified both fails and is absent. |
| 8 | Dependency half: "must contain neither `libloading` nor any loader crate" | :799 | REAL (passively) | `rg -n 'libloading' Cargo.lock` -> 0 hits. (No `cargo tree` run — builds banned; lockfile census is equivalent for the normal edge set.) |
| 9 | "The kit is implemented in `crates/testkit` (`ConformanceRunner`, doc 01 §2.8)" | :825-826 | FALSE | `rg -l 'ConformanceRunner' crates/` -> 0 hits. testkit exists but has no conformance runner. The kit (§7.1-7.8) is PAPER-ONLY. |
| 10 | Manifest "schema-validated by `schemas/plugin.schema.json`" | :257-258 | PAPER-ONLY | `ls schemas/` -> commands.schema.json, generated/, settings.toml — no plugin.schema.json. (Doc marks naming [PROPOSAL]; the file simply does not exist.) |
| 11 | §5.1 runtime candidates wasmtime/wasmi with "[VERIFY-P0] ... audit results pending" | :620-640, :1055 | STALE (contradicted by ADR-002) | ADR-002 accepted 2026-10-19, wasmi 2.x (doc 01 §7 row; crates/plugin-host-wasm/Cargo.toml:9-10 "wasmi 2.0.0 — the ADR-002 runtime (accepted 2026-10-19)"). The candidate table and Q4 were never refreshed. |
| 12 | plugins/builtin registration pattern (§4.1 code sketch with `include_str!("../../plugins/builtin/...")`) | :528-548 | PAPER-ONLY | No plugins/builtin/ dir; no register() with BuiltinRegistration exists (PluginRegistry absent per #1). Doc discloses via its own status note — honest, still unimplemented. |
| 13 | `system.list-plugins` / `plugin-enable` / `plugin-disable` runtime surface (§3.4, cites doc 05) | :475-479 | REAL-UNPROVEN (command ids) | Catalog ids exist in crates/commands/src/catalog/ids.rs (per prior audits W18-A05 row 8: "Catalog ids exist (ids.rs:121-122); NO host implements them" — that audit proved apps/control-server dispatch returns NotReady). Runtime surface not wired. |
| 14 | Lifecycle events `plugins.loaded/failed/crashed` | :112,840 | REAL-UNPROVEN | supervisor.rs exists in plugin-host-wasm ("doc 05 §3 `plugins.crashed` event shapes" per lib.rs:29-30); event emission code present; end-to-end delivery to a shipped UI unverified here. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| :799 token scan "must return zero hits (allowlist: empty)" | crates/codecs/src/video/h264/hw/vaapi/mod.rs:33 "runtime `dlopen` loader" (audited ADR-013 exception) | The code is the sanctioned VA-API seam (ADR-013-va-api-hw-decoder.md exists in docs/adr/); the doc's zero-hit scan spec is wrong as written — an allowlist entry for the audited exceptions is required and none exists. |
| :620-640/:1055 runtime choice "audit results pending [VERIFY-P0]" | ADR-002 accepted 2026-10-19; wasmi 2.0.0 in Cargo.toml:57 | ADR-002 and the lockfile win; §5.1 is stale prose. |

## 4. Fabrication / inflation indicators
POSITIVE: this doc pre-empts fabrication better than any doc audited in this chunk —
the [LEAD-ATTENTION] blockquote self-reports non-implementation with re-verification dates
(:379-395), and the end-marker "When reality disagrees with this spec, the spec loses" is
honored by the code's own lib.rs doc comments. NEGATIVE indicators: §7's present tense
"The kit is implemented in crates/testkit" (:825) describes code that does not exist — a
present-tense claim of implementation with zero implementation; and §6's CI "proof
commands" column names checks that do not run (no dlopen/Lib scan script in scripts/ or
tools/). Both read as spec-aspirations written as facts.

Findings register:
- F-B001 [S3] §7 "kit is implemented in crates/testkit (ConformanceRunner)" — FALSE, no such type.
- F-B002 [S3] §6 zero-hit token scan contradicted by live dlopen loaders (vaapi) and absent runner.
- F-B003 [S4] §5.1 candidate table stale vs accepted ADR-002 (wasmi).
- F-B004 [S4] schemas/plugin.schema.json absent while §2.1 describes schema validation.
