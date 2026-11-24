# CLAIM AUDIT: docs/feature-matrix.toml rows 209-221 (ui-dialog tail + menu-item head)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 13 rows (10 ui-dialog: rows 209-218; 3 menu-item: rows 219-221) |
| REAL 1 / ACCURATE-absence 8 / SHALLOW (inflated status) 2 / PARTIAL-defensible-with-defects 2 | (row verdicts below) |
| Internal contradictions found | 3 (rows 210, 220, 221 vs shipped-host reality) |

## 1. Blunt verdict
Ten of thirteen rows are honest: the eight not_started/unsupported rows describe absent code
that is genuinely absent, and row 209 (goto-time) is real end to end — dialog, Ctrl+T wiring,
`playback.seek` which IS in the shipped host's SERVED table. The three menu rows are where
this chunk rots. Row 210 (bookmarks) claims "implemented" w/l/m with notes "create/jump/delete
dialog live over the engine BookmarkList" — but `media.get/add/remove-bookmarks` are NOT in
the CLI host's 15-command SERVED table (`apps/cli/src/dispatch.rs:38-57`), so every shipped
host answers them with the typed refusal `unsupported-for-host` (:155 `other =>
Err(unserved(other))`, :170 "`{id}` is not served by this build's command-line host");
`BookmarkList` (crates/playback-engine/src/bookmarks.rs) has ZERO callers in the tree; the
ONLY transport that serves the commands is the browser-fallback DemoTransport
(ui/src/client/demo-transport.ts:225,607). Only the jump half (actions.seek) works. Row 220
(convert/quit) claims "quit wired"/"Quit live in the menubar" — the sole quit menu item is
`disabled: true` (PlayerScreen.tsx:483-487, comment: "no quit seam exists in the web app —
disabled"), `menu.file.exit` is a DEAD i18n key (w26.e14:43 dead-key list), the Ctrl+Q
`ui_action = "quit"` has no handler (PlayerScreen handles only goto-time/bookmarks/
av-effects/advanced-controls, :793-816), and desktop-tauri registers no native menubar.
Nothing user-visible in row 220 works. Row 221's transport menu is real (PlayerScreen.tsx
canvas menu) but play/stop are the only SERVED halves; prev/next/shuffle/repeat send
`playlist.*` commands that are catalog ids (ids.rs:140-147) absent from SERVED — refused in
every shipped host. The evidence strings for rows 219-221 also mis-attribute the menus to
`AppShell.tsx`/`MenuBar.tsx`; the menus actually live in PlayerScreen.tsx's canvas
MenuPopup (:1139-1149). The parity tool agrees with reality, not the matrix: menu-item
category = 0/16 on all platforms (baseline xtask-parity-report.txt:27).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | row 209 status implemented w/l/m; "GotoTimeDialog.tsx + parseTimeInput.ts (+ .test.tsx/.ts each); Ctrl+T via DIALOG_KEYMAP" | :2876-2886 | REAL | All evidence files exist (ls: GotoTimeDialog.tsx/.test.tsx, parseTimeInput.ts/.test.ts, keymap.ts); Ctrl+T wired: keymap.ts:87 `jump-to-time = { key = "Ctrl+T", ui_action = "goto-time", ... when = "has-media" }`; PlayerScreen.tsx:793-795 `if (binding.uiAction === "goto-time" && hasMedia) { ... setGotoOpen(true) }` (sed-verified); dialog submit = `actions.seek(seconds)` (GotoTimeDialog.tsx:84) and `PlaybackSeek` IS in SERVED (apps/cli/src/dispatch.rs:45); UI tests pass in baseline ui-test.txt (1705 passed per spec §13). |
| 2 | row 210 status implemented w/l/m; evidence "engine list crates/playback-engine/src/bookmarks.rs" | :2888-2898 | SHALLOW (UI-only) / notes FALSE at runtime | Dialog + tests exist; Ctrl+B wired (keymap.ts:88; PlayerScreen:796-800 opens dialog). BUT: `media.get/add/remove-bookmark` commands ids exist (crates/commands/src/catalog/ids.rs:156-158) yet are ABSENT from SERVED (dispatch.rs:38-57, 15 ids) → `route()` `other => Err(unserved(other))` (:155) with message "`{id}` is not served by this build's command-line host" (:170); `rg -l BookmarkList crates/ apps/` -> bookmarks.rs ONLY (orphan); the wire commands are served only by ui/src/client/demo-transport.ts (:225, :607) — the browser-fallback simulated engine (app-transport.ts: "friendly DemoTransport simulation ... Nothing silently pretends"). Dialog honestly shows load-failed error (BookmarksDialog.tsx:92-97) — but the feature the row describes (create/list/delete) cannot work in any shipped host. |
| 3 | row 211 status not_started x4; evidence "" | :2900-2910 | ACCURATE (feature absent; staged UI exists) | AddSubtitlesDialog.tsx EXISTS (325 lines + full subtitles/ suite: FilenameMatchControl, MatchedPairsList, matching.ts, tests; consumed by PlaylistPanel/PlaylistScreen). Its own header discloses: "no shipped host can produce candidates or commit a selection yet ... without `onSave` Save renders honest-disabled" (sed :25-31). Feature-level not_started is defensible: nothing works; staging is disclosed in-code. |
| 4 | row 212 status not_started x4; evidence "" | :2912-2922 | ACCURATE (feature absent; gated screen exists) | ConvertScreen.tsx EXISTS (406 lines + convertQueue 191 + convertTargets 64 + tests). Header: "the queue lane is now DISABLED until that seam exists — the primary action renders disabled-from-data" and "the catalog carries NO convert.* / record.* commands (verified against crates/commands)" (sed :28-34). rg over apps/cli/src/dispatch.rs SERVED: no convert/record ids — consistent. |
| 5 | row 213 status not_started x4 | :2924-2934 | ACCURATE | `rg -il "StreamOutput|stream-output wizard" ui/src` -> 0 components; no sout UI. |
| 6 | row 214 status not_started x4 (Messages/log window) | :2936-2946 | ACCURATE | `rg -il "MessagesDialog|log viewer|verbosity" ui/src` -> no log-viewer component (only settings rows for log level in settings areas). |
| 7 | row 215 status not_started x4 (Plugins manager) | :2948-2958 | ACCURATE | No plugin-manager UI (`rg -il "PluginsDialog" ui/src` -> 0); corroborated by doc 06 [LEAD-ATTENTION]: "no `PluginRegistry` type exists ... `plugins/builtin/` does not exist" (doc 06 :381-386, verified in my doc-06 audit). |
| 8 | row 216 status not_started x4 (About/Update) | :2960-2970 | ACCURATE | No About dialog component (`rg -il "AboutDialog|about-update" ui/src` -> only i18n keys; e14 dead-key list includes "menu.help.about"). |
| 9 | row 217 status unsupported x4 + blocked_reason "By design: we are a player, not a streaming server" | :2972-2983 | ACCURATE | No VLM code anywhere (`rg -il "vlm|broadcast.*vod" ui/src crates/ apps/` -> nothing user-facing); blocked_reason present per schema rule. |
| 10 | row 218 status not_started x4 (EPG) | :2984-2995 | ACCURATE | No EPG UI (`rg -il "ProgramGuide|EPG" ui/src` -> 0). |
| 11 | row 219 status partial w/l/m; evidence "AppShell.tsx File menu (menu.file.open-media → OpenMediaDialog.tsx; ... open-network-stream → NetworkStreamScreen.tsx) over MenuBar.tsx" | :3002-3012 | ACCURATE status; IMPRECISE evidence (S4) | The menu items live in PlayerScreen.tsx canvas menu (:463-477: "open-media"/"open-network-stream" with onSelect → `lumen:open-media` event / navigate("/open-network-stream")); AppShell owns the listener (:311 "ONE listener for the whole shell") + the OpenMediaDialog mount (:851) — the functional chain is real. Local-file open is proven by baseline rc-smoke (headless-decode PASS). Notes disclose disc/capture/clipboard/Recent absent — consistent (canvas menu record item `disabled: true`, :343-351). Renderer is MenuPopup (:1139-1149), not MenuBar.tsx (MenuBar appears unwired in production — matches w26.e14 dead-key findings for menu.file.* keys). |
| 12 | row 220 status partial w/l/m; evidence "menu.file.exit (quit wired)" + notes "Quit live in the menubar" | :3014-3024 | FALSE (quit half) — nothing user-visible works | PlayerScreen.tsx:482-487: quit item `disabled: true` with comment "no quit seam exists in the web app — disabled." (sed-verified). `menu.file.exit` is a DEAD i18n key (docs/waves/w26/findings/w26.e14.md:43 dead-key list includes "menu.file.exit"). Ctrl+Q binding exists (keymaps/default.toml:57 `ui_action = "quit"`) with NO handler (`rg 'uiAction === "quit"' ui/src` -> 0; PlayerScreen:793-816 handles only goto-time/bookmarks/av-effects/advanced-controls). desktop-tauri registers only lumen_dispatch (app.rs:293 `.invoke_handler(tauri::generate_handler![lumen_dispatch])`), no native menubar (`rg -n 'Menu' apps/desktop-tauri/src/app.rs` -> 0). Convert/Stream absent = accurate. Net: the row's working substance is zero; "partial" + "quit wired" is stale or fabricated. |
| 13 | row 221 status partial w/l/m; evidence "AppShell.tsx Playback menu (menu.playback.play-pause/stop/previous/next/shuffle + repeat off/all/one submenu; engine-reported shuffle/repeat checkboxes)" | :3026-3036 | PARTIAL defensible; evidence mis-attributed; half the items unserved | The transport menu is real but lives in PlayerScreen.tsx canvas menu: play (onSelect actions.togglePlayPause :309), stop (:316), previous/next (:328-339, disabled when queue empty), shuffle checkbox (:373-378 `onSelect: playlist.setShuffle`), repeat off/all/one radio (:381-388 `playlist.setRepeat`). Play/pause/stop ride SERVED commands; prev/next/shuffle/repeat send `playlist.next/prev/set-shuffle/set-repeat` (ids.rs:140-147) which are NOT in SERVED (dispatch.rs:38-57) → typed refusal in every shipped host (same `unserved` arm). Notes disclose Speed-submenu absence — accurate (no speed submenu in the menu; rate rides hotkeys). "engine-reported shuffle/repeat checkboxes": the checkbox state is engine-truth-gated (PlayerScreen:304 `const repeat = playlist.repeat`, comment "engine truth only — null checks nothing") but in a shipped host the engine cannot report playlist state (no served commands) — the honest-null face. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Row 210 notes: "create/jump/delete dialog live over the engine BookmarkList" | apps/cli/src/dispatch.rs:38-57 SERVED lacks media.*-bookmark; :155 unserved arm; BookmarkList has zero callers | The host table wins: only jump (playback.seek) is live; create/list/delete are typed refusals in every shipped host. "implemented w/l/m" overstates; SHALLOW. |
| Row 220 evidence: "menu.file.exit (quit wired)"; notes "Quit live in the menubar" | PlayerScreen.tsx:483-487 `disabled: true`; w26.e14:43 dead-key list; no uiAction quit handler; no native menubar in desktop-tauri | The code wins: quit is not wired anywhere. |
| Rows 219-221 evidence: menus "AppShell.tsx ... over MenuBar.tsx" | PlayerScreen.tsx:463-487 canvas menu rendered via MenuPopup (:1139) | The menu content is real; the evidence's file attribution is wrong (AppShell owns dialog mounts/listeners, PlayerScreen owns the menus). |

## 4. Fabrication / inflation indicators
- Row 210: "implemented" on 3 platforms for a feature whose wire commands every shipped host
  refuses; notes assert "live" for paths that cannot execute outside the demo transport.
  The bookmark jump is real; the row as written inflates 3 of its 4 verbs.
- Row 220: "partial" status + "quit wired"/"Quit live" with zero working substance found —
  the strongest inflation candidate in this chunk.
- Rows 211/212: not_started despite substantial staged code — the CONSERVATIVE direction
  (under-claiming), disclosed in-code by the staged components themselves. Not inflation.
- Parity tool corroboration: menu-item 0/16 and ui-dialog 4/15 on every platform
  (xtask-parity-report.txt:27-28) — the strict definition already refuses to credit these
  rows. Feature-audit R4 check: no missing evidence paths in rows 209-221 (baseline
  feature-audit's 2 violations are rows 23 and 163, outside this chunk).

## 5. Matrix rows (row id, platform statuses claimed, evidence strings, code actually found, executed?, verdict)

| Row | id | Statuses claimed (w/l/m/web) | Evidence (verbatim key parts) | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 209 | ui-dialog-goto-time | impl/impl/impl/ns (2884) | "GotoTimeDialog.tsx + parseTimeInput.ts (+ .test.tsx/.ts each); Ctrl+T via DIALOG_KEYMAP" (2885) | Dialog + tests + keymap + PlayerScreen uiAction handler (793) + actions.seek → PlaybackSeek (SERVED) | Not executed by me; UI test baseline 1705 pass; rc-smoke command-layer-play PASS (baseline rc-smoke.txt) | REAL (code-complete claim accurate) |
| 210 | ui-dialog-bookmarks | impl/impl/impl/ns (2896) | "BookmarksDialog.tsx (+ .test.tsx); engine list crates/playback-engine/src/bookmarks.rs; Ctrl+B" (2897) | Dialog real; BookmarkList ORPHAN (rg: zero callers); media.*-bookmark ids exist but UNSERVED (dispatch.rs:38-57,155,170); served only by DemoTransport | Not executed; host table inspected directly | SHALLOW — UI real, feature non-functional in shipped hosts; jump-only works |
| 211 | ui-dialog-subtitle-add | ns/ns/ns/ns (2908) | "" (2909) | AddSubtitlesDialog.tsx (325 L) staged, host-less by its own header; Save honest-disabled | Nothing to execute | ACCURATE absence (staging disclosed in-code) |
| 212 | ui-dialog-convert | ns/ns/ns/ns (2920) | "" (2921) | ConvertScreen.tsx (406 L) gated; "no capability shown that is not implemented" honored (disabled-from-data) | Nothing | ACCURATE absence (gated staging) |
| 213 | ui-dialog-stream-output | ns/ns/ns/ns (2932) | "" (2933) | None | Nothing | ACCURATE |
| 214 | ui-dialog-messages | ns/ns/ns/ns (2944) | "" (2945) | None | Nothing | ACCURATE |
| 215 | ui-dialog-plugins | ns/ns/ns/ns (2956) | "" (2957) | None (consistent with doc 06: no registry) | Nothing | ACCURATE |
| 216 | ui-dialog-about-update | ns/ns/ns/ns (2968) | "" (2969) | None (dead i18n keys only) | Nothing | ACCURATE |
| 217 | ui-dialog-vlm | unsup x4 (2980) | "" + blocked_reason (2982) | None; by-design exclusion | Nothing | ACCURATE (blocked_reason present) |
| 218 | ui-dialog-program-guide | ns/ns/ns/ns (2993) | "" (2994) | None | Nothing | ACCURATE |
| 219 | menu-media-open | partial/partial/partial/ns (3010) | "AppShell.tsx File menu (menu.file.open-media → OpenMediaDialog.tsx; ... NetworkStreamScreen.tsx) over MenuBar.tsx" (3011) | Chain real: PlayerScreen canvas items (463-477) → AppShell listener (311) → OpenMediaDialog mount (851); local-open proven by rc-smoke | rc-smoke headless-decode PASS (baseline) | Status ACCURATE; evidence file attribution imprecise (S4) |
| 220 | menu-media-convert-quit | partial/partial/partial/ns (3022) | "menu.file.exit (quit wired); Convert/Stream items absent" (3023) | Quit item DISABLED (PlayerScreen 483-487); menu.file.exit dead key (e14:43); no quit uiAction handler; no native menubar (app.rs:293); Convert/Stream absent (true) | Nothing works to execute | Status INFLATED — quit-wired claim FALSE; row substance = zero |
| 221 | menu-playback-transport | partial/partial/partial/ns (3034) | "AppShell.tsx Playback menu (... play-pause/stop/previous/next/shuffle + repeat off/all/one submenu; engine-reported shuffle/repeat checkboxes)" (3035) | Menu real in PlayerScreen canvas (300-388): play/stop SERVED; prev/next/shuffle/repeat → playlist.* UNSERVED (ids.rs:140-147 vs SERVED); Speed submenu absent (disclosed) | Play/stop executed in rc-smoke; playlist.* not executed (unserved) | Status PARTIAL-defensible; evidence mis-attributed; 5 of 8 verbs unserved |

Notes: ns = not_started; impl = implemented; unsup = unsupported. Chunk = lines 2876-3037, 13 rows exactly (209th [[feature]] marker at :2876, 222nd at :3038; `rg -n '^\[\[feature\]\]' | sed -n '209p;222p'`).

Findings register:
- F-D001 [S2] Row 210 "implemented"/"live" inflated: media.*-bookmark unserved by every shipped host; BookmarkList orphan; only DemoTransport serves.
- F-D002 [S2] Row 220 "quit wired"/"Quit live in the menubar" FALSE: sole quit item disabled, dead key, no handler, no native menubar; row's partial status has no working substance.
- F-D003 [S3] Row 221: prev/next/shuffle/repeat menu verbs send playlist.* commands unserved by every shipped host; evidence file attribution wrong (AppShell vs PlayerScreen).
- F-D004 [S4] Rows 219-221 evidence strings cite AppShell.tsx/MenuBar.tsx; actual menu surface is PlayerScreen.tsx canvas MenuPopup; MenuBar.tsx appears production-unwired.
- F-D005 [S4] Rows 211/212 carry not_started despite staged code — conservative, disclosed in-code; record-keeping could note the staged surfaces.
