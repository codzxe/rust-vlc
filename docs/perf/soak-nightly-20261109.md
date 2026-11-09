# Soak nightly — the 4 h tier wired as CI (issue #18, card W13-AA)

Status: **plumbing + workflow are real and committed; the 4 h
measurement itself has NOT been taken** — it is the nightly job's job
(the successor protocol, §4). The only real number below is the
CI-short 2 min profile run this session (§3). The 5 MB/h ceiling is
`[PROPOSAL until first real measurement]`; the 50 MB/h default is the
asserted status quo.

| Field | Value |
|---|---|
| Card | W13-AA (issue #18 — soak nightly; branch `w12/18-soak`) |
| Captured | 2026-11-09, single session, idle 2-core sandbox (the §3 profile) |
| Harness | `crates/playback-engine/tests/soak/main.rs` (card 3.E4, doc 13 wave 3.E) — env-configured, not a hardcoded long test |
| Nightly invocation | `SOAK_MINUTES=240 SOAK_LEAK_MB_PER_H=5 SOAK_REPORT=soak-report.md cargo test -p playback-engine --test soak --release -- --nocapture` |
| Workflow | `.github/workflows/soak-nightly.yml` — cron `0 3 * * *` UTC + `workflow_dispatch`, ubuntu-latest, `timeout-minutes: 300` |
| Asserted ceiling | 50 MB/h default (`CEILING_MB_PER_H`), tightened to **5 MB/h `[PROPOSAL]`** only inside the nightly job via `SOAK_LEAK_MB_PER_H` |
| Real profile so far | CI-short 2 min, debug, defaults — §3 (verbatim tail) |
| 4 h baseline | **PENDING first scheduled run** — not DNF-by-sandbox anymore, see §4 |

## 1. The plumbing change (this card, commit 33e1118..61e0824 range)

Duration was already env-driven (`SOAK_MINUTES`, default `2` = the
CI-short value exactly); the **asserted leak ceiling was the hardcoded
`CEILING_MB_PER_H = 50.0`**. It is now `SOAK_LEAK_MB_PER_H` with that
same 50 MB/h default (parse errors refuse loudly, never default — the
`SOAK_MINUTES` discipline; unit-tested in the harness's config-parsing
module). The report and the assertion message carry the configured
value, so a tightened nightly failure names its own ceiling. The
CI-short path is byte-for-byte unchanged (unset env ⇒ old behavior).

## 2. The nightly workflow shape

`Soak-Nightly`: schedule + manual dispatch only — **non-blocking by
trigger shape** (the CI-Full issue #5 posture), never to be added to
required checks; `continue-on-error` is deliberately NOT set — a red
nightly is a triage item within 24 h, not a swallowed one. Toolchain
pinned and asserted exactly like ci.yml (`dtolnay/rust-toolchain@
d124f86…` 1.99.1 branch + the rust-toolchain.toml equality assert);
checkout/cache/upload-artifact pins copied verbatim (v7.0.1 / v2.9.2 /
v7.0.2). No `--ignored`: the harness's own convention is a plain
`#[test]` whose duration is env-driven. The harness writes the full
markdown report to `SOAK_REPORT`; the job uploads it as
`evidence-soak-nightly-ubuntu-latest-<run_id>` with `if: always()` so a
failing soak still ships its evidence.

## 3. The REAL 2-min CI-short profile (this session, this machine)

Command: `cargo test -p playback-engine --test soak` (debug profile,
all env unset ⇒ defaults: 2 min, 1 s cadence, 50 MB/h ceiling).
Verbatim tail of the captured log (`--release` not used; cargo
swallows the test's stdout markdown report on pass — use `--nocapture`
or `SOAK_REPORT` to see it, which the nightly does):

```text
test tests::minutes_default_and_valid_shapes ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 120.03s
```

12 tests = the soak itself (`soak_loops_stay_memory_bounded`, ~120 s
real wall) + the harness's config/memory/report/scenario unit tests.
This is the smoke-tier proof that the plumbing change compiles, runs,
and passes at CI-short scale — **it is NOT a 4 h datum and proves
nothing about the 5 MB/h target** (a 2 min window extrapolates ~24×).

## 4. The successor protocol (why the 4 h run is CI's, not this sandbox's)

The blocked-run record, pre-consolidation
(`git show 1ff2c70^:NEXT-STEPS-w7h.md`, task 7-h, 2026-10-24):
"Background-proc reaping (7-g's finding) reconfirmed: `setsid nohup`
gen died at tool-call end; foreground resume completed it." — this
sandbox reaps every background process when the spawning tool call
ends, and the tool-call/bash timeout (10 min) is far under 240 min;
chunked restarts are INVALID for this metric (the harness fits ONE
least-squares slope over a single post-warmup window). Hence:

1. The nightly job (this card) runs the 4 h profile on a real
   scheduled host, uploading `soak-report.md` as evidence.
2. The follow-up card (7-g's remainder) copies the report →
   `docs/perf/soak-4h-YYYYMMDD.md`, the raw log → `docs/perf/raw/`,
   commits, and writes the worklog D-10 block.
3. That first real measurement then DECIDES the `[PROPOSAL]`: slope
   ≤ 5 MB/h ⇒ target blessed as asserted ceiling; 5–50 MB/h ⇒ record
   the number, renegotiate or fix per triage (`phases.ranked_by_growth`
   names the top suspect in the assertion message itself); > 50 MB/h ⇒
   a gross leak the generous ceiling already fails on.

## 5. Open follow-ups

- First scheduled run (03:00 UTC after merge) produces the first
  artifact; until then every 4 h number in any doc would be fiction.
- The workflow's cron sits on the hour per the issue spec; CI-Full
  deliberately uses 03:17 off-the-hour — shift if contention bites.
