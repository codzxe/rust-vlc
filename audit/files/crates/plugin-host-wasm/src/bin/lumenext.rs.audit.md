# AUDIT: crates/plugin-host-wasm/src/bin/lumenext.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-21 |
| Lines / functions | 303 / 9 |
| File verdict | UNREACHABLE (dev tool; built by the workspace build but not shipped by any packaging lane; exercised only by an integration test and the README walkthrough) |
| Reachable from shipped entry point? | NO as a shipped artifact. The `lumenext` bin is declared in Cargo.toml:73-75 and compiled by the workspace release build (`build-release-workspace.log.txt:231` "Compiling plugin-host-wasm"), but `scripts/packaging/gate.sh` ships only `lumen`/`desktop`/`lumen-desktop-tauri`. In-tree callers: `tests/plugin_examples_conformance.rs:446,463` drives the real binary via `CARGO_BIN_EXE_lumenext`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | Read of the whole file; Read tests/plugin_examples_conformance.rs:30-115,440-480 (CLI round-trip test); comparison against install.rs `validate_package` (471-562) and error.rs refusals; sed -n verification of quotes; grep `WasmInstance::spawn\|\.spawn\(` (no spawn in bin — confirmed "never executes" claim). |

## 1. One-paragraph verdict
A small, careful, panic-free CLI: typed `Failure` classes with documented exit codes (0/1/2), safe argument parsing (no index panics — `args.get(i+1)` for flag values, positional guard), and — verified by reading every line — it truly never installs and never executes guest code (no `ExtensionStore`, no `spawn`, only `read_package`/`stamp_manifest_hash`/its own ZIP writer). The one substantive defect is truth in labeling: `inspect` prints `VALID` after running only the structural half of the installer's chain — origin forgery, refused kinds, out-of-window api versions and all four signature cross-checks that `ExtensionStore::install` enforces are NOT checked, so packages the installer refuses as `OriginForgery`/`KindRefused`/`ApiVersion`/`SignatureUnparsable` sail through `lumenext inspect` with exit 0. The header's "the same structural chain the installer runs" and the round-trip test's identical comment are both false as written. Second-tier: the ZIP writer casts sizes/offsets to u32 unchecked (a ≥4 GiB module writes wrapped, silently corrupt fields) and computes each entry's CRC twice.

## 2. What actually works (verified)
— Verified by code trace (no baseline run reaches this crate): the stored-ZIP writer emits local headers + central directory + EOCD with correct magic bytes, LE fields, and CRC-32 per entry — shape-consistent with `read_package` (the round-trip test `lumenext_cli_round_trip_packages_and_inspects_both_examples` drives the real binary and asserts installability of its artifact, but was never run in baseline).
— Exit-code contract (header line 9) matches `Failure::code` (Usage/Validation→1, Io→2) and `main`'s mapping — consistent.
— README walkthrough outputs match this code's print statements exactly (`lumenext: packaged {id} v{version} (kind …, api v…)` at 157-163; `… VALID (kind …, origin …, api v…)` at 214-221) — the walkthrough text is internally consistent with the code, though still not run evidence.

## 3. Findings

### F-001 [S2]
Where: crates/plugin-host-wasm/src/bin/lumenext.rs:1-7 and 62-65 (claims) vs 181-200 (code)
Code:
```rust
//! … It WRITES packages (stamping the real module
//! digest into the manifest's provenance) and VALIDATES them (the
//! same structural chain the installer runs); it never installs and
//! never executes guest code …
```
and
```rust
    lumenext inspect <pkg>
        Validates an archive end to end (structure, CRCs, wasm magic,
        manifest schema, digest equality) and prints its identity.
        Never installs, never executes.";
```
What is wrong: `inspect` runs `read_package` (structure/CRC/magic/caps) + `PluginManifest::from_toml` (schema) + digest equality — and stops there. The installer's chain (`install.rs validate_package`, 471-562) additionally enforces: origin == Extension (`OriginForgery`), `kind_extension_legal` (`KindRefused`), `negotiate_api_version` (`ApiVersion`), and the four signature bookkeeping refusals (k1-k4, `SignatureUnparsable`). None of those run in `inspect`; a `builtin`-origin, `demuxer`-kind, `api_version = 2` package with a declared-but-absent signature prints `VALID (kind demuxer, origin builtin, api v2)` and exits 0.
Why it matters: the tool's `VALID` verdict is the developer's (and any CI's) signal that a package is installable; it under-reports the installer's refusals, producing exactly the class of misleading-tool-output the baseline correction already flagged twice for `lumen` (`engine-info` hard-coded decoders, `decode-bench` fall-through). The header and the round-trip test's comment (`conformance.rs:462` "inspect: the same structural chain the installer runs") both assert the false equivalence. `package` (142-143) likewise validates only the manifest schema, so the toolchain will happily package what install must refuse.
What correct looks like: call the library's `validate_package` (make it `pub(crate)`-shared or add a `pub fn validate` in `package.rs`) and print `VALID` only on the full chain; or print `STRUCTURALLY VALID (installer may still refuse: origin/kind/api/signature)` and fix the doc comments.
Confidence: HIGH — Verification: side-by-side read of inspect (176-233) vs validate_package (install.rs:471-562); every missing check named with its installer error variant.

### F-002 [S4]
Where: crates/plugin-host-wasm/src/bin/lumenext.rs:253, 262-263, 269, 297-300
Code:
```rust
        offsets.push(out.len() as u32);
        …
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
```
What is wrong: sizes and offsets are cast to u32 with no range check and no write-side cap. A module ≥ 4 GiB (or total output ≥ 4 GiB) silently wraps the compressed/uncompressed size and offset fields, producing a corrupt archive; the reader's 64 MiB package cap (`PACKAGE_CAP_BYTES`) would refuse it later, but the writer reports success (`lumenext: packaged …` exit 0).
Why it matters: silent-corruption writer path; requires a pathological input (no cap check at `read()` either), so S4, but it contradicts the crate's typed-refusal house style — every other boundary refuses loudly.
What correct looks like: refuse `data.len() > u32::MAX as usize` (or the package cap) with `Failure::Validation` before writing.
Confidence: HIGH — Verification: read of zip_stored; no length checks present.

### F-003 [S4]
Where: crates/plugin-host-wasm/src/bin/lumenext.rs:254 and 272
Code:
```rust
        let crc = crc32(data);
        out.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]);
```
and
```rust
    for ((name, data), offset) in entries.iter().zip(&offsets) {
        let crc = crc32(data);
```
What is wrong: `crc32` is computed once per entry in the local-header loop and again per entry in the central-directory loop — every entry's CRC is hashed twice.
Why it matters: trivial waste and copy-paste surface (the two computations could drift if one loop were edited); cosmetic for a 2-entry archive.
What correct looks like: compute CRCs once into a Vec alongside offsets.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Failure::code` | 34-39 | REAL | — | Matches the documented 0/1/2 contract. |
| `impl Display for Failure` | 42-50 | REAL | — | Prefixes usage/refused/io. |
| `main` | 67-80 | REAL | — | Maps results to exit codes; usage errors append USAGE text. |
| `run` | 82-89 | REAL | — | Two subcommands; unknown/missing → typed usage. |
| `package` | 93-172 | REAL | S2 (via F-001) | Safe arg loop (no index panics); stamps digest via the shared `stamp_manifest_hash`; manifest schema validated only (no origin/kind/api checks — packages the installer will refuse); default out name `{id}-{version}.lumenext` under the source dir. |
| `inspect` | 176-233 | REAL (defect) | S2 | Structural chain + digest equality only; prints `VALID` for packages install refuses (F-001); signature status printed honestly per `SignatureStatus`. |
| `read` | 237-239 | REAL | — | Typed Io failure with path context. |
| `write` | 241-243 | REAL | — | Typed Io failure with path context. |
| `zip_stored` | 249-303 | REAL (defects) | S4 | Correct stored-ZIP shape (magics, LE, EOCD); unchecked u32 casts (F-002); double CRC (F-003). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `lumenext_cli_round_trip_packages_and_inspects_both_examples` (conformance.rs:440-480) | YES (happy path) | Drives the REAL binary (`CARGO_BIN_EXE_lumenext`), asserts exit 0, `packaged`, `VALID`, `sha256 matches`, and that the CLI-written artifact installs through the real store — catches writer/reader drift and stamp breakage. Does NOT feed adversarial packages through the CLI (the §9.4 malicious-manifest matrix uses the library path only), so F-001's gap is invisible to it. |
| Baseline evidence | NO RUN | No `plugin-host-wasm` test binary appears in either baseline workspace test log. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "VALIDATES them (the same structural chain the installer runs)" | lumenext.rs:3-5 header | FALSE | F-001: four installer refusals absent from inspect. |
| "Validates an archive end to end" | lumenext.rs:62-63 USAGE | FALSE (overstates) | F-001. |
| "it never installs and never executes guest code" | lumenext.rs:5-7 | TRUE | Whole-file read: no `ExtensionStore`/`WasmInstance` symbol; imports are read_package/stamp/crc32 only. |
| "inspect: the same structural chain the installer runs" | conformance.rs:462 comment | FALSE | Same as F-001. |
| README walkthrough outputs (package/inspect) | plugins/extensions/README.md:104-131 | REAL-UNPROVEN (consistent) | Print statements match verbatim; no run evidence. |

## 7. Dead / duplicated / unreachable code
- No dead symbols. Duplication: double CRC (F-003); `zip_stored` duplicates the ZIP-encoding knowledge of `package.rs`'s reader by design (writer/reader twin — acceptable, tested round-trip).
- The whole binary is unshipped (see header row).

## 8. Dependencies & flags
- Uses only `plugin_host_wasm` library surface (`crc32`, `package::{read_package, stamp_manifest_hash, module_digest}`) + std. No feature flags, no unsafe, no panics (all `Failure`-typed). Consistent with crate pin style.

## 9. Open questions for the next agent
- package.rs auditor: does `stamp_manifest_hash` refuse a manifest that already carries a diverging FULL digest (the USAGE text at 58-59 promises "a diverging full hash is refused")? The refusal lives in your file; the CLI's contract depends on it.
- If the orchestrator runs `cargo test -p plugin-host-wasm --locked`, the CLI round-trip will finally have run evidence — recheck the README walkthrough against its output.
