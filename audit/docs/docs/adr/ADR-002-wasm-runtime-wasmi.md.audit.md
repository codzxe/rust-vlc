# CLAIM AUDIT: docs/adr/ADR-002-wasm-runtime-wasmi.md
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-S2) |
| Date | 2026-11-24 |
| Claims extracted | 7 |
| REAL 6 / REAL-UNPROVEN 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Decision record whose every follow-through hook exists in-tree: the evidence pack and
dependency record exist, wasmi 2.0.0 is pinned exactly as decided, the runtime crate
implements the ADR's conditions (lumen:v1.* import namespace, store-level memory cap +
fuel metering), and the ADR register's row matches the status line. The candidate table
(wasmtime/wasmer/fizzy rejections) is evidence-pack territory — the pack exists; the
rejections' reasoning is not re-derivable here.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Status: accepted (2026-10-19, Phase 0 gate); evidence pack + dependency record | :3-6 | REAL | ls docs/adr -> ADR-002-evidence.md present; ls docs/dependencies/records -> wasmi.md present; docs/adr/README.md:14 register row "ADR-002 ... accepted | 2026-10-19 (Phase 0 gate)". |
| 2 | wasmi 2.0.0 is the v1 WASM extension runtime (pinned) | :19,:26 | REAL | crates/plugin-host-wasm/Cargo.toml:9-10 "**wasmi 2.0.0** — the ADR-002 runtime (accepted 2026-10-19; full R1–R10 record at docs/dependencies/records/wasmi.md)" — pin + credit byte-consistent. |
| 3 | No-WASI posture: extensions see only our versioned `lumen:v1.*` capability imports | :30-32 | REAL | crates/plugin-host-wasm/src/state.rs:52 import example `lumen:v1.http::http_get`; manifest.rs gates `n.fs`/`n.http`/`n.ui`/`n.devices` permissions per manifest — no ambient filesystem/network on the import seam. |
| 4 | Resource limits: memory cap 32–64 MiB/instance, fuel metering resampled to epoch deadlines | :33-36 | REAL | limits.rs:7-8 "(ADR-002): the store-level `wasmi::StoreLimits` memory cap and fuel metering", :21-30 `memory_bytes`/`startup_fuel`/`call_fuel`, :46 "Default linear-memory ceilings per doc 06 §5.4 row 1: 32 MiB for ..." — the ADR's condition 3 is implemented as code with the ADR cited. |
| 5 | Interpreter-speed acceptance: extensions non-realtime; revisited-not-worked-around if realtime emerges | :37-39 | REAL (posture) | No jit engine or native-wasm path exists in the tree (the runtime crate is wasmi-only per Cargo.toml); the posture is enforced by architecture, not just prose. |
| 6 | Candidate table: wasmtime 49.0.3 fail (H1, ~129-crate C closure); wasmer fail (C/C++); fizzy fail (maintenance) | :17-22 | REAL-UNPROVEN | The four-candidate table's rejections are reasoned in the evidence pack (ADR-002-evidence.md exists, :5); tarball scans were performed 2026-10-19 on then-current versions — not re-derivable without re-fetching wasmtime/wasmer (multi-hundred-MB closures, out of load discipline). |
| 7 | Conformance kit (doc 06 §7) targets wasmi semantics; nested-wasm web spike is Phase 7 [VERIFY-P0] | :40-42,:48-49 | REAL (kit exists) | The register notes the plugin conformance kit (issue #46, wave 23) exercises the runtime; plugin-api tests (doc06_examples.rs, testkit_wiring.rs) exist. Nested-wasm spike remains future-work — no false claim of it having run. |

## 3. Contradictions
None. The register's two-file design note (README.md:15/:33 — evidence pack is input,
never "superseded") keeps the numbering honest.

## 4. Fabrication / inflation indicators
None. The [VERIFY-P0] nested-wasm item is carried forward as an explicit unknown, and
the runtime risk shift (R-13 -> wasmi maintenance watch) is stated as a consequence,
not a solved problem.

Commands run: ls docs/adr + docs/dependencies/records; rg wasmi in Cargo.tomls; rg lumen:v1/n.* in plugin-host-wasm + plugin-api; rg limits/fuel/MiB in limits.rs; rg ADR-002 in README/docs 01.
