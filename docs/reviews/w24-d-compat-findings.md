# W24-L review (r2) — compat-matrix lane audit (w24/d, issue #53, cards 8.B1–8.B2)

Read-only audit. Auditor: W24-L, wave 24 review scout (round 2, resumed; round 1's
partial verification artifacts under `/tmp/w24l-verify` were re-checked and
extended — every check below re-run by me on the fresh worktree
`/home/z/my-project/wt/w24l`, branch `w24/l` == `origin/w24/d` @ `2e01a2c`,
unless marked otherwise).

- **Target:** `git diff origin/main...origin/w24/d` — 25 files, +14,485/−15:
  `tests/compat/**` (runner: config/corpus/decode/oracle/proc/sha256/report,
  `compat-matrix.json`, `Cargo.lock`, regen manifest, `.gitignore`),
  `docs/corpus/compat-*` (md/json/raw-rows/transitions outbox + gap-triage
  addendum + applied ledger), `docs/feature-matrix.toml`, the xtask
  `compat-matrix apply-transitions` writer (+ one dispatch-line registration),
  `.github/workflows/compat-matrix.yml`, `tests/compat/ci/derive_config.py`,
  root `.gitignore`.
- **9 commits** on top of the merge base `c339ee8`; all inside the lane's
  ownership (scope check PASS — with one omission, P1-1 below).

## Verdict

**0 P0 / 1 P1 / 2 P2 / 10 P3.** The lane's core claims all verify: the matrix
evidence is real, reproducible, and internally consistent end-to-end; the
status transitions were genuinely applied through the parity tool with a
machine-written ledger and zero hand-edited statuses; the gap triage's five
flips match the run's evidence exactly. The one P1 is procedural (mandatory
worklog append missing), not evidentiary. The P2s are honesty sharpness issues
in the report surface (refusals inside claimed-implemented container rows are
counted but never surfaced; the wasm lane's cell text is not derived from the
step outcomes it cites).

## Findings

### P1-1 — mandatory worklog append missing (AGENTS.md §4): the 9-commit lane shipped with zero worklog entries

- **File:line:** repo root `worklog.md` (no W24-D section); AGENTS.md §4
  ("After finishing your Task ID: append a section using exactly this
  format").
- **Evidence:** `git diff origin/main...origin/w24/d -- worklog.md` → empty
  (the only expected-scope file absent from the diff). `rg -i 'w24.?d|8\.b1|
  8\.b2|compat' worklog.md` → 0 hits; the live log still ends at the w23/f
  entry. Sibling lanes did append (e.g. `origin/w24/h` worklog.md +35 lines,
  "worklog: W24-H r7 closeout"). The task card's audit scope lists "worklog"
  as expected lane content.
- **Impact:** the append-only shared log is the wave-merge provenance record;
  a lane this size (a new tool, a 166-file evidence run, 6 matrix status
  flips, a CI workflow) landing with no worklog entry breaks the protocol
  every other lane follows and leaves the merge-time reader without the
  lane's own account.
- **Suggested fix:** the fix lane appends the W24-D entry (9 commits' worth:
  runner build, real run, parity tool, apply + gap triage, CI job) to root
  `worklog.md` in the exact §4 format — content is fully reconstructible from
  the commit messages, which are unusually detailed.

### P2-1 — 4 valid-class corpus refusals inside claimed-`implemented` container rows are counted but never surfaced: §6.2's "all decoded … (43 files)" overstates, and the "zero unexplained gaps" AC is not literally true

- **File:line:** `tests/compat/src/report.rs:584-609` (valid bucket =
  probe-format-matched only; refused bucket = extension-matched refusals),
  `:621-625` (row verdict — `"refused"` only when `valid.is_empty()`),
  `:890-911` (`container_verdict` sees only `valid`), `:1169-1256`
  (`container_gap` arms fire only on the all-refused / demux-fatal verdicts);
  rendered symptom in `docs/corpus/compat-matrix-20261110.md:65,72` (cells
  say "41/41" and "8/8 files open+demux" while the table's corpus-files
  column says 43 and 10) and `:230` (§6.2: "Containers, all decoded through
  the real open path … `container-avi` (43 files) … `container-wav` (10)").
- **Evidence (all from the committed raw rows, re-parsed by me):** exactly 6
  valid-class files are refused; beyond the two triaged ones (asf-head.wmv →
  the §6.1 asf flip; 0316-audio-only.flac → §6.3's not_started row), FOUR sit
  inside claimed-`implemented` container rows and are in no disposition list:
  - `tests/compat/.regen-cache/0257-avi-video.avi` — `refused_demux`:
    "unsupported in avi (demux.avi.no-supported-streams)" (manifest: MPEG-4
    ASP video, "blocked codec — honest partial cell");
  - `tests/compat/.regen-cache/0146-avi-video.avi` — `refused_demux`:
    "unsupported in avi (demux.avi.no-supported-streams)" (WMV2);
  - `tests/compat/.regen-cache/0370-audio-only.wav` — `refused_demux`:
    "unsupported in wav (demux.wav.unsupported-format)" (FLAC-in-WAV);
  - `tests/compat/.regen-cache/0241-audio-only.wav` — `refused_demux`:
    "unsupported in wav (demux.wav.unsupported-format)" (MP3-in-WAV).
  These four ARE in the headline "6 refused" count and in the rows' `files`
  arrays (hence 43/10), but the native cells render "41/41"/"8/8", no gap
  rule fires (mixed rows → verdict `decoded`), and §6.2's "all decoded"
  prose contradicts its own denominators. A reader must mine the raw JSONL to
  explain the 43-vs-41 and 10-vs-8 deltas — the exact class of
  claim-vs-run disagreement the card's every-gap-dispositioned rule targets.
  (The refusals themselves are typed and arguably codec-level — the AVI/WAV
  demuxers refusing streams whose codecs are blocked/unwired — but that
  attribution is precisely what the report should say, not leave inferred.)
- **Suggested fix (bounded):** (a) in `format_cell`, when both buckets are
  non-empty, append "; N extension-matched corpus files REFUSED at the open
  path — <worst message>" to the native cell detail; (b) either extend
  `container_gap` with an informational arm for mixed rows or (simpler)
  add a §6.3-style bullet per affected row: "container-avi: 41/43 open+demux,
  2 refused no-supported-streams (blocked-codec video files — codec-level,
  next step: the codec rows own it)"; (c) make §6.2's per-row counts equal
  the cells' denominators (41 and 8, not 43 and 10) with the refused count
  named. No status flip is required — the claims are defensible once the
  refusals are visible.

### P2-2 — the web-wasm cell's status/detail is hard-coded, not derived from the recorded step outcomes: a failed build step would still render "engine-core-only … module builds"

- **File:line:** `tests/compat/src/report.rs:995-1003` (format rows) and
  `:1122-1128` (codec rows) — the `LaneKind::Steps` cell is a constant
  string ("wasm32 module builds (see the lane's recorded steps); … DECODE
  refused …") that never consults the lane's `StepRecord`s.
- **Evidence:** `LaneRecord.steps` carries each step's `outcome`
  (`StepRecord`, report.rs:58-70), and §2 of the committed md renders them
  ("cargo build -p lumen-web --target wasm32-unknown-unknown → **exit 0**
  (19.9s)"; "cargo test -p lumen-web → **exit 0** (22.7s)") — but nothing
  feeds those outcomes into the cells. In the committed run both steps are
  exit 0, so the shipped evidence is honest; the latent gap is that on a
  future run where the wasm build FAILS, every web-wasm cell would still say
  "engine-core-only … builds", contradicting the lane's own recorded steps in
  the same document.
- **Suggested fix:** derive the Steps-lane cell from its `LaneRecord`: all
  steps exit 0 → current "engine-core-only" text; any nonzero/timeout →
  status `steps-failed` with the worst step outcome quoted. (The
  `LaneRecord` is built at report.rs:502-515 before the cells are rendered —
  the data is already in hand; the cell functions just need the lane record
  passed in.)

### P3-1 — verify_diff's key-count check defeats the R6 notes-insertion path: a row without `notes` can never be edited

- **File:line:** `tools/xtask/src/cmd/compat_matrix.rs:666-677` (insert a
  `notes` line when `notes_line` is None) vs `:750-756`
  (`entries_before.len() != entries_after.len()` → "added/removed keys …
  refusing to write").
- **Evidence:** inserting `notes = "…"` after the evidence line adds one key
  to the parsed row, so `verify_diff` always rejects the rewrite the insert
  path just built. Latent only — all 252 shipped rows carry `notes`
  (`scan_blocks` + my tomllib parse: 0 notes-less rows) — but the branch's
  comment claims it satisfies R6 "by construction" and it cannot ever run.
- **Suggested fix:** in `verify_diff`, allow `notes` to be a new key exactly
  when the group's row had none before (mirror the insert condition), or
  replace the insert with a loud error "row lacks `notes` — add one first"
  so the dead path is not silently unreachable.

### P3-2 — the runner's verdict logic has zero unit tests (only sha256.rs is tested)

- **File:line:** `tests/compat/src/oracle.rs:250-315` (`prefix_verdict`,
  `f32_verdict`), `:361-405` (`compare_row_committed` applicability),
  `report.rs:371-374` (the `lumen_fresh_md5` applicability check),
  `config.rs:286-338` (load/validate). The workspace's only `#[cfg(test)]`
  is `sha256.rs:167-208` (4 vector tests — all pass via clippy build; the
  FIPS vectors are also cross-verified transitively, see green list).
- **Evidence:** these are pure, easily-fixtureable functions carrying the
  run's central honesty claims (bit-exact prefix, tolerance thresholds,
  committed-record applicability). By contrast the xtask writer has 10 tests.
  The config/2 platform validation and schema refusal were only ever
  smoke-tested by hand (`/tmp/compat-smoke/config-bad-platform.json`; I
  re-ran the binary: exit 2 with the correct message).
- **Suggested fix:** add `#[cfg(test)]` vectors to oracle.rs (the 2026-11-10
  rows are ready-made fixtures: one bit-exact prefix with tail, one
  tolerance-fail, one vlc_bytes > lumen_bytes → NotBitExact, one
  committed-applicable vs fresh-realization pair) and a config-test for the
  platform/schema rejections.

### P3-3 — renderer hard-codes "(linux)" in the gap-triage bullets; `refusal_count` is dead code making "{}/{} files open+demux" always n/n

- **File:line:** `tests/compat/src/report.rs:1737`
  (`"- **{}** (linux): …"` — the platform is config-driven everywhere else,
  config/2); `:940-946` + `:1032-1044` (`valid.len() - refusal_count(valid)`
  — `valid` only contains probe-format-matched rows and every
  Refused{Probe,Demux,Input} record has `probe_format: None`
  (decode.rs:529-560), so `refusal_count(valid)` is provably 0 and the first
  number always equals the second).
- **Evidence:** a windows/macOS CI run (via `derive_config.py`) would render
  "(linux)" on its own transitions bullets — contradicting the config/2
  explicit-attribution design the same file implements. The dead subtraction
  is harmless but signals an intent (surface refusals in the detail) that
  P2-1 shows was never wired.
- **Suggested fix:** use `native_platform_of(config)` at :1737; delete
  `refusal_count` or (better, subsumed by P2-1's fix) make it count the
  row's `refused` bucket.

### P3-4 — "the pinned VLC VLC version 3.0.23 …" — doubled word in the oracle source line

- **File:line:** `tests/compat/src/report.rs:468-477` (format string "live
  run: the pinned VLC {version_line}") + `compat-matrix.json:83`
  (`version_line` itself begins "VLC version …").
- **Evidence:** committed md line 9: "| Oracle | live run: the pinned VLC
  VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd) …".
- **Suggested fix:** drop the literal "VLC" from the format string (the pin
  already names it).

### P3-5 — LumenRefused oracle rows are labeled `vlc_source: committed-record` though no VLC side was consulted

- **File:line:** `tests/compat/src/report.rs:349-361` (the no-dump branch
  pushes `VlcSource::CommittedRecord` with empty vlc_md5/vlc_bytes).
- **Evidence:** schema consumers reading `vlc_source` would infer a
  committed-record comparison happened; the verdict (`LumenRefused`) tells
  the real story, but the label is still wrong for rows where neither side
  ran. No such row exists in the committed run (all 35 comparable rows
  decoded).
- **Suggested fix:** add a `VlcSource::None` variant (or emit
  `NoComparison` with the refusal as `deviation: None`), keeping
  `committed-record` for rows where the record was actually applied.

### P3-6 — a first-audio limb with a decode fatal still feeds its partial PCM dump to the oracle comparison

- **File:line:** `tests/compat/src/decode.rs:449-475` — the dump is captured
  from the audio sink before the fatal is dispositioned; `report.rs`
  compares whatever bytes were accepted.
- **Evidence:** if a fatal truncates the payload AFTER the VLC window, the
  prefix method could false-pass a damaged decode. Latent: in the committed
  run all 35 comparable rows decoded with no limb fatal (verified: every
  comparable raw row's first audio track is `status: decoded`).
- **Suggested fix:** null the dump when the owning limb ends `fatal`
  (move the dump capture after the fatal check), or have the oracle row
  record the truncation.

### P3-7 — the 0385-dynres-video.mkv manifest entry can never regenerate; the FAILED outcome has no next step

- **File:line:** `tests/compat/regen-corpus-manifest.json` (entry
  `0385-dynres-video.mkv`, argvs reference the `corpus-big/_tmp` concat
  intermediate); rendered at `docs/corpus/compat-matrix-20261110.md:302`
  ("**FAILED**: ffmpeg step failed …").
- **Evidence:** the argv chain's input is a machine-local intermediate that
  no longer exists (commit 7223106's message says so); every future run on
  any host will record the same FAILED row. The runner handles it honestly
  (typed failure, row excluded from the sweep) but neither §7 nor the
  manifest records the fix path.
- **Suggested fix:** re-author the entry to be self-contained (single-pass
  ffmpeg argv, re-pin sha256), or record "next step: re-author the argv
  chain" in the manifest entry's `purpose` and §7.

### P3-8 — the committed config's claims snapshot is now stale vs the post-apply matrix (safe by design, but a local re-run emits 5 unappliable proposals)

- **File:line:** `tests/compat/compat-matrix.json:553-799` (claims: the five
  flipped rows still say `implemented`; live matrix says `partial`).
- **Evidence:** safe by construction — `config.rs:47-51` documents that a
  stale snapshot can only produce rejected proposals, and the applier's
  STALE check (compat_matrix.rs:470-481, unit-tested) rejects them
  all-or-nothing; the CI path never sees this (`derive_config.py` refreshes
  claims from the live matrix — I re-ran it: derived claims show
  `codec-audio-pcm: partial`, and the derived-config re-run emitted 0
  transitions). Cost: a local `run` with the committed config re-emits the
  five from=implemented proposals as noise and renders "claimed (linux)"
  columns that disagree with the matrix the report sits next to.
- **Suggested fix:** refresh the committed claims snapshot in the fix lane
  (one JSON edit, no runner change), or add a header note in the config
  pointing local re-runs at `ci/derive_config.py`.

### P3-9 — `read_capped` reads the entire child output file before clipping (unbounded read, bounded retention)

- **File:line:** `tests/compat/src/proc.rs:168-176` (`std::fs::read(path)`
  then clip to 64 KiB head/tail).
- **Evidence:** a chatty child (the corpus argvs mitigate with
  `-loglevel error`, but lane steps are arbitrary) could stream gigabytes to
  the temp file; the process would read it all into memory to keep 64 KiB.
- **Suggested fix:** read at most `OUTPUT_CAP` from the head and seek-tail
  `OUTPUT_CAP` (two bounded reads).

### P3-10 (observation) — the committed md is a hand-edited hybrid: §6/§8 were rewritten post-apply (3fdbf6d) away from the runner's rendering

- **File:line:** commit `3fdbf6d` diff of `docs/corpus/compat-matrix-20261110.md`
  (§6 header "the transitions outbox" → "every claimed-supported row
  dispositioned", the proposals list → the applied-tables §6.1-6.3, note 8
  "are PROPOSALS" → "WERE proposals … now APPLIED").
- **Evidence:** the edit is self-describing (it names the tool, the outbox,
  the addendum and the ledger) and every number in it verifies against the
  machine artifacts (6 ledger lines; 5+1 provenance; feature-audit
  192→187/142→147 exact). The machine JSON still carries the raw 5 proposals,
  so the canonical record is intact. Residual risk: the .md is no longer
  byte-reproducible from the runner, and nothing in the file marks WHICH
  sections are post-run editorial.
- **Suggested fix (optional):** a one-line marker under the header ("§6 and
  the last note were hand-updated after the apply — the JSON is the
  machine-rendered record"), or a `render` subcommand that regenerates §1-§5,
  §7 from the JSON so future runs don't fork the prose.

## Matrix counts cross-check (check 1 — all green)

| Claim (md ↔ json ↔ jsonl) | Verified value | Result |
|---|---|---|
| Corpus files 166 = 46 committed + 54 malformed + 66 regen | manifest 67 entries − 1 FAILED regen = 66 offered; 46+54+66 = 166; raw-rows = 166 lines | ✓ |
| Outcome counts 84 decoded / 22 partial / 6 refused / 0 demux-fatal / 54 malformed-typed | recomputed from raw rows: valid-class 112 = 84+22+6+0; malformed 54 redistribute as 13/11/2/20/8 (sum 54) — headline aggregates exact | ✓ |
| Oracle 35 rows = 29 bit-exact + 6 f32 tolerance-FAIL + 0 pass/other | verdict Counter over the 35 rows: `{bit_exact_prefix: 29, tolerance_fail: 6}`; md table (35 rows parsed) ↔ JSON: bytes, truncated md5s, formats — ALL MATCH | ✓ |
| Prefix arithmetic | all 29: tail = lumen_bytes − vlc_bytes exact, tail > 0 | ✓ |
| f32 verdicts | recomputed from recorded deviations vs 1e-4/1e-3 (config-sourced thresholds): verdicts correct, samples ≤ min(len)/4 on all 6 | ✓ |
| Oracle rows ↔ raw rows | 35/35 `lumen_md5` == the raw row's first decoded audio track `pcm_md5` | ✓ |
| Committed-record cross-check (2026-11-09) | 20 same-realization rows: vlc_md5/vlc_bytes match the record exactly; 15 fresh-realization rows differ — the SAME 15/20 split the run's own `lumen_fresh_md5` applicability check produced | ✓ |
| Prior-art agreement (2026-10-23 report) | same-realization rows' Lumen md5s identical across the 10-23 and 11-10 runs (e.g. 0121 `36355267afc2…`, 0236 `2fa047d2f34c…`, 0324 `1767da988df2…`); every divergence is a fresh-realization row | ✓ |
| Format/codec cell details | avi 41 valid + 2 refused + 9 malformed; mp4 14+14; matroska 11+9; wav 8+0(+2 refused); pcm 48 files; aac 23 files / 142 concealed; aac-lc 4 decoded + 1 REFUSED; alac/vp9 1 REFUSED each — all recomputed from raw rows | ✓ |
| Windows/macOS cells | every cell in every row = `blocked` with "blocked-by-external: {reason} [ref; next step; tracking]" — nothing inferred | ✓ |
| Post-apply feature-audit claim (252 rows, 192→187 implemented / 142→147 partial) | origin/main parse: 192/142; HEAD: 187/147 (my re-run of `xtask feature-audit`: PASS, 252 rows, 187/147) — exactly the 5 linux flips | ✓ |

## Verified-green list (re-run by me on w24/l @ 2e01a2c)

- `cargo fmt --all --check` — root workspace CLEAN; `cargo fmt --check` in
  `tests/compat` (standalone workspace) CLEAN.
- `cargo clippy --locked --all-targets` in `tests/compat` — CLEAN (under the
  mirrored deny lints: `unwrap_used`, `expect_used`, `panic`, `todo`,
  `unimplemented`, `dbg_macro`, `unsafe_code` — so the no-unwrap/panic policy
  is machine-proven for the runner; `#![forbid(unsafe_code)]` at main.rs:45).
- `cargo test -p xtask --locked compat_matrix` — **10/10 pass**
  (`applies_a_transition_end_to_end`, `dry_run_writes_nothing`,
  `rejects_stale_from_and_writes_nothing`, `rejects_verified_transitions`,
  `rejects_unknown_rows_platforms_and_unresolved_evidence`,
  `coalesces_same_row_and_platform_and_is_idempotent`,
  `json_parser_roundtrips_the_outbox_shapes`,
  `outbox_parser_enforces_the_schema`, `scans_blocks_and_finds_rows`,
  `escape_toml_covers_the_writing_surface`).
- `cargo run -p xtask --locked -- feature-audit` — **PASS** (252 rows;
  implemented 187 / partial 147 — the applied state, exact).
- `cargo run -p xtask --locked -- lint-literals` — **PASS** (0 violations,
  452 allowlisted).
- `python3 tests/compat/ci/derive_config.py {linux,windows}` — derives
  correctly (native lane re-attributed + platform set; this-OS blocked lane
  dropped; sibling-lane citations rewritten; claims refreshed from the LIVE
  matrix — `codec-audio-pcm: partial` post-apply; reports redirected);
  refuses `web` with exit 1.
- Runner config/2 validation (live): `compat-matrix run --config
  config-bad-platform.json` (native platform `web`) → exit 2, "native-decode
  platform must be one of windows/linux/macos".
- `.github/workflows/compat-matrix.yml` — valid YAML (safe_load);
  `workflow_dispatch` only (no scheduled/PR spend); `permissions: read`;
  `fail-fast: false`; 3-OS matrix; SHA-pinned actions identical to
  ci-full.yml's pins (checkout v7.0.1 `3d3c42e…`, toolchain `d124f86…`,
  rust-cache v2.9.2 `6323deb…`, upload-artifact v7.0.2 `cf430e0…`);
  additive (new file, no required-check or other-workflow edits); the
  billing/3-OS blocker posture is documented in-header with the parked-until-
  provisioned promotion path.
- Parity discipline: `docs/feature-matrix.toml` diff vs origin/main = EXACTLY
  5 hunks (container-asf-wmv, codec-video-vp9, codec-audio-pcm,
  codec-audio-aac-lc, codec-audio-alac), linux-only status cells +
  machine-written notes/evidence appends matching the applier's format; the
  two aac-lc proposals coalesced per design (one cell edit, two ledger
  lines, merged reasons — visible in the row's notes). Ledger
  (6 lines) = outbox (5) + gap-triage addendum (1), timestamps and reasons
  byte-consistent. **Zero hand-edited statuses.**
- Evidence reproduction (beyond the required 10-row spot-check — all 35
  oracle rows plus the whole sweep): the dev's three smoke re-runs
  (`/tmp/compat-smoke`, commits ecc615e/4c3f6b8 states: linux 20:55,
  windows-config 21:08, CI-derived linux 21:17) reproduce the committed
  166 raw rows with **zero non-timing differences** (outcomes, tracks,
  packets, frames, concealed counts, pcm_md5s, refusal messages — all
  identical); the CI-derived re-run with refreshed claims emits **0
  transitions** (the loop closes: no re-proposing after the flips).
- Manifest integrity: 31/31 deterministic sha256 pins re-verified
  independently (python hashlib over the regen cache); 35/35 comparable
  entries' `oracle_rel` keys resolve in the committed 2026-11-09 record;
  `dump_pcm` set exactly on the comparable family.
- Scope: all 25 diff files within the lane's granted ownership
  (tests/compat/**, docs/corpus/compat-*, feature-matrix.toml via the tool,
  xtask module + one dispatch line, the new workflow, .gitignore
  target-xtask with the c339ee8 precedent) — the only expected-scope
  omission is the worklog (P1-1).

## Top 5 (by action value)

1. **P1-1** — append the missing W24-D worklog entry (protocol, merge-time
   provenance).
2. **P2-1** — surface the 4 in-row refusals (avi×2, wav×2) in the cells +
   triage; align §6.2's counts with the cell denominators.
3. **P2-2** — derive the web-wasm cell from the recorded step outcomes.
4. **P3-2** — unit tests for `prefix_verdict` / `f32_verdict` /
   committed-applicability / config rejections.
5. **P3-8** — refresh the committed config's claims snapshot (or point local
   re-runs at the deriver) so the next run doesn't emit 5 stale proposals.
