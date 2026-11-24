# CLAIM AUDIT: docs/feature-matrix.toml rows-248-252 (hotkey category, tail)
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 5 rows (hotkey-navigation, hotkey-nav-disc, hotkey-snapshot-record, hotkey-interface, hotkey-global) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 0 / 0 / 3 (rows 248/250/251 partial = data+editor only) / 0 / 1 (row 252 honest not_started) / 1 FALSE evidence element (row 251 "quit wired") / 1 (row 248 engine helper module) |
| Internal contradictions found | 2 (row 251 evidence vs disabled quit menu; row 248 "engine chapter helpers live" vs zero callers) |

## 1. Blunt verdict
The `partial` statuses here are deflated-but-dirty: the working subset each note admits is
"seed + editor live" (keymap DATA rendered by ShortcutsScreen/HotkeysEditor — real, verified
files), while every ENGINE half cited as evidence is dead on arrival: chapter helpers have
zero non-test callers, take-screenshot is a catalog entry the CLI host typed-refuses, and the
one hard lie is row 251's evidence string "quit wired in ui/src/components/appshell/AppShell.tsx"
— that file contains no quit wiring at all and the quit menu item ships `disabled: true`.
Parity math agrees nothing here counts (hotkey 0/17 on all platforms, baseline
xtask-parity-report.txt:10), so the inflation is prose-level, not metric-level.

## 2. Row ledger
| Row | id | Statuses claimed | Evidence strings | Code actually found (rg) | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 248 | hotkey-navigation | partial w/l/m, not_started web | keymap recipes; engine chapters.rs; ShortcutsScreen + HotkeysEditor | keymaps/default.toml:122-125 bindings real (chapter-prev Shift+P / next Shift+N / title Shift+O,B); chapters.rs pure fns chapter_at/next/previous/chapter_target (chapters.rs:58-108) — NO non-test caller in repo (only lib.rs:110 re-export); CLI host refuses chapters (host.rs:1068-1085 "no chapter source exists anywhere in the vertical"); editors exist | No run (UI untested here; baseline ui-test passes editors' own tests) | SHALLOW — partial honest for data/editor; engine citation is UNREACHABLE code |
| 249 | hotkey-nav-disc | blocked w/l/m, unsupported web | (empty — required for blocked) | disc-menu Shift+M binding feature-gated (keymap:128); platform-disc-dvd row exists (matrix:2524); no disc nav code | No | consistent planning row; no false claim found |
| 250 | hotkey-snapshot-record | partial w/l/m, not_started web | keymap bindings; "engine plan snapshot.rs"; editors | snapshot Shift+S→video.take-screenshot + record Shift+R→record-toggle real (keymap:161-162); snapshot.rs is a PATH PLANNER only (`pub fn plan(...) -> Option<SnapshotPlan>`, no pixel capture); video.take-screenshot handled nowhere: catalog command.rs:243-244 only, CLI dispatch.rs:154-155 `other => Err(unserved(other))`; PlayerScreen.tsx:343 "No record command in the v1 catalog" | No | SHALLOW — plan+catalog-only, no capture path exists |
| 251 | hotkey-interface | partial w/l/m, not_started web | keymap; "quit wired in ui/src/components/appshell/AppShell.tsx"; editors | keymap quit Ctrl+Q ui_action quit real (keymap:57); show-controls-fullscreen I real + handled (PlayerScreen.tsx:844); **AppShell.tsx contains ZERO quit handling** (`rg -i 'quit' AppShell.tsx` → 0 matches); quit menu ships disabled: PlayerScreen.tsx:481-488 "no quit seam exists in the web app — disabled." + `disabled: true,`; no dispatcher for uiAction "quit" anywhere in ui/src | No | evidence element FALSE — "quit live" claim contradicted by code |
| 252 | hotkey-global | all not_started | (empty) | no global-hotkey code in crates/apps (rg GlobalHotkey/global_key → none) | No | PAPER-ONLY, honest |

## 3. Findings
F-001 [S1] — Row 251 evidence cites wiring that does not exist.
Where: docs/feature-matrix.toml:3392-3396 vs ui/src/components/appshell/AppShell.tsx (no quit anywhere) vs ui/src/screens/player/PlayerScreen.tsx:481-489.
Code (matrix): `quit wired in ui/src/components/appshell/AppShell.tsx`
Code (PlayerScreen.tsx:481-488): `// Window-level close is host-owned (the MiniPlayer close note) — no` / `// quit seam exists in the web app — disabled.` / `id: "quit",` … `accelerator: acceleratorOf("media.quit"),` / `disabled: true,`
What is wrong: the evidence string is a fabricated wiring claim; the shipped UI deliberately disables quit. Why it matters: evidence strings are the anti-hallucination substrate (R4); a false one here poisons the row's "partial" credibility and the backfill-issue trail. What correct looks like: evidence citing the disabled menu item, or dropping "quit wired". Confidence: HIGH — Verification: ran `rg -i 'quit' AppShell.tsx` (exit 1, 0 matches) + read PlayerScreen.tsx.
F-002 [S2] — Row 248's "engine chapter helpers live" are unreachable.
Where: crates/playback-engine/src/chapters.rs:58-108; only reference is lib.rs:110 re-export; apps/cli/src/host.rs:1068-1073: "the host's typed refusal: no chapter source exists anywhere in the vertical (seek.rs: 'hosts resolve chapters to...'".
What is wrong: live code with zero shipped callers is cited as row evidence. Why it matters: "partial" counts on evidence that nothing reaches. Confidence: HIGH.
F-003 [S2] — Row 250's snapshot capability does not exist end-to-end.
Where: apps/cli/src/dispatch.rs:154-155 `// ---- everything else: the honest typed refusal ----` / `other => Err(unserved(other)),` — video.take-screenshot falls here; snapshot.rs:131 `pub fn plan(&self, explicit_path: Option<&Path>) -> Option<SnapshotPlan>` (filename planning only).
What is wrong: pressing Shift+S sends a command the only shipped host refuses. Row note admits the remainder; evidence wording ("engine plan") is technically honest — the row is judged SHALLOW, not FALSE. Confidence: HIGH.
F-004 [S3] — control-server stub dispatch.
Where: apps/control-server/src/dispatch.rs:55-63: only SystemGetEngineInfo and SystemListCommands succeed; every other command → `ErrorCode::NotReady`. Scope note: affects any web-UI → engine command path. Confidence: HIGH (read).

## 4. Contradictions
| Matrix says | Code says | True |
|---|---|---|
| 251: "quit wired in AppShell.tsx" | AppShell.tsx: no quit; PlayerScreen quit menu `disabled: true` | code — evidence FALSE |
| 248: "engine chapter helpers live" | zero callers; host typed refusal | helpers exist but unreachable |
| Header (L73): "`evidence` is "" everywhere at Phase 0 exit" | rows 248-252 carry non-empty evidence while claiming only partial | header is about verified-status rows; partial rows carrying evidence strings is schema-legal (R4 gates verified, not partial) — no violation |

## 5. Fabrication / inflation indicators
One fabricated citation (F-001). No metric inflation: partial does not enter parity counts;
baseline parity-report hotkey = 0/17 everywhere. The backfill narrative ("issue #20") frames
each row's remainder honestly in `notes` — the prose overstates only the quit element.
