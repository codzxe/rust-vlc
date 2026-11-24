# CLAIM AUDIT: docs/MASTER-PROMPT.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL 6 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 3 / FALSE 0 / UNREACHABLE 0 | |
| Internal contradictions found | 1 (layout block vs tree) |

## 1. Blunt verdict
The root brief is structurally honest: every load-bearing fixed decision (100% Rust
engine, React+TS UI, Tauri shell, wasm target, one command layer, parity-as-evidence)
is checkable in the tree and checks out — including the zero-C-sources scan. The
monorepo crate registry matches 1:1. The rot is confined to §4's layout sketch: four
named directories/tools never landed (config/, layouts/, profiles/, dependency-audit +
license-audit, benches/ root, plugins/builtin/), and one app the sketch never named
(desktop-tauri) did. Nothing in the brief is false; parts of the sketch are fiction
that later docs (07 §9 C4 "parked planes") quietly inherited.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Engine: 100% Rust." | :31 | REAL | rg --files -g '*.c' -g '*.h' -g '*.cpp' crates apps -> 0 files (executed) |
| 2 | "Engine … owns playback state, probing, demuxing, decoding, …" | :31-33 | REAL | crates/{media-probe,demux,codecs,playback-engine,...} all present; doc 03 audit cross-ref |
| 3 | "Parity is the share of inventory features in docs/feature-matrix.toml that are verified by automated evidence." | :16 | REAL | matrix 252 rows (tomllib); xtask parity-report runs (baseline: 0/252 verified, windows 66/252, linux 61/252) |
| 4 | "The project must never be described as all-Rust." | :36 | REAL | apps/desktop-tauri INFRA-BLOCKED.md + webview exception documented; A08 precedent re-verified |
| 5 | crates/ registry (30 named crates incl. platform-{win,linux,macos}) | §4 | REAL | all named crates exist in crates/ (31 dirs incl. _template scaffold) |
| 6 | "config/ themes/ keymaps/ layouts/ profiles/" | :84 | PAPER-ONLY (3 of 5) | ls: themes/ keymaps/ EXIST; config/ layouts/ profiles/ MISSING (doc 07 §9 C4 "parked planes") |
| 7 | "locales/ en-US.toml" | :85 | REAL | locales/en-US.toml present |
| 8 | "tools/ xtask … config-validator feature-audit dependency-audit license-audit parity-report bench-vs-vlc" | :97-98 | SHALLOW (3 of 6 named tools as dirs) | tools/ has xtask + bench-vs-vlc + corpus/perf/mem-audit/sweep/vta; config-validator/feature-audit/parity-report are xtask subcommands (in-tree, run in baseline); dependency-audit + license-audit exist NOWHERE — the Phase-0 automated dependency audit is still-owed per audio-codecs.md:23-25 |
| 9 | "apps/ desktop (Tauri) web (wasm host) cli control-server" | :95 | REAL | all four exist + an extra apps/desktop-tauri the sketch never names (naming drift, safe direction) |
| 10 | "plugins/ builtin/ (compile-time) extensions/ (WASM examples)" + "benches/" | :99-100 | PAPER-ONLY (in part) | plugins/ has extensions/ only (no builtin/ dir); root benches/ absent (benches live as crates/*/examples/*_bench.rs) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| §4 layout lists config/, layouts/, profiles/ | tree: absent; doc 07 validator: "parked plane" | The dirs never landed; the sketch is aspirational text never reconciled |

## 4. Findings
F-001 [S4] §4 layout block vs tree: 3 directories (config/layouts/profiles), 2 tools
(dependency-audit, license-audit), plugins/builtin/, and root benches/ named in the
authoritative brief do not exist. Downstream docs cite the sketch as if real (doc 07's
"§4" references). Confidence: HIGH — Verification: ran (ls loop, rg --files scan).
F-002 [S4] Clean-room+oracle discipline claims are corroborated: pinned VLC oracle used
externally (dumps 04-17), zero VLC sources in-tree (C-scan + no vlc/ dir). REAL.

## 5. Fabrication / inflation indicators
None: the brief's own honesty rules ("Never claim 'exactly like VLC' … beyond that
evidence", "record in the matrix, do not hide") are the audit's own vocabulary; the
defect is an unreconciled layout sketch, not inflation.
