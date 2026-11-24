# AUDIT: crates/plugin-host-wasm/src/bin/lumenext.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-24 |
| Lines / functions | 303 / 9 (main, run, package, inspect, read, write, zip_stored, Failure::{code,Display}) |
| File verdict | REAL (executed: 13 probes, all behaviors verified) — developer tool, NOT a shipped entry point |
| Reachable from shipped entry point? | NO. Bin of plugin-host-wasm; release lane builds only `-p lumen-cli -p desktop`. Reached by devs and by tests via `CARGO_BIN_EXE_lumenext` (conformance suite, which I ran: passes). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | 13 direct executions of target/debug/lumenext (see §2) + conformance suite run in file-2 audit |

## 1. One-paragraph verdict
The tool is real and behaves exactly as its header documents on every path I attacked: exit codes 0/1/2 match the spec, CRC-32 tampering of either entry is refused, a CRC-fixed one-hex-char digest lie is refused with the precise expected message, garbage input is refused as "not a ZIP archive", and packaging stamps the true module digest. The one defect: "structure" validation trusts only the central directory — local file headers and the EOCD cd_size field can be corrupted to arbitrary garbage and inspect still prints VALID with exit 0, while the USAGE text promises end-to-end structure validation. Not a shipped binary: this is the developer-side packager; no app lane compiles it.

## 2. What actually works (verified)
All executed by me against target/debug/lumenext (built Nov 24, same tree):
- `lumenext` (no args) -> exit 1, usage on stderr (doc: usage=1) ✓
- `lumenext frobnicate` -> exit 1 "unknown command" ✓
- `lumenext package /nonexistent-dir` -> exit 2 "io: reading /nonexistent-dir/manifest.toml" (doc: io=2) ✓
- `lumenext inspect <garbage>` -> exit 1 "refused: not a ZIP archive" ✓
- `lumenext package plugins/extensions/metadata-provider --out /tmp/meta.lumenext` -> exit 0, prints id/version/kind/api, module bytes, digest, output path ✓
- `lumenext inspect /tmp/meta.lumenext` -> "VALID ... sha256 matches", permissions "fs-sandboxed-read", exit 0 ✓
- flipped byte inside module payload -> exit 1 "refused: CRC-32 mismatch for entry `plugin.wasm`" ✓
- flipped byte inside manifest text -> exit 1 "refused: CRC-32 mismatch for entry `manifest.toml`" ✓
- flipped EOCD cd_offset byte -> exit 1 "malformed ZIP structure: bad central-directory header" ✓
- CRC-corrected manifest with ONE hex char of the sha256 changed -> exit 1 "refused: provenance.hash does not match the module (module digests to sha256:ed7ec5be…)" — the digest-equality gate is real ✓
— plus conformance-suite coverage (file-2 audit): package+inspect round trip on both examples, 5/5 passed.

## 3. Findings
F-001 [S3]
Where: crates/plugin-host-wasm/src/bin/lumenext.rs:62-65 (claim) vs read_package behavior
Code:
```
    lumenext inspect <pkg>
        Validates an archive end to end (structure, CRCs, wasm magic,
        manifest schema, digest equality) and prints its identity.
```
What is wrong: "Structure" validation is central-directory-only. I flipped (a) a byte of the local-header entry name `manifest.toml` (offset 40, 'o'→0x90), (b) a byte of the module entry's local-header CRC field (offset 928), and (c) a byte of the EOCD cd_size field — in all three cases inspect printed `VALID ... sha256 matches` and exited 0. A ZIP whose local headers disagree with its central directory is malformed; strict tools reject or warn. Why it matters: the tool's stated contract ("end to end") overstates what is checked; a corrupted-in-transit package can pass inspect and only fail later (or worse, be hand-repaired inconsistently and still called VALID). Integrity itself is NOT bypassed (CD CRC + digest equality hold — I proved both fire). What correct looks like: read_package (src/package.rs, other agent's file) should compare local vs central name/sizes/CRC and EOCD cd_size against the actual CD extent, or the USAGE should say "central-directory structure". Confidence: HIGH — Verification: ran (three byte-flip probes, all VALID exit 0).

F-002 [S4]
Where: crates/plugin-host-wasm/src/bin/lumenext.rs:262-263, 269, 297-298
Code:
```
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
```
What is wrong: Unchecked `as u32`/`as u16` truncating casts in zip_stored — a module >4 GiB or archive >4 GiB silently wraps sizes/offsets; >65535 entries would wrap the EOCD count (impossible here: 2 entries). No zip64. Dev-tool blast radius, but the writer would emit a corrupt package claiming success. What correct looks like: checked conversion -> Validation failure. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/plugin-host-wasm/src/bin/lumenext.rs:184
Code:
```
    let manifest_text = String::from_utf8(package.manifest_toml.clone())
```
What is wrong: Needless `.clone()` — `package.manifest_toml` could be moved (`String::from_utf8(package.manifest_toml)`); package is not used afterwards except module_wasm/signature. Also arg parsing (L100-114) consumes any token after `--wasm`/`--out` as the path, even one starting with `--`, so `--out --wasm` silently sets out="--wasm" instead of a usage error. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 67-80 | REAL | – | exit codes verified by execution |
| run | 82-89 | REAL | – | dispatch; unknown cmd verified |
| package | 93-172 | REAL | – | happy path + io-error path executed; stamps digest via library fn |
| inspect | 176-233 | REAL | S3 | digest-equality verified by execution; F-001 structural tolerance |
| read/write | 237-243 | REAL | – | typed Io failures, exit 2 verified |
| zip_stored | 249-303 | REAL | S4 | F-002 casts; layout byte-verified against ZIP spec offsets (namelen at 26 etc.) |
| Failure::code/Display | 32-50 | REAL | – | 1=usage/validation, 2=io — matches header L9 and observed exits |

## 5. Test honesty
No dedicated unit tests in this file; the conformance suite (tests/plugin_examples_conformance.rs, `lumenext_cli_round_trip...`) drives package+inspect happy paths for real. Error paths had NO automated coverage before this audit — I covered them by direct execution (13 probes). Gap: the local-header-tolerance hole (F-001) is invisible to the suite.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exit codes: 0 success, 1 usage/validation failure, 2 I/O failure" | :9 | REAL | executed: 0/1/1/1/2 observed as documented |
| "never installs and never executes guest code" | :5-7, 65 | REAL | read: no ExtensionStore/WasmInstance/spawn in file |
| "Validates an archive end to end (structure, CRCs, wasm magic, manifest schema, digest equality)" | :62-64 | SHALLOW | CRCs/digest/schema verified by execution; "structure" is CD-only — local-header + EOCD cd_size corruption accepted as VALID (F-001); wasm magic not directly probed by me (UNVERIFIED, lives in read_package) |
| "a diverging full hash is refused" (stamp) | :58-59 | REAL (library) | stamp_manifest_hash is package.rs (other agent); CLI-side digest equality refusal executed |

## 7. Dead / duplicated / unreachable code
None in-file. zip_stored duplicates the ZIP writer shape of the crate reader's inverse (declared twin, comment L245-248) — acceptable pairing, drift risk noted.

## 8. Dependencies & flags
plugin_api (PluginManifest), plugin_host_wasm lib (crc32, package::{ENTRY_*, module_digest, read_package, stamp_manifest_hash}, SignatureStatus). std-only otherwise. No features, no unsafe.

## 9. Open questions for the next agent
- src/package.rs read_package: confirm whether local-vs-central consistency and EOCD cd_size are intentionally unchecked (F-001) and whether install.rs inherits the same tolerance — if install also skips those checks, the "verified at install" claim inherits the same hole.
- Is there any doc claiming `lumenext` ships to end users? It is a dev bin; nothing in the release lane builds it.
