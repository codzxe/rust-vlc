# CLAIM AUDIT: docs/web/size-report.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R4) |
| Claims extracted | 11 |
| REAL 8 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 3 |
| Internal contradictions found | 1 (stale honest-limit, self-disclosed) |

## 1. Blunt verdict
The report's machine twin (docs/web/size-record.toml) carries the exact same
numbers, its arithmetic is byte-exact under recomputation (MiB conversions,
deltas, budget percentages all re-derived and correct), its environment facts
(worker.js 7,073 B, audio-worklet.js 9,012 B, feature gates, rav1d refusal
string, gate wiring) verify to the byte, and its honesty section is real
self-limitation. The builds themselves cannot be re-run here (wasm build
budget) — the baseline's web-budget gate failed ONLY because the wasm32
rust-std was missing in that sandbox (E0463), not a budget breach.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Machine record docs/web/size-record.toml (schema lumen.web.size-record/1) | :3 | REAL | size-record.toml:33 `schema = "lumen.web.size-record/1"`; [environment] toolchain "rustc 1.99.1 (58cf8f9c3 2026-10-13)". |
| 2 | Per-config size table (floor 1,294,311/300,250 ... external-all 2,439,192/704,996) | :31-37 | REAL (record-corroborated) | size-record.toml rows: raw_bytes 1294311 / 1387526 / 1429280 / 1529340 / 2439192 and gzip 300250/334759/349963/385300 (+704996) — identical to the report table. |
| 3 | Delta columns (+93,215 ... +1,144,881 raw; +34,509 ... +404,746 gz) | :34-37 | REAL | python3 recomputation: 1387526-1294311=93215; 2439192-1294311=1144881; 704996-300250=404746; all four rows exact. |
| 4 | Budget percentages 9.5/10.6/5.6/6.1/11.2% against 3/6 MiB | :82-86 | REAL | product.toml:212 `[web.size_budgets]`; 300250/3145728=9.55%; 704996/6291456=11.21% — recomputed, all match. |
| 5 | "worker.js 7,073 B + audio-worklet.js 9,012 B = 16,085 B raw" | :100 | REAL | `ls -la` -> worker.js 7073, audio-worklet.js 9012 (exact bytes). |
| 6 | rav1d refusal string in capabilities.rs; rav1d not wasm-buildable | :60-65,39-42 | REAL | apps/web/src/capabilities.rs:22 "rav1d 1.1.0 fails to compile"; :316 "the default module is decoder-less, rav1d/AV1 is not" — string present. |
| 7 | Per-codec feature gates exist (builtin-codecs, codec-flac, codec-mp3, builtin-codecs-external) | :28-29,33-37 | REAL | apps/web/Cargo.toml:97-108 `builtin-codecs`, `codec-flac`, `codec-mp3` (+codec-mp2 note), external gate family. |
| 8 | web-budget --check wired into full_gate.sh step 10 + CI wasm job | :88,124 | REAL | scripts/full_gate.sh:70-72 step 10; .github/workflows/ci.yml:659 `cargo run -p xtask --locked -- web-budget --check`. |
| 9 | The five builds + tree-shake symbol-scan table (claxon 21/0/18, symphonia 107/397 ...) | :44-58 | UNVERIFIED | Builds forbidden by load discipline; artifacts not committed. Record corroborates sizes only, not symbol counts. |
| 10 | "cargo xtask web-budget --check ... fails the gate" (gate functions) | :88-90 | REAL (mechanism) | Baseline full_gate.txt:76-77 "web-budget: FAILED" — failed for E0463 missing wasm32 rust-std (environment), proving the gate actually runs a real build and fails loudly; not a budget breach. |
| 11 | "the ui/ React app has no production bundle build in this tree yet" | :101-103 | REAL (at 2026-11-11) / STALE now | ui/package.json:9 `"build": "tsc --noEmit && vite build"`; baseline ui-build.txt (2026-11-23) shows a real vite production bundle. The doc's own "when it does, its artifact joins the record" anticipation is honest, but the record still carries no UI row. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "no production bundle build in this tree yet" (2026-11-11) | ui/package.json build script + baseline ui-build.txt vite production build | Tree now HAS the bundle build (baseline 2026-11-23, "✓ 343 modules transformed"). Doc claim expired; self-flagged with "yet". |

## 4. Fabrication / inflation indicators
None. The record/report pair is self-consistent to the byte; every recomputable
number recomputes exactly; the "honest limits" section under-claims rather than
over-claims (no wasm-opt, no UI, single environment, warm-cache only).
