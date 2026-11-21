# AUDIT: ui/src/screens/settings/areas/updates-about.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 73 / 2 (AboutPanel, AboutArea) |
| File verdict | REAL (honest, build-injected, well-placed) — with one false justification: the engine-version surface it defers to does not exist anywhere in the UI |
| Reachable from shipped entry point? | YES as web-UI source: `areas/index.tsx:141 {advancedOpen ? <AboutArea /> : null}` — the About fold inside the global settings screen's Advanced disclosure; route /settings/global. Real shell: no GUI binary (W14). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cat src/build-info.ts` + `grep -n "__UI" vite.config.ts` (:33-34 define from package.json — injection real, no literal); `grep -rn "get-engine-info" src/` (callers: ONLY this file's comment, the demo-transport handler, the generated catalog — NO UI surface calls it); `grep -n version package.json Cargo.toml` (ui 0.1.0 "lumen-ui" vs workspace 0.1.3 — drift); `grep -rn "AboutArea" areas/index.tsx` (:141 advanced-gated mount); updates-about.test.tsx count (6, baseline-passed); cross-ref my settings.css#part2 audit F-001 (this file is the cited non-consumer of the dead dl-grammar rules) |

## 1. One-paragraph verdict
A small file done mostly right: the package name/version are build-injected through vite's `define` (verified end-to-end — package.json → vite.config.ts:33-34 → build-info.ts, zero hardcoded literals), the "Check for updates" duplicate row was genuinely moved to Startup, no fake "Check now" button is invented for a catalog command that doesn't exist, and the panel's out-of-frame placement behind the Advanced disclosure is documented with the exact displacement it fixed (141px, y786 vs y645). The defect is the justification for what it does NOT show: the header claims runtime engine/protocol versions "come from system.get-engine-info … on surfaces that have a client" — and no such surface exists; grep finds zero UI callers of `system.get-engine-info`. The engine version (0.1.3) appears nowhere in the UI, while the About panel shows the UI package version (lumen-ui 0.1.0) — three patch releases adrift from the engine users actually run, with a comment pointing at a phantom consumer.

## 2. What actually works (verified)
— verified by vite.config.ts:33-34 + build-info.ts read → `__UI_PACKAGE_NAME__/__UI_PACKAGE_VERSION__` are defined from package.json at config load; the About panel's values cannot drift from the manifest it ships in (the file's core honesty claim holds for the UI side).
— verified by `grep -rn "AboutArea"` → mounted exactly as documented (areas/index.tsx:141, advanced-gated); updates-about.test.tsx (6 tests, baseline-passed) pins the row grammar and testids.
— verified by catalog grep → no update command exists; correspondingly no button is rendered (the honest-surface rule is applied, not just stated).
— cross-verified my settings.css#part2 F-001: this file renders `settings-field__row/__lead/__label/__value` — NOT the `<dl>`/`settings-about__rows` grammar the CSS comment attributes to it; the dead 30-line CSS family's cited consumer is this file, and it doesn't consume it.

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/settings/areas/updates-about.tsx:13-16
Code:
```
//     protocol versions come from system.get-engine-info (doc 05 §2.12) on
//     surfaces that have a client; this static About panel sticks to the
//     build-injected package data.
```
What is wrong: the deferral target does not exist. `rg "get-engine-info"` over ui/src returns exactly three hits: this comment, the DemoTransport's case handler (demo-transport.ts:404), and the generated command catalog — no component, screen, or hook ever calls it. The engine/protocol version is therefore displayed NOWHERE in the UI, while the About panel shows `lumen-ui 0.1.0` against a workspace/engine at `0.1.3` (Cargo.toml:34; the engine self-reports "lumen 0.1.3" per baseline MP4 smoke).
Why it matters: an About panel whose stated design is "REAL values only" shows a version three patch releases behind the running engine and provides no way to learn the engine/protocol version at all — bug reports filed from this panel misidentify the build; the comment launders the gap by citing a consumer that was never written (and per W5/W13, even the engine's own get-engine-info output is hard-coded literals — the whole version-surface chain is hollow).
What correct looks like: an engine-version row fed by `system.get-engine-info` on this very surface (it has a client via the settings provider), or an honest comment: "engine version is not displayed anywhere at HEAD".
Confidence: HIGH — Verification: repo-wide caller grep; version fields read from both manifests; W5/W13 hard-coded-engine-info evidence cited from wave prompt/baseline.
Cross-ref: W13's engine-info audit (hard-coded output) — the UI side of the same hollow chain.

### F-002 [S4]
Where: ui/src/screens/settings/areas/updates-about.tsx:24-26
Code:
```
// w26.gfix-B (g10-P2-3): the About rows render in the settings ROW grammar
// (label left, value right in the control column) — the legacy stacked
// .settings-field__head grammar is gone.
```
What is wrong: accurate for THIS file, but it is the exact sentence that falsifies settings.css:687-688's claim of a "<dl> wrapper (w26.d2 a11y restructure, updates-about.tsx)" — the restructure's CSS landed (30 lines, settings.css:689-716) and its consumer never did. Two files tell opposite stories about the same grammar; this one is right, the CSS is dead.
Why it matters: confirms settings.css#part2 F-001 from the consumer side; the cleanup should delete the CSS family, not add the dl here.
Confidence: HIGH — Verification: this file's JSX read (no dl/dt/dd anywhere).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AboutPanel` | 34-59 | REAL | S3 | two rows (package, version) from build-info; data-about-* test hooks; F-001's phantom deferral in the header comment |
| `AboutArea` | 63-73 | REAL | — | single section wrapper; advanced-gated mount verified |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| updates-about.test.tsx (6, baseline-passed) | YES | row grammar, testids, value rendering asserted (incl. `.settings-about__value` presence — which is more than the dead CSS family ever gets) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Version data: REAL values only — injected at build time … never a hardcoded literal" | updates-about.tsx:10-13 | REAL | vite define chain verified |
| "Runtime engine/protocol versions come from system.get-engine-info … on surfaces that have a client" | updates-about.tsx:13-15 | FALSE | zero UI callers of get-engine-info (F-001) |
| "No 'check now' action exists in the command catalog … no button is rendered" | updates-about.tsx:21-22 | REAL | catalog grep + JSX read |
| settings.css:687 "The <dl> wrapper (… updates-about.tsx)" | cross-file | FALSE | this file has no dl (F-002; confirms settings.css#part2 F-001) |

## 7. Dead / duplicated / unreachable code
None in-file. (The dead code is on the CSS side — settings.css:689-716.)

## 8. Dependencies & flags
build-info.ts (asset-group g003), i18n messages. No flags.

## 9. Open questions for the next agent
- Does any doc/matrix row claim the UI displays engine or protocol versions (doc 05 §2.12 surfaces)? If yes, that row is FALSE-at-HEAD — docs wave should reconcile with F-001.
- ui/package.json 0.1.0 vs workspace 0.1.3: is there a release process note pinning these together? If the numbers are meant to move independently, the About panel should label its value "UI package" explicitly (it does have the label "ui-package" key — check the en-US string says so; if the label reads just "Version", the drift is user-invisible mislabeling).
