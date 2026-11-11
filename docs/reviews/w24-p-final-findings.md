# W24-R14 — final-state audit of the compat-fix lane W24-P (issue #53, wave 24)

- **Auditor:** W24-R14 (wave 24 review scout, round 14 — final-state audit of lane P's pushed tip).
- **Audited state:** `origin/w24/p` @ `c48dc76` (remote ref confirmed via `git ls-remote origin`;
  local ref identical). **Pushed, 11 commits on `origin/w24/d` (`2e01a2c`)**; checked out read-only
  into worktree `wt/w24r14` (branch `w24/r14`). This closes R10's snapshot caveat ("re-run on its
  final pushed tip"): the 8 commits R10 did not see are `74bda46` (P3-2 tests), `2a43b78` (P3-3),
  `8f72b30` (P3-4), `b30a925` (P3-5), `cd7d5c5` (P3-7), `b5aff90` (IN-8 guard), `6a1677c` (IN-8
  data dedupe), `c48dc76` (R10-3).
- **Delta (full, `w24/d...w24/p`):** 9 files, +1,114/−55 — `tests/compat/src/{config,oracle,report}.rs`
  (+205/+246/+471), `tools/xtask/src/cmd/compat_matrix.rs` (+186), `docs/feature-matrix.toml` (+8/−1),
  `docs/corpus/compat-matrix-20261110.{md,json}`, `tests/compat/regen-corpus-manifest.json`, `worklog.md`
  (+19, the W24-D entry only). The unaudited-8 sub-range is +710/−29 over 7 files.

## Verdict headline

**1 P1 / 0 P2 / 1 P3 (scout findings on the fix lane).** Every technical ask is genuinely closed:
IN-8 is fixed twice over (applier idempotence guard + committed-data dedupe with provenance, both
unit-tested), R10's three findings all landed with tests, the 0385 regen gap is honestly documented
as a next step, and the no-regression battery is fully green (counts, raw rows, additions-only
tests, clippy/fmt/lint). The P1 is the one thing that did NOT land: **there is no W24-P worklog
entry on the final pushed state** — the mandatory AGENTS.md §4 append is missing, and with it the
explicit fixed-vs-deferred list R10 asked for (R10-1's second limb). Four of L's P3s
(P3-1/P3-6/P3-8/P3-9) are deferred with the deferral recorded nowhere on the branch.

## What was actually run / verified (evidence for every claim below)

- `source scripts/cargo-env.sh && cd tests/compat`: `cargo clippy --all-targets --locked --
  -D warnings` — **0 warnings** (mirrored deny lints, `unsafe_code` deny); `cargo test` —
  **27 passed / 0 failed / 0 ignored** (4 sha256 + 10 oracle + 5 config + 8 report).
- `cargo clippy -p xtask --all-targets --locked -- -D warnings` — **0 warnings**;
  `cargo test -p xtask --locked --bin xtask compat_matrix` — **12 passed / 0 failed**, including
  the two new IN-8 tests `coalesced_evidence_is_deduped`, `recited_evidence_is_not_reappended`.
- `cargo fmt --check` in `tests/compat` and `cargo fmt -p xtask -- --check` — **both clean**.
- `cargo xtask lint-literals` — **PASS (0 violations, 452 allowlisted)**.
- **Raw evidence immutable:** `git rev-parse` of `docs/corpus/compat-matrix-20261110.raw-rows.jsonl`
  → blob `469f246…` **identical** on `w24/d` and `w24/p`; no `.jsonl` appears in the diff.
- **Counts unchanged (recounted by me from the committed JSON, not read from the md):** `counts`
  block byte-identical to `w24/d` (python `==` on parsed blocks) — files_total **166**; decoded
  **84** / decoded_partial **22** / refused_probe **2** + refused_demux **4** (= 6 refused) /
  demux_fatal **0** / malformed_typed **54** (sum 84+22+2+4+0+54 = 166 ✓); oracle **29**
  bit_exact_prefix + **6** tolerance_fail (recounted from `oracle.rows`: 35 rows, verdicts
  29/0/6/0/0; every `vlc_source` = `live_run`, which is why P3-5 needed no committed-artifact
  edit). The 5 committed transitions all carry `platform: "linux"`, so the P3-3 renderer change
  is text-identical on this data (no md edit needed — none made). **Matrix counts unchanged: PASS.**
- **Committed-artifact diffs are exactly the findings-named edits:** md — P2-1 avi/wav cells +
  §6.2 rewrite (pre-audited), P3-4 header line, P3-7 §7 row, and the provenance blockquote updated
  in step by each editorial commit; JSON — only the two P2-1 cell details + the P2-1 editorial note
  (pre-audited). `feature-matrix.toml` — only the aac-lc evidence dedupe + its 7-line provenance
  comment (statuses/notes untouched, verified in the diff).
- **Data dedupe independently re-verified:** my own full-file scan of all `evidence = "…"` lines in
  `feature-matrix.toml` (python, segment-split on `";"`) finds **zero duplicated segments on any of
  the 252 rows**; the run-report citation appears exactly once on each of its four citing rows
  (`container-asf-wmv`, `codec-video-vp9`, `codec-audio-aac-lc`, `codec-audio-alac`).
- **Manifest health:** `tests/compat/regen-corpus-manifest.json` parses; **67 entries**; the 0385
  entry's `argvs`/`sha256`/`deterministic` are byte-identical to `w24/d` (only `purpose` grew).
- **Tests additions-only:** zero deleted `assert`/`#[test]` lines in the full range (the `-` lines
  are all replaced implementation arms: the two hard-coded Steps cells, the dead `refusal_count`,
  the old format strings, `CommittedRecord`→`None`); no `#[ignore]`/`cfg(ignore)`/`should_panic`
  anywhere in `tests/compat/src` or the xtask cmd; sha256's 4 vectors intact; `#[test]` counts
  d→p: oracle 0→10, config 0→5, report 0→8, xtask compat_matrix 10→12.
- **Scope:** the 9 touched files are all findings-named (or their committed-artifact render
  surfaces) + `worklog.md`. No CI/workflow/other files. **PASS.**

## Per-R10-finding closure verdicts

| R10 finding | Verdict | Notes |
|---|---|---|
| R10-1 (P2), limb 1 — IN-8 applier guard | **CLOSED (verified)** | `b5aff90`: two guard limbs in `edit_row`'s evidence step — (a) within a coalesced group each DISTINCT citation appends once; (b) a citation already a `"; "`-segment of the row's CURRENT (unescaped, via the new `string_value_of` toml_lite parse) evidence value is never re-appended, and an all-present group appends nothing (no bare `"; "` tail). Two unit tests prove both limbs (same-citation-twice → once; a second outbox file re-citing the same artifact across a further flip → evidence byte-identical while the flip lands); both pass here. |
| R10-1 (P2), limb 2 — IN-8 committed-data dedupe | **CLOSED (verified)** | `6a1677c`: the duplicated segment removed by hand with a 7-line provenance comment naming finding/cause/fix-commit/untouched-fields; my independent full-file scan confirms zero duplicated evidence segments remain anywhere. |
| R10-1 (P2), limb 3 — fixed-vs-deferred accounting | **NOT CLOSED — this audit's P1** | No W24-P worklog entry exists (`git log w24/d..w24/p -- worklog.md` → only `a776c24`, the W24-D entry R10 already audited; the last `Task ID:` in `worklog.md` is W24-D). See R14-1. |
| R10-2 (P3) — oracle/config unit tests | **CLOSED (verified)** | `74bda46` (+ the `b30a925` echo tests): 10 oracle tests (`prefix_verdict` ×4 — bit-exact-with-tail / whole-payload / in-window mismatch / oracle-longer-than-decode; `f32_verdict` ×3 with power-of-two offsets so RMS/peak assert exactly; `committed_record_applies`; `compare_row_committed` integer + f32-echo) and 5 config tests (schema refusal, `web`-as-native refusal, two-native-lanes, the structural rejections, minimal-valid control) — every asserted message matches the implementation's real error strings (checked line-by-line). The applicability rule was factored into `oracle::committed_record_applies` with unchanged semantics (old closure vs new fn compared verbatim). |
| R10-3 (P3) — steps_cell wasm prose | **CLOSED (verified)** | `c48dc76`: the decode-refusal clause is now a fourth `steps_cell` parameter; both production call sites pass their own wording (the format_cell clause byte-identical to the old text); new test `steps_failure_detail_is_parameterized_not_wasm_prose` asserts a hypothetical `cli-smoke` steps lane renders its own clause and never the string "wasm32". Passes. |

## Per-W24-L-finding verdicts (final state)

| L finding | Verdict | Notes |
|---|---|---|
| P1-1 W24-D worklog entry | FIXED (R10-verified; intact) | `a776c24`, unchanged since |
| P2-1 in-row refusals | FIXED (R10-verified; intact) | `16d64b0` |
| P2-2 wasm cell derivation | FIXED (R10-verified; intact) | `022d679` |
| P3-1 verify_diff defeats notes-insertion | **DEFERRED, unrecorded** | still present at the tip: the `notes_line: None` arm inserts a `notes` key (`compat_matrix.rs:671-682`) while `verify_diff` rejects any key-count change ("added/removed keys … only status/notes/evidence are writable", `:793-798`) |
| P3-2 oracle/config unit tests | FIXED | `74bda46` |
| P3-3 hard-coded "(linux)" + dead `refusal_count` | FIXED | dead code gone with `16d64b0`; platform now `transition.platform` (`2a43b78`) |
| P3-4 "VLC VLC" | FIXED | renderer format string + committed md line, provenance recorded (`8f72b30`) |
| P3-5 `VlcSource::CommittedRecord` mislabel | FIXED | new `VlcSource::None` ("none" in the md table) for never-consulted rows; no committed artifact affected (all 35 rows `live_run`, verified) (`b30a925`) |
| P3-6 first-audio limb dump-vs-fatal | **DEFERRED, unrecorded** | `tests/compat/src/decode.rs` untouched in the range |
| P3-7 0385 regen entry | **CLOSED AS DOCUMENTED DEBT** | `cd7d5c5`: an honest next-step record, not a fix — the manifest `purpose` and report §7 now name the permanent gap (machine-local concat intermediate) and the re-authoring next step, explicitly deferring to the corpus owner; argvs/sha256/deterministic untouched, manifest parses (67 entries). Correct scope call for a review lane; the actual re-authoring still needs an owner. |
| P3-8 stale claims snapshot | **DEFERRED, unrecorded** | `tests/compat/compat-matrix.json` untouched in the range |
| P3-9 `read_capped` unbounded read | **DEFERRED, unrecorded** | `tests/compat/src/proc.rs` untouched in the range |
| P3-10 unmarked hand-edited md | FIXED (beyond ask; intact) | provenance blockquote + JSON editorial note, each editorial commit extended them |
| IN-8 aac-lc double-append | FIXED (both halves) | `b5aff90` + `6a1677c` |

## Scout findings on the fix lane

### R14-1 (P1) — no W24-P worklog entry on the final pushed state; the fixed-vs-deferred accounting R10 asked for is absent

- **File:line:** `worklog.md` (EOF — the last entry is W24-D at line 778, which is `a776c24`'s
  reconstruction, R10-audited); `AGENTS.md` §4 ("Worklog protocol (mandatory) … After finishing
  your Task ID: append a section using exactly this format", lines 64-89).
- **Evidence:** `git log --oneline w24/d..w24/p -- worklog.md` → `a776c24` only; none of the 8
  post-R10 commits touch `worklog.md` (verified per-commit with `--name-only`). The wave briefing
  listed "worklog accounting" among the 8 — it did not land. The W24-D entry's closing line —
  "Residuals the audit flagged (P2: in-row refusal surfacing, wasm-cell derivation; P3: unit-test
  coverage, wording/labeling) **are being fixed by W24-P on w24/p**" — is now the ONLY accounting
  on the branch, and it cannot be reconciled: it names two P3 classes while the branch actually
  fixed P3-2/3/4/5 + IN-8 and documented P3-7, and it says nothing of the four deferrals
  (P3-1, P3-6, P3-8, P3-9 — each re-verified untouched at the tip).
- **Impact:** this is the same §4 violation shape as L's P1-1 (a finished lane shipping without
  its mandatory entry — the very finding W24-P's first commit fixed for W24-D). A merge-time
  reader reconciling the wave against L's findings + R10's asks must re-derive the
  fixed/deferred split from commit messages (as this audit did); the deferred P3s risk being
  assumed fixed.
- **Remediation (bounded, one append-only commit):** a W24-P entry in the exact §4 shape carrying
  the fixed-vs-deferred table (this doc's two tables are ready-made content), correcting the
  record from W24-P's own entry (per §4's never-edit rule, the W24-D text itself stays untouched —
  the reconciliation lives in the new entry, e.g. by restating what actually landed).

### R14-2 (P3) — the IN-8 re-cite guard dedupes only exact `"; "`-separated segments

- **File:line:** `tools/xtask/src/cmd/compat_matrix.rs` (`edit_row`, evidence step: `present =
  existing_evidence.split("; ")` compared with `==` against the whole proposal citation).
- **Evidence:** a citation whose text itself contains `"; "`, or a hand-reworded variant of an
  existing citation, would not match any single segment and would re-append (the failure is
  over-append, never data loss). Current citations are path-like strings without `"; "`, so this
  is theoretical; the within-group limb (`citations.contains`) has the same shape and the same
  bound. Also noted: the guard's `string_value_of` requirement adds a loud-error path for rows
  whose `evidence` is not a single-line quoted TOML string (all 252 shipped rows are; the error is
  clear, and the alternative was comparing against the raw escaped line — worse).
- **Suggested fix (if ever touched again):** one doc-comment line on the guard naming the exact
  whole-segment contract, or a segment-normalizing compare. Not merge-blocking.

## Bottom line for the Lead

Everything technical R10 asked for landed and verifies green: IN-8 closed in both the tool and the
data, all three R10 findings fixed with tests, the 0385 gap honestly documented, and the full
no-regression battery passes on the pushed tip (166 / 84-22-6-0-54 / 29+6; raw rows blob-identical;
additions-only tests; clippy/fmt/lint-literals clean; scope clean). The lane is one append-only
worklog commit away from fully closeable: **R14-1** (the W24-P §4 entry with the fixed-vs-deferred
list, and an explicit disposition for P3-1/P3-6/P3-8/P3-9). R14-2 is a theoretical robustness note
on the new guard. After R14-1's append, this lane is merge-ready from this audit's perspective.
