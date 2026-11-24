# DATA AUDIT: docs/baselines/raw/linux/probe-work/run_probes.sh (credit: W19-A06, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | Bash probe-harness script (37 lines) — drives the pinned VLC 3.0.23 oracle headless over synthetic fixtures |
| Executed? | Historical (probe runs live in docs/baselines/raw/linux/05-... dumps); fixture tree /home/z/my-project/tmp/probe is gone (volatile) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## Verdict
A real, coherent probe runner; its on-repo references all exist:
- The playlist loop's 6 files (m3u/pls/xspf/asx/b4s/cue _sample) == exactly the 6
  files present in `docs/baselines/raw/linux/probe-work/pls/` ✓ (verified by ls);
  `dir/` exists for the `dir_access` probe ✓; `gen_py.py` is the fixture generator
  (the clean-room note's "python struct-writers") ✓.
- The oracle invocation `-I dummy --play-and-exit --aout dummy --vout dummy -vv`
  matches the 08-vlc-dummy-smoke.txt idummy transcript family (W19-R7 verified the
  idummy module cross-reference in dump 05:380).
- Per-run `timeout 25` + exit-code capture to status.txt (the loop INTENTIONALLY
  continues past failures — correct design for a fault-probe harness, so the absence
  of `set -e` here is deliberate, not hygiene debt).
- No secrets, no network, off-repo absolute paths only for the volatile /tmp scratch
  (documented pattern of this baselines family).
S4 #1: unquoted `$P`/`$EXTRACT` expansions (paths contain no spaces today — latent
only). S4 #2: no date/provenance header in the script itself (the surrounding dumps
carry the dates). The script is evidence-infrastructure, not shipped CI — no further
defects.

Commands run: ls probe-work + probe-work/pls + /home/z/my-project/tmp/probe (gone);
sed -n full read; cross-read 08-vlc-dummy-smoke.txt references.
