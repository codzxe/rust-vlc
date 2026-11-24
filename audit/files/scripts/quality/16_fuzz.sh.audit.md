# AUDIT: scripts/quality/16_fuzz.sh
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Lines / functions | 50 / 0 named (16a census + list/run/repro dispatch) |
| File verdict | REAL guards, STALE evidence logic (seed census provably records a false zero on the real tree) |
| Reachable from shipped entry point? | NO (dev tooling) but wired: 99_run_all.sh:52 `J 16_fuzz.sh list` (list mode only; run/repro are manual) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | bash -n (0); sandbox execution of a copied {common.sh,16_fuzz.sh} pair (both not_run paths fire, exit 0); 16a logic replica against the real tree (grep/ls counts); rg callers/targets; sed quote verifications |

## 1. One-paragraph verdict
The guard discipline is honest and I executed it: with fuzz/ or cargo-fuzz
absent, the script records precise NOT RUN lines and exits 0. But its own
evidence logic is stale against the current tree: the 16a "seed corpus"
section greps for top-level `seed_*` entries, while the repo's seeds live in
`fuzz/corpus/<target>/` (35 corpus dirs, 97 files — fuzz/README.md:99-109)
— so on the REAL tree the script records "seed count: 0", a false-clean
number that contradicts the script's own note ("Seeds are the repo's own
real-media corpus") and the baseline ("fuzz-registry: CLEAN — 35 targets,
corpora present"). Second-tier defects: the targets.txt evidence truncates
at 40 of 106 grep lines (35 targets cannot fit), the `run` subcommand never
checks cargo-fuzz's exit code, and the `repro` default target "parser" does
not exist in the 35-target registry (all fuzz_*-prefixed).

## 2. What actually works (verified)
- `bash -n scripts/quality/16_fuzz.sh` -> 0.
- Sandbox execution (copied pair, repo untouched):
  ```
  === 16a. fuzz targets present in the repo ===
  NOT RUN [fuzz]: fuzz/Cargo.toml missing
  NOT RUN [cargo-fuzz]: not installed (run scripts/quality/00_install_tools.sh)
  exit=0
  ```
  — both honest-failure paths fire with actionable messages.
- 16a census against the real tree (same greps, output to scratch): the
  target-pattern grep matches 106 lines in fuzz/Cargo.toml; `ls fuzz | grep -c
  '^seed_'` -> 0 (the seeds are in fuzz/corpus/, F-001).
- Wiring: 99_run_all.sh:52 `J 16_fuzz.sh list`.
- Registry consistency: fuzz/Cargo.toml has exactly 35 [[bin]] targets —
  byte-consistent with baseline full_gate.txt:57 "35 targets" and with
  fuzz/README.md's committed-corpora claim.

## 3. Findings
F-001 [S3]
Where: scripts/quality/16_fuzz.sh:15-16
Code:
```
  ls fuzz | grep -E '^seed_' | head -40 | tee -a "$OUT/fuzz/targets.txt"
  say "seed count: $(ls fuzz | grep -c '^seed_')"
```
What is wrong: the seed census looks for TOP-LEVEL `seed_*` entries, but the repo's seeds live under `fuzz/corpus/<target>/` (35 dirs / 97 files; fuzz/README.md:99-109 "Seed corpora (committed ...): fuzz/corpus/<target>/ holds 1–6 small seeds per target"). Empirical replica on the real tree: `ls fuzz | grep -c '^seed_'` -> 0. Why it matters: the recorded evidence (targets.txt + "seed count: 0") is a FALSE-CLEAN record — a reader concludes no seeds are committed while 97 real-media seed files exist; it contradicts the script's own note (:47 "Seeds are the repo's own real-media corpus (D-11), not synthetic input") and the baseline's "corpora present". What correct looks like: `find fuzz/corpus -mindepth 1 -maxdepth 1 -type d | wc -l` + per-target file counts (or delegate to scripts/fuzz_registry_check.sh, which already reconciles corpora). Confidence: HIGH — Verification: ran the exact census commands against the real tree.
F-002 [S4]
Where: scripts/quality/16_fuzz.sh:12
Code:
```
  grep -E '^\[\[bin\]\]|^name\s*=|^path\s*=' fuzz/Cargo.toml | head -40 | tee "$OUT/fuzz/targets.txt"
```
What is wrong: the pattern matches 106 lines in the real fuzz/Cargo.toml (35 targets × ~3 lines + the package name); head -40 truncates the evidence file mid-registry — targets.txt cannot hold all 35 targets. What correct looks like: drop the head (or `rg -c '^\[\[bin\]\]'` + a names-only listing). Confidence: HIGH — Verification: counted 106 matches vs 40-line cap.
F-003 [S4]
Where: scripts/quality/16_fuzz.sh:31-33
Code:
```
      cargo fuzz run "$target" -- -max_total_time="$secs" -rss_limit_mb=4096 \
        2>&1 | tee "$OUT/fuzz/$target.log" | tail -30
      cp -r fuzz/artifacts/"$target" "$OUT/fuzz/artifacts-$target" 2>/dev/null || true
```
What is wrong: the run's exit code is never captured (no rc, no not_run on failure — contrast 14_crap.sh:16-21 which does it right); a failed fuzz run (build error, unknown target) leaves the sweep green with only a log file as evidence. The artifacts cp is `|| true` + 2>/dev/null — acceptable for "artifacts may not exist", but combined with the un-gated run the whole `run` subcommand cannot fail. Confidence: HIGH — Verification: read (cargo-fuzz absent, run path not executable here).
F-004 [S4]
Where: scripts/quality/16_fuzz.sh:40
Code:
```
      if need cargo-fuzz; then cargo fuzz run "${3:-parser}" "$art"; fi
```
What is wrong: the default repro target is "parser" — no such target exists in the 35-target registry (names are fuzz_codec_*, fuzz_demux_*, ... — rg '^name = "parser"' fuzz/Cargo.toml -> no hit). `16_fuzz.sh repro <artifact>` without an explicit target is a guaranteed cargo-fuzz failure; the default is stale from an earlier naming era. Confidence: HIGH — Verification: rg over the target registry.

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 16a census | 9-19 | REAL, stale-layout | S3/S4 | F-001 seed census false-zero; F-002 truncated evidence; missing-fuzz not_run executed |
| list | 21-25 | REAL | — | need-guarded; cargo-fuzz absent → not_run (executed) |
| run | 26-37 | REAL, un-gated | S4 | F-003; default 3600s; rss cap 4096 MB |
| repro | 38-41 | REAL, wrong default | S4 | F-004 "parser" target absent |
| note | 44-50 | accurate | — | ASan default, artifact preservation policy — substantively TRUE (corpora committed) |

## 5. Test honesty
No tests. Mutation reasoning: the 16a census defect is invisible to anything
in-tree because nothing consumes targets.txt (fuzz_registry_check.sh does its
own reconciliation) — a self-contained evidence file that nobody checks is
where rot lives.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Seeds are the repo's own real-media corpus (D-11), not synthetic input" | 16_fuzz.sh:47 | TRUE (substance) / contradicted by the script's own census | fuzz/README.md:99-109 + 35 corpus dirs / 97 files — but the :15-16 census records 0 (F-001) |
| "fuzz-registry: CLEAN — 35 targets, diff empty, corpora present" | baseline full_gate.txt:57 | TRUE | my count: exactly 35 [[bin]] targets in fuzz/Cargo.toml |

## 7. Dead / duplicated / unreachable code
The seed-census lines (:15-16) are dead against the current layout (match
nothing) — the live seed truth lives in fuzz_registry_check.sh's own
reconciliation (full_gate step 6). The run/repro subcommands are manual-only
(no workflow calls them — 99_run_all uses `list` only).

## 8. Dependencies & flags
needs: cargo-fuzz (absent here; note honestly says it "needs nightly for
sanitizers"). Reads fuzz/Cargo.toml; writes $OUT/fuzz/{targets.txt,list.txt,
$target.log,artifacts-$target}. No feature flags.

## 9. Open questions for the next agent
- Does the fuzz/ layout predate the corpus/ reorganization (worklog
  "seed corpus committed" history)? The census was probably correct when
  written — date the rot for the incident log.
- cargo-fuzz's `list`/`run`/`repro` subcommand surface is
  UNVERIFIED-offline; if `run` here differs from the installed
  cargo-fuzz's CLI, F-003's un-gated failure becomes doubly silent.
