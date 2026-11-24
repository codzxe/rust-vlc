# AUDIT: ui/src/screens/player/customize/panels.ts
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 227 / 6 (2 tile factories, catalog const, 3 lookup/arg helpers) |
| File verdict | REAL data catalog with a FALSE header: claims capability-gating that was retired (w26.k08); 3 of its 31 tiles name commands absent from the generated wire union — guaranteed dead controls |
| Reachable from shipped entry point? | YES — CustomizePanels.tsx (39 tests baseline PASS) and toolbarItems.tsx consume CUSTOMIZE_TILES/tileById/cycleArgs; customize surface opens from the player menu (`open-panel:customize`, layout.ts DEFAULT menu) and ControlBar. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | rg "video.record"/"video.toggle-pip"/"video.set-tone-mapping" in generated/commands.ts (all exit 1 = ABSENT; take-screenshot present); read capabilities.ts header (policy retirement); rg useCommandCapabilities consumers (none in customize surface, comment-only in convertQueue.ts); baseline ui-test.txt:30 CustomizePanels 39 tests PASS |

## 1. One-paragraph verdict
Pure-data tile catalog in frame order — but its scope contract is a lie relative to current policy. The header (L6-8) says engine-bound tiles are "capability-gated in capabilities.ts — hidden until the engine lists the command, per doc 05 §3 'no dead controls'". capabilities.ts:5-12 states that gating was RETIRED in w26.k08: the box renders the FULL 31-tile catalog against any listing, and `useCommandCapabilities` now has "NO product consumer inside the customize surface anymore". Consequence: the tiles `record` (command "video.record"), `pip` ("video.toggle-pip") and `hdr` ("video.set-tone-mapping") — self-labeled "forward-declared" — name commands that do NOT exist in the generated 1.D4 wire union (rg-verified absent; `video.take-screenshot`/`video.set-zoom`/`playlist.set-repeat` etc. are present). Users can place them in the toolbar; every click dispatches a command no engine build can know, and the dispatch seam in toolbarItems.tsx:113 papers over this with `as unknown as CommandRequest`. Dead controls shipped, doc 05 §3 violated, header false.

## 2. What actually works (verified)
— verified by rg: 10 engine-bound tiles reference REAL wire commands (playback.set-rate, playback.seek, playback.stop, playlist.set-repeat, playlist.set-shuffle, video.take-screenshot, video.set-zoom) — all present in the generated CommandRequest union (commands.ts:24).
— verified by baseline: CustomizePanels.test.tsx 39 tests PASS — catalog order, cycle args, and the (now-retired) capability hook are pinned.
— cycleArgs (L215-227): correct default-anchored cycling; tileById never guesses (null for unknown).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/customize/panels.ts:6-8 (with capabilities.ts:5-12; panels.ts:130-161)
Code:
```
//   * engine-bound tiles name the command they dispatch (capability-gated
//     in capabilities.ts — hidden until the engine lists the command, per
//     doc 05 §3 "no dead controls");
```
and the tiles it claims are gated:
```
    // Forward-declared (no PiP command in the 1.D4 wire union yet).
    id: "pip",
    ...
    command: "video.toggle-pip",
```
What is wrong: The gating is gone. capabilities.ts:5-12 (read): "the customize box renders the frame's FULL 31-tile catalog against ANY `system.list-commands` listing … engine truth governs DISPATCH only" and the listing hook has no product consumer in this surface. So the three forward-declared tiles (record/pip/hdr) render unconditionally, can be placed in the saved toolbar, and dispatch commands ABSENT from the wire schema (rg exit 1 for all three in generated/commands.ts). Every activation is a guaranteed engine refusal — the exact "dead control" doc 05 §3 forbids, now shipped by policy change while this header still promises the opposite. Why it matters: a user customizes their bar, clicks Record, nothing happens (error envelope at best); the UI silently advertised a capability no build has. What correct looks like: either restore per-tile gating on the command listing (the engine can never list these three — they'd disappear) or type `command` as the wire union so the catalog cannot drift. Confidence: HIGH — Verification: ran rg (absent), read both headers, read dispatch site.
F-002 [S3]
Where: ui/src/screens/player/customize/panels.ts:92 (type root) and ./types.ts:51,92
Code:
```
export type CommandArgs = Readonly<Record<string, unknown>>;
...
  readonly command: string | null;
```
What is wrong: The catalog types the command as a bare string and args as an untyped record, deliberately severing the generated wire union. That is what allows F-001's drift to compile cleanly, and it forces the consumer's double-cast `as unknown as CommandRequest` (toolbarItems.tsx:113) — the dispatch seam has NO compile-time protection against sending garbage to the engine. The catalog-vs-wire drift has already happened three times. What correct looks like: `command: CommandRequest["cmd"]` (the generated union) with a separate forward-declared escape hatch that is visibly gated. Confidence: HIGH — Verification: read types.ts:46-92 + toolbarItems.tsx:107-115.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| uiTile | 61-67 | REAL | — | UI-owned, never dispatches |
| seekTile | 69-82 | REAL | — | relative-seek ms args; real wire command |
| CUSTOMIZE_TILES | 89-187 | REAL | S3 | 31 tiles, frame order; 3 forward-declared dead commands (F-001) |
| CUSTOMIZE_TILE_IDS / TILE_BY_ID | 189-196 | REAL | — | |
| tileById | 199-201 | REAL | — | null for unknown |
| DEFAULT_TOOLBAR_LAYOUT | 209-212 | REAL | — | full order, empty band (frame -1) |
| cycleArgs | 215-227 | REAL | — | default-anchored cycle, modulo wrap |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| CustomizePanels.test.tsx (39, baseline PASS) | PARTIAL | pins catalog order/args and dispatch grammar, but no test asserts catalog commands exist in the wire union — the drift F-001 documents passes all 39 tests silently |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "engine-bound tiles … capability-gated … hidden until the engine lists the command" | panels.ts:6-8 | FALSE | capabilities.ts:5-12 retires the gating; hook has no consumer |
| "no dead controls" (doc 05 §3 as cited) | panels.ts:8 | FALSE in effect | 3 tiles dispatch commands absent from the wire schema |
| "Forward-declared … no PiP command in the 1.D4 wire union yet" | panels.ts:131,147,155 | TRUE (self-aware) | rg: absent from commands.ts — the comment admits what the header denies |

## 7. Dead / duplicated / unreachable code
No dead code in-file; the record/pip/hdr tiles are live-rendered but functionally dead on dispatch (F-001).

## 8. Dependencies & flags
icon registry names, ./types (loose by design — F-002). No feature flags.

## 9. Open questions for the next agent
- Rust side: what does lumen_dispatch answer for an unlisted command string — typed error envelope or panic? If panic, F-001 escalates to S1 (remotely triggerable via UI click).
