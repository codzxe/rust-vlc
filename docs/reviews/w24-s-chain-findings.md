# W24-S chain (w24/f → w24/k → w24/s) — gate-document coherence review (audit W24-R12)

- **Auditor**: W24-R12 (wave 24 review scout 12 — gate-document chain coherence pass), read-only
  on the chain tip; fresh worktree `/home/z/my-project/wt/w24r12`, branch `w24/r12` off
  `origin/w24/s` (`faeca6b`).
- **Audited chain** (the Phase-7 exit-gate evidence document, issue #51, card 7.L1-V):
  - `origin/w24/f` `833944e` (2026-11-10 17:16:47Z) — original report `docs/gates/phase7-exit.md`
    + round-1 logs `a4e35e7` (16:51:45Z) + r2 battery `2bf5b66` (17:12:38Z);
  - `origin/w24/k` `ef15bcc` (21:52:12Z) — review W24-R3's fixes (retroactive r1 log headers,
    wildcard 32→30, `d33886d`→`d33886e`, R-20→R-16, OF-3 fix-in-flight, two line ranges, R3
    re-verification pointers);
  - `origin/w24/s` `faeca6b` (23:42:58Z, fix commit `4277355` 23:42:25Z) — review W24-R9 IN-6/IN-5
    fixes (7.I1 deliverable repoint at `docs/web/browser-matrix-20261110.{md,json}`, lane-A tip
    `771259c`→`b22bff5` time-anchored, honesty-pass half still pending, 7.K1 budgets-pending).
- **Method**: full end-to-end read of the 280-line report; every § section cross-checked against
  every other; all 18 logs in `docs/gates/logs/` inventoried, the 11 `phase7-exit-*.log` headers
  read, the 3 retroactively-annotated r1 logs byte-diffed against `origin/w24/f` (prepend-only
  proof); every cited path resolved at the chain tip or at its named branch; every cited commit
  rev-parse'd; ~30 numeric claims recounted from the logs (awk/rg tallies, not eyeball); chain
  facts re-anchored against commit timestamps; w24/g (fixes lane) and w24/a (lane A) diffed to
  establish what the doc's fix-status rows describe vs what actually exists.

**Verdict**: the chain is **internally coherent and evidentially sound** — no section contradicts
another, every recountable number is exact, every citation resolves (two with a local-branch
caveat), and the K/S surgical edits are accurate, provenance-honest, and prepend-only where they
touch logs. **One material gap stands between this chain tip and "one coherent story" for the
Lead's merge**: the report's closure narrative is anchored at report time (17:16Z) and never
acknowledges the fixes lane `w24/g`, which — before both the K and S edits — landed fixes for
OF-2/3/4/5 and is already merged into the integration rehearsal. The doc therefore *understates*
closure progress (conservative direction, no honesty breach); a Lead reading only the gate doc
would not learn that 4 of 5 findings are already fixed on a merged lane.

**Severity counts: P0=0, P1=0, P2=1, P3=3, plus 4 informational observations.**

---

## P2 — fix before/at wave-24 merge

### P2-1. The chain tip's fix-in-flight statuses are stale w.r.t. the fixes lane `w24/g` — the report nowhere mentions `w24/g`, and three §3 statements are false as read at tip time

`docs/gates/phase7-exit.md` §1 (closure path), §3 (OF-2/OF-4/OF-5 owner/next-step rows), §6.
`rg -c 'w24/g|lane G' docs/gates/phase7-exit.md` → **0 hits**. But `w24/g` (off `c339ee8`;
first fix commits 17:56:10–18:00:29Z, style follow-ups to 19:05:52Z) contains:

| OF | w24/g commit (time) | What it did | What the chain tip still says |
|---|---|---|---|
| OF-2 | `1c48902` (17:56:10Z) | derives the seed-count pins from `schemas/settings.toml` + adds a cross-artifact shape guard (`lib.rs` +33, `integration.rs` +98, `help.rs` +79) | §3 OF-2: "**not owned by any wave-24 lane** … 2-line pin bumps ×3 by the Lead or a granted lane"; §1: "apply the OF-2 pin bumps" |
| OF-3 | `64757b1` (17:56:43Z) | regenerates `ui/src/screens/settings/generated/schema.ts` (+105) — **byte-identical** to lane A's `771259c` output (`git diff w24/a w24/g -- ui/.../schema.ts` → empty) | §3 OF-3 / §1: fix in flight **on `w24/a` `771259c`** only; "Lead re-verifies … alongside OF-1's `70ed69c` on the same branch" |
| OF-4 | `ba9659f` (17:59:31Z) | roots the reverse-control proof in the compiling worktree, not the binary's location — implements the doc's `[PROPOSAL]` | §3 OF-4: "Owner/next step: Lead (test infra) … Recorded as `[PROPOSAL]`" |
| OF-5 | `de9b341` (18:00:29Z) | corrects the `codec-video-h264` note: "MF never landed, retract the over-claim" | §3 OF-5: "correct the note to 'VT + VA-API lanes landed (MF retracted — not implemented)'"; §1: "correct the OF-5 matrix note" |

`w24/g` is already merged into the integration rehearsal (R9's snapshot `286bacd` = 17 merges
incl. `g`; R9's seam table independently confirms "Lane G's OF-5 retraction present verbatim").
So at the chain tip (`faeca6b`, 23:42Z — ~5 h after g's fixes): "not owned by any wave-24 lane"
is false, the OF-2 "pin bumps" next-step is superseded by a materially different (stronger)
fix, OF-4's `[PROPOSAL]` is implemented, OF-5's correction is landed, and OF-3 is carried by
*two* lanes with identical content (benign duplication, but undiscoverable from the doc). The
timeline makes the staleness a chain-level fact, not an F-report defect: K edited the doc at
21:52:12Z and S at 23:42:25Z, both **after** g's 18:00Z fixes; S even refreshed the lane-A tip
citation to `b22bff5` (23:22:22Z) in the same edit, so the doc knowingly tracks post-report
branch state for 7.I1 while leaving the OF rows frozen at report time.

This is not an internal contradiction (§1/§3 are time-consistent as a 17:16Z snapshot, and the
7.I1 row discloses its own time anchor) and it errs conservative — the program's honesty law
forbids over-claiming, not under-reporting progress. But the merge story is incomplete until
the Lead either (a) lands a short fix-lane addendum on the chain (OF-2/3/4/5 fixed on `w24/g`,
commits listed, OF-3 duplicated identically on `w24/a`; §1 closure path reduced to "land lane A
(OF-1) + verify g's fixes + re-run §8"), or (b) consciously accepts the §6/§8 framing ("OF-2/3/4
fix + full battery re-run at wave-24 merge"), which remains correct as a *criterion* even while
the §3 owner rows understate reality. **Owner**: gate-doc fixer chain or Lead (docs-only, ~10
lines).

## P3 — accuracy polish

### P3-1. §1's blanket root-cause claim is false for OF-4

`phase7-exit.md:10-11`: "5 open findings (OF-1…OF-5), **all with single-commit root causes in
already-merged wave-23 commits**". True for OF-1 (`45c5c6f` w23c), OF-2/OF-3 (`c71cd5a` under
`95c0743` w23d), OF-5 (`9cff3c4`/`67a139c` w23a — all verified wave-23 merges). False for
OF-4: §3 OF-4 itself gives an **environmental** root cause (`repo_root()` walking up from
`env!("CARGO_BIN_EXE_xtask")` under the orchestrator's shared `CARGO_TARGET_DIR`), and the
test file was last touched in **wave 22** (`2bc8421`, "7.w22 merge fix") — no wave-23 commit
caused it. The verdict's most-read sentence over-generalizes; §3's own taxonomy is accurate.
Conservative direction; fix is a one-clause hedge ("four of five…; OF-4 is environmental").

### P3-2. §1's closure path enumerates only 4 of the 5 findings (omits OF-4)

§1 lists OF-1/OF-3 (land lane A), OF-2 (pin bumps), OF-5 (matrix note) — OF-4's Lead-owned
test-infra fix is absent from the closure-path sentence, though §6 ("OF-2/OF-3/OF-4 fix + full
battery re-run") and §8 ("expect 0 failures **after OF-2/OF-4 fixes**") both carry it. A §1-only
reader under-counts the closure work. (Compounds P2-1: OF-4 is also the finding whose status
drifted furthest — from "unowned proposal" to "fixed on a merged lane".)

### P3-3. The 7.I1/7.K1 deliverable citations resolve only against the local, unpushed `w24/a` branch

`docs/web/browser-matrix-20261110.md` + `.json` exist on `w24/a` (`b22bff5` — verified via
`git ls-tree`) and **nowhere else**; `git ls-remote --heads origin w24/a` → empty (lane A has
never been pushed; S's own worklog discloses "w24/a is unpushed — checked out in lane A's busy
worktree"). The doc's phrase "Deliverables **on the lane**" is accurate, and the row honestly
labels the work IN-FLIGHT, but a remote-only consumer of `origin/w24/s` cannot verify the
citation. Under this sandbox's shared-repo model that is workable; the merge checklist must
know lane A's content reaches main only via the local worktree (or a push). No doc change
demanded; recorded for the Lead.

## Informational observations (no action demanded)

### O-1. Round-1 clippy log changed filename at the r2 battery commit

At `a4e35e7` the round-1 clippy log was committed as `phase7-exit-clippy.log` (242 lines);
`2bf5b66` renamed it to `phase7-exit-clippy-r1.log` and put the r2 run in the original name.
Body verified byte-identical across the rename, and the retrofitted header's claim ("committed
as `a4e35e7` = `c339ee8` + the 4 round-1 logs") is true in content — but the header does not
mention the rename, and `git show a4e35e7:docs/gates/logs/phase7-exit-clippy-r1.log` finds
nothing (the path did not exist at that commit). Provenance is fully reconstructible via
`git log --follow`; disclosed here so nobody reads the header's "committed as a4e35e7" as a
path-anchored claim.

### O-2. The doc mixes two time anchors, one disclosed, one implicit

The 7.I1/7.K1 rows describe W24-S fix-time state (23:42Z, tip `b22bff5` — self-disclosed
"**at W24-S fix time**"); §1/§3/§5–§8 describe report-time state (17:16Z). This is managed
honestly (R3's P3-6 documented exactly this aging for the old `771259c` tip; S re-anchored the
row rather than letting it silently rot), but the OF rows carry no time anchor and their
"fix in flight" facts age the same way (see P2-1). A single "statuses as of <commit, time>"
gloss in §3 would make the convention uniform.

### O-3. wasm artifact mtime predates its log's capture header — cache reuse, coherent

`phase7-exit-wasm.log` header: 2026-11-10T16:56:57Z, "Finished in 0.12s"; the recorded
artifact line shows `lumen_web.wasm` 5,998,961 B mtime **16:16**. Explanation: the build was
served from the shared warm `CARGO_TARGET_DIR` (the 0.12 s finish), so `ls` reports the
original artifact creation time. The doc claims only the size + exit codes (both verified);
§7 discloses the shared-target-dir posture. No defect; noted because it is the one place a
log's header date and body timestamp diverge.

### O-4. "doc 09 §12 Q5" resolves, though doc 09 labels the item "5." not "Q5"

`phase7-exit.md` 7.K1 cites "doc 09 §12 Q5"; `docs/09-web-wasm-target.md` §12 ("Open questions
for the Lead") item **5** is exactly "Size budget numbers — 3 MB / 6 MB baselines (§2.4) are
proposals; pin after the first real build", and `ci.yml:527`'s "SIZE BUDGETS: NONE enforced"
gloss cites the same §2.4 baselines. Substance accurate; citation style differs from the
target doc's numbering. Cosmetic.

---

## Verification appendix (what was checked and passed)

### A. Internal coherence (audit item 1) — PASS with P2-1/P3-1/P3-2 as the only seams

- **§1 ↔ §2**: "18 of 22 battery checks green" — recounted: 22 rows, 4 FAIL (#3, #4, #16,
  #17) → 18 PASS. Exact.
- **§1 ↔ §3 ↔ §4 ↔ §6 ↔ §8 (fix-in-flight)**: OF-1 `w24/a` `70ed69c` (16:17:56Z) and OF-3
  `w24/a` `771259c` (16:33:59Z) — both commits exist on `w24/a`, both predate the report
  (17:16:47Z), both do what is claimed (70ed69c: both `/discovery`+`/convert` routes, no
  markers; 771259c: `schema.ts` +105). §4 7.D1/7.E2-3 "route blocked by OF-1, fix in flight",
  §6 "OF-1/OF-3/OF-5 … lane A's owned files", §8 "expect 0×4 after OF-1/OF-3" — all agree.
  (The w24/g half of the story is P2-1.)
- **S's edits vs §1's characterization of #49**: §1 "(#49) honestly in-flight on `w24/a`";
  7.I1 after S: "IN-FLIGHT — no pass claimed", deliverables = the real dated files (verified
  on `w24/a`), honesty-pass half "still pending — no branch touches the web column"
  (re-verified true: the only feature-matrix.toml changes anywhere are lane D's linux flips +
  lane G's OF-5 note); 7.K1 budgets-pending (re-verified: `rg web-budget` → 0 hits on any
  ref; no branch's diff touches `ci.yml`; `full_gate.sh` has no web-budget step; `ci.yml:527`
  "SIZE BUDGETS: NONE enforced" verbatim). No pass is claimed anywhere for #49; §5 ("the
  flip-to-implemented pass is exactly in-flight issue #49's deliverable") and §8's criterion
  ("#49's browser matrix merged **with its honesty pass**") are consistent with "still
  pending". **Coherent.**
- **§4 per-card statuses**: 11 LANDED rows each backed by named suites (all spot-counts exact,
  see D); 7.I1/7.K1 IN-FLIGHT; 7.B's truth-table pointer (`hw/mod.rs:38-65`) contains the MF
  retraction verbatim; the Minor hevc numbering-drift note verified (`hevc/hw.rs:7` vs
  doc 13 §10 ordering in `hw/mod.rs:43`).
- **§5 ↔ parity log**: 64/252 windows/linux/macos, web 0/252 (0.0 %), strict verified-only
  0/252 ×4, 252 rows, cells blocked=133 implemented=192 not_started=505 partial=142
  unsupported=36 (sum 1008 = 252×4) — all verbatim in `phase7-exit-parity.log`.
- **§6 ↔ plans**: lane→issue map matches `docs/plans/phase8-cards.md` (P8-2=#53→D, P8-3=#54→B,
  P8-5=#56→C, P8-7=#58→E, P8-8=#59 `[LEAD]`); #52 closure commits `14496c7`/`7f3443d` exist
  (wave 23); the 6.H1 Medium → 8.A2 inheritance matches phase8-cards §0.4 item 4; the R-16
  blocker citation matches `docs/14-risk-register.md:107` (K's R-20→R-16 fix correct — R-20 is
  documentation drift, :111).

### B. Logs (audit item 2) — PASS

All 11 `phase7-exit-*.log` carry headers. Convention as amended by K (§2 preamble): r2 logs
have capture-time headers (worktree/commit/date/cmd + exit markers in body — `FMT_EXIT=0`,
`CLIPPY_EXIT=0`, `DENY_EXIT=0`, `WASM_EXIT=0`, `WASM_CANARY_EXIT=0`, `WASM_LOCKED_EXIT=0`,
`WASM_CANARY_LOCKED_EXIT=0`, `UI_EXIT[×4]`, `XTASK_EXIT[sbom --check]=0`,
`TEST_RERUN_*_EXIT=101`, `PARITY_EXIT=0`, `FEATURE_AUDIT_EXIT=0`); the parity log's header
records date+cmd with exit markers in body (as disclosed); the 3 r1 logs (test / clippy-r1 /
evidence-rerun) carry retrofitted headers, and `evidence-rerun`'s header honestly says the exit
code "was not recorded at capture time" and labels the exit-0 reading an inference.

**Prepend-only proof** (byte-diff vs `origin/w24/f`): `tail -n +10` of `phase7-exit-test.log`
and `phase7-exit-clippy-r1.log`, `tail -n +12` of `phase7-exit-evidence-rerun.log` — all three
bodies byte-identical to the w24/f originals; K's whole-log diff = +29/−0 across exactly those
3 files. All other 15 logs in `docs/gates/logs/` are byte-identical f→s (S touched no logs).

**Header facts TRUE**: worktree `/home/z/my-project/wt/w24f` (log bodies show
`wt/w24f` compile paths throughout); `a4e35e7` = `c339ee8` + exactly the 4 round-1 logs
(`git diff c339ee8 a4e35e7 --stat` → 4 files, 6,644 insertions, 0 deletions — "no source
changes by this lane" holds); `c339ee8` = `origin/main`. **Dates plausible**: r1 headers "before
16:51:45Z" = `a4e35e7`'s commit time; r2 headers 16:56:26 → 17:12:23 (wasm `--locked` section)
all bracketed by `a4e35e7` (16:51:45Z) and `2bf5b66` (17:12:38Z); report `833944e` 17:16:47Z
after all captures. `evidence-rerun` body: "3 passed; 0 failed; 1 ignored" as claimed.

### C. Cross-references (audit item 3) — PASS (2 with the P3-3 caveat)

- **In-repo paths at the chain tip: 26/26 exist** (App.tsx, settings-schema lib/integration,
  help.rs, schema.ts, rebrand_skeleton.rs, feature-matrix.toml, h264/hw/mod.rs, hevc/hw.rs,
  platform-{windows,linux,macos}, DiscoveryScreen.tsx/.test.tsx, plugins/extensions/{both},
  apps/web/src/{render,audio,io}, KNOWN-LIMITS.md, sweep-control.md, phase8-cards.md,
  full_gate.sh, ci.yml, settings.toml, all 11 phase-7 logs, + host-side `cargo-env.sh`).
- **Line-level citations spot-checked exact (14/14)**: `lib.rs:155`, `integration.rs:117`,
  `help.rs:303-324` (panic `:310:9` inside), `hw/mod.rs:51-56` (K's re-measured range — MF
  bullet spans exactly 51-56), `App.tsx:15-24`/`:100-104`, `rebrand_skeleton.rs:120-134`/`:492`,
  `feature-matrix.toml:416` (H.264 notes), `KNOWN-LIMITS.md:63`, `ci.yml:469` (the
  `9f12ed4c…c8d5f` pin line — matches the doc's quoted SHA), `conformance.rs:727`,
  `server.rs:918`, `hevc/hw.rs:7`, and the `hw/`-directory truth (only `vaapi/` +
  `videotoolbox/`, no `mediafoundation.rs`).
- **Branch-qualified citations: 5/5 resolve.** R3-review doc
  `docs/reviews/w24-f-gate-findings.md` **exists on `origin/w24/r3`** (`27f1330`) as cited by
  all three "independently re-verified by review W24-R3" pointers (§2 #15, §2 #22, §7), and its
  text contains the claimed verifications. `docs/web/browser-matrix-20261110.{md,json}` exist
  on local `w24/a` (`b22bff5`) — see P3-3.
- **Negative claim verified**: the undated `docs/web/browser-matrix.md` exists on **no** branch
  (swept main + a–k + r1–r9 + l/m/n/q/t locally and via refs) — exactly as the 7.I1 row states.
- **Commit citations**: all rev-parse and content-verified (`70ed69c`, `771259c`, `b22bff5`,
  `45c5c6f`, `c71cd5a`, `3116aec`, `67a139c`, `9cff3c4`, `d33886e` (post-K), `14496c7`,
  `7f3443d`, `c339ee8`, `a4e35e7`, `2bf5b66`).

### D. Numbers (audit item 4) — PASS, all exact

| Claim (§) | Recount | Verdict |
|---|---|---|
| §2 #3: 4418 passed / 4 failed / 49 ignored, 306 targets, exit 101 | awk over 306 `test result:` blocks: 4418/4/49; `TEST_EXIT=101`; 4 failing targets listed at log end (lumen-cli lib, settings-schema lib + integration, xtask cli) | exact |
| §2 #6/#7: wasm artifact 5,998,961 B; exit 0 both (plain + `--locked`) | log: `5998961` B; `WASM_EXIT=0`, `WASM_CANARY_EXIT=0`, `WASM_LOCKED_EXIT=0`, `WASM_CANARY_LOCKED_EXIT=0` | exact |
| §2 #18: l10n 611 keys, missing 0, duplicates 0, orphans 0 | log: "611 MessageKeys, 611 en-US strings: missing 0, duplicates 0, orphans 0" | exact |
| §2 #15: 30 wildcard warnings | `rg -c 'warning\[wildcard\]'` → 30 | exact (K's fix) |
| §2 #17: 705/706 passed; 2 suites fail to load; 1 settings-drift | log: "Tests 1 failed \| 705 passed (706)"; "Test Files 3 failed" = 2 transform-error load failures (`App.test.tsx`, `i18n/pseudo.render.test.tsx` on App.tsx markers) + 1 `generated.test.ts` drift | exact |
| §2 #2: clippy r2 21.8 s warm, r1 cold 1 m 14 s | logs: "21.80s" / "1m 14s" | exact |
| §2 #8–#14 xtask: 5/5 checks, 97 edges, 39 crates, unsafe 39/39, static-mut 0/627, codegen 64/26, schema 32/7/14, fixtures 55, literals 0-viol/452-allow/18,602/558, identity 453, sbom 121,900 B | all verbatim in `phase7-exit-xtask.log` | exact |
| §1: 18 of 22 green | recounted | exact |
| §4 spot-counts: codecs 506, lumen-web 111, control-protocol 47 + conformance 25, control-server 11, platform-linux 13+15, seam 16×2, device-discovery 37, playback-engine 81, plugin-host 44, crash-isolation 6, examples 5, Discovery/Convert 8/8 | all found in `phase7-exit-test.log` / `phase7-exit-ui.log` | exact |
| OF-1 symptom: typecheck exit 2, 21 TS errors, eslint 1 error | log: 21 `error TS` lines, `UI_EXIT[typecheck]=2`, "✖ 29 problems (1 error, 28 warnings)" | exact |

### E. Honesty rules (audit item 5) — PASS

- **No runtime claims for compile-only items**: §4 7.A darwin/windows FFI "cited from lane
  evidence … not re-runnable in this sandbox (targets not installed, 1.1 GB free disk)";
  7.B VT "compile-verified … runtime hardware NOT exercised — `[VERIFY]`"; VA-API
  failure-path host-tested vs "driver-mediated decode compile-verified only"; MF retracted.
  `[VERIFY]`/`[PROPOSAL]` markers used per AGENTS §3.4.
- **Blocked labels carry owner + next step**: §6 external-blocker table (4 rows, owner = user,
  next step each); §4 7.A BLOCKED (hardware) → §6 runner row (R-16).
- **NOT-PASSED verdict + closure path explicit**: §1, bolded, with "valid gate outcome:
  failures are recorded, not papered over" and the §8 re-verification battery + pass criteria
  ("OF-1…OF-5 closed or consciously accepted").
- **Manual-run honesty**: §2 #22 (rebrand control) and §2 #15/§7 (cargo-deny SHA) carry
  "independently re-verified by review W24-R3" pointers that resolve (see C); the
  `evidence-rerun` exit-code inference is labeled as inference in the log header and disclosed
  in §2's preamble.

---

## Coherence verdict

**Coherent, with one merge-relevant gap.** The chain f→k→s reads as one internally
consistent, honestly-anchored evidence story: the verdict, battery ledger, findings, per-card
table, parity snapshot, launch list, and re-verification battery agree with each other and with
the logs; K's and S's corrections are accurate and prepend-only; every number recounts exact;
every citation resolves. The gap (P2-1) is not a contradiction *inside* the document but a
missing chapter *of the chain*: the fixes lane `w24/g` closed OF-2/3/4/5 hours before the tip's
own edits, and the document never says so — its closure narrative stops at report time while
its 7.I1 row tracks fix-time state. Recommended Lead action: a ~10-line fix-lane addendum on
the gate doc (or an explicit conscious-acceptance note in the merge record) before merge;
everything else in the chain is merge-ready as-is.

**Cross-reference resolution rate: 45/45 citations resolved or true-as-stated (100 %)** —
26/26 in-repo paths + 14/14 line-level cites + 5/5 branch-qualified cites (3 remote-verifiable
on `w24/r3`, 2 local-only on unpushed `w24/a`, P3-3) + the undated-path negative claim
verified. Zero dangling references at the chain tip.
