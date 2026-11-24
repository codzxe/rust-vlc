# DATA AUDIT: docs/baselines/raw/linux/07-vlc-list-plugins-attempt.txt (credit: W19-A04, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | honest FAILURE record: `--list-plugins` does not exist on VLC 3.0.23 |
| Cross-verified | against dumps 13/14/15 (same binary's --help transcripts) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
An honest-failure dump — the rarest and best evidence class in this repo: the task card
asked for `--list-plugins`, the flag does not exist on 3.0.x, and the agent recorded the
failure verbatim (usage error + exit=1) plus the correct equivalents (--list /
--list-verbose, dumps 05/06) instead of faking a plugin dump. Cross-checks I ran:
1. The quoted help excerpt `-l, --list, --no-list    print a list of available modules`
   byte-matches the same line in THREE independent sibling transcripts from the same
   binary: 13-options-advanced-longhelp.txt:3336, 14-options-full-help.txt:5483,
   15-options-help-and-probes.txt:391 (rg verified, exact string match).
2. No `list-plugins` string exists anywhere in the recorded --help transcripts (rg across
   the linux raw dumps) — consistent with "this flag does NOT exist" on 3.0.23.
3. Error format `vlc: unknown option or missing mandatory argument \`--list-plugins'` +
   "Try `vlc --help'" is the genuine VLC 3.0 usage-error grammar.
4. The attribution "it is a VLC 4.x flag" is UNVERIFIED from this sandbox (no VLC 4
   binary) but is a side note, not evidence.
The run itself cannot be repeated (tmp/pinning binary gone). Quotes byte-verbatim from
Read output. Zero defects; model evidence-hygiene for the whole baseline lane.
