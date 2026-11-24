# CLAIM AUDIT: docs/12-performance-benchmarking.md (720 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 12 |
| REAL 4 / REAL-UNPROVEN 1 / SHALLOW 1 / STUB 0 / PAPER-ONLY 5 / FALSE 1 / UNREACHABLE 0 | |
| Internal contradictions found | 1 (baseline-directory claim vs actual layout) |

## 1. Blunt verdict
A well-disciplined program doc whose *methodology* claims held where it mattered — the
Phase-2/3 baseline (docs/perf/baseline.md) honestly implements §1.1 ("no target before a
baseline") and §1.5 (numbers labeled host-measured because `ts_ns` never landed). But the
harness story is inflated: §3 describes a full `tools/bench-vs-vlc` program (samplers, run
records, regression detection, results/<run-id>/); what exists is a 531-line lib + 236-line
md5 + one PCM test, and the actual baseline capture was produced by a DIFFERENT harness
the doc never mentions (`tools/perf/baseline.py`, 967 lines Python). The claimed baseline
home `docs/baselines/perf/<platform>/<suite>.json` does not exist — real numbers live in
`docs/perf/`. "Status: implemented" in the header is generous: methodology implemented,
harness partially, instrumentation deltas absent.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Header "Status: implemented (awaiting Lead review)" | :6 | SHALLOW (methodology yes, machinery partial) | Implemented: tools/perf/baseline.py (967 lines, baseline.md:9 cites it), docs/perf/baseline.md + raw data. Not implemented: ts_ns (rg over crates/commands/src, ui/src/generated -> 0), system.get-stats / probe-completed / first-frame-presented (rg over crates/commands/src -> 0), benches/ (no benches dir in any crate, `ls crates/*/benches apps/*/benches` -> empty), xtask bench/profile subcommands (ls tools/xtask/src/cmd -> no such files), results/<run-id>/ (no results/ in tools/bench-vs-vlc). |
| 2 | "No performance target in this project is set before a recorded baseline exists" | :26 | REAL | docs/perf/baseline.md:2-3 "Status: **baseline capture only — no targets claimed** (doc 12 §1.1: a target before a baseline is theater" — the discipline is quoted and held. |
| 3 | "Until that proposal is accepted, the harness stamps events at command-layer receipt and labels those numbers `host-measured`" | :67-69 | REAL | baseline.md:23-26 "every wall-clock number below is **host-measured** ... because the engine's proposed `ts_ns` ... is not in the v1 catalog". rg ts_ns crates/commands/src -> 0. |
| 4 | Harness = `tools/bench-vs-vlc` (pure Rust, CI-built, embeds engine through command layer only) | :162-166 | SHALLOW (differential-digest core REAL; §3 perf-machinery absent) | tools/bench-vs-vlc = src/lib.rs (531) + src/md5.rs (236) + tests/lumen_pcm.rs (560) + run.py (1001) + mock_vlc.py (121) + test_run.py (544) — the card-3.E3 PCM-digest differential harness, REAL but narrow. Absent from doc 12 §3: sampler threads, results/<run-id>/ layout (`ls tools/bench-vs-vlc/results` -> no-results-dir), regression detection (§3.6), perf metrics M-06..M-08 machinery. Also not a workspace member (Cargo.toml:41) so no CI lane runs it. |
| 5 | "Baselines ... are committed under `docs/baselines/perf/<platform>/<suite>.json`" | :271-273, :351-352 | FALSE (dir absent) | `ls docs/baselines/perf/` -> "No such file or directory". Actual perf data: docs/perf/raw/{decode.csv, summary.json, ...} (summary.json keys: decode_ms, peak_rss_kib, probe_ms, seek_ms, startup_ms — python3 json load). docs/baselines/ holds only VLC pins (vlc-3.0.23-*.json/md + raw/{linux,macos,windows}). |
| 6 | §4.2 reconciliation: doc 02 uses `docs/baseline/` (singular), this doc proposes `docs/baselines/` (plural) — "the Lead picks one" | :355-357 | RESOLVED-BY-REALITY | `ls docs/baseline` -> no-singular; `ls docs/baselines` -> exists. Plural won; perf numbers were NOT placed under it (see #5) — the reconciliation happened without either proposed shape for perf. |
| 7 | §2.4 instrumentation deltas (ts_ns, session.probe-completed, video.first-frame-presented, system.get-stats) | :147-167 | PAPER-ONLY (labeled [PROPOSAL]) | rg over crates/commands/src for all four names -> 0 hits. Doc 05 carries none of them. |
| 8 | "in-tree `criterion` micro-benchmarks (benches/ per MP §4) run in CI as **advisory** comments only" | :304-305 | FALSE (as repo state) | No benches/ directories anywhere (`ls crates/*/benches apps/*/benches` -> nothing); `rg -l criterion crates/*/Cargo.toml apps/*/Cargo.toml tools/*/Cargo.toml` -> 0. No criterion dependency exists. |
| 9 | `cargo xtask profile <scenario>` wraps scenarios with platform profiler | :579-580 | PAPER-ONLY (marked [PROPOSAL]) | tools/xtask/src/cmd/ has no profile/bench subcommand (ls: check_architecture, check_identity, check_panics, codegen, compat_matrix, config_validator, feature_audit, fixtures, gate, lint_literals, mod, parity_report, rebrand, sbom, schema_gen, tokens, toml_lite, visual, visual_help, web_budget). |
| 10 | VLC pin per platform under docs/baselines (§4.1 Phase 0: "pin VLC 3.0.23 per platform") | :331-332 | REAL | `ls docs/baselines/` -> vlc-3.0.23-linux.md, vlc-3.0.23-macos.md, vlc-3.0.23-windows.md, vlc-3.0.23.json, hardware-template.md, raw/{linux,macos,windows}. |
| 11 | Copy ledger "[PROPOSAL]: a reviewed, in-repo table enumerating every copy" | :468 | PAPER-ONLY | `rg -li 'copy.?ledger' docs/ crates/` -> only this doc + worklog/archive-2026-10.md. No ledger table exists. |
| 12 | Toolchain facts "re-verified via network on 2026-10-19 (crates.io API...)" (criterion :502, wide/pulp :522, portable-simd nightly) | :10, :502, :522 | UNVERIFIED (network) | Sandbox has no network; the crate-update dates are plausible and self-consistent but cannot be re-verified. rust-toolchain pin: UNVERIFIED (not re-read this pass). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| :271/:351 baselines committed under docs/baselines/perf/<platform>/<suite>.json | docs/perf/baseline.md:10 "Raw data `docs/perf/raw/` (committed: CSV/JSONL per family + summary.json + environment.json)" | Reality wins; the proposed path was never created. |
| :6 "Status: implemented" | :10 "No code was written; no benchmark was run" (the doc's own verification field) | Both were true at authoring time; as of audit day the implemented fraction is the methodology (baseline.py + baseline.md) — not the §2.4/§3.6/§3.8 machinery (see ledger #1,4,7,8). |

## 4. Fabrication / inflation indicators
The doc's core honesty devices work in practice: baseline.md quotes §1.1 back at itself
and claims no targets. The inflation risk is "Status: implemented" + the elaborate §3
harness description against a PCM-digest-only tools/bench-vs-vlc — a reader would expect samplers,
run records and regression detection; none exist. Also note the actual measurement work
moved to a Python harness (tools/perf/baseline.py) that this doc never names.

Findings register:
- F-C001 [S3] ":271 docs/baselines/perf/" — directory does not exist.
- F-C002 [S3] ":304 criterion benches run in CI as advisory" — no benches/criterion anywhere.
- F-C003 [S4] ":6 Status: implemented" overstates: §3's perf machinery (samplers/regression/results) unwritten, harness is the narrower 3.E3 PCM-digest tool; actual baseline capture used tools/perf/baseline.py (967 lines) which this doc never names.
- F-C004 [S4] ts_ns envelope absent (:147, :688) — acknowledged in baseline.md; doc 12 Q1 remains open with every latency metric still host-measured.
