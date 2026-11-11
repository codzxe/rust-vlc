# W24-R10 — audit of the compat-fix lane W24-P (W24-L findings on the compat-matrix lane, issue #53)

- **Auditor:** W24-R10 (wave 24 review scout, round 10 — fix-lane audit).
- **Audited state:** local branch `w24/p` @ `022d679` — **NOT pushed** (`git fetch origin w24/p`
  → exit 128, no remote ref; `git branch -r` has no `origin/w24/p`). The local ref is reachable,
  so this audit covers the full committed delta `origin/w24/d...022d679` (3 commits: `a776c24`
  worklog / P1-1, `16d64b0` in-row refusals / P2-1, `022d679` web-wasm cell derivation / P2-2),
  checked out read-only into worktree `wt/w24r10` (branch `w24/r10`). **Lane P may still be
  running** in `wt/w24p` — per scout rules I never entered that worktree; uncommitted work there
  is NOT audited. Everything below is a snapshot verdict on `022d679`.
- **Delta:** 4 files, +415/−37 — `tests/compat/src/report.rs` (+409, the two P2 fixes + 7 unit
  tests), `docs/corpus/compat-matrix-20261110.md` (3 surgical line rewrites + a provenance
  blockquote), `docs/corpus/compat-matrix-20261110.json` (2 cell details + 1 appended editorial
  note), `worklog.md` (+19, the W24-D entry).

## Verdict headline

**0 P0 / 0 P1 / 1 P2 / 2 P3 (scout findings on the fix lane).** Both of L's P2s and the P1 are
genuinely, testably fixed; the fixes are reporting-only — the 166-row matrix counts are
byte-identical before/after, the raw `.jsonl` evidence is untouched (same git blob), and every
hand-edit carries provenance. The one P2 is a finish-line item, not a defect in what landed:
**IN-8 (aac-lc evidence double-append) is not addressed at the audited tip, and nothing on the
branch accounts for the fixed-vs-deferred split of L's ten P3s** (the branch may not be done —
snapshot caveat applies).

## What was actually run / verified (evidence for every claim below)

- `cargo test` in `tests/compat` (standalone workspace) — **11/11 pass**: the 4 pre-existing
  sha256 vectors **plus the 7 new report tests** (`report::tests::mixed_row_surfaces_its_refusals`,
  `distinct_refusal_messages_are_each_counted`, `pure_row_has_no_refusal_clause`,
  `failed_wasm_step_renders_steps_failed`, `passing_wasm_steps_render_engine_core_only`,
  `steps_lane_without_records_claims_nothing`, `all_refused_row_stays_refused`). 0 ignored.
- `cargo clippy --locked --all-targets` in `tests/compat` — **0 warnings** under the mirrored
  deny lints (`unwrap_used`/`expect_used`/`panic`/`todo`/`unimplemented`/`dbg_macro`/
  `await_holding_lock` deny, `unsafe_code` deny — `[workspace.lints]` in tests/compat/Cargo.toml).
- `cargo fmt --check` in `tests/compat` — clean.
- Raw-rows immutability: `git rev-parse` of
  `docs/corpus/compat-matrix-20261110.raw-rows.jsonl` — blob `469f246…` **identical** on
  `origin/w24/d` and `022d679`; the diff touches no `.jsonl`.
- Counts cross-check (recomputed by me from the raw rows with python): 166 rows; valid-class
  outcome distribution 84 decoded / 22 partial / 6 refused (2 probe + 4 demux) / 0 demux-fatal;
  54 malformed all-typed; the 6 valid-class refusals are exactly `asf-head.wmv`,
  `0316-audio-only.flac`, `0257-avi-video.avi`, `0146-avi-video.avi`, `0370-audio-only.wav`,
  `0241-audio-only.wav` — the avi/wav four with exactly the messages the new cells quote.
  JSON `counts` block byte-identical to `origin/w24/d` (files_total 166, decoded 84,
  decoded_partial 22, refused_probe 2, refused_demux 4, demux_fatal 0, malformed_typed 54,
  oracle 29/0/6/0/0/0). md §1 headline line and the §3 table (statuses + corpus-files column
  43/11/14/6/10/1/10/4/5/2/1/1/2) unchanged. **Matrix counts unchanged: PASS.**
- Surgical-edit fidelity: the new md/JSON avi + wav cell details reproduce **string-for-string**
  what the fixed renderer emits (`{valid}/{valid+refused} files open+demux (…)` + the
  `; N extension-matched corpus files REFUSED at the open path — <msg ×count>` clause +
  error-contract), with the real counts (41/43, 8/10; messages from the raw rows).
- §6.2 rewrite fact-check: every per-row figure matches the §3 cell details; the four files'
  codec attributions match the regen manifest's own `purpose` lines ("AVI MPEG-4 ASP (blocked
  codec — honest partial cell)", "AVI WMV2 …", "WAV FLAC-in-WAV", "WAV MP3-in-WAV"); the
  "blocked-codec video rows own them — §6.3" cross-ref resolves (§6.3 lists
  `codec-video-mpeg4-asp` among the blocked rows); the WAV fmt-chunk attribution
  (PCM/IMA-ADPCM 0x0011/MS-ADPCM 0x0002/WAVEFORMATEXTENSIBLE) matches
  `crates/demux/src/wav/format.rs`.
- Worklog truthfulness: the 9 commit hashes + order + per-commit descriptions match
  `git log c339ee8..origin/w24/d` exactly; every verification result quoted in the Stage
  Summary appears verbatim in L's green list (25 files +14,485/−15; counts all-green; three
  smoke re-runs zero non-timing diffs; CI-derived re-run emits 0 transitions; 10/10 xtask
  tests; fmt both workspaces; clippy deny-lints; lint-literals 0 violations; 192→187/142→147).
- P2-2 reachability: `proc::run_bounded` returns `Ok(ProcRecord)` for nonzero exits and
  timeouts (only spawn-level failures are `Err`), so a failing wasm step is **recorded** in the
  `LaneRecord` (report.rs:502-515, built before the matrix at :617-621) and `steps_cell`'s
  failure arm is genuinely reachable, not dead code. Outcome strings (`exit {n}`,
  `terminated (…)`, `killed (timeout)`) all satisfy `!= "exit 0"`.
- No test weakening: diff adds tests only; no `#[ignore]`/`cfg(ignore)` anywhere in
  `tests/compat` (grep 0 hits); the 4 sha256 tests intact; no asserts deleted (report.rs had
  no `#[cfg(test)]` before).

## Per-L-finding verdicts

| L finding | Verdict | Notes |
|---|---|---|
| P1-1 worklog append missing | **FIXED (verified)** | `a776c24`; see §1 below |
| P2-1 in-row refusals unsurfaced | **FIXED (verified)** | `16d64b0`; cells + §6.2 + JSON + tests; see §2 |
| P2-2 wasm cells hard-coded | **FIXED (verified)** | `022d679`; derived + tests; see §3 |
| P3-1 verify_diff notes-insertion dead path | **not addressed** | xtask untouched at this tip |
| P3-2 oracle/config unit tests | **not addressed** | new tests cover the report layer only — `prefix_verdict`/`f32_verdict`/applicability/config rejections still untested |
| P3-3 "(linux)" hard-code + dead `refusal_count` | **half fixed** | dead `refusal_count` deleted (subsumed by P2-1); `report.rs:1807` still literal `"- **{}** (linux): …"` |
| P3-4 "VLC VLC" doubled word | **not addressed** | md line 9 still doubled |
| P3-5 `VlcSource::CommittedRecord` mislabel | **not addressed** | report.rs:448 unchanged |
| P3-6 first-audio limb dump vs fatal | **not addressed** | decode.rs untouched |
| P3-7 0385-dynres regen entry | **not addressed** | manifest untouched |
| P3-8 stale claims snapshot | **not addressed** | committed config still claims `implemented` for all five flipped rows (re-verified by parse) |
| P3-9 `read_capped` unbounded read | **not addressed** | proc.rs:168-176 unchanged |
| P3-10 (observation) unmarked hand-edited md | **fixed (beyond the ask)** | provenance blockquote under the md header + JSON editorial note name every editorial layer (post-apply §6, P's avi/wav/§6.2 edits) |
| IN-8 aac-lc evidence double-append | **not addressed at this tip** | feature-matrix.toml untouched — the duplicated citation is still on the `codec-audio-aac-lc` evidence line; no idempotence guard added to the applier |

### 1. P1-1 — the W24-D worklog entry (a776c24): truthful, exact format, append-only

+19 lines at the end of root `worklog.md`, in the exact AGENTS.md §4 shape (`---` / Task ID /
Agent / Task / Work Log / Stage Summary). The Agent line honestly discloses the reconstruction
("Entry appended 2026-11-10 by W24-P … reconstructed from the 9 commit messages, cross-checked
against W24-L's r2 audit"). All nine commit hashes, their order and their one-line descriptions
match `git log c339ee8..2e01a2c` exactly. Every verification result cited in the Stage Summary
is quoted accurately from L's green list (checked item-by-item — no inflation, no dropped
caveats; the CI-billing/`verified` caveat is preserved). No prior entry was edited (pure append
at EOF). **No findings.**

### 2. P2-1 — in-row refusals surfaced (16d64b0): the fix does what L asked, (a)+(c)+prose-(b)

- **Code:** `format_cell`'s NativeDecode mixed-row arm now renders the honest denominator
  `valid.len()/(valid.len()+refused.len())` and, when `refused` is non-empty, appends
  `"; N extension-matched corpus file(s) REFUSED at the open path — <breakdown>"`;
  `refusal_breakdown` (BTreeMap, deterministic alphabetical order) names each distinct typed
  message with its count. The dead `refusal_count` subtraction is deleted (P3-3's second half).
- **Artifacts:** md §3 `avi` cell → "41/43 files open+demux (…); 2 extension-matched corpus
  files REFUSED at the open path — unsupported in avi (demux.avi.no-supported-streams) ×2;
  error-contract: 9 malformed variants…"; `wav` → "8/10 … unsupported in wav
  (demux.wav.unsupported-format) ×2". JSON cell details updated identically + an appended
  POST-RUN EDITORIAL NOTE (lane, branch, date, finding, exact scope, raw-rows-immutable
  statement). §6.2's "all decoded … (43 files)" → "decoded … (41/43 open+demux) …
  (8/10 open+demux)" plus one explanatory sentence naming all four files, their codecs, the
  typed messages, and why no container flip is required — every fact in it verifies (raw rows,
  manifest purposes, §6.3 cross-ref, wav/format.rs).
- **Completeness:** I recomputed the refusal partition from the raw rows — avi and wav were the
  *only* mixed rows with hidden refusals (the other two valid-class refusals sit in all-refused
  rows, flac/asf, which the pre-existing arm already surfaced). Nothing was left hidden.
- **Tests:** `mixed_row_surfaces_its_refusals` genuinely exercises the exact path L flagged
  (2 probe-matched valid + 1 extension-matched `RefusedDemux` → asserts status `decoded`,
  "2/3 files open+demux", the REFUSED clause, and the quoted message `×1`); controls for the
  pure row, the all-refused row, and the per-message breakdown. All pass.

### 3. P2-2 — web-wasm cells derived from recorded step outcomes (022d679)

`steps_cell(lane, steps, pass_detail)`: first step with `outcome != "exit 0"` → status
`steps-failed`, detail quoting that step's purpose, argv and recorded outcome (and never the
"module builds" claim — asserted); no recorded steps → `not-covered` / "nothing claimed"; all
`exit 0` → the pass detail. Both `format_cell` and `codec_cell` route through it, fed by
`lane_steps(&lanes, &lane.id)` from the `LaneRecord`s built before the matrix is rendered. The
pass-path text is **byte-identical** to the old hard-coded strings (asserted by exact-string
tests), so the committed md — whose two recorded steps were both `exit 0` — needed no web-wasm
edits (and got none; consistent). Failure/timeout paths are reachable (see evidence list).
Tests cover failed/timeout/pass/no-records for both row kinds. **Fix verified.**

## Scout findings on the fix lane

### R10-1 (P2) — IN-8 unaddressed and no fixed-vs-deferred accounting anywhere on the branch at the audited tip

- **File:line:** `docs/feature-matrix.toml` (`codec-audio-aac-lc` evidence line — the citation
  `docs/corpus/compat-matrix-20261110.md (this run's matrix table + raw rows)` still appears
  twice, joined by `; `); `tools/xtask/src/cmd/compat_matrix.rs` (no evidence-append dedupe
  guard); absence of any fixes-response note (no W24-P worklog entry, no doc listing what of
  L's P3-1..P3-10 and IN-8 was fixed vs deferred).
- **Evidence:** the P diff does not touch `feature-matrix.toml` or `tools/`; the double-append
  is verified still present (IN-8, w24-integration-findings.md §IN-8, was reported against the
  merged tree pre-P and named this lane's tooling as the owner). Of L's ten P3s only P3-3's
  second half (dead `refusal_count`) and P3-10's optional marker landed; P3-1/2/4/5/6/7/8/9 and
  P3-3's first half are untouched — and nothing on the branch says so. The W24-D worklog entry
  P appended (its first commit) closes with "Residuals the audit flagged (P2: …; P3: unit-test
  coverage, wording/labeling) **are being fixed by W24-P on w24/p**" — accurate when written,
  but it now overstates the tip: the P2s landed, the named P3 classes (oracle unit-test
  coverage = P3-2; wording/labeling = P3-4/P3-5) did not.
- **Impact:** a merge-time reader reconciling the fix lane against L's findings + IN-8 has to
  re-derive the fixed/deferred split from commit messages (as this audit did); the IN-8
  duplicated evidence string ships into the wave merge if P stops here.
- **Caveat:** lane P is plausibly still running (branch unpushed, worktree active) — this may
  simply be mid-flight. If further commits land, re-check; if this is the final state, the
  bounded remediation is: one dedupe edit on the aac-lc evidence line + an idempotence guard in
  the applier (IN-8's own suggestion), and either a short fixes-response section in the W24-P
  worklog entry or a one-line "deferred to <card/lane>" list.

### R10-2 (P3) — P3-2 remains the highest-value deferred item; the seven new tests all live in the renderer

- **File:line:** `tests/compat/src/oracle.rs` (`prefix_verdict`, `f32_verdict`,
  `compare_row_committed`), `config.rs` (load/validate) — still zero `#[cfg(test)]`; the new
  coverage is entirely `report.rs` cell rendering (valuable, but it tests the fix, not the
  oracle honesty claims L flagged).
- **Suggested fix:** L's original suggestion stands unchanged (the 2026-11-10 rows are
  ready-made fixtures). Not a regression — just the sharpest remaining gap.

### R10-3 (P3) — `steps_cell`'s failure detail embeds wasm-specific prose for any Steps lane

- **File:line:** `tests/compat/src/report.rs` (`steps_cell`, the failure arm appends
  "DECODE refused — the wasm32 module ships decoder-less (apps/web/src/capabilities.rs); no
  decode claimed" unconditionally).
- **Evidence:** only semantically wrong if a future config adds a non-web Steps lane (none
  exists; the config schema allows it). The pass detail is parameterized; the failure detail is
  half hard-coded. Cosmetic, one-string refactor if it ever matters.

## Regression / scope checks — all PASS

- **Raw evidence immutable:** `.raw-rows.jsonl` blob identical (`469f246…`); no `.jsonl` in
  the diff.
- **Counts unchanged:** JSON `counts`, md §1 headline, §3 table (statuses + corpus-files
  column) all byte-identical; the §6.2 "(43 files)" → "(41/43 open+demux)" change is a display
  refinement of the *same* denominators (43/10 remain in the corpus-files column), not a count
  shift. Reporting-only confirmed — no status flips, no transitions re-emitted, feature-matrix
  untouched.
- **Provenance:** md provenance blockquote (runner render / post-apply §6 / P's surgical
  edits, each with commit or branch + findings-doc citation); JSON editorial note; worklog
  disclosure. All three name their author, date and reason.
- **Tests not weakened:** additions only; no `#[ignore]`; no deleted asserts; 11/11 pass.
- **Strict Rust:** clippy `--all-targets` 0 warnings under the mirrored deny lints; fmt clean.
- **Scope:** 4 files — report.rs (P2-1/P2-2/P3-3-half), the matrix md + json (P2-1 render
  surface + P3-10 marker), worklog.md (P1-1). All findings-named (or their machine-record
  counterpart) + the worklog. Nothing out of scope.

## Bottom line for the Lead

Merge-quality of what **is** committed: green across the board — the P1/P2 fixes are real,
tested, provenance-clean and count-preserving. Before wave close, confirm with lane P: (1)
IN-8's one-line dedupe + applier guard, (2) an explicit fixed-vs-deferred list (or more
commits), (3) ideally P3-2's oracle vectors. If lane P is still running, this audit should be
re-run on its final pushed tip.
