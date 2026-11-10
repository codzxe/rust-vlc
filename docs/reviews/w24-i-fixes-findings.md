# W24-R7 review — lane I review-fixer audit (primary) + lane B soak-nightly.yml cross-check (secondary)

Read-only audit. Auditor: W24-R7, wave 24 review scout 7 (dual scope).

- **Target 1 (primary, 80%):** `origin/w24/i` (docs review-fixer lane —
  W24-R2's findings fixed), diffed `origin/w24/e...origin/w24/i`.
  **7 commits** on top of `origin/w24/e` (the task card said 8 — the
  actual count is 7: f00385c, 88c680d, d6c3920, eddd1db, 61fe4f3,
  cda3d8a, ca42190; discrepancy is in the task description, not a lane
  fault). Touches exactly the 7 files the worklog claims
  (docs-sweep, release-checklist, container-avi, audio-pcm,
  streaming-dash, rc-smoke.sh, worklog) — ownership respected, each
  commit scoped to its named finding.
- **Target 2 (secondary, 20%):** `origin/w24/b`'s
  `.github/workflows/soak-nightly.yml` only.

Fresh worktree `/home/z/my-project/wt/w24r7`, branch `w24/r7` off
`origin/w24/i` @ `ca42190`. Every check below re-run by me unless
marked otherwise.

## Verdict (Target 1)

**All 3 P2s and all 5 fixable P3s are correctly fixed and independently
re-verified.** Every fix matches or improves on W24-R2's suggested
disposition; no fix introduces a false claim; the lane's all-green
scope and the 37/73/0 verdict survive intact (I re-parsed the matrix
myself). P3-9's deferral-to-Lead is per the reviewer's own "none
required" disposition — accepted. **One NEW P2** (markdown table
rendering, below) and **one P3 observation** (pre-existing record
staleness surfaced by the P2-1 verification) are the only findings.

### Per-fix verdicts

| Finding | Fix claimed | My independent verification | Verdict |
|---|---|---|---|
| P2-1 (§1.1 video row) | claimed cell now "mjpeg, av1, h264, hevc, prores-dnxhd, vp9"; records cell lists `video-vp8-vp9.md`; h264-software moved to §1.2 register | Own tomllib parse of `docs/feature-matrix.toml`: claimed video-codec rows (desktop implemented/partial) = **av1, h264, hevc, mjpeg, prores-dnxhd, vp9** — exactly the fixed cell. `codec-video-h264-software` = **blocked ×4** (win/linux/mac/web all `blocked`); `codec-video-vp9` = implemented ×3. All 7 named record files exist; `video-h264-software.md` states the blocked ×4 state. Matrix recompute re-run: 73 format rows, 37 claimed (14/6/9/8), 0 web — under both the linux-only and any-desktop counting rules. `docs/guides/getting-started.md` §3 already agreed (VP9 listed) | **FIXED** |
| P2-2 (phantom rc-smoke file) | AVI + PCM records now cite `ff001e5` commit message + release-checklist §2, check renumbered 4 | `git grep rc-smoke-2026` on the branch: **zero hits outside the worklog's own "before" narrative**. `ff001e5` commit message records "headless-decode (60/60+60/60 frames) … PASS". `scripts/rc-smoke.sh` check 4 IS headless-decode and its default fixture IS `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` (exists) — the AVI parent fixture, so "this same fixture" is literally true. Checklist §2 documents the 8-check numbering (4 = headless-decode) | **FIXED** |
| P2-3 (bash -n loop) | loop in BOTH places (checklist row 1.2 + audit §7) | Both locations record `for f in scripts/rc-smoke.sh scripts/release/*.sh; do bash -n "$f" || exit 1; done` — a real loop, identical text. **I re-ran the loop: all 8 sh scripts pass individually, exit 0** (rc-smoke.sh + gen-wxs, make-app, make-appimage, make-dmg, package-deb, package-windows, release-notes). Pitfall re-demonstrated: one-shot `bash -n good1 bad2 good3` → exit 0 with a syntax error in bad2. BUT see NEW-1: the checklist's copy breaks GFM table rendering | **FIXED (with 1 new P2)** |
| P3-4 (anchored count) | `rg -c '^\[\[setting\]\]'` → 32 with correction note | Re-run: unanchored → **33**, anchored → **32**; schema line 13 is a comment containing the literal `[[setting]]` (verified). §7 records the anchored pattern + correction; §3's settings row now cites the anchored command | **FIXED** |
| P3-5 (7 vs 8 scripts) | "7 `scripts/release/*.sh`" both places | `ls scripts/release/*.sh \| wc -l` → **7**; audit §7 and checklist row 1.2 both now say 7 release scripts + rc-smoke.sh = 8 sh. Script name lists match my `ls` exactly | **FIXED** |
| P3-6 (dash test count) | 191 pinned to `93b7614`, current 206 stated | Re-counted: dash per-file 12 (resolve) + 4 (fuzz) + 4 (abr) + 7 (sched) = **27**; crate-wide today = **206** `#[test]` fns; at `93b7614` (via `git grep -c`) = **193** (= 191 lib tests + 2 ignored — consistent). Matrix `network-dash` evidence does record "streaming 191 lib tests (2 ignored) incl. 27 dash"; `93b7614` is the 6.C2 flip commit. "Stale by 13" = 206−193 ✓ | **FIXED** |
| P3-7 (wc -c portability) | `wc -c <` replaces `stat -c %s` in both the test and the PASS echo | `bash -n` OK. On a real 9339-byte file: `stat -c %s` = 9339 = `wc -c <` (byte-identical, no padding). Full check-6 condition simulated on a fake 1.5 MiB `\0asm` module → prints the same PASS line; missing-file path short-circuits at `[ -f ]`, no stray stderr. `2>/dev/null \|\| echo 0` fallback preserved; even a hypothetical padded `wc` output is tolerated by bash's `[ -gt ]` (tested). Linux semantics identical; macOS now correct instead of always-FAIL | **FIXED** (strictly better than R2's stat/stat fallback suggestion — one POSIX form) |
| P3-8 (stale w24/c note) | strike-through with ls-remote evidence | Re-ran `git ls-remote origin w24/c` → **fb97822** (matches); `git ls-tree origin/w24/c` confirms `docs/packaging/evidence/linux-lane-20261110.log` + all 19 `scripts/packaging/*` drivers; `fb97822` is **NOT an ancestor of origin/main** (rows still execute-at-wave-merge — correct); `origin/w24/a` still absent (the §7 row's other half remains true). Strike follows the file's own §8 rule ("strike the blocker — never delete history silently") | **FIXED** |
| P3-9 (worklog split) | deferred to Lead | R2's own disposition: "None required; optionally split at wave merge if the Lead wants" — deferral matches it and is recorded in the W24-I worklog entry | **ACCEPTED (deferred per reviewer)** |

### New findings (Target 1)

#### NEW-1 (P2) — checklist row 1.2's raw `||` breaks the GFM table row: rendered command is truncated and the Status cell renders empty

- **File:line:** `docs/release-checklist.md:51` (the P2-3 fix itself)
- **Evidence:** The loop text `bash -n "$f" || exit 1` sits inside a
  markdown TABLE cell with two raw, unescaped `|` characters. GFM
  splits table cells on unescaped pipes **before** inline parsing, so
  pipes inside code spans still split cells (GitHub's own docs: escape
  as `\|`). The row has 7 unescaped pipes in a 4-column table. Verified
  with a real GFM table parser (markdown-it, `table` enabled): the
  rendered row shows Command/evidence = `` `for f in
  scripts/rc-smoke.sh scripts/release/*.sh; do bash -n "$f `` —
  truncated, no closing backtick, **missing `|| exit 1; done`** — and
  Status = **empty**; the explanation text and the "[verified locally]
  all OK" status are dropped entirely (cells beyond the header count
  are discarded). Ironic class-regression: the row whose fix is "the
  command must check what it claims" now *renders* an incomplete
  command (a loop with no failure exit) and an unverified status.
  Raw markdown is intact and copy-pasteable — this is a rendering
  defect, not a data defect. The audit §7 copy is safe (list item,
  not a table).
- **Suggested fix:** escape both pipes in the code span:
  `bash -n "$f" \|\| exit 1; done`. Verified with the same parser:
  that form renders the full loop inside a proper code span with the
  status cell intact. One-line, two-character change.

#### NEW-2 (P3, pre-existing — surfaced by the P2-1 verification, NOT introduced by this lane) — two video records' status front-matter is stale vs the post-issue-#7 matrix

- **File:line:** `docs/formats/video-vp8-vp9.md:12-13`,
  `docs/formats/video-prores-dnxhd.md:3`
- **Evidence:** Both records were authored 2026-10-25 (card 5.P2b-I /
  5.P1-I) and last touched at `3116aec`/`2bb488d`; the matrix flipped
  on 2026-11-09 via the issue #7 admissions. Today: matrix says
  `codec-video-vp9` = implemented ×3 (rusty_vp9 admitted, 6.M-vp9)
  while the record's status says "vp9: blocked (candidate named,
  validation pending)"; matrix says `codec-video-prores-dnxhd` =
  partial ×3 (DNxHD admitted via filmcraft-dnx, evidence in the matrix
  row) while the record's status says "deferred (later) — decision
  record". The fixed §1.1 records cell now leans on both records for
  claimed rows. The cell's wording is accurate ("dual-row record: VP8 +
  VP9 decision state"), the records are honest about their authoring
  date, and the current evidence lives in the matrix rows +
  dependency records — so this is drift, not a false claim. R2 did not
  flag it (its status-vs-matrix sampling hit other rows).
- **Suggested fix:** a dated addendum in each record noting the
  2026-11-09 admissions (mirroring how the matrix rows point at
  `docs/dependencies/records/*.md`), or a §1.1 cell annotation. Owner:
  whoever owns `docs/formats` at wave merge — not a blocker for this
  lane.

#### Notes (no action)

- The task card's "8 commits on top of origin/w24/e" is off by one —
  there are 7. No lane fault.
- Audit §7's ENOSPC substitution note (`ast.parse` instead of
  `py_compile`) is now moot AND confirmed: sandbox disk recovered
  (54% used); I re-ran the literal `python3 -m py_compile
  scripts/release/product-env.py scripts/release/gen-icon.py` → OK
  (and the ast.parse form → OK).
- `lint-literals` via the shared cached xtask binary re-run → PASS,
  0 violations, 452 allowlisted — matches the W24-I worklog's
  verification battery exactly. Untouched §7 claims re-confirmed:
  keymap 150 bindings, 5 crash keys, `node --check` mjs OK.
- No new overclaims: every correction is annotated in place as a
  post-review correction naming W24-R2's finding ID; verdicts
  unchanged and re-confirmed. No ownership violations: per-commit
  file scoping verified (each commit touches only its finding's
  files).

## Verdict (Target 2 — `origin/w24/b` soak-nightly.yml)

**PASS on all four checks. No findings.**

1. **YAML validity:** `python3 yaml.safe_load` → valid; jobs =
   `soak`, `soak-scenarios`, `soak-lsan`. Also re-parsed with a
   strict duplicate-key-detecting SafeLoader subclass → clean. (The
   `on:` key parses as boolean `True` under PyYAML — the standard
   YAML-1.1 quirk, pre-existing in main's file too, harmless for
   GitHub's parser.) actionlint is not installed in this sandbox —
   noted as a method limit, not a finding.
2. **Additive:** `diff origin/main:soak-nightly.yml …
   origin/w24/b:soak-nightly.yml` → **only addition hunks, 0 deleted
   or modified lines**. The pre-existing `soak` job (all 6 steps,
   byte-identical, same name) is untouched; the two new jobs are
   appended after it.
3. **Conditionality / billing-blocked labels:** the workflow's trigger
   shape is unchanged (schedule `0 3 * * *` + workflow_dispatch only —
   the non-blocking posture; no pull_request/push, so it can never
   gate a merge; concurrency group + `cancel-in-progress: false`
   unchanged). Neither new job sets `continue-on-error` — consistent
   with the file's documented "non-blocking comes from the trigger
   shape, not from hiding the result" posture. Billing-blocked honesty
   labeling is present and correct: workflow-level "HONESTY NOTE —
   NONE OF THESE JOBS HAS EVER EXECUTED (CI billing-blocked …
   docs/KNOWN-LIMITS.md R-16)" (R-16 verified to exist and say exactly
   that); `soak-scenarios`' header marks the 4 h nightly profile
   itself `[VERIFY on first run]` (sandbox evidence at the bounded
   profile only); `soak-lsan` carries `[PROPOSAL]` in its job name +
   full honesty notes (nightly toolchain deliberately off the stable
   pin, LSAN false-positive surface documented, "do NOT delete the
   job").
4. **Scenario file references exist on `origin/w24/b`:**
   `tests/soak/scenarios-nightly.toml` (parsed: 5 scenarios ×
   `duration_min` 48 = **240 min = 4 h exactly**, matching the job
   name "4 h × 5 scenarios" and `timeout-minutes: 300` = 4 h run +
   build/cache margin), `tests/soak/scenarios.toml` (LSAN step 2's
   CI-short profile), `tests/soak/scenarios-sandbox.toml`,
   `tests/soak/scenarios.schema.json`, `scripts/soak-scenarios.sh`
   (`bash -n` OK; `--profile/--release/--report` flags and honest
   exit-code propagation match the workflow's invocation;
   `SOAK_SCENARIOS_REPORT` lands at the uploaded artifact path),
   `crates/playback-engine/tests/soak_scenarios/` (main/manifest/
   report/runner.rs; `SOAK_SCENARIOS` env contract verified in
   main.rs), `docs/soak/soak-report-20261110.md`, and all **4 fixtures**
   referenced by the nightly profile
   (`tests/fixtures/generated/{valid/avi_mjpeg_pcm.avi,
   valid/avi_mjpeg_2pcm.avi, malformed/avi_mjpeg_pcm__trunc3.avi,
   malformed/avi_mjpeg_pcm__noidx0.avi}`). Pinned action SHAs are
   identical across all three jobs and to the pre-existing job.

## Verification runs (selected outputs)

```
# T1 P2-3 loop (re-run): all pass, exit 0
for f in scripts/rc-smoke.sh scripts/release/*.sh; do bash -n "$f" || exit 1; done   # → OK ×8
ls scripts/release/*.sh | wc -l   # → 7
bash -n good1.sh bad2.sh good3.sh # bad2 has a syntax error → exit 0 (pitfall confirmed)

# T1 P3-4 / §7 re-confirmations
rg -c '\[\[setting\]\]'  schemas/settings.toml  # → 33 (unanchored, as diagnosed)
rg -c '^\[\[setting\]\]' schemas/settings.toml  # → 32 (anchored, as recorded)
rg -c '= \{ key = ' keymaps/default.toml        # → 150
rg -c 'key = "extensions\.crash' schemas/settings.toml  # → 5

# T1 P3-6
rg -c '#\[test\]' crates/streaming/src/dash/*.rs            # → 12+4+4+7 = 27
rg -c '#\[test\]' crates/streaming/src (summed)             # → 206 today
git grep -c '#\[test\]' 93b7614 -- crates/streaming/src/**  # → 193

# T1 P3-7
stat -c %s f → 9339 ; wc -c < f → 9339 (identical)
check-6 condition on fake 1.5 MiB \0asm module → "wasm-web-build: PASS (1572864 bytes, \0asm magic)"
missing-file path → FAILED, no stray stderr ; bash -n rc-smoke.sh → OK

# T1 P3-8
git ls-remote origin w24/c  → fb97822a83a9… ; ls-remote origin w24/a → (empty)
git merge-base --is-ancestor fb97822 origin/main → NOT merged (rows stay execute-at-merge)

# T1 gates
/tmp/my-project/target/debug/xtask lint-literals → PASS, 0 violations, 452 allowlisted
python3 -m py_compile scripts/release/{product-env,gen-icon}.py → OK (disk recovered; also ast.parse OK)
node --check scripts/rc-smoke-web.mjs → OK

# T1 matrix recompute (tomllib)
claimed video-codec rows: av1, h264, hevc, mjpeg, prores-dnxhd, vp9 ; h264-software blocked ×4
format rows 73 = 21+16+19+17 ; claimed 37 = 14/6/9/8 ; web 0

# T1 GFM rendering proof (NEW-1)
markdown_it table render of row 1.2 as-written → command cell truncated at "$f",
status cell EMPTY, trailing cells dropped; with `\|\|` → full loop in <code>, status intact

# T2
yaml.safe_load(soak-nightly.yml @ w24/b) → valid; jobs: soak, soak-scenarios, soak-lsan
strict duplicate-key parse → clean ; diff vs main → additions only (0 deletions)
scenarios-nightly.toml → 5 × 48 min = 240 min; all 4 fixtures + 7 referenced files exist
bash -n scripts/soak-scenarios.sh (from w24/b) → OK
```

## Disposition summary

| Severity | Count | Items |
|---|---|---|
| P0/P1 | 0 | — |
| P2 (new) | 1 | NEW-1: row 1.2 `\|\|` escaping (2-char fix) |
| P3 (new, pre-existing) | 1 | NEW-2: vp8-vp9 / prores-dnxhd record status staleness |
| Fixed-and-verified | 8 | P2-1, P2-2, P2-3, P3-4, P3-5, P3-6, P3-7, P3-8 |
| Deferred (per reviewer) | 1 | P3-9 worklog split — Lead's call at wave merge |
| Target 2 findings | 0 | soak-nightly.yml PASS on all 4 checks |

Recommended next action: lane I (or the merger) lands the NEW-1
two-character fix before wave-24 merge; NEW-2 can ride the next
docs-formats pass.
