# DATA AUDIT: docs/baselines/raw/linux/08-vlc-dummy-smoke.txt (credit: W19-A09, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | `vlc -I dummy --play-and-exit vlc://quit` headless smoke transcript, exit=0 |
| Cited by | worklog archive-2026-10.md:500 (filename-collision record); internal to M1 lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
REALISTIC and internally consistent but not re-runnable (binary at tmp/pinning/extract is
gone; no vlc in this sandbox). What I verified:
1. The transcript's error sequence (vlcpulse "Connection refused", dbus autolaunch failure
   without $DISPLAY, globalhotkeys "no suitable interface module") is exactly the noise a
   headless VLC emits before the dummy module loads — and then the run completes:
   `dummy interface: using the dummy interface module...` -> `idummy demux: command
   'quit'` -> `main playlist: end of playlist, exiting` -> `exit=0`.
2. Cross-evidence: the `idummy` demux module invoked here is present in the SAME binary's
   module list — dump 05 (05-vlc-list.txt:380) `idummy  Dummy input` and dump 06:928 —
   the dumps are mutually consistent artifacts of one binary.
3. The invocation matches the contract it claims to prove: docs/02*.md:131 (M1) specifies
   `-I dummy ... --play-and-exit` as the headless lane; this dump exercises the interface
   half (`vlc://quit` instead of a media file — interface smoke, not the full M1).
S4: the run predates the M1 full form (no --verbose 2 / --no-video / --no-audio flags), so
it proves the dummy-interface half only — the dump's own header scopes it correctly
("headless smoke run via dummy interface"). Quotes byte-verbatim from Read output.
Verdict per claim: REAL-UNPROVEN (authentic-looking, unrepeatable from this sandbox).
