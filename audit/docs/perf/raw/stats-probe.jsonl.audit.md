# CLAIM AUDIT: docs/perf/raw/stats-probe.jsonl
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 2 probe records (raw artifact) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 2 REAL (corroborates code) / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Raw probe log of a live control session on avi_mjpeg_pcm.avi; both records are failure
evidence and both corroborate the shipped code exactly: `system.get-stats` is not even a
catalog variant (exit 64, unknown variant), and `system.get-state-snapshot` is a catalog
command the CLI host typed-refuses.

## 2. Claim ledger
| # | Claim (recorded behavior) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | `system.get-stats` → exit 64, `[control.invalid-argument] unknown variant` with the full 60-command expected-set in stderr | line 1 | REAL | stderr list matches the catalog exactly (60 ids incl. video.take-screenshot, playlist.*, media.get-chapters) — cross-checked against ui/src/generated/commands.ts cmd union; variant rejection is serde-derive behavior |
| 2 | `system.get-state-snapshot` → error `unsupported-for-host`, "`system.get-state-snapshot` is not served by this build's command-line host" | line 2 | REAL | matches apps/cli/src/dispatch.rs:154-155 catch-all `other => Err(unserved(other))` (sed-verified quote) and unserved() message format (dispatch.rs:159-170) |
| — | session walks idle→opening→buffering→stopped→idle, tracks mjpeg + pcm_s16le, duration 2,000,000 µs | stdout_lines | REAL | consistent with baseline mp4-smoke engine-info decoders (mjpeg, pcm_s16le) |

## 3. Contradictions
None. This artifact is direct evidence for the CLI-host-limited-serving finding (my
matrix-rows-248-252 ledger F-003/F-004): the shipped host serves ~13 of the 60
catalog commands; every other command answers a typed refusal.

## 4. Fabrication / inflation indicators
None — the artifact records failures, not wins.
