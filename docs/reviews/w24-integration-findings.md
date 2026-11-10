# W24-R9 — INTEGRATION audit of the wave-24 rehearsal tree (cross-lane seams)

- **Auditor**: W24-R9 (wave 24 review scout 9 — integration focus), read-only on the
  rehearsal branch; fresh worktree `/home/z/my-project/wt/w24r9`, branch `w24/r9`.
- **Audited snapshot**: `w24-integration-rehearsal` @ `286bacd` ("rehearsal: merge
  w24/r7") — **17 merges**: lanes b, c, d, e, f, g, h, i, j, k + reviews r1–r7.
  Diff basis: `git diff origin/main...286bacd` (126 files, +37,333/−228).
- **Live-tip note**: the rehearsal branch **moved during this audit** — it is now
  `3f2b96c` ("rehearsal: merge w24/r8"), adding merges of `w24/l`, `w24/n`, `w24/r8`
  (20 merges total). Every finding below was **re-verified against the live tip**;
  where the newer merges change the verdict it is stated explicitly.

Method: per-lane diff decomposition (each seam's changes must equal the union of the
contributing lanes' diffs — no more, no less), TOML/YAML machine parsing, `bash -n`
plus one live gate-script execution, path-reference resolution across 76 wave-24
docs (238 unique referenced paths), full-tree marker and TODO/FIXME greps scoped to
the wave diff vs `origin/main`.

---

## Severity summary

| Severity | Count | IDs |
|---|---|---|
| P0 | 1 | IN-1 |
| P1 | 3 | IN-2, IN-3, IN-4 |
| P2 | 3 | IN-5, IN-6, IN-7 |
| P3 | 3 | IN-8, IN-9, IN-10 |

---

## P0 — integration blocker at the audited snapshot

### IN-1. The rehearsal snapshot does not compile its UI: committed conflict markers still in `ui/src/App.tsx` (OF-1 unresolved; fix lives on unmerged lane A)

- **File:line**: `ui/src/App.tsx:15–24` and `ui/src/App.tsx:100–104`.
- **Evidence**: full-tree marker grep (task seam 7) finds exactly one real-hit file:
  `<<<<<<< HEAD` / `=======` / `>>>>>>> w23/c` twice — the discovery-vs-convert
  import/route blocks from wave-23 merge `45c5c6f`. The markers are **pre-existing on
  `origin/main`** (verified: `git show origin/main:ui/src/App.tsx` has the same 4
  marker lines), i.e. not introduced by wave 24 — but the 17-merge rehearsal carries
  no fix: `git merge-base --is-ancestor w24/a 286bacd` → **not an ancestor**. Lane A
  holds the fix (`70ed69c`, "resolve committed merge markers … keep BOTH
  discovery+convert; unblocks UI typecheck/build"). The merged gate report records
  the consequence: `docs/gates/phase7-exit.md:55` battery check 16 "UI
  `npm run typecheck` / `lint` — **FAIL** (TS1185/TS1005 parse errors…) — OF-1", and
  its closure path explicitly requires "land lane A's in-flight fixes (OF-1 `w24/a`
  `70ed69c` …)". Raw markers = guaranteed TS syntax errors, so every UI gate/suite
  (typecheck, eslint, vitest incl. the `DiscoveryScreen`/`ConvertScreen` 8/8 suites
  the gate report cites) fails on the snapshot tree.
- **Live tip (3f2b96c)**: **resolved** — the `w24/r8` merge carries lane A's commits
  (r8 was cut on lane A), and `App.tsx` at the tip has **0 markers**.
- **Suggested fix**: none left for OF-1 itself; the Lead's wave-merge battery must
  re-run the UI suite on the tip (this audit did not re-run `tsc`; marker absence is
  verified, green suites are not).

---

## P1 — real, bounded

### IN-2. `scripts/full_gate.sh` has a numbering hole: step 10 does not exist anywhere — 9 → 11, and the reserving comment points at a lane that never landed it

- **File:line**: `scripts/full_gate.sh:70–72`.
- **Evidence**: the merged file's steps are 1–9 then **11** ("packaging gate", lane
  C, commit `1668fb1`). Lines 70–71 comment: "Step 10 is the W24-B soak lane's slot
  (issue #54, card 8.C1 — sibling worktree, lands via its own commit; numbering per
  the wave-24 plan)." Verified against every branch: `git diff origin/main...w24/b
  -- scripts/full_gate.sh` is **empty** (lane B never touched it), and no branch
  (a–k, r1–r8, l, m, n) contains any step-10 body — `rg "web-budget"` across the
  whole repo and all refs: **0 hits**. So the audit brief's "lane a appended step 10
  (web-budget)" matches nothing in the repository: not in the rehearsal, not on lane
  A, not on any lane. The comment's own attribution (soak lane) also never landed.
  What *is* true: steps present are in numeric order, no duplicated numbers,
  `bash -n scripts/full_gate.sh` **passes**, and step 11 is genuinely executable —
  I ran `bash scripts/packaging/gate.sh` in the snapshot worktree: **exit 0, "PASS
  (packaging config + scripts + local-provable halves)"**.
- **Live tip**: unchanged — still 9 → 11.
- **Suggested fix**: either land the reserved soak step (lane B follow-up) or
  renumber step 11 → 10 and delete the stale comment; separately, the Lead should
  reconcile the wave-24 plan's gate-slot table with reality (two different
  attributions for slot 10 — "soak" in-tree, "web-budget" in the wave brief — and
  neither exists).

### IN-3. Lanes B, D, G carry **no worklog entry at all** in the integrated tree (AGENTS.md §4 mandatory); two of the three known-fix branches are unmerged

- **File:line**: `worklog.md` (root, ends at the `W24-R7` entry at the snapshot);
  AGENTS.md §4 "Single shared log … (append-only) … After finishing your Task ID:
  append a section".
- **Evidence**: per-lane check `git diff origin/main...origin/w24/{b,d,g} --
  worklog.md` and `-- docs/worklog/` → **all empty**; no `Task ID: 7.w24b/7.w24d/
  7.w24g` exists anywhere in the integrated tree (root log or otherwise). This is
  already partially known: R4's P1-1 (`docs/reviews/w24-b-soak-findings.md:31–41`)
  documents the missing W24-B entry and the **dangling citation** it causes
  (`docs/soak/soak-report-20261110.md:253–262` §10 points readers at "the W24-B
  worklog entry" for the verification battery's actual results — which therefore
  exist nowhere). Fixes exist but were not merged at the snapshot: `w24/m`
  (`56d74fe` appends the W24-B entry), `w24/p` (`a776c24` appends a reconstructed
  W24-D entry, cross-checked against the W24-L audit), `w24/n` (`5f96b5f` appends
  the retroactive `7.w24g` entry, Lead-assigned).
- **Live tip**: **one of three closed** — `w24/n` merged ⇒ `7.w24g` (+ `7.w24n`)
  present at `worklog.md:835/946`; **W24-B and W24-D entries still missing**
  (`w24/m`, `w24/p` still unmerged), and the soak-report §10 citation still
  dangles.
- **Suggested fix**: merge `w24/m` and `w24/p` before wave close (they also carry
  the R4 P2/P3 soak fixes — see IN-4); verify the soak report's §10 pointer then
  resolves.

### IN-4. Known, already-fixed review findings are still live in the integrated tree because fix-lane `w24/m` (soak) is unmerged

- **File:line**: `docs/soak/soak-report-20261110.md` (R4 P2-3 wrong post-warmup
  span in §2's churn row), `tests/soak/scenarios.schema.json:5` (R4 P2-4 $comment
  cites `docs/soak/regression-guards.md`, which does not exist — content lives in
  the soak report §6.2), plus R4 P2-2 (dead worst-slope fold line) and the P1-1
  worklog item above.
- **Evidence**: all four have single-purpose fix commits on `w24/m` (`3295b1b`,
  `a25a427`, `aa04202`, `56d74fe` — verified in `git log origin/main..w24/m`);
  `git merge-base --is-ancestor w24/m 286bacd` → not an ancestor; the dangling
  `docs/soak/regression-guards.md` reference is still present in the integrated
  tree (my path-resolution sweep lists it unresolved).
- **Live tip**: still unmerged — all four findings still open in-tree.
- **Suggested fix**: merge `w24/m` (it is lane B's sanctioned fix lane, all commits
  scoped to R4 finding IDs).

---

## P2 — real, bounded

### IN-5. Lane A's content is absent at the snapshot and only partially present at the live tip — and the wave brief's expected lane-A artifacts (web column, step 10, ci.yml budget gate) exist on **no branch at all**

- **File:line**: `docs/feature-matrix.toml` (web column), `scripts/full_gate.sh`
  (step 10), `.github/workflows/ci.yml`.
- **Evidence**: at the snapshot, `w24/a` is not an ancestor (see IN-1) and none of
  its files exist (`ui/e2e/web/`, `docs/web/`, `ui/playwright.web.config.ts` — all
  absent). Independent of the merge state, the three deliverables this audit was
  asked to verify "survived the merge" **do not exist on any branch**:
  - web column: lane A's diff touches no `docs/feature-matrix.toml`; the integrated
    TOML's web column is unchanged vs main (web `partial` = 4, `implemented` = 0
    both on main and integrated — machine-parsed, 252 rows);
  - step 10 "web-budget": zero hits repo-wide and across all refs (IN-2);
  - ci.yml budget gate: `ci.yml` at snapshot **and** live tip is byte-identical to
    `origin/main` (blob-hash compared).
  Lane A's actual committed scope is the browser-matrix harness (issue #49, card
  7.I1: `ui/e2e/web/**`, `ui/playwright.web.config.ts`, `ui/scripts/web-matrix-run.mjs`,
  `docs/web/browser-matrix-20261110.json`, `crates/playback-engine/src/events.rs`).
- **Live tip**: `w24/r8`'s merge brings lane A **up to `9a13c39` only** — the lane's
  final commit `389bfcc` ("record fragment BEFORE verdict + first real chromium
  matrix capture") is **not** an ancestor of the tip, so the in-tree
  browser-matrix record is one commit stale relative to lane A's head.
- **Suggested fix**: merge `389bfcc` (or rebase r8's audit note); the Lead should
  re-baseline the wave-24 deliverable list for the web lane — the web-column pass
  and budget gate are unstarted, whomever they were assigned to.

### IN-6. The merged gate report promises `docs/web/browser-matrix.md`; the deliverable that exists is `docs/web/browser-matrix-20261110.json` (name drift — reference dangles at snapshot **and** live tip)

- **File:line**: `docs/gates/phase7-exit.md:192` (§4 row 7.I1: "Delivers
  `docs/web/browser-matrix.md` + web feature-matrix honesty pass").
- **Evidence**: `docs/web/browser-matrix.md` exists on **no** branch (checked main,
  a–k, r1–r8, l, m, n); lane A delivered `docs/web/browser-matrix-20261110.json`
  (present at the live tip via r8). The row is honestly labelled "IN-FLIGHT — no
  pass claimed", and R3's own P3-6 (`docs/reviews/w24-f-gate-findings.md`) noted
  the file "does not exist yet on w24/a" — but even post-merge the literal path
  never resolves. Also note the second half of that row ("web feature-matrix
  honesty pass") remains undelivered (IN-5).
- **Suggested fix**: repoint the citation at the `.json` (and the gate's §5 web
  snapshot), or land a short `browser-matrix.md` summary that links the JSON.

### IN-7. Merged review docs cite unmerged lane-A paths — dangling at the audited snapshot (resolved at the live tip)

- **File:line**: `docs/reviews/w24-f-gate-findings.md:120` (R3 P3-6: "the branch
  has since advanced to `d1cba99` (browser-matrix harness:
  `ui/playwright.web.config.ts`, `ui/scripts/web-matrix-run.mjs`)").
- **Evidence**: at the snapshot neither path exists in-tree (lane A unmerged), so a
  reader following the merged report's citation hits nothing. This is the general
  cross-lane seam: reports merged before the lane they reference. At the live tip
  both paths resolve (via r8).
- **Suggested fix**: none needed post-r8; noted as a pattern — merge order matters
  for citation integrity (reports referencing sibling lanes should land after, or
  with, those lanes).

---

## P3 — nits, fix opportunistically

### IN-8. `codec-audio-aac-lc` evidence string double-appended in the merged feature-matrix (parity tool appends per-transition without dedupe)

- **File:line**: `docs/feature-matrix.toml` — `codec-audio-aac-lc` `evidence =`
  line (the exact string `docs/corpus/compat-matrix-20261110.md (this run's matrix
  table + raw rows)` appears **twice**, joined by `; `).
- **Evidence**: lane D's ledger
  (`docs/corpus/compat-matrix-transitions-applied.jsonl`) contains **6 transitions
  over 5 rows** — two target `codec-audio-aac-lc` (concealment finding + refused
  `aac-lc` decoder finding, ledger lines 2–3), and the applier appended the same
  evidence reference once per transition. Pre-existing verbatim on `origin/w24/d`
  (count = 4 phrase hits there too) — the merge preserved it faithfully; this is a
  lane-D tooling wart, not a merge artifact. Everything else about the parity
  discipline checks out (see seam verdict below).
- **Suggested fix**: one-line doc dedupe + an idempotence guard in
  `xtask compat-matrix apply-transitions` (skip an evidence append whose exact
  string is already present).

### IN-9. `.gitignore` union duplicate: `target-xtask/` listed twice

- **File:line**: `.gitignore` (entry near the top, "the c339ee8 precedent", and
  again at the bottom, "isolated xtask build dir").
- **Evidence**: lanes D (`ecc615e`) and G (`56b59c1`) each added the same pattern
  with different comments; the textual union kept both blocks (both merge diffs
  show clean auto-merges, no conflict). Functionally harmless (idempotent ignore).
  The rest of the union is sane: `target-w23e/`, `target-w*/`, `/appimagetool.AppImage`,
  `/type2-runtime`, `__pycache__/`, packaging-evidence log exceptions — all present,
  no contradictions.
- **Suggested fix**: drop one of the two blocks at wave close.

### IN-10. Review agents R1/R3/R4/R6 wrote per-agent log files under `docs/worklog/` instead of appending to the root `worklog.md` (inconsistent AGENTS.md §4 adherence across review lanes)

- **File:line**: `docs/worklog/w24-r1-review.md`, `w24-r3-review.md`,
  `w24-r4-review.md`, `w24-r6-review.md` (plus lane H's supplementary
  `w24-h-review-fixes.md` — H *also* has a correct root entry, so that one is
  extra detail, not a substitution).
- **Evidence**: each file carries a proper `Task ID:` worklog entry; none of R1/R3/
  R4/R6 appended to the root log (their rehearsal merges add 0 worklog lines),
  while R2/R5/R7 (and R8 at the tip) did append to the root per §4. AGENTS.md §4:
  "Never create separate log files (the `docs/worklog/` archive is Lead-maintained
  bookkeeping … agents append to the root `worklog.md` only)"; the archive README's
  convention is one file per phase cluster — these are neither. Content is
  preserved (nothing lost), but the "live" log an agent reads at session start is
  missing four review entries.
- **Suggested fix**: Lead folds the four files' entries into the root log (or
  ratifies the per-review-file convention and updates §4); going forward review
  scouts append to the root like R2/R5/R7/R8 did.

---

## Per-seam verdicts (audit brief, in order)

| # | Seam | Verdict | Detail |
|---|---|---|---|
| 1 | **feature-matrix.toml coherence** | **PASS with 1 P3 + 1 N/A** | Lane D's 5 linux `implemented→partial` transitions (asf-wmv, vp9, pcm, aac-lc, alac) present and byte-consistent with the 6-line ledger (6 = 5 rows + aac-lc double transition); linux counts machine-recounted: implemented 64→59, partial 46→51; TOML parses clean (252 rows). Lane G's OF-5 retraction present verbatim in the `codec-video-h264` notes ("Media Foundation … NOT IMPLEMENTED … claim was false"), status cells untouched — exactly as R5 verified. **No row hand-edited outside the parity tool**: integrated diff (34 lines) decomposes exactly into lane D (32) + lane G (4), zero overlap, zero residue. Lane A's web column pass: **absent — does not exist on any branch** (IN-5). AAC evidence dup = IN-8. |
| 2 | **full_gate.sh steps 10+11** | **FAIL (numbering hole)** | Steps in order 1–9 then 11; **step 10 does not exist** — comment reserves it for lane B, which never landed it (empty diff on w24/b); no "web-budget" step anywhere (IN-2). No duplicated step numbers. `bash -n` PASS on the merged file and on `scripts/packaging/gate.sh`; step 11 **executed live in this audit: exit 0 / PASS**. |
| 3 | **Workflow files** | **PASS** | All 6 workflows `yaml.safe_load` clean; no duplicated job ids or display names within any file; workflow names distinct (CI, CI-Full, Compat-Matrix, Release, Soak-Nightly, UI). Action pins **byte-identical across all files** (checkout `3d3c42e` v7.0.1, rust-toolchain `d124f86`, rust-cache `6323deb` v2.9.2, upload-artifact `cf430e0` v7.0.2, download-artifact `9000827` v8.0.2). cargo-deny pinned exactly once (ci.yml, 0.20.2 SHA-256-verified) — release.yml references that posture in prose without re-pinning (consistent); compat-matrix.yml and soak-nightly.yml don't use it. Triggers sane: compat-matrix manual-dispatch-only (billing/hardware blockers documented), soak-nightly cron 03:00 vs ci-full 03:17, release tag-gated. Workflow→repo-path references all resolve (WiX `tools/…` paths are runner-side nupkg-internal; `tests/soak/main.rs` comment shorthand = `crates/playback-engine/tests/soak/main.rs`, exists). ci.yml additive budget gate: **absent** (IN-5). |
| 4 | **worklog.md union integrity** | **PASS on mechanics / FAIL on coverage** | Union is clean: across all 17 rehearsal merges, worklog.md diffs show **0 deleted lines, additions only** (verified per merge); the 10 root entries (C, E, F, H, I, J, K, R2, R5, R7) each appear exactly once, in merge order; diff vs main is pure append (context starts at the pre-wave tail). AGENTS.md §4 append-only respected — no prior entry rewritten. Coverage gaps: lanes **B, D, G have no entry anywhere** (IN-3); R1/R3/R4/R6 used side files (IN-10). Live tip adds `7.w24g`, `7.w24n`, `W24-L r2`, `W24-R8` root entries (lane n + l + r8 merges) — B and D still missing. |
| 5 | **Cross-lane file collisions** | **PASS with 1 P3** | `docs/reviews/` at snapshot: 7 reports (b, c, e, f, g, h, i) — all present, distinct, no collisions; lane-D and lane-A reports absent because reviewer branches `w24/l`/`w24/r8` were unmerged (both present at the live tip → 9 distinct). `docs/worklog/` new files distinct. `.gitignore` union sane apart from the `target-xtask/` duplicate (IN-9). `tests/compat/` is a deliberate standalone workspace (empty `[workspace]`, bench-vs-vlc precedent) with its own Cargo.lock (98 pkgs) — root `Cargo.lock`/`Cargo.toml` **unchanged** by the wave (blob-identical), no conflict. |
| 6 | **Docs cross-references** | **PASS with enumerated exceptions** | 238 unique referenced paths across 76 wave-24 docs; all resolve except: lane-A paths at the snapshot (IN-7; resolved at tip), `docs/web/browser-matrix.md` name drift (IN-6), `docs/soak/regression-guards.md` (R4 P2-4, fix unmerged — IN-4), and by-design runtime-cache paths (`tests/compat/.regen-cache/*` — gitignored, manifest+sha256 pins are the committed truth). Lane e/i/j → packaging runbook refs (`docs/packaging/README.md`, `docs/packaging/evidence/*`) all exist; release-checklist refs to `scripts/rc-smoke.sh`, `scripts/rc-smoke-web.mjs`, all 7 `scripts/release/*.sh`, all 17 `scripts/packaging/*` — all exist; gate-report refs resolve (crate-relative test paths verified under their crates; host-absolute `scripts/cargo-env.sh` exists on dev hosts — portability nit). R2's phantom `docs/audit/rc-smoke-2026-11-10.md` citations were properly fixed by lane I (re-verified: zero citations remain in `docs/formats/`). |
| 7 | **Sanity greps** | **1 real hit (pre-existing, P0-logged) / TODO scan PASS** | Merge markers: exactly one real file — `ui/src/App.tsx` (4 marker lines; pre-existing on main, IN-1; **0 markers at live tip**). All other marker-string hits are the pre-existing conflict-*resolution* helper scripts (`scripts/resolve_*.py`) — intentional. TODO/FIXME/XXX/`unimplemented!`/`todo!(` scoped to the wave's changed non-test code files: **zero new hits** vs main (2 false-positive `\uXXXX` doc-comment matches only); tree-wide non-test code still carries exactly the 2 pre-existing allowlisted hits (`crates/audio-output/src/cpal.rs:36`, `crates/codecs/src/audio/aac/error.rs:13`), unchanged by the wave. |

---

## Integration readiness verdict

**NOT READY at the audited snapshot (`286bacd`, 17 merges)** — four blockers, all
bounded and all already characterised by the wave's own review chain:

1. UI does not compile (OF-1 markers; lane A unmerged) — IN-1.
2. `full_gate.sh` step-10 slot empty with a stale reserving comment — IN-2.
3. Worklog coverage: lanes B/D/G entry-less, soak-report §10 citation dangling — IN-3.
4. Lane B's known P1/P2 fixes (w24/m) and lane D's worklog fix (w24/p) unmerged — IN-4.

**The live tip (`3f2b96c`, 20 merges: +l, n, r8) closes the biggest one** — lane A's
content (incl. the OF-1 fix and the a/d review reports) is in, `7.w24g`/`7.w24n`/
`W24-L`/`W24-R8` worklog entries landed, App.tsx is marker-free. Still open at the
tip: `w24/m` + `w24/p` unmerged (IN-3/IN-4), lane A's final commit `389bfcc` not an
ancestor (IN-5), step 10 still absent (IN-2), `browser-matrix.md` name drift (IN-6),
and the web-column/budget-gate deliverables exist nowhere (IN-5). The seams that
*were* rehearsed (feature-matrix parity, workflows, gate step 11, worklog union
mechanics, file-collision hygiene, docs xrefs, marker/TODO hygiene) are in good
shape — the merge machinery itself introduced **zero** regressions; every defect
found is inherited from a lane's own state or from lanes still outside the
rehearsal.

**Recommended merge order to green**: `w24/m` → `w24/p` → lane-A tip `389bfcc` →
re-run the phase7-exit §8 battery + UI suite on the result → then the rehearsal is
a faithful "all content lanes + all reviews" state.
