# CLAIM AUDIT: docs/web/browser-matrix-20261110.json (652 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 6 |
| REAL 4 / REAL-UNPROVEN 2 / FALSE 0 / PAPER-ONLY 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A genuinely honest evidence artifact. It records per-scenario FAILURES, not only passes:
7 of 17 scenarios fail on chromium in BOTH isolation modes (`open-play-pause-stop`,
`play-to-end`, `media-info-and-tracks`, `seek-not-served`, `event-envelope-invariants`,
`idempotent-pause-echo`, `autoplay-gate-parks-and-resumes` — python3 json walk below), and
4 of 5 browser lanes are marked `not-runnable-in-sandbox` with zero modes rather than
invented results.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | schema `lumen.web.browser-matrix/1`; git `3755dc35...` branch w24/a `dirty: true` | json head | REAL | python3 json.tool -> VALID; dict keys schema/generated/git/capture/tooling/artifact/fixture/lanes; git.dirty=True disclosed. Commit hash UNVERIFIED (no git in sandbox). |
| 2 | chromium lane `evidence: "playwright-run"`, 2 modes, 17 scenarios each | lanes[0] | REAL | python3: lanes=5, chromium has modes isolation-off + isolation-coop-coep, each `suite.scenarios` len 17, with wasmResponses showing 13x `status: 200, contentType: application/wasm, contentLength: 1294311`. |
| 3 | Scenario statuses: 10 pass / 7 fail per mode (core playback fails) | lanes[0].modes[*].suite | REAL (honest failure recording) | python3 census (both modes identical): fail = open-play-pause-stop, play-to-end, media-info-and-tracks, seek-not-served, event-envelope-invariants, idempotent-pause-echo, autoplay-gate-parks-and-resumes (7/17); pass = boot-handshake, renderer-ladder-consistency, open-rtsp/file/garbage refused, open-http-pending-lane, unserved-config-command, fetch-range both, audio-context-and-worklet (10/17). |
| 4 | firefox/webkit/safari/safari-ios lanes `evidence: "not-runnable-in-sandbox"` | lanes[1-4] | REAL (honest environment limit) | python3: 4 lanes with 0 modes each, evidence string exact. Tooling records only a chromium executable (`chromiumExecutable: /home/z/.agent-browser/browsers/chrome-...`). |
| 5 | artifact `lumen_web.wasm`, 1,294,311 bytes, sha256 4a0f... | artifact | REAL-UNPROVEN | Bytes self-consistent with every wasmResponses contentLength (1,294,311). Artifact not on disk (`ls target/wasm32-unknown-unknown/release/lumen_web.wasm` -> not committed), sha256 not recomputable — UNVERIFIED at hash level. |
| 6 | Consumers: doc 09 web matrix + phase7-exit gate + w24 reviews | whole file | REAL | `rg -ln 'browser-matrix-20261110'` -> docs/09-web-wasm-target.md, docs/gates/phase7-exit.md, docs/reviews/w24-{vw-fixes,integration,s-chain,a-web,a-newcommits}-findings.md, ui/e2e/web/matrix.config.json. |

## 3. Contradictions
None within the file. Cross-check note for later auditors: docs/gates/phase7-exit.md cites
this matrix — verify it does NOT claim web playback works; the matrix itself records
`open-play-pause-stop: fail` in both chromium modes.

## 4. Fabrication / inflation indicators
None. Opposite: the artifact self-incriminates (7 failing scenarios incl. the whole
open/play/pause/stop loop, dirty git tree, 4 unrunnable lanes). This is what evidence
files should look like.
