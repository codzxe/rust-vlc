# CLAIM AUDIT: docs/guides/getting-started.md
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-R4) |
| Claims extracted | 11 |
| REAL 7 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 1 / UNVERIFIED 3 |
| Internal contradictions found | 2 (both staleness-class) |

## 1. Blunt verdict
A user-facing quickstart whose core facts hold up: the exact decode-bench
example line is byte-identical to the orchestrator baseline's recorded run,
the verb list matches help.rs, the ADR references and session test exist, the
honest "you will not hear or see anything" framing matches the null-audio /
skeleton-desktop reality. Two staleness defects: the `lumen --version` example
says 0.1.0 while the CLI crate (and product.toml) now say 0.1.3; the "37 of
73" claimed-format count is 38 today.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`cargo build --release --locked -p lumen-cli -p desktop`" works | :17 | REAL | Baseline audit/runs/baseline/cargo-build-release-lumen-cli.txt + cargo-build-release-desktop.txt: exit 0 (spec §13: release lane builds clean). |
| 2 | "Two binaries ... `lumen` (the CLI) and `lumen-desktop` (ADR-004 composition-root skeleton ... opens no window)" | :20-22 | REAL | apps/desktop/README.md + docs/adr/ADR-004-os-webview.md exist; desktop's windowless skeleton posture matches the baseline's desktop build. |
| 3 | "lumen --version → lumen 0.1.0" | :27-29 | FALSE (stale example) | apps/cli/Cargo.toml:34 `version = "0.1.3"`; product.toml:27 `display_version = "0.1.3"` — current output would be "lumen 0.1.3". Doc verified at 2026-11-10/commit 559b897 when 0.1.0 was true; today it misleads a new user. |
| 4 | Full verb set (open/close/play/pause/toggle/stop/seek/info/time/duration/tracks/engine-info/commands/setting/decode-bench/advance/help/quit) | :63-67 | REAL | apps/cli/src/help.rs:41-88 carries every verb incl. "seek 90s | +10s | -1m | 50% | chapter 3" (:57), "decode-bench <path>" (:83), "advance [ticks]" (:88). |
| 5 | "the deterministic open→play→pause→seek→stop session is pinned by apps/cli/tests/session_script.rs" | :74-75 | REAL | apps/cli/tests/session_script.rs exists. |
| 6 | decode-bench output line (bytes=425516 v_frames=60 v_decoded=60 a_frames=60 a_decoded=60 wall_s=0.008) | :80-83 | REAL | Baseline audit/runs/baseline/rc-smoke.txt:130 BENCH line is byte-identical incl. wall_s=0.008; fixture tests/fixtures/generated/valid/avi_mjpeg_pcm.avi = 425,516 bytes on disk. |
| 7 | "audio output is the typed null device (ADR-005 ... no real audio backend)" | :70-71 | REAL | docs/adr/ADR-005-audio-output.md exists; engine-info baseline lists pcm/mjpeg only, null-device posture per ADR. |
| 8 | "37 of 73 format rows as of 2026-11-10" (implemented or partial) | :88 | REAL (at date) / 38 today | python3 matrix count: 73 format rows (container 21 + video 16 + audio 19 + subtitle 17), linux implemented 28 + partial 10 = 38 — one row landed after the doc date. |
| 9 | REPL transcript (`lumen open ... ok: accepted session=1 ...`) | :42-59 | REAL-UNPROVEN | Session behavior pinned by session_script.rs; baseline mp4-smoke.md:38+ records the same event/state grammar (Idle→Opening→...→Playing) for the engine; the avi fixture path exists. Not re-run (cargo budget). |
| 10 | UI: "npm ci ... vite dev server on http://localhost:5173"; "Playing media through the UI is not wired to the engine yet" | :102-112 | REAL (wiring) / UNVERIFIED (run) | ui/package.json has dev script (vite); baseline ui-npm-ci.txt passes. The not-wired honesty matches the 4.H1 engine-handoff posture pinned across w26 logs. |
| 11 | Links: settings.md, shortcuts.md, troubleshooting.md, KNOWN-LIMITS.md | :115-119 | REAL | All four files exist (docs/guides/ listing + docs/KNOWN-LIMITS.md). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "lumen 0.1.0" | apps/cli/Cargo.toml:34 = 0.1.3 | Crate wins. Stale example (S4) — but the doc's mechanism sentence ("reports the CLI crate's own version") is correct, which makes the staleness self-explaining. |
| "37 of 73" claimed rows | Matrix now: 38 of 73 | Tree wins; +1 post-date landing. Not fraud — dated count. |

## 4. Fabrication / inflation indicators
None. The guide's honesty section (no audio heard, no video seen, typed
errors, no silent fake success) is exactly the posture the audit baseline
independently confirms (mp4-smoke: v_frames=0 with "no decoder in this
build" honesty; rc-smoke: real headless-decode PASS on the avi fixture).
