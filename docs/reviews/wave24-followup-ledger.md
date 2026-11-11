# Wave 24 — Consolidated Follow-up Ledger (W24-AC)

- **Auditor**: W24-AC (wave 24 lane AC — consolidated follow-up ledger), read-only synthesis of the whole wave.
- **Branch**: `w24/ac` off `origin/w24/r16` (`702ad89`). Files touched by this lane: this ledger + the repo-root `worklog.md` entry. Nothing else; main untouched.
- **Purpose**: ONE deduplicated, prioritized ledger of EVERY finding filed in wave 24 (review scouts R1–R16, integration scout R9's IN-*, compat scout L, fixer audits T, plus the gate lane F's own OF-* findings), tracking each lifecycle FILED → FIXED (by which lane/commit) → VERIFIED (by which audit) | DEFERRED (reason, owner) | OPEN (owner, next step).
- **Method**: all 17 wave-24 review reports read in full (16 collected at the integration-rehearsal tip `88db8a1` + `w24-vw-fixes-findings.md` on this branch); every fixer-lane branch log read (`origin/w24/{h,i,j,k,m,n,p,q,s,u,v,w,x,y}` + local `w24/{a,z,aa,ab,r15}`); fix commits, worklog entries, and the rehearsal merge chain cross-checked (`git log`/`rev-parse`/`ls-tree`/`show`); current tree state re-verified for every OPEN/pending row (e.g. `full_gate.sh` step-10 hole, lane-A honesty note, `.gitignore` dedupe) on 2026-11-11.

## Sources inventory (every report, its branch, its tip)

| Report | File | Branch @ tip | Scope audited |
|---|---|---|---|
| R1 | `docs/reviews/w24-c-packaging-findings.md` | `origin/w24/r1` `cfc196c` | packaging lane `w24/c` (issue #56) |
| R2 | `docs/reviews/w24-e-docs-findings.md` | `origin/w24/r2` `64054c9` | docs lane `w24/e` (issue #58) |
| R3 | `docs/reviews/w24-f-gate-findings.md` | `origin/w24/r3` `27f1330` | Phase-7 exit gate `w24/f` (issue #51) |
| R4 | `docs/reviews/w24-b-soak-findings.md` | `origin/w24/r4` `a8b5f06` | soak lane `w24/b` (issue #54) |
| L | `docs/reviews/w24-d-compat-findings.md` | `origin/w24/l` `54840f4` | compat-matrix lane `w24/d` (issue #53) |
| R5 | `docs/reviews/w24-g-fixes-findings.md` | `origin/w24/r5` `6cfdc06` | fix lane `w24/g` (OF-2..OF-5) |
| R6 | `docs/reviews/w24-h-fixes-findings.md` | `origin/w24/r6` `5e8416c` | fix lane `w24/h` (R1's findings) |
| R7 | `docs/reviews/w24-i-fixes-findings.md` | `origin/w24/r7` `404d27e` | fix lane `w24/i` (R2's) + `w24/b` soak-nightly.yml |
| R8 | `docs/reviews/w24-a-web-findings.md` | `origin/w24/r8` `bf54628` | lane A committed work @ `9a13c39` (issue #49) |
| R9 | `docs/reviews/w24-integration-findings.md` | `origin/w24/r9` `d47d578` | rehearsal tree @ `286bacd`, live tip `3f2b96c` |
| T | `docs/reviews/w24-mq-fixes-findings.md` | `origin/w24/t` `8a29077` | fix lanes `w24/m` (R4's) + `w24/q` (R6's P4s) |
| R10 | `docs/reviews/w24-p-fixes-findings.md` | `origin/w24/r10` `56ca81a` | fix lane `w24/p` @ `022d679` (L's findings) |
| R11 | `docs/reviews/w24-a-newcommits-findings.md` | `origin/w24/r11` `eaf7970` | lane A new commits `9a13c39..b22bff5` |
| R12 | `docs/reviews/w24-s-chain-findings.md` | `origin/w24/r12` `5224bd5` | gate-doc chain `w24/f→k→s` (issue #51) |
| R13 | `docs/reviews/w24-u-fixes-findings.md` | `origin/w24/r13` `4874ab5` | fix lane `w24/u` (T's F-1) |
| R14 | `docs/reviews/w24-p-final-findings.md` | `origin/w24/r14` `98fb76e` | final state of `w24/p` @ `c48dc76` |
| R16 | `docs/reviews/w24-vw-fixes-findings.md` | `origin/w24/r16` `702ad89` | fix lanes `w24/v` (R12's) + `w24/w` (plan-doc drift) |
| Gate | `docs/gates/phase7-exit.md` (lane F, `w24/f` `833944e`) | — | its own OF-1..OF-5 findings (verified by R3) |

**Not filed / not pushed (checked, not invented):**
- **R15** — no `origin/w24/r15` exists; local `w24/r15` == rehearsal tip `88db8a1` (no own commits); its worktree is clean and contains no r15 findings doc. Whatever R15 was assigned (per the wave pattern, a battery/audit round) produced nothing reachable — the phase7-exit **§8 re-run battery remains unexecuted** (owner: Lead at wave merge).
- **AA** — local `w24/aa` == rehearsal tip `88db8a1`, no own commits, unpushed. No findings filed.
- **AB "size audit"** — local `w24/ab` == lane A tip `c1c37b2`, no own commits, unpushed; no review/audit doc anywhere. The 7.K1 size work itself exists on lane A (`30153fa` feature gates, `b85966f` `docs/web/size-report.md` + `size-record.toml`, `c1c37b2` `[web.size_budgets]` config) — see IN-5.

**Integration-rehearsal state** (`w24-integration-rehearsal` @ `88db8a1`, through the r14 merge): everything through lanes b–n, p–w and reviews r1–r14 + T + L is merged, **except**: `w24/x` (R14-1 fix `3117bb7`), `w24/y` (R13 F-U1..3 fixes `a6d6cb7`), and lane A's post-`b22bff5` commits (`30153fa`, `b85966f`, `c1c37b2`, the 7.K1 work — lane A itself is still **unpushed**: no `origin/w24/a`).

## Status legend

- **F+V** — FIXED and independently VERIFIED (fix commit + verifying audit named).
- **F(pend)** — fix landed (commit named) but no independent audit has verified it yet.
- **DEFERRED** — consciously deferred, recorded, owner named.
- **OPEN** — not fixed; owner + next step named.
- **NO-ACTION** — filed but the filing report itself demanded no action (informational / accepted).

---

## 1. Finding lifecycle tables

### 1.0 Gate lane F's own findings (OF-*) — filed in `docs/gates/phase7-exit.md` (`w24/f` `833944e`); all five substance-verified by R3 (`w24/r3` `27f1330`)

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| OF-1 | P0 | Committed merge markers in `ui/src/App.tsx` (UI does not compile) | lane A `70ed69c` (w24/a, unpushed) | R8 (marker count 0); R9 at live tip `3f2b96c`; in rehearsal via r8 merge `fc51243` | **F+V** — merge to main pending (Lead) |
| OF-2 | P1 | Stale seed-count pins (27 vs 32) in settings-schema/cli | `w24/g` `1c48902` (pins derived + shape guard) | R5 (43/0, 8/0, 125/0 suites); V's addendum `978ed91` R16-verified | **F+V** |
| OF-3 | P1 | Stale `ui/.../generated/schema.ts` | lane A `771259c` + `w24/g` `64757b1` (byte-identical, "one artifact, two lanes") | R5 (settings:check green); R16 (byte-identity re-proven) | **F+V** |
| OF-4 | P2 | Rebrand reverse-control test mis-rooting under shared target dir | `w24/g` `ba9659f` (CARGO_MANIFEST_DIR rooting) | R5 (12/0+3i); R16 | **F+V** |
| OF-5 | P1 | feature-matrix note over-claims Media Foundation as landed | `w24/g` `de9b341` (MF retracted) | R5 (note == code); R9 seam 1 (verbatim in merged tree); R16 | **F+V** |

### 1.1 R1 — packaging (`w24-c-packaging-findings.md`, `w24/r1` `cfc196c`): 10 findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| P1-1 | P1 | `sign-macos.sh` `|| true` demotes hard-fails to unsigned fallback | `w24/h` (rc-contract rewrite; H closeout `45c7136`) | R6 (negative tests re-run from scratch) | **F+V** |
| P1-2 | P1 | `release.yml` zero `secrets.*` mappings | `w24/h` (22 mappings + visibility guard) | R6 (guard semantics simulated both directions) | **F+V** |
| P2-1 | P2 | .wxs hardcodes docs list vs config | `w24/h` (gen-wxs emits per-`PACKAGING_DOCS` rows) | R6 (functional + refusal tests) | **F+V** |
| P2-2 | P2 | WIX_EXE POSIX path breaks pwsh `Test-Path` | `w24/h` (drive-letter conversion, `[VERIFY]` labeled) | R6 (structural) | **F+V** — live-run VERIFY remains (user CI billing) |
| P2-3 | P2 | evidence table digests contradict the log | `w24/h` (regenerated; also `00821d3` on w24/c) | R6 (all 5 digests grep-found, stale ones absent) | **F+V** |
| P2-4 | P2 | "every artifact packages BOTH binaries + docs" false for AppImage/.app | `w24/h` (per-artifact payload table) | R6 (cross-checked vs builders) | **F+V** |
| P3-1..P3-4 | P3 | product-name literals / signtool newest-dir-only / sandbox path in runbook / app-zip missing sidecar | `w24/h` `7f2f894` | R6 (replica tests; per-item verification) | **F+V** (all four) |

### 1.2 R2 — docs lane (`w24-e-docs-findings.md`, `w24/r2` `64054c9`): 9 findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| P2-1 | P2 | §1.1 table misclassifies h264-software, omits vp9 | `w24/i` (`f00385c`…`ca42190`) | R7 (own tomllib parse) | **F+V** |
| P2-2 | P2 | format records cite never-committed rc-smoke doc | `w24/i` | R7 (zero hits re-grep) | **F+V** |
| P2-3 | P2 | `bash -n` multi-file pitfall in checklist row 1.2 | `w24/i` `d6c3920` | R7 (loop re-run 8/8) — introduced R7 NEW-1, fixed by J (§1.8) | **F+V** |
| P3-4..P3-8 | P3 | anchored count / 7-vs-8 scripts / dash 191-vs-206 / GNU stat / stale w24/c note | `w24/i` (`eddd1db`, `61fe4f3`, `cda3d8a`) | R7 (each re-run) | **F+V** (all five) |
| P3-9 | P3 | single consolidated W24-E worklog entry (r1–r4) | — | R7: "ACCEPTED (deferred per reviewer)" | **DEFERRED** — optional per-round split at wave merge, owner Lead |

### 1.3 R3 — gate evidence (`w24-f-gate-findings.md`, `w24/r3` `27f1330`): 7 findings + 2 obs

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| P2-1 | P2 | §2 header-convention claim unmet by 3 r1 logs | `w24/k` `ef15bcc` (retroactive headers, prepend-only) | R12 (byte-diff prepend-only proof, +29/−0) | **F+V** |
| P3-1..P3-5 | P3 | wildcard 30≠32 / `d33886d` hash typo / R-20→R-16 miscite / OF-3 stale next-step / two line-range drifts | `w24/k` `ef15bcc` | R12 (all recounts + citations exact) | **F+V** (all five) |
| P3-6 | P3 | 7.I1 tip snapshot aging | none demanded (correct at report time; S later re-anchored the row) | R12 (managed honestly) | **NO-ACTION** |
| O-1, O-2 | obs | target-dir relocation changes OF-4 failure mode at §8 re-run; 12-vs-11 log brief discrepancy | — | — | informational (§8 battery should note) |

### 1.4 R4 — soak lane (`w24-b-soak-findings.md`, `w24/r4` `a8b5f06`): 9 findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| P1-1 | P1 | W24-B worklog entry missing; §10 citation dangles | `w24/m` `56d74fe` | T (entry truthful; battery matches R4's re-run) | **F+V** |
| P2-2 | P2 | `INFINITY` fold initializer → dead worst-slope headline | `w24/m` `aa04202` | T (empirical revert/re-run discrimination proof) | **F+V** |
| P2-3 | P2 | churn row carried the 1M run's span 849,815 | `w24/m` `3295b1b` | T (arithmetic recomputed; runner now records span) | **F+V** |
| P2-4 | P2 | schema $comment cites nonexistent regression-guards.md | `w24/m` `a25a427` | T (repoint verified) | **F+V** |
| P3-1..P3-5 | P3 | sink bytes never folded / 29.4-min figure / unlabeled pre-09ad85a calibration / silent VmRSS absence / hardcoded op-shape constants | `w24/m` (`3aa4b62`, `0b34933`, `7f6ed4d`, `dde8674`, `3fd6f5f`) | T (live CI-short run + per-item proofs) | **F+V** (all five) |

### 1.5 L — compat-matrix lane (`w24-d-compat-findings.md`, `w24/l` `54840f4`): 13 findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| P1-1 | P1 | W24-D worklog entry missing (9-commit lane) | `w24/p` `a776c24` (retroactive, cross-checked vs L) | R10 (truthful, exact format, append-only); R14 (intact) | **F+V** |
| P2-1 | P2 | 4 in-row refusals counted but never surfaced; §6.2 overstates | `w24/p` `16d64b0` | R10 (string-for-string vs renderer; recomputed partition) | **F+V** |
| P2-2 | P2 | web-wasm cell hard-coded, not derived from step outcomes | `w24/p` `022d679` | R10 (reachability + byte-identical pass path) | **F+V** |
| P3-1 | P3 | verify_diff defeats the notes-insertion path | — (untouched at `c48dc76`) | R14 (re-verified untouched) | **DEFERRED** — recorded by W24-X's W24-P entry; owner: compat-matrix owner (Lead to assign) |
| P3-2 | P3 | zero unit tests for verdict/config logic | `w24/p` `74bda46` (+`b30a925`) | R14 (10 oracle + 5 config tests, messages checked line-by-line) | **F+V** |
| P3-3 | P3 | "(linux)" hard-code + dead `refusal_count` | `w24/p` `2a43b78` + `16d64b0` | R14 | **F+V** |
| P3-4 | P3 | "VLC VLC" doubled word | `w24/p` `8f72b30` | R14 | **F+V** |
| P3-5 | P3 | `VlcSource::CommittedRecord` mislabel | `w24/p` `b30a925` (new `VlcSource::None`) | R14 (no committed artifact affected) | **F+V** |
| P3-6 | P3 | first-audio limb dump vs fatal | — (decode.rs untouched) | R14 (re-verified) | **DEFERRED** — recorded by W24-X; owner: compat-matrix owner |
| P3-7 | P3 | 0385-dynres regen entry can never regenerate | `w24/p` `cd7d5c5` (documented debt: purpose + §7 next-step) | R14 ("correct scope call"; re-authoring still needs an owner) | **F+V** as documented debt — **re-authoring OPEN** (corpus owner, Lead to assign) |
| P3-8 | P3 | stale claims snapshot (5 unappliable proposals on local re-run) | — (config untouched) | R14 (re-verified) | **DEFERRED** — recorded by W24-X; owner: compat-matrix owner |
| P3-9 | P3 | `read_capped` unbounded read | — (proc.rs untouched) | R14 (re-verified) | **DEFERRED** — recorded by W24-X; owner: compat-matrix owner |
| P3-10 | P3 | unmarked hand-edited md hybrid | `w24/p` (provenance blockquote + JSON note, beyond the ask) | R10; R14 (extended by each editorial commit) | **F+V** |

### 1.6 R5 — audit of `w24/g` (`w24-g-fixes-findings.md`, `w24/r5` `6cfdc06`): verified OF-2..5 FIXED (§1.0); own findings R5-1..R5-4

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| R5-1 | P2 | shared-target-dir binary clobber live (unprovenanced shared-dir test results are not evidence) | — (`scripts/cargo-env.sh` is a sandbox-side helper, not a repo file; no branch addresses it) | — | **OPEN** — owner Lead/tooling; [PROPOSAL]: per-lane `CARGO_TARGET_DIR` suffixing or fingerprint namespacing |
| R5-2 | P3 | g lane's worklog entry missing | `w24/n` `5f96b5f` (retroactive `7.w24g`, Lead-assigned) | R9 live-tip check (`7.w24g`/`7.w24n` present); rehearsal merge `856788f` | **F+V** |
| R5-3 | P3 | tautological placement.screen assert in integration.rs | `w24/n` `a9258aa` | merged into rehearsal via `856788f`; **no dedicated independent audit** (test-only hardening; rides the workspace suite) | **F(pend)** |
| R5-4 | note | Rust shape guard intentionally excludes the UI artifact (settings:check + vitest own it) | — | — | **NO-ACTION** — accepted design; Lead ratification listed in §4 |

### 1.7 R6 — audit of `w24/h` (`w24-h-fixes-findings.md`, `w24/r6` `5e8416c`): verified all 10 R1 findings (§1.1); own P4 observations

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| P4-1 | P4 | wiring guards enumerate only the 8 known secret names | `w24/q` `a6626fa` (guards derive set from packaging.toml) | T (synthetic 9th-key scenario; old guard passes silently, new fails) | **F+V** |
| P4-2 | P4 | doc basenames unescaped in `File/@Source` (gen-wxs.sh) | `w24/q` `f1d45c8` (+`5c9795b`) | T (pre-fix bug reproduced; real-config output byte-identical) | **F+V** |
| P4-3 | P4 | README "41 steps" stale | `w24/q` `b0901a0` | T (recount 42/42) | **F+V** |
| P4-4 | P4 | space-separated PACKAGING_DOCS can't represent spaced paths (opaque error) | `w24/q` `35b3b7e` | T (space + tab fixtures → actionable rc 1) | **F+V** |

### 1.8 R7 — audit of `w24/i` + soak-nightly cross-check (`w24-i-fixes-findings.md`, `w24/r7` `404d27e`): verified R2's fixes (§1.2); own findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| NEW-1 | P2 | row 1.2's raw `\|\|` breaks GFM table rendering (command truncated, status cell empty) | `w24/j` `60e3db8` | merged into rehearsal `3b2c79e`; **no dedicated audit** (2-character escape; R7 pre-verified the suggested form renders correctly) | **F(pend)** |
| NEW-2 | P3 | vp8-vp9 / prores-dnxhd record status front-matter stale vs 2026-11-09 matrix | `w24/j` `c5aade9` (dated addenda) | merged into rehearsal `3b2c79e`; **no dedicated audit** | **F(pend)** |
| — | — | Target 2: `w24/b` soak-nightly.yml — PASS on all 4 checks, no findings | — | — | closed (no findings) |

### 1.9 R8 — lane A web (`w24-a-web-findings.md`, `w24/r8` `bf54628`): 7 findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| P2-1 | P2 | final browser-matrix evidence not committed (chromium modes `[]`, record predates fixes) | `389bfcc`+`ca21cb5` (capture + stamp regen) — **half-closed** | R11: "HONEST" mechanism, but the final-tree re-run is still outstanding (≡ R11 P2-1) | **OPEN** — owner lane A: re-run the matrix on the final tree and commit the fresh record |
| P2-2 | P2 | `report.markdownName` dead knob | lane A `ca21cb5` (`renderMatrixMarkdown()`) | R11 (byte-identical re-render; knob no longer dead) | **F+V** |
| P3-1..P3-4 | P3 | HARNESS_TIMEOUT_MS literal / playwright timeout literal / spawn-error skips shutdown / parseRange multi-range | lane A `b22bff5` (waits.timeoutMs, runner.timeoutMs, `.finally`, comma→200) | R11 (each verified in code) | **F+V** (all four) |
| P3-5 | P3 | worklog entry not yet committed | — (re-verified absent at lane A tip `c1c37b2`) | R11 (≡ R11 P3-4) | **OPEN** — owner lane A close-out |

### 1.10 R9 — integration (`w24-integration-findings.md`, `w24/r9` `d47d578`): 10 findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| IN-1 | P0 | rehearsal snapshot does not compile UI (OF-1 markers; lane A unmerged) | lane A `70ed69c` (via r8 merge `3f2b96c`) | R9 at live tip (0 markers); note: green UI suites not re-run — Lead's §8 battery owes that | **F+V** — main merge pending |
| IN-2 | P1 | `full_gate.sh` numbering hole: step 10 never landed; stale reserving comment (9→11) | — (re-verified present at rehearsal tip `88db8a1`, line 70) | this ledger, 2026-11-11 | **OPEN** — owner Lead: land the reserved soak step or renumber 11→10 + delete comment; reconcile the wave-plan gate-slot table (soak vs web-budget attribution) |
| IN-3 | P1 | lanes B/D/G carry no worklog entry | B: `w24/m` `56d74fe`; D: `w24/p` `a776c24`; G: `w24/n` `5f96b5f` — all merged into rehearsal | T (B); R10 (D); R9 live tip (G) | **F+V** |
| IN-4 | P1 | R4's fixed findings still live because w24/m unmerged | rehearsal merge `c694387` | T (fix quality) + merge | **F+V** |
| IN-5 | P2 | lane-A content absent/partial; web column, step 10, ci.yml budget gate exist on **no branch** | partially: lane A `c1c37b2` lands `[web.size_budgets]` as CONFIG + `b85966f` size report (both unpushed/unmerged; `web_budget.rs` in-flight in the busy worktree); web column + ci.yml gate still untouched everywhere | S's re-verification (still true); this ledger re-check | **OPEN** — owner Lead: re-baseline the wave-24 web-lane deliverable list (honesty-pass half + budget gate); see §4 |
| IN-6 | P2 | gate report promises undated `docs/web/browser-matrix.md` (name drift) | `w24/s` `4277355` (+ rehearsal `5ecad77`); plan-doc origin closed by `w24/w` `8e97c56` | R12 (citations resolve); R16 (W verified) | **F+V** |
| IN-7 | P2 | merged review docs cite unmerged lane-A paths | r8 merge `3f2b96c` | R9 at live tip | **F+V** (no action remained) |
| IN-8 | P3 | aac-lc evidence string double-appended (applier lacks dedupe) | `w24/p` `b5aff90` (idempotence guard) + `6a1677c` (data dedupe) | R14 (guard limbs unit-tested; full-file scan: zero duplicated segments on all 252 rows) | **F+V** |
| IN-9 | P3 | `.gitignore` `target-xtask/` duplicated | rehearsal integration resolution `369c2b4` | this ledger (rehearsal tree: single entry + explanatory note) | **F+V** |
| IN-10 | P3 | R1/R3/R4/R6 wrote side files under `docs/worklog/` instead of root worklog | — (files still present at rehearsal tip) | this ledger | **OPEN** — owner Lead: fold the four entries into root `worklog.md` or ratify the per-review-file convention and update AGENTS.md §4 |

### 1.11 T — audit of `w24/m` + `w24/q` (`w24-mq-fixes-findings.md`, `w24/t` `8a29077`): verified §1.4/§1.7; own findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| F-1 | follow-up | `build-msi.ps1:161,164` same unescaped `$base` (outside Q's ownership) — plus the script never parsed (4 ParserErrors) | `w24/u` `f5524e5` (escaping + parse repair + `-SelfTest`) | R13 (pwsh parse 4→0; red/green mutants; cross-twin diff) | **F+V** |
| OBS-1 | P4 | M's §Profile "27.3 min" parenthetical has a mixed derivation | — | — | **OPEN (optional)** — suggest "~26–27 min" or name the derivation; no owner assigned |
| OBS-2 | nano | Q worklog says "4 commits" but lists 5 SHAs (6 on branch) | — (append-only; can't edit) | — | **OPEN (optional)** — recorded here; no action |

### 1.12 R10 — audit of `w24/p` @ `022d679` (`w24-p-fixes-findings.md`, `w24/r10` `56ca81a`): verified L's P1/P2 fixed (§1.5); own findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| R10-1 | P2 | IN-8 unaddressed + no fixed-vs-deferred accounting at the audited tip | IN-8: `w24/p` `b5aff90`+`6a1677c`; accounting: W24-X `3117bb7` (see R14-1) | R14 (IN-8 both limbs; accounting limb self-verified by X against R14's tables) | **F+V** for IN-8; accounting limb **F(pend)** (see R14-1) |
| R10-2 | P3 | oracle/config unit tests remain the sharpest gap | `w24/p` `74bda46` | R14 | **F+V** |
| R10-3 | P3 | `steps_cell` failure detail embeds wasm-specific prose | `w24/p` `c48dc76` | R14 (parameterized; new test) | **F+V** |

### 1.13 R11 — lane A new commits (`w24-a-newcommits-findings.md`, `w24/r11` `eaf7970`): verified R8's P2-2/P3-1..4 closed (§1.9); own findings — **none fixed yet; lane A still running** (dirty worktree: `docs/web/size-record.toml` modified, `tools/xtask/src/cmd/web_budget.rs` uncommitted)

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| P1 | P1 | honesty note's non-runnability claims are FALSE (recorded Chromium exists and runs; the stated impossibility justified skipping the mandated re-run) | — (still present at lane A tip `c1c37b2`, `web-matrix-run.mjs:168-170`) | this ledger re-check | **OPEN** — owner lane A: correct the renderer prose + re-`--restamp` |
| P2-1 | P2 | committed capture predates the committed harness (R8 P2-1's second half) | — | this ledger re-check | **OPEN** — owner lane A: re-run the matrix on the final tree, commit the fresh record |
| P3-1 | P3 | matrix.config.json comment misdescribes the evidence shape (chromium-1200/143 vs actual chrome-157) | — (still present) | this ledger re-check | **OPEN** — owner lane A |
| P3-2 | P3 | `capture.commit` lacks a dirty-tree flag | — | — | **OPEN** — owner lane A (suggest `gitInfo()` records `git status --porcelain` non-empty) |
| P3-3 | P3 | hardcoded "735 lines" literal drifted (750 after b22bff5) | — (still present, mjs:315 + .md:171) | this ledger re-check | **OPEN** — owner lane A |
| P3-4 | P3 | no lane-A worklog entry (≡ R8 P3-5) | — | this ledger re-check | **OPEN** — owner lane A close-out |

### 1.14 R12 — gate-doc chain (`w24-s-chain-findings.md`, `w24/r12` `5224bd5`): verified the f→k→s chain (§1.3); own findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| P2-1 | P2 | chain tip's fix-status rows stale w.r.t. `w24/g` (report never mentions it; 3 §3 statements false at tip time) | `w24/v` `978ed91` (§3 fix-lane addendum) | R16 (all 10 claims verified: hashes, timeline, R5/R9 citations, OF-3 byte-identity, tables intact) | **F+V** |
| P3-1 | P3 | §1 blanket root-cause claim false for OF-4 | `w24/v` (R12's own hedge, landed) | R16 | **F+V** |
| P3-2 | P3 | §1 closure path omits OF-4 | `w24/v` | R16 | **F+V** |
| P3-3 | P3 | 7.I1/7.K1 deliverable citations resolve only against local unpushed `w24/a` | — (no doc change demanded) | R12 (recorded for the Lead) | **NO-ACTION** — Lead-awareness: lane A content reaches main only via local worktree or a push |
| O-1..O-4 | obs | clippy log rename provenance; mixed time anchors; wasm mtime cache reuse; doc 09 §12 "Q5" vs "5." numbering | — | — | informational |

### 1.15 R13 — audit of `w24/u` (`w24-u-fixes-findings.md`, `w24/r13` `4874ab5`): verified T's F-1 closed (§1.11); own findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| F-U1 | LOW | gen-wxs.sh `--self-test` lacks the raw-text escape-set pin (mutant still passes) | `w24/y` `afa8f35` | **pushed `origin/w24/y` `a6d6cb7`, NOT merged into rehearsal, no dedicated audit** | **F(pend)** — Lead merges/verifies |
| F-U2 | LOW | README predates `-SelfTest` (proof row credits only gen-wxs.sh) | `w24/y` `c09c401` | same | **F(pend)** |
| F-U3 | LOW | gate.sh wires NEITHER self-test | `w24/y` `216adab` (gate.sh step 7) | same | **F(pend)** |
| I-1..I-3 | info | unused escaped description; trailing-newline asymmetry; `[VERIFY at first live run]` boundaries (wix build, release.yml pwsh parse, Windows PowerShell 5.1) | — | — | informational |

### 1.16 R14 — final-state audit of `w24/p` (`w24-p-final-findings.md`, `w24/r14` `98fb76e`): verified R10's asks closed (§1.12); own findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| R14-1 | P1 | no W24-P worklog entry on the final pushed state; fixed-vs-deferred accounting absent | `w24/x` `3117bb7` (W24-P + W24-X entries, reconstructed from R14's tables) | **pushed `origin/w24/x`, NOT merged into rehearsal; X self-verified hashes/deferrals; no independent audit** | **F(pend)** — Lead merges; optionally scout-verifies |
| R14-2 | P3 | IN-8 guard dedupes only exact `"; "`-separated segments (theoretical) | — | — | **OPEN (optional)** — one doc-comment at next touch of `compat_matrix.rs`; no owner assigned |

### 1.17 R16 — audit of `w24/v` + `w24/w` (`w24-vw-fixes-findings.md`, `w24/r16` `702ad89`): verified §1.14's V fixes + W's plan-doc fix; own findings

| ID | Sev | Finding | Fixed by | Verified by | Status |
|---|---|---|---|---|---|
| W-P3-1 | P3 | W's worklog side note "docs/web/ is empty on every other branch" false at commit time (rehearsal + r11 carried copies) | — (append-only worklog; landed doc unaffected) | R16 (no action demanded; this report is the record) | **NO-ACTION** — recorded |
| V-I1, W-I1, W-I2 | info | "exactly as prescribed" understates OF-5's richer delta; R12 section attribution imprecise; W's base-selection note consistent | — | — | informational |

---

## 2. Fixer-chain cross-check (task §3)

| Chain | Hop | Fixed | Verified by | Chain verdict |
|---|---|---|---|---|
| r1→h(→q→u→y) | R1's 10 → H | `w24/h` (10 fixes, closeout `45c7136`) | R6 (10/10) | ✅ |
| | R6's P4-1..4 → Q | `w24/q` (`a6626fa`,`f1d45c8`+`5c9795b`,`b0901a0`,`35b3b7e`) | T (4/4, empirical) | ✅ |
| | T's F-1 → U | `w24/u` `f5524e5` | R13 (PASS) | ✅ |
| | R13's F-U1..3 → Y | `w24/y` (`afa8f35`,`c09c401`,`216adab`, tip `a6d6cb7`) | **none yet** (pushed, unmerged) | ⚠ last hop unverified |
| r2→i(→j) | R2's 9 → I | `w24/i` (8 fixed, 1 deferred) | R7 (8 fixed + 1 accepted-deferral) | ✅ |
| | R7's NEW-1/NEW-2 → J | `w24/j` (`60e3db8`,`c5aade9`) | **none** (merged into rehearsal `3b2c79e`) | ⚠ unverified |
| r3→k(→s→v→w) | R3's 7 → K | `w24/k` `ef15bcc` | R12 (corrections accurate, prepend-only) | ✅ |
| | R9's IN-6 → S | `w24/s` `4277355` | R12 | ✅ |
| | R12's P2-1/P3-1/P3-2 → V | `w24/v` `978ed91` | R16 (PASS) | ✅ |
| | S's flagged plan-doc drift → W | `w24/w` `8e97c56` | R16 (PASS) | ✅ |
| r4→m | R4's 9 → M | `w24/m` (9 commits, worklog `12da2cc`) | T (9/9, incl. empirical discrimination proof) | ✅ |
| l→p(→x→z) | L's 13 → P | `w24/p` (11 commits, `a776c24`→`c48dc76`) | R10 + R14 | ✅ |
| | R14-1 → X | `w24/x` `3117bb7` | **self-verified only** (pushed, unmerged) | ⚠ unverified |
| | → Z | `w24/z` == `3117bb7`, **no own commits** (lane idle) | — | ⚠ idle |
| r5→n | R5-2, R5-3 → N | `w24/n` (`5f96b5f`, `a9258aa`) | R5-2: R9 live tip; R5-3: none | ⚠ half-verified; R5-1 remains OPEN (Lead/tooling) |
| r6→q | (see r1→h chain) | ✅ | | |
| r8→a | R8's 7 → A | P2-2 + P3-1..4 fixed (`ca21cb5`, `b22bff5`) | R11 | ✅ with P2-1 half + P3-5 open |
| r10/r14→p | R10's 3 + R14's asks → P | `w24/p` (IN-8, tests, parameterization) | R14 | ✅ (R14-1 → X, above) |
| r11→a | R11's 6 → A | **none fixed yet** (lane A's post-R11 commits are 7.K1 work; honesty note, config comment, "735 lines" all still present at tip `c1c37b2`) | this ledger re-check | ❌ open — lane still running |
| r12→s/v | (see r3→k chain) | ✅ | | |
| r13→u/y | (see r1→h chain) | U ✅ (R13); Y ⚠ unverified | | |

**P1/P2 closure check (rule: every P1/P2 ends FIXED+VERIFIED or explicitly OPEN with owner):**
- P0 (2): OF-1, IN-1 — both F+V (wave merge to main pending, Lead).
- P1 (12): 9 F+V (R1 P1-1/P1-2, R4 P1-1, L P1-1, IN-3, IN-4, OF-2, OF-3, OF-5); 1 F(pend) (R14-1 — X, self-verified, unmerged); 2 OPEN with owners (IN-2 → Lead; R11 P1 → lane A).
- P2 (24): 19 F+V (R1×4, R2×3, R3 P2-1, R4×3, L×2, R8 P2-2, IN-6, IN-7, R10-1 [IN-8 limbs], R12 P2-1, OF-4); 1 F(pend) (R7 NEW-1 — J, merged, unreviewed); 4 OPEN-numbered = 3 unique (R5-1 → Lead/tooling; IN-5 → Lead; R8 P2-1 ≡ R11 P2-1 → lane A). ✅ No P1/P2 is silently dropped.

---

## 3. Summary counts (task §4)

**Findings filed (numbered rows in the 17 reports + the gate's OF-*): 101**

| Severity | Count | Items |
|---|---|---|
| P0 | 2 | OF-1, IN-1 |
| P1 | 12 | R1 P1-1, P1-2; R4 P1-1; L P1-1; IN-2, IN-3, IN-4; R11 P1; R14-1; OF-2, OF-3, OF-5 |
| P2 | 24 | R1×4; R2×3; R3 P2-1; R4×3; L×2; R5-1; R7 NEW-1; R8 P2-1, P2-2; IN-5, IN-6, IN-7; R10-1; R11 P2-1; R12 P2-1; OF-4 |
| P3 | 53 | R1×4; R2×6; R3×6; R4×5; L×10; R5×2; R7 NEW-2; R8×5; IN-8/9/10; R10×2; R11×4; R12×3; R14-2; R16 W-P3-1 |
| P4 / LOW / nit-class | 10 | R6 P4-1..4; R13 F-U1..3; T F-1; T OBS-1, OBS-2 |

Plus **13 informational observations/notes** demanding no action (R3 O-1/O-2; R5-4; R12 O-1..O-4; R13 I-1..I-3; R16 V-I1/W-I1/W-I2). Total tracked rows: 114.

**Disposition:**

| Status | Count | Detail |
|---|---|---|
| FIXED + VERIFIED (independent audit) | **71** | OF-1..5 (5); R1 (10); R2 (8); R3 (6); R4 (9); L (9); R5-2 (1); R6 (4); R8 (5); R9 (7: IN-1/3/4/6/7/8/9); T F-1 (1); R10 (3); R12 (3) |
| FIXED, verification pending | **7** | R5-3 (N); R7 NEW-1, NEW-2 (J — merged into rehearsal, never audited); R13 F-U1/F-U2/F-U3 (Y — pushed, unmerged); R14-1 (X — pushed, unmerged, self-verified) |
| DEFERRED (recorded, owner named) | **5** | R2 P3-9 (Lead, optional); L P3-1, P3-6, P3-8, P3-9 (compat-matrix owner; recorded in W24-X's W24-P worklog entry) |
| OPEN (owner + next step) | **15 numbered = 13 unique** | P1: IN-2 (Lead), R11 P1 (lane A). P2: R5-1 (Lead/tooling), IN-5 (Lead), R8 P2-1 ≡ R11 P2-1 (lane A). P3: R8 P3-5 ≡ R11 P3-4 (lane A worklog), R11 P3-1/P3-2/P3-3 (lane A), IN-10 (Lead), R14-2 (optional), L P3-7 re-authoring (corpus owner). P4/nit: T OBS-1, OBS-2 (optional, unowned) |
| NO-ACTION (closed by the filing report's own disposition) | **3 + 13 info** | R3 P3-6; R12 P3-3; R16 W-P3-1; R5-4 (accepted design); + the 13 informational notes |

Checks: 71 + 7 + 5 + 15 + 3 = 101 numbered ✓. Every P1/P2 is accounted for (§2).

**Top open items (prioritized):**
1. **R11 P1** (lane A) — the browser-matrix honesty note makes a materially false non-runnability claim and the mandated final-tree re-run was skipped on that false premise (P2-1). Lane A is still running (in-flight `web_budget.rs`); the false claim is still in `web-matrix-run.mjs:168-170` at tip `c1c37b2`.
2. **IN-2** (Lead) — `full_gate.sh` step-10 hole + stale comment; reconcile with the wave-plan gate-slot table.
3. **IN-5** (Lead) — web-lane deliverable re-baseline: web feature-matrix honesty pass + budget gate exist on no branch; lane A's 7.K1 work (size budgets as CONFIG, size report) is unpushed/unmerged and does not yet wire any gate.
4. **R5-1** (Lead/tooling) — shared-target-dir clobber: unprovenanced shared-dir test results are not evidence; per-lane `CARGO_TARGET_DIR` [PROPOSAL] undecided.
5. **Unverified fix lanes** (Lead) — `w24/j` (merged, unaudited), `w24/x` + `w24/y` (pushed, unmerged, unaudited); `w24/z` idle.
6. **Lane A close-out** — worklog entry (R8 P3-5 ≡ R11 P3-4) + P3-1..P3-3 accuracy items.
7. **IN-10** (Lead) — four review side-files under `docs/worklog/` vs AGENTS.md §4.
8. **Deferred compat P3s** (Lead to assign) — L P3-1/P3-6/P3-8/P3-9 + the 0385 re-authoring (P3-7).
9. **Wave merge itself** — rehearsal lacks w24/x, w24/y, lane A's final tip; the phase7-exit §8 battery + UI suite re-run on the final tree is unexecuted (the would-be R15 round never filed).

---

## 4. Lead-decision items (task §5)

**Merge / wave-close mechanics**
1. **Merge order to green** (R9's recommendation, updated to current state): merge `w24/x` (R14-1 worklog append) → `w24/y` (F-U1..3 packaging polish) → take lane A's **final** tip (push it; currently `c1c37b2` + in-flight `web_budget.rs`, with R11's P1/P2 still open on it) → re-run the phase7-exit **§8 battery + UI suite** on the final tree → then main merge. (R9 §"Integration readiness verdict"; gate doc §6 "Lead re-verifies #49 at wave merge".)
2. **§8 battery additions** (R3 P3-7 + R9 IN-1 note): commit the raw outputs for the two manual controls (rebrand run + cargo-deny sha256sum) and actually re-run the UI suite (marker-absence is verified; green suites are not).
3. **J/X/Y unreviewed fixes**: J's NEW-1/NEW-2 are merged but were never audited; X's R14-1 and Y's F-U1..3 are pushed but unmerged and unaudited — merge them and (optionally) have a scout verify, or accept on the strength of their disclosed self-verification.

**Plan-doc / deliverable decisions**
4. **IN-5 re-baseline**: the wave brief's lane-A artifacts (web feature-matrix column pass, full_gate step 10 "web-budget", ci.yml budget gate) exist on no branch; the plan doc's 7.I1 cell was corrected (W, R16-verified) but the web-lane deliverable list itself needs the Lead's re-baseline. Lane A's 7.K1 CONFIG work (`c1c37b2`) + in-flight `web_budget.rs` partially address the budgets half — the Lead decides whether they land this wave or move to Phase 8.
5. **IN-2 gate-slot reconciliation**: two different attributions for slot 10 ("soak" in-tree comment, "web-budget" in the wave brief) and neither exists — land one or renumber 11→10 and delete the stale comment.
6. **COOP/COEP §12 Q8 confirmation** (R11 informational): the off-by-default decision is documented (doc 09 §9.5) but the Lead's confirmation is implicit, not recorded — record it.
7. **ADR-013 ratification** (VA-API activation) — carried into Phase 8 by the gate doc §6; still a Lead decision.

**Accepted risks / conventions to ratify**
8. **R5-4** (accepted design): the Rust-side shape guard intentionally excludes the UI artifact (`settings:check` + vitest own it; `cargo test --workspace` green ≠ UI artifact fresh) — ratify as designed.
9. **IN-10** (worklog side-files): fold `docs/worklog/w24-{r1,r3,r4,r6}-review.md` entries into the root `worklog.md`, or ratify the per-review-file convention and update AGENTS.md §4 (going forward, scouts append to root like R2/R5/R7/R8 did). The supplementary `w24-h-review-fixes.md` is extra detail, not a substitution.
10. **R5-1** [PROPOSAL]: per-lane `CARGO_TARGET_DIR` suffixing or fingerprint namespacing in `scripts/cargo-env.sh` (sandbox-side tooling, outside the repo) — the collision class hit R5's own audit twice and is documented by w23e/OF-4.
11. **R2 P3-9 / R7**: optional per-round worklog split for W24-E at wave merge (reviewer's own disposition: "none required").
12. **R12 O-2**: make the gate doc's time-anchor convention uniform (a single "statuses as of <commit, time>" gloss for the OF rows).
13. **W-P3-1** (R16): recorded, no action demanded (append-only worklog; the landed plan-doc text is unaffected).

**Assignments owed**
14. **Compat-matrix deferred P3s** — L P3-1 (verify_diff notes-insertion), P3-6 (dump-vs-fatal), P3-8 (stale claims snapshot), P3-9 (read_capped) + R14-2 (guard doc-comment): assign an owner (recorded as open in W24-X's W24-P entry; none assigned yet).
15. **0385-dynres regen re-authoring** (L P3-7's residual): the manifest entry can never regenerate (machine-local concat intermediate); needs the corpus owner.
16. **T's nits** — OBS-1 (M's 27.3-min derivation), OBS-2 (Q worklog count label): optional, unowned; recorded here.
17. **Lane A close-out checklist** — R11 P1 (honesty note), P2-1 (final-tree re-run), P3-1..P3-3 (config comment, dirty flag, "735 lines"), P3-4 (worklog entry): all still open at `c1c37b2`; lane still running.

---

## 5. User-owned blockers (task §6)

| Blocker | What it blocks (issues/cards) | Risk/issue tracking | Concrete next step |
|---|---|---|---|
| **CI Actions billing** (every run dies at 0 steps) | all CI gates; strict-verified parity (0/252, #51 §5); browser-matrix CI leg (#49); release (#56), soak-nightly (#54), compat-matrix (#53) workflows — none has ever executed | `docs/gates/phase7-exit.md` §6 blocker table; `docs/KNOWN-LIMITS.md` R-16; soak-nightly.yml honesty note (R4/T-verified); every `[VERIFY at first live run]` label (R1/R6/T/R13) | user enables billing; first action = the documented `workflow_dispatch` dry-run of `release.yml` (R6's merge posture), then the CI legs of the §8 battery |
| **3-OS runner machines** | 7.A mac/windows device-enum runtime tests; 8.B1 compat matrix (3-OS); 8.D1 perf-vs-VLC (#55); 8.G2 RC smoke (#58) | R-16 (`docs/14-risk-register.md` — self-hosted runners, spend decisions go to the user); `compat-matrix.yml` parked-until-provisioned (L-verified); rc-smoke 3-OS rows checklist-blocked | user provides/provisions runners (windows-latest + macOS Intel/ARM or self-hosted); then run the parked workflows |
| **Apple Developer account / key custody (R-35)** | 8.E2 macOS notarization; cascades to SBOM-all-platforms and the RC | R-35; `phase7-exit.md` §6; `sign-macos.sh` honest-unsigned sidecar path (R1 P1-1 area, R6-verified) | user decision at the first signed-build attempt (provision the account + secrets named in `packaging.toml [signing.macos]`; the workflow wiring is now real — R1 P1-2 fixed) |
| **Windows code-signing certificate** | 8.E1 Authenticode signing + the config-gated MSI upgrade-path test | `phase7-exit.md` §6 ("user procures cert, `[VERIFY]` — escalate if absent"); `sign-windows.sh` + release.yml guards (R6/T-verified) | user procures the cert and sets the two `WINDOWS_SIGNING_*` secrets; then the first live `release.yml` run validates the whole chain |
| **ADR-010 codec-patent sign-off** | **any public binary/artifact** — hard trigger Phase-8 packaging cards 8.E1–8.E3, and per R-03 this also covers a publicly shipped web bundle (card 7.K1) if it precedes Phase 8 | R-03 (`docs/14-risk-register.md`, exposure 9, User-decision); `docs/adr/ADR-010-codec-patent-shipping.md` (**proposed** — 24-item counsel ledger, options a–d, recommended posture (a) patent-clean default tier `[PROPOSAL]`); `docs/KNOWN-LIMITS.md` | user + counsel decide on ADR-010 (accept distribution risk vs split/regional builds) **before any binary-distribution plan or public artifact**; nobody in this repo may make the legal determination |

---

## 6. Ambiguities & method limits (honesty)

- **R15 / AA / AB produced nothing reachable** (see §"Not filed"): `origin/w24/{r15,aa,ab}` do not exist; the local branches sit at rehearsal/lane-A tips with no own commits and clean worktrees. No findings were invented for them; the §8 battery re-run and the "size audit" are recorded as NOT FILED, with the Lead's merge battery as the fallback owner.
- **F(pend) bucket** (7 items): fixed-but-unverified is a real state in this wave — J (merged), X, Y (pushed/unmerged), N's R5-3 (merged, test-only). Each row names exactly what was checked (branch, tip, merge state) and what is missing (an independent audit).
- **R10-1's accounting limb** is counted F+V at the finding level only because R14-1 tracks it separately as F(pend); the split is stated in both rows.
- This ledger is a read-only synthesis: no battery was re-run, no source file was modified; every claim above cites a report/branch/commit or was re-verified directly in git (`rev-parse`, `ls-tree`, `show`, `log`, worktree status checks on 2026-11-11).
- Severity labels are the filing reports' own (T uses P4/"follow-up"/"nit", R13 uses LOW/INFO); they are bucketed as "P4/LOW/nit-class" for counting, never upgraded.

*— W24-AC, 2026-11-11. Sources: the 17 reports in §"Sources inventory" + worklog entries (W24-K/S/W/R16 and W24-P/W24-X on `w24/x`) + the rehearsal merge chain `w24-integration-rehearsal` @ `88db8a1`.*
