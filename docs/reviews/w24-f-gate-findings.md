# W24-F gate evidence — review findings (audit W24-R3)

- **Auditor**: W24-R3 (wave 24 review scout 3), read-only audit of `origin/w24/f`
  (Phase-7 exit gate report, issue #51, card 7.L1-V). Worktree `/home/z/my-project/wt/w24r3`.
- **Audited diff**: `git diff origin/main...origin/w24/f` — 13 files:
  `docs/gates/phase7-exit.md` (261 lines), 11 logs under `docs/gates/logs/phase7-exit-*.log`
  (test 6,390 / deny 1,011 / xtask 959 / clippy-r1 242 / ui 303 / parity 73 / test-rerun 43 /
  wasm 26 / clippy 20 / evidence-rerun 12 / fmt 5 lines), `worklog.md` (+15).
  Verified: the lane's commits touch **only** docs + worklog — `a4e35e7` = `origin/main`
  (`c339ee8`) + the 4 round-1 logs, nothing else ("no source changes by this lane" holds).
- **Verdict**: the gate evidence is **substantially sound**. The headline numbers are real
  (I recounted them), the 5 open findings are accurately diagnosed with correct file:line and
  root-cause commits, the fix-in-flight pointer (w24/a `70ed69c`) exists and does what is
  claimed, per-card evidence pointers resolve, blocked items carry owner + next step, and no
  overclaim was found (VT/MF/VA-API labels are conservative and correct). The findings below
  are documentation/convention defects in the evidence report itself — none change the
  NOT-PASSED verdict or its closure path.

**Severity counts: P0=0, P1=0, P2=1, P3=6 (total 7), plus 2 informational observations.**

---

## P2 — fix before/at wave-24 merge

### P2-1. §2 header-convention claim is not met by 3 of the 11 logs (evidence-convention breach in the gate's own ledger)

`docs/gates/phase7-exit.md:28-29` states: *"Logs carry per-run headers with worktree,
commit, date and exit code."* Verified per log:

| Log | Header (worktree/commit/date) | Exit marker |
|---|---|---|
| `phase7-exit-test.log` (the gate's central evidence) | **none** — starts at `Finished \`test\` profile` | `TEST_EXIT=101` (end) |
| `phase7-exit-clippy-r1.log` | **none** — starts at `Downloading crates` | `CLIPPY_EXIT=0` (end) |
| `phase7-exit-evidence-rerun.log` | **none** | **none** — no exit marker at all |
| `phase7-exit-parity.log` | date + cmd only (no worktree/commit) | `PARITY_EXIT=0` / `FEATURE_AUDIT_EXIT=0` |
| fmt / clippy / deny / ui / wasm / xtask / test-rerun (r2) | full header | exit markers present |

The round-1 provenance paragraph (`phase7-exit.md:56-59`) documents only the fmt case
("round 1's fmt log was empty with no exit marker") and does not acknowledge that the other
three r1 logs lack headers — `evidence-rerun.log` additionally lacks any exit marker, so its
"PASS (3 passed / 1 ignored)" row (#5) has no recorded exit status in the artifact itself.

**Mitigations (why this is P2, not higher)**: (a) provenance is anchored by the git commit
`a4e35e7` (logs committed before any other work, verified by diff against `origin/main`);
(b) I independently recounted the test log — 306 `test result:` lines, **4418 passed /
4 failed / 49 ignored**, exit 101, the 4 failing targets listed at the end — all exactly as
claimed; (c) the evidence suite's 3 passed / 1 ignored `regen` is verifiable from the log
body; (d) r2 re-runs (clippy, test-rerun, fmt) carry full headers and corroborate.

**Suggested fix**: either amend the §2 sentence to "r2 logs carry per-run headers; r1 logs
are provenance-anchored by commit `a4e35e7`" or prepend minimal headers
(worktree/commit/date/exit) to the three r1 logs when the branch merges. At the §8 re-run,
capture headers for every log including manual controls (see P3-7).

---

## P3 — accuracy polish (none change the verdict)

### P3-1. §2 row 15: wildcard warning count is 30, not 32

`phase7-exit.md:47` claims *"32 non-blocking `wildcard` path-dep warnings"*.
`rg -c '^warning\[wildcard\]' docs/gates/logs/phase7-exit-deny.log` → **30** (30 crates,
summing to the per-crate "found N wildcard dependencies" lines). Non-material — all are
warnings, `DENY_EXIT=0`, and "advisories ok, bans ok, licenses ok, sources ok" is verbatim
in the log — but a gate ledger should count what the log contains. **Fix**: change 32 → 30.

### P3-2. §4 7.F1 cites a non-existent commit hash (`d33886d`)

`phase7-exit.md:171` cites "#45 (`d33886d` wave 22)". `git rev-parse d33886d` fails — no
such commit. The real merge is **`d33886e`** ("merge wave 22: plugin-host-wasm v1 — wasmi
runtime, capability enforcement, .lumenext (issue #45, 7.w22d)"), i.e. a one-character
typo; the content claim (plugin-host-wasm 44/0, verified in the test log) is accurate.
All 21 other commits cited across §3/§4 were verified to exist with matching subjects.
**Fix**: correct the hash.

### P3-3. §6 blocker table: "user provides runners (R-20)" miscites the risk register

`phase7-exit.md:223` labels the 3-OS runner blocker "(R-20)". In
`docs/14-risk-register.md`, **R-20 is documentation-drift risk** (register rows at :111,
:33, :46); no R-row is about 3-OS machines — the closest is **R-16** (CI runner budget /
self-hosted runners, "spend decisions go to the user"). The blocker itself is real and the
MP §17 anchor checks out (`docs/MASTER-PROMPT.md:358-361`: "A blocked item needs a reason,
the investigation done…"), and R-35 (Apple key custody) is cited correctly. **Fix**: cite
R-16 or drop the parenthetical.

### P3-4. §3 OF-3's "next step" is stale: the fix already sits on the w24/a tip the report itself cites

OF-3 (`phase7-exit.md:105-106`) frames the fix as future work: *"fix = `npm run
settings:generate` + commit. Fold into lane A's existing UI pass (OF-1)"*, and §1's closure
path says "apply the OF-2/OF-3 pin bumps + regeneration". But §4 7.I1
(`phase7-exit.md:176`) cites w24/a's branch tip as **`771259c`** — and `771259c`
(2026-11-10 16:33:59, before the report's commit at 17:16:47) is exactly the OF-3 fix:
"fix(ui): regenerate settings screens after w23d's schemas/settings.toml crash-backoff rows
(settings:check drift — unblocks UI test suite)", regenerating
`ui/src/screens/settings/generated/schema.ts` (+105 lines). The report thus understates
progress (conservative direction — no overclaim), but a findings ledger should reflect that
OF-3's fix is already in flight on the cited branch, like OF-1's. OF-2's Rust-side pin
bumps remain genuinely unowned/unfixed (verified: not on w24/a). **Fix**: reword OF-3's
owner/next-step to "fix in flight on w24/a (`771259c`); Lead re-verifies at wave merge".

### P3-5. Minor file:line drift in §3 citations (quoted text is verbatim-accurate)

- OF-5 (`phase7-exit.md:135`) cites `crates/codecs/src/video/h264/hw/mod.rs:53-58` for the
  Media Foundation retraction bullet; the bullet actually spans **lines 51-56** (`rg -n`
  "Media Foundation" → :51; "retracted." → :56). The truth table as a whole at :38-65
  (cited in §4 7.B) is correct.
- OF-2 (`phase7-exit.md:86`) cites `apps/cli/src/help.rs:302-321`; the test
  `areas_block_lists_the_six_areas` actually spans **303-324** (fn at :304, last assert
  "6 options" at :323). The panic location in the log (:310:9) falls inside both ranges.
- OF-2's other two cites are exact: `crates/settings-schema/src/lib.rs:155` (the
  `assert_eq!(schema.len(), 27, …)` line) and `tests/integration.rs:117`
  (`contains("27 entries")`) — both confirmed in source and in the r1/r2 failure output.

**Fix**: nudge the two ranges.

### P3-6. §4 7.I1: w24/a tip has moved past the cited `771259c` (snapshot aging, not an error)

`phase7-exit.md:176` says "branch tip `771259c`". At report time (17:16) this was correct
(next lane-A commit `ff6e6aa` is 17:52); the branch has since advanced to `d1cba99`
(browser-matrix harness: `ui/playwright.web.config.ts`, `ui/scripts/web-matrix-run.mjs`).
Not a defect — but readers re-checking the pointer today will find a different tip.
`docs/web/browser-matrix.md` does not exist yet on w24/a, consistent with the honest
"IN-FLIGHT — no pass claimed" label. No action needed beyond awareness at merge.

### P3-7. Two evidence assertions rest on uncommitted manual runs

- §2 row 22 (`phase7-exit.md:54`): the rebrand reverse-control is evidenced only as
  "shell run 2026-11-10, reproduced in §3 OF-4" — no log file backs it.
- §2 row 15 / §7 (`phase7-exit.md:47`, `:237-238`): the cargo-deny binary SHA-256
  verification is asserted in the deny log's header line, but the raw `sha256sum` output
  was not committed and the tarball (`/tmp/cargo-deny-0.20.2-x86_64-unknown-linux-musl/`)
  is no longer on disk.

Both claims **check out** — I independently reproduced the rebrand control on the same
source state (`cargo run -q -p xtask -- rebrand --config product.toml --dest /tmp/…` →
`result: no-op`, exit 0, dest `README.md` byte-identical to the worktree per `diff -q`),
and the cited pin `9f12ed4c…c8d5f` matches `.github/workflows/ci.yml:469` verbatim — but a
gate whose §2 promises logged exit codes should capture logs for manual controls too.
**Fix**: at the §8 re-run, commit both raw outputs (rebrand run + sha256sum) as logs.

---

## Informational observations (no action demanded)

### O-1. The shared target dir was relocated *after* the gate ran — OF-4's failure mode will change at the §8 re-run

`scripts/cargo-env.sh` (rewritten by w24/a, disk emergency, post-gate) now sets
`CARGO_TARGET_DIR=/tmp/my-project/target`; the old `/home/z/my-project/rust-vlc/target` was
deleted. The report's §7 statement ("build logs legitimately show
`/home/z/my-project/rust-vlc/target/...` paths") was accurate for its session and the logs
confirm it. Consequence for the Lead's §8 battery: `repo_root()`
(`tests/rebrand/rebrand_skeleton.rs:120-134`) walking up from
`/tmp/my-project/target/debug/…` will find **no** workspace root (`/tmp/my-project` has no
`Cargo.toml`) and hit the `unreachable!` instead of diffing against the stale checkout —
same red, different panic. OF-4's substance (test broken under a shared target dir; the
tool itself is a true no-op) and its proposed fix (derive root from
`env!("CARGO_MANIFEST_DIR")` or an explicit `--root`) both remain valid.

### O-2. Task brief said "12 log files"; the branch adds 11 `phase7-exit-*.log` files

All 11 exist and are internally referenced by the report; no referenced log is missing
(the only gap is the manual-control absence in P3-7). Counting discrepancy is in the task
description, not the branch.

---

## Verification sampling (what was checked — 24 claims)

**Battery ledger (§2), 22 of 22 rows inspected, 20 verified exact, 2 with the discrepancies
noted above:**

1. **#3 test counts recounted independently**: 306 `test result:` lines; sums =
   4418 passed / 4 failed / 49 ignored; `TEST_EXIT=101`; 4 failing targets listed at end
   (`-p lumen-cli --lib`, `-p settings-schema --lib`, `-p settings-schema --test
   integration`, `-p xtask --test cli`) — exact.
2. **#4 rerun log**: all 4 failures reproduced with matching panic locations
   (`help.rs:310:9`, `settings-schema/src/lib.rs:155:9`, `integration.rs:117:5`,
   `rebrand_skeleton.rs:492:17`), `TEST_RERUN_EXIT=101` — exact.
3. **#1 fmt**: header (worktree/branch/commit/date/cmd) + `FMT_EXIT=0`; r1 fmt log
   verified 0 bytes at `a4e35e7` (the documented-and-rerun convention) — exact.
4. **#2 clippy**: r2 `CLIPPY_EXIT=0` in 21.80s; r1 `CLIPPY_EXIT=0` in 1m 14s — exact.
5. **#5 evidence suite**: 3 passed (`final_frame_is_current`, `ledger_matches_the_artifacts`,
   `committed_transcripts_match_fresh_runs`) + 1 ignored `regen` — exact (marker missing → P2-1).
6. **#6/#7 wasm**: `WASM_EXIT=0`, `WASM_CANARY_EXIT=0`, `WASM_LOCKED_EXIT=0`,
   `WASM_CANARY_LOCKED_EXIT=0`; artifact 5,998,961 B — exact.
7. **#8-#14 xtask**: check-architecture 5/5 (97 internal edges, 39 nodes, 0 cycles,
   unsafe-lints 39/39, static-mut 0/627 files, testkit-direction PASS); codegen 64/26;
   schema-gen 32 entries / 7 screens / 14 areas; fixtures 55; lint-literals 0 violations /
   452 allowlisted / 18,602 sites / 558 files; check-identity 0/453; sbom no-drift 121,900 B
   — all exact against `phase7-exit-xtask.log`.
8. **#15 deny**: `DENY_EXIT=0`, "advisories ok, bans ok, licenses ok, sources ok" verbatim;
   SHA pin `9f12ed4c…c8d5f` == `ci.yml:469` — exact; wildcard count 30 ≠ 32 (P3-1).
9. **#16-#18 UI**: typecheck exit 2 with 21 TS errors (TS1185/TS1005 etc.); eslint
   "✖ 29 problems (1 error, 28 warnings)" exit 1; vitest "Tests 1 failed | 705 passed
   (706)" with `App.test.tsx` + `i18n/pseudo.render.test.tsx` failing to load on App.tsx
   diff markers and `generated.test.ts:80` settings-drift; l10n "611 MessageKeys … missing
   0, duplicates 0, orphans 0" exit 0 — all exact.
10. **#19/#20 parity**: windows/linux/macos 64/252, web 0/252 (0.0%); verified-only 0/252
    everywhere; feature-audit 252 rows, phase-7 rows = 25, cells blocked=133 implemented=192
    not_started=505 partial=142 unsupported=36 — exact.
11. **#21/#22**: cross-refs resolve; #22 reproduced independently (see P3-7).
12. **18/22 green count re-derived** from the table: 18 PASS + 4 FAIL rows — exact.

**Open findings OF-1..OF-5 — all five verified:**

- **OF-1**: markers at `ui/src/App.tsx:15-24` and `:100-104` confirmed in worktree and
  `origin/main` (count 2); root cause confirmed — merge `45c5c6f`'s combined diff
  (`--cc`) shows the conflict region being committed, with the commit message exactly as
  quoted; **fix-in-flight verified**: local `w24/a` contains `70ed69c` (6 deletions —
  the markers; both `/discovery` and `/convert` routes retained). P0 reasonable (UI
  typecheck/build broken).
- **OF-2**: all three pins at the cited lines; `schemas/settings.toml` counted 27 entries
  at `3116aec`, 32 at `c71cd5a`, 32 at HEAD; `c71cd5a` is the issue-#46 crash-backoff
  commit; log failure output matches (left 32 / right 27; "32 entries"; the 7-area list
  verbatim). P1 reasonable.
- **OF-3**: `generated.test.ts:80` failure present in the UI log (runs the generator
  `--check` internally); stale `ui/src/screens/settings/generated/schema.ts` on main;
  fix commit exists on w24/a (see P3-4). P1 reasonable.
- **OF-4**: `repo_root()` at `:120-134`, assert at `:492`; the r2 log's failing byte
   arrays counted = **3569 vs 8032 bytes**, matching the on-disk stale checkout README
   (3569 B, "Phase 1 — Foundation planning (in progress)") and worktree README (8032 B,
   Phase-6 status + Install section); manual no-op control **reproduced by this audit**
   (exit 0, byte-identical dest README). P2 reasonable.
- **OF-5**: matrix note at `docs/feature-matrix.toml:416` says "VT/MF/VA-API … landed …
  compile-verified" while `hw/mod.rs` (MF bullet :51-56) retracts MF; `hw/` contains only
  `vaapi/` + `videotoolbox/`; `git ls-tree 67a139c` confirms no MF file ever existed;
  both over-claiming commit messages quoted accurately; row `status` fields honest
  (verified `windows/linux/macos = "partial", web = "not_started"`); fix not yet on w24/a
  (verified). P1 reasonable (honesty). The "Minor" HEVC lane-numbering drift note also
  checks out (`hevc/hw.rs:7` numbers 7.B1 VA-API / 7.B2 VT / 7.B3 MF).

**Per-card pointers (§4) — 11 spot-checked, all resolve:**

- 7.A: platform-linux 13 (`linux_integrations`) + 15 (`seam_behaviour`) = 28/0; macos/windows
  `seam_behaviour` 16/0 each; named tests `drm_scanner_reads_the_real_host_sysfs_honestly`,
  `pulse_probe_reports_this_hosts_truth`, `discover_on_a_foreign_host_reports_wrong_os_everywhere`
  (×2) all `ok` in the log; `#![allow(unsafe_code)]` + "SAFETY-SCOPED MODULE" + per-block
  SAFETY comments confirmed in `platform-linux/src/linux/dl.rs` (:24) and `pulse.rs` (:34,
  29 SAFETY comments); `cargo check --target` labels confirmed in
  `platform-windows/src/lib.rs:41` and `platform-macos/src/lib.rs:40`.
- 7.B: truth table at `hw/mod.rs:38-65`; codecs 506 unit tests; `h264_hw_path.rs` +
  `hevc_hw_path.rs` integration files exist and ran; VA-API failure-path tests green
  (typed no-hardware/permission paths); settings claims verified
  (`product.toml:164` `hw-decode = false`, `:187` `hw_decode_policy = "software-only"`,
  `installed_bridges`/`negotiate_lane` in the hw module); ADR-013 status "proposed" —
  the `[PROPOSAL]` label is accurate.
- 7.C: playback-engine 81/0, audio-output 33/0, renderer-wgpu 60/0 — exact.
- 7.D: device-discovery 37/0; `DiscoveryScreen.test.tsx` (8 tests) green in the UI log;
  route wiring inside the OF-1 conflict region (verified), resolved in `70ed69c`.
- 7.E: transcode 50/0; `tests/record.rs` + `tests/mp4_remux.rs` ran in the workspace log.
- 7.F1: plugin-host-wasm 44/0 (commit hash typo → P3-2).
- 7.F4: all 6 named tests present in
  `crates/plugin-host-wasm/tests/plugin_crash_isolation.rs` (trap-on-entry :94, mid-call
  :140, host-fault :186, resource-exhaustion :225, restart-backoff :291, playback-unaffected
  proof :596 with `PlaybackFixture` :494) and all `ok` in the log.
- 7.G: `plugin_examples_conformance` 5/0; `plugins/extensions/{metadata-provider,ui-panel}`
  exist; `3de83e5` is the walkthrough-docs commit.
- 7.H: lumen-web 111/0; wasm build + artifact verified (above).
- 7.J: control-protocol 47/0 + conformance 25/0 + control-server 11/0;
  `conformance.rs:727` (`non_loopback_bind_is_refused_at_construction_conformance`) and
  `server.rs:918` (`loopback_binds_succeed_on_both_families`) are exact line hits; the
  other named security tests (`handshake_success_full_session_round_trip`,
  `wrong_token_closes_4401_and_counts`, `five_failures_lock_out_even_the_right_token`)
  all `ok`; `docs/security/sweep-control.md` exists with the loopback/token/deny-by-default
  content.

**Blocked labels (§6)**: all four external blockers carry owner (= user) + next step;
MP §17 anchor verified; R-35 verified; R-20 miscited (P3-3). §6's phase-8 references check
out (`phase8-cards.md` §0.4 item 4 inherits the 6.H1 Medium as 8.A2's first triage item;
P8-4=#55, P8-6=#57; #52 merge commits `14496c7`/`7f3443d` exist).

**Overclaim scan (§4/§5/§7)**: none found. VT runtime explicitly `[VERIFY]`-unexercised;
VA-API driver-mediated decode "compile-verified only"; MF retracted; cross-target FFI
labels "cited from lane evidence … not re-runnable in this sandbox"; web column 0/252
"honest by conservatism" with MKV/MP4/Ogg rows verified `web = "not_started"`; strict
verified-only 0/252 stated as the headline debt; disk-budget and shared-target-dir
caveats recorded. The one overclaim the report found (OF-5) is real and correctly
characterized.

**Log hygiene**: all 11 logs scanned — no secrets, tokens, credential-bearing URLs, or
private keys; no truncated-output-masquerading-as-pass (the only "empty" log case, r1 fmt,
is explicitly documented and was re-run with a recorded header).

---

*Auditor note: `origin/w24/a` was not fetchable from the remote, but a local `w24/a` ref
exists in the shared clone, so the in-flight-fix claims (70ed69c, tip 771259c) were
verified against it. Everything else was verified against the audited worktree at
`833944e` (= `origin/w24/f`).*
