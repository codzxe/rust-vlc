# AUDIT: plugins/extensions/metadata-provider/plugin.wasm (BINARY)
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-24 |
| Size / sha256 | 764 bytes / ed7ec5bef684d20f36d0e0bb95ede58feb0caecd8c0777071cac6f6eea39ce82 |
| Type / magic | WebAssembly (wasm) binary module version 0x1 (MVP) — magic bytes `0061736d` (`\0asm`), version 1 |
| File verdict | REAL (loaded and executed by its conformance suite — I ran it) |
| Reachable from shipped entry point? | NO (the plugin-host crate ships nowhere; the example is a developer artifact) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `stat`, `sha256sum`; `which wasm-tools wasm2wat wasm-objdump` -> none (fallback: hand parse via python, below); `cargo test -p plugin-host-wasm --test plugin_examples_conformance` -> 5 passed, 0 failed |

## 1. One-paragraph verdict
A genuine, tiny (764-byte) wasm extension module, byte-identical to what the
repo's own `lumenext-examples` generator regenerates (the suite's drift check
passes), loaded and executed through the real host path by
`metadata_provider_example_passes_conformance` — which I ran: it spawns the
module under production limits with a real PathJail, performs the api-version
handshake, reads a real generated sidecar through the fs lane, and asserts the
exact tag bytes. Imports/exports parsed by hand (no wasm-tools in this
sandbox): 6 imports all inside `lumen:v1.*` and exactly matching the api.rs
catalog, 3 exports.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p plugin-host-wasm --test
plugin_examples_conformance` -> "test result: ok. 5 passed; 0 failed"
(incl. committed_example_modules_match_regeneration,
metadata_provider_example_passes_conformance, lumenext_cli_round_trip...).
— verified by python parse of the binary (wasm-tools unavailable, exit 1):

```
IMPORT: ('lumen:v1.host', 'api_version', 'func type=0')
IMPORT: ('lumen:v1.host', 'report_manifest', 'func type=1')
IMPORT: ('lumen:v1.log', 'log_write', 'func type=2')
IMPORT: ('lumen:v1.fs', 'fs_open', 'func type=2')
IMPORT: ('lumen:v1.fs', 'fs_read', 'func type=3')
IMPORT: ('lumen:v1.fs', 'fs_close', 'func type=4')
EXPORT: ('init', 'func', 6)
EXPORT: ('extract', 'func', 7)
EXPORT: ('memory', 'memory', 0)
```
Sections: type(1)/import(2)/function(3)/memory(5)/export(7)/code(10)/data(11).

## 3. Findings
None. What was checked, and how:
- Magic/version: `0061736d` + version 1 (file(1) + python struct read).
- Drift: the committed blob == regeneration output (test executed, passes) —
  no silent hand-edits.
- Import policy consistency: the module's 6 imports need `base` (host, log)
  and `fs-sandboxed-read` (fs); manifest.toml declares exactly
  `fs-sandboxed-read = true` — spawn would be refused otherwise (imports.rs
  check_import_policy), and the spawn succeeds.
- Manifest pairing: id `ext.example.metadata`, kind `metadata-provider`
  (v1-legal kind per kind.rs), api_version 1 (inside the [1,1] window),
  origin `extension` (not a forged builtin). [provenance] hash ships the
  documented placeholder `sha256:…(64 hex)`; `lumenext package` stamps the
  real digest (stamp_manifest_hash) and any diverging full digest is the
  tamper refusal — placeholder-by-design, verified by the round-trip test.
- Doc/matrix citation check: no feature-matrix row cites this binary as
  evidence (docs/feature-matrix.toml:2043 discusses the metadata-provider
  KIND as a design note only) — nothing fabricated at the binary's expense.

## 4. Provenance / consumers
- Generator: `crates/plugin-host-wasm/src/bin/lumenext-examples.rs` (the
  in-repo wasm emitter; drift-checked).
- Loading tests: plugin_examples_conformance.rs::metadata_provider_example_
  passes_conformance (real spawn + jail + sidecar corpus); the other suites
  (plugin_install_lifecycle, plugin_hello_world) use synthetic modules, not
  this blob.

## 5. Open questions for the next agent
- ui-panel/plugin.wasm (the sibling blob) belongs to another assignment —
  same checks apply (regeneration drift, import/permission pairing).
