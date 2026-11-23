# AUDIT: crates/codec-api/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W06-A03 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Lines / items | 27 / 1 package + 1 lints + 1 dependency, 0 features, 0 dev-deps |
| File verdict | REAL — the manifest's every header claim verified true against the lockfile, the source tree, and the docs |
| Reachable from shipped entry point? | YES — apps/cli/Cargo.toml:50 `codec-api = { path = "../../crates/codec-api" }` (direct dep of lumen-cli; also via crates/codecs/Cargo.toml:84) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg (use-statements, doc refs, consumers); sed; wc -l; Cargo.lock inspection; all exit 0 |

## 1. One-paragraph verdict
A 27-line manifest whose header makes five hard claims, all of which I
verified: (1) `media-types` is the ONLY dependency — confirmed by
Cargo.lock:109-113 (`dependencies = [ "media-types" ]`) and by an rg sweep
of every `use` statement in src/ and tests/ (only `std::`, `crate::`,
`media_types::` appear); (2) no serde/thiserror — same lockfile proof; (3)
no codec implementations — src/ contains exactly the seven contract modules
(capability, config, contract, decode, encode, hw, lib), zero decoders;
(4) hardware stays abstract per ADR-001 — ADR-001-hardware-decode.md
exists and hw.rs is the opaque-contract module (see hw.rs audit); (5) no
dev-dependencies because the contract suite lives in the library — verified:
`crates/codecs/tests/contract.rs:8` does `use codec_api::contract;` with no
dev-dep edge, and codec-api's own `tests/contract_suite.rs` compiles against
regular deps only (baseline: 8/8 pass, W06-A03 log entry 3). The
doc-01-§2.4 registry row (doc 01:223: `codec-api ★ | crate | L1 |
media-types | 1 | — (contract)`) matches the manifest byte-for-byte in
substance. This is the cleanest manifest audited so far in this run.

## 2. What actually works (verified)
— verified by `rg -n "^use " crates/codec-api/src crates/codec-api/tests`
-> only media_types + std imports (list pasted in run log); `rg -n -A 4
'name = "codec-api"' Cargo.lock` -> single dep `media-types`; `rg -n
"codec_api::contract" crates/` -> 2 consumers (codecs/tests/contract.rs:8,
codec-api/tests/contract_suite.rs:11) proving the no-dev-dep design claim.

## 3. Findings
None. Zero-findings checklist (what was checked and how):
- Dependency-minimization claim ("ONLY dependency", "no serde/thiserror"):
  Cargo.lock package node + full use-statement sweep. PASS.
- "NO codec implementations": src/ module census vs the seven contract
  modules; no decode loops, no engines. PASS.
- "NO selection policy": rg for registry/decoder_for in codec-api — none
  (registry lives in crates/codecs/src/lib.rs). PASS.
- Card/ADR/doc citations: 1.D1 = docs/13-agent-wave-program.md:282; 1.D1-R =
  docs/worklog/archive-2026-10.md:895 (Lead-verified salvage card, "Deps:
  media-types only (C5 honored)"); ADR-001 = docs/adr/ADR-001-hardware-decode.md;
  doc 01 §2.4 row = docs/01-architecture.md:223. All resolve. PASS.
- `[lints] workspace = true`: root Cargo.toml:53/59 defines
  [workspace.lints.rust]/.clippy]; inheritance legal (crate not excluded). PASS.
- publish=false + version 0.1.0: consistent with every other workspace crate. PASS.

## 4. Per-function table (config keys)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comment block | 1-10 | REAL | - | all 5 claims verified (above) |
| [package] description | 16 | REAL | - | names exactly what the crate holds incl. the contract suite |
| [lints] workspace | 19-20 | REAL | - | root lints exist |
| [dependencies] media-types | 22-23 | REAL | - | path dep, lockfile-confirmed sole dep |
| no-dev-deps comment | 25-27 | REAL | - | design verified by real cross-crate consumer |

## 5. Test honesty
N/A (manifest; but the claim it makes about test placement is proven by the
passing contract_suite — baseline + W06-A03 log entry 3, 8/8).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "media-types is the ONLY dependency ... no serde/thiserror" | Cargo.toml:8-10 | REAL | Cargo.lock:109-113; use-sweep |
| "the contract suite lives in the library itself ... without a new edge" | Cargo.toml:25-27 | REAL | codecs/tests/contract.rs:8 consumes it, no dev-dep |
| "Registry: doc 01 §2.4 (codec-api, layer L1, deps: media-types only)" | Cargo.toml:3 | REAL | doc 01:223 row matches |
| "C5 (card 1.D1-R)" | Cargo.toml:8 | REAL | worklog archive-2026-10.md:895,907 "C5 honored" |

## 7. Dead / duplicated / unreachable code
None in this file. (Cross-file note for the codecs owner: doc 01:238 still
lists plugin-api as a codecs dependency while crates/codecs/Cargo.toml keeps
it dev-only — already flagged as F-003 in the codecs/Cargo.toml audit.)

## 8. Dependencies & flags
media-types (path) only; no features, no build script, no default-feature
traps; workspace lints inherited.

## 9. Open questions for the next agent
- hw.rs (next): does the "hardware stays abstract per ADR-001" claim hold
  at the type level (no platform types leaking into the contract)?
