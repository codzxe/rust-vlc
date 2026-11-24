# CLAIM AUDIT: fuzz/README.md
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 8 / 2 / 1 / 0 / 1 / 0 / 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
This README is the honest version of a not-yet-run fuzz campaign: it states
its own status in caps ("SCAFFOLDED, NOT WIRED"), names the outstanding Lead
decision, and admits "No such run has happened yet" for the 10-minute CI
shape. The registry-side numbers I could check all reproduce (35 bins, 35
target files, registry gate CLEAN+PASS in the committed baseline), but its
corpus claim is stale by 7 files ("97" vs 104 on disk) and its
compile-verification claims are dated self-declarations I cannot re-run
(cargo absent in this sandbox).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Lumen registers one coverage-guided fuzz target per landed parser … **35 targets**" | README:4-13 | REAL | `rg -c '\[\[bin\]\]' fuzz/Cargo.toml` → 35; `ls fuzz/fuzz_targets/*.rs \| wc -l` → 35 |
| 2 | "the registry-vs-implementation diff is pinned empty by scripts/fuzz_registry_check.sh" | README:14-15 | REAL | baseline full_gate.txt:56-58: "fuzz-registry: CLEAN — 35 targets, diff empty, corpora present / fuzz-registry: PASS" |
| 3 | "Status: SCAFFOLDED, NOT WIRED — one Lead decision outstanding" | README:17 | REAL | read; the LEAD-DECISION block L19-42 states the nightly-toolchain dependency |
| 4 | "cargo check in fuzz/ … all 33 targets compile — verified 2026-10-24 … re-verified with all 35 … 2026-11-09" | README:38-40, 189-193 | REAL-UNPROVEN | self-declared dated runs; cannot re-run (cargo/rustc NOT installed in this sandbox); no baseline artifact for it |
| 5 | "`fuzz/corpus/<target>/` holds 1–6 small seeds per target (97 files, 1.4 MB total, every file < 50 KB)" | README:99-102 | SHALLOW (stale count) | `find fuzz/corpus -type f \| wc -l` → **104** files (not 97); `du -sh` → 1.4M ✓; files > 49 KB → 0 ✓; per-target counts 1–7 (fuzz_demux_mpd has 7 — also breaks "1–6") |
| 6 | "It is wired into scripts/full_gate.sh as step 6" | README:132-133 | REAL | full_gate.txt:56 (step 6 header) + PASS |
| 7 | "**No such run has happened yet** — the nightly wiring is the outstanding LEAD-DECISION" | README:165-168 | REAL | read — the honest admission that the libFuzzer campaign itself never ran |
| 8 | "cargo test -p demux -p codecs -p subtitles -p playlists --lib — … 29 new deterministic hostile-input tests … all pass" | README:194-197 | REAL-UNPROVEN | cannot run cargo; baseline cargo-test-workspace.txt covers the workspace with 4744 passed / 2 failed (the 2 are first-vertical files per AGENT-SPEC §13) — consistent but the "29" count not individually verified |
| 9 | "crates/codecs/tests/fuzz_loop_stable.rs … fuzz_codec_pcm + fuzz_codec_mjpeg, N each" + demux table row | README:204-207 | REAL | both files exist (ls) |
| 10 | "fuzz/Cargo.toml … [empty workspace] … 'current package believes it's in a workspace' … reproduced on stable 1.99.1" | README:170-179 | REAL-UNPROVEN | fuzz/Cargo.toml has its own [workspace] (read); the reproduction is a dated self-declaration |
| 11 | "corpus resolution … $LUMEN_FUZZ_CORPUS/<target>/ if the env var is set … fuzz/corpus default" | README:112-116 | REAL | scripts/fuzz_registry_check.sh:64 `corpus_root="${LUMEN_FUZZ_CORPUS:-fuzz/corpus}"` (read) |
| 12 | "copied from the committed CC0 fixtures (tests/fixtures/CORPUS.md + the crate test fixtures)" | README:101-103 | REAL | tests/fixtures/CORPUS.md exists (ls); spot-checked seeds below in the corpus data audits |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "97 files, 1.4 MB total" (README:101) | disk: 104 files, 1.4 MB | disk is true (find \| wc -l = 104); the README count is stale — the 6.C1c/issue-#14 corpus additions (mpd/http_headers/hls corpora) grew past the written number |

## 4. Fabrication / inflation indicators
None — the opposite: the doc volunteers its own non-execution (L17, L165-168)
and the LEAD-DECISION dependency. The stale file count (claim 5) is
maintenance drift, not inflation.

## 5. Matrix rows
N/A (not a matrix chunk).

## 6. Test-honesty verdict for the campaign the doc describes
The 35 fuzz targets are REAL code (registry-gated, compile-claimed) whose
actual fuzzing NEVER RAN — the effective fuzz coverage today is the
stable-runnable layer: the in-crate hostile-input tests and the two
fuzz_loop_stable.rs seeded loops (both exist, both covered by the baseline
workspace test run). The doc is honest about this; any external claim that
"Lumen fuzzes X" would be FALSE — the doc itself never claims that.
