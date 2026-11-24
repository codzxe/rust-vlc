# AUDIT: tests/integration/first-vertical/evidence/transcripts/wire-session.json
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| What | transcript/1 evidence (69 lines) for the `wire-session` scenario (the binary's --control-stdio NDJSON mode): 7 commands (open/pause/get-current-time/tracks.list/get-duration/stop/close), 10 events recorded AS RAW WIRE STRINGS (escaped JSON lines, honest raw capture — state machine idle→opening→buffering→paused→stopped→idle across gens 0→2), observables: exit_code 0, stderr_empty true, 7 response lines verbatim |
| Consumers (rg) | tests/integration/first-vertical/tests/evidence.rs:212 (replay comparator); tests/integration/first-vertical/tests/wire_mode.rs:10 ("the catalog's own serde forms end to end") |
| Verdict | REAL. Baseline cargo-test-workspace.txt L2785+ `test result: FAILED. 1 passed; 2 failed` — the two failures are the `open` scenario and final_frame (see open.json audit F-001); wire-session's comparison is inside the passing set, so this transcript MATCHED a fresh run on the baseline host. The event stream is coherent (gen/origin transitions consistent with pause/stop/close; seq contiguous 1-10). No findings.
