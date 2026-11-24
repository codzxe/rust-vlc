# CLAIM AUDIT: docs/dependencies/records/wasmi.md
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL: 4 REAL-UNPROVEN: 4 SHALLOW: 0 STUB: 0 PAPER-ONLY: 0 FALSE: 0 UNREACHABLE: 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A clean pre-audit record for wasmi 2.0.0 whose sandbox-unverifiable
registry facts are properly marked [VERIFY-P0/P1]. The strongest checkable
claim — the tarball sha256 — matches the workspace Cargo.lock checksum
byte-for-byte, and the admission materialized exactly as the record
proposed: `crates/plugin-host-wasm/Cargo.toml:57` pins
`wasmi = { version = "2.0.0", default-features = false, features = ["std",
"validate", "auto-dispatch"] }`, with fuel metering and StoreLimits actually
used in the extension host. This record is NOT related to any codec
bit-exactness claim.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | sha256 `78693fcdd618e0fc34af59c6b8efa9ac5d58c68df940beff4bedddb6acfe7c27` matches the sparse-index record | :114-116 | REAL | Cargo.lock:1265 wasmi 2.0.0 checksum is byte-identical: `78693fcdd618e0fc34af59c6b8efa9ac5d58c68df940beff4bedddb6acfe7c27` |
| 2 | Proposed pin `version-range = ">=2.0.0, <3"`, `no-default-features, std` (+ validate) | :125-130 | REAL | crates/plugin-host-wasm/Cargo.toml:57 `wasmi = { version = "2.0.0", default-features = false, features = ["std", "validate", "auto-dispatch"] }` (^2.0.0 = <3); lock deps = spin, wasmi_collections, wasmi_core, wasmi_ir, wasmparser (Cargo.lock:1263-1273) — the no-default-features shape |
| 3 | "fuel metering: `Store::get_fuel`/`set_fuel` ... (src/store/mod.rs, src/func/caller.rs)" | :73 | REAL (in-tree corroboration) | crates/plugin-host-wasm/src/instance.rs:108 `.set_fuel(limits.startup_fuel)` and :186 `.set_fuel(limits.call_fuel)` — fuel actually used |
| 4 | "`StoreLimitsBuilder` with `memory_size(limit)` total-store cap" | :72 | REAL (in-tree corroboration) | instance.rs uses `wasmi::StoreLimitsBuilder` + `store.limiter(...)` (state.rs: `StoreLimits` fields) |
| 5 | "boundaries.toml external_allowlist" carries wasmi | (boundaries.toml:564-590) | REAL | `external_allowlist = ["wasmi", "toml"]` — wasmi is one of two sanctioned production externals (ADR-002) |
| 6 | "Total downloads 25,285,206 ... 2 advisories ever, both closed, neither affecting 2.0.0" | :100-105 | REAL-UNPROVEN | registry facts; no network from sandbox |
| 7 | "0 native source files; 0 extern \"C\"; build.rs present but benign (cfg emission only)" | :12-26 | REAL-UNPROVEN | tarball inspection; no vendored tarball in sandbox to re-scan |
| 8 | "bus factor: small (single primary maintainer)" | :20-22, :106-107 | REAL-UNPROVEN | crates.io publisher data not re-checkable; noted as risk in the record itself |

## 3. Contradictions
None found. The record's [VERIFY] marking is consistent with what it could
not check (GitHub 403s, static.crates.io 403 recorded honestly at :111-113).

## 4. Fabrication / inflation indicators
None. This is a pre-audit record that explicitly says the crate was "never
built or executed" at record time (:5-6) and defers acceptance to the Lead.

## 5. Matrix rows
N/A. Cross-ref: the wasm extension-host feature rows cite plugin-host-wasm,
whose engine is this wasmi — the record underwrites a real, in-tree
dependency (not a paper claim).

## 6. What actually works (verified)
— Cargo.lock:1263-1273 (wasmi 2.0.0 + subcrates, checksum match);
crates/plugin-host-wasm/Cargo.toml:57; rg fuel/set_fuel/StoreLimitsBuilder
in plugin-host-wasm/src (16+ uses of `wasmi::` across instance/imports/
state/limits).
