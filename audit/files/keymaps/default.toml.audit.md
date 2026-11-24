# AUDIT: keymaps/default.toml
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2025-06-14 |
| Lines / functions | 281 / 150 bindings in 8 sections + 1 macos override section (135 distinct label-msgids, 36 command-bearing rows) |
| File verdict | SHALLOW (real, provenance-rich data; 7/150 rows consumed by the shipped app; the advertised hotkey surface is 95% unwired) |
| Reachable from shipped entry point? | PARTIAL: 7 bindings reach runtime behavior via hardcoded excerpts (ui/src/screens/player/keymap.ts DEFAULT_KEYMAP_TOML/DIALOG_KEYMAP_TOML + PlayerScreen.tsx:191 CUSTOMIZE_DIALOG_KEYMAP filter of the generated data). The file itself is loaded by NO runtime code — only by the dev-time generator ui/scripts/gen-hotkeys-data.mjs. |
| Findings | S0: 0 S1: 0 S2: 3 S3: 3 S4: 2 |
| Commands run | python3 collision/count scripts over the TOML (outputs in §2); rg traces through ui/src, apps/cli, apps/desktop, crates/commands; sed/od quote verification; baseline ui-test.txt:171 (hotkeys drift check passes). |

## 1. One-paragraph verdict
The hotkey map is a meticulously sourced data file (VLC 3.0.23 clean-room dump + Figma mining, per-row provenance and [VERIFY] ledger) that almost nothing uses. The shipped UI dispatches exactly 7 of its 150 bindings — the runtime "keymap" is three hardcoded TOML excerpts in ui/src/screens/player/keymap.ts (whose header admits "The keymap LOADING pipeline has not landed"). 143 rows, including every VLC-parity essential (Space play/pause, S stop, N/P tracks, arrows/volume, all 8 macOS overrides), are wired to nothing. Worse: 3 frame-step rows target `playback.step-frame`, a command the catalog defines but BOTH shipped hosts refuse (Wave 3's finding, confirmed and extended to the desktop shell), and 18 of the 36 command-bearing rows name commands the CLI answers with `unsupported-for-host`. The HotkeysEditor lets users rebind all 150 rows into a localStorage store no runtime code ever reads.

## 2. What actually works (verified)
— The data pipeline: keymaps/default.toml -> ui/src/screens/settings/areas/hotkeys/generated/bindings.ts is drift-gated (`gen-hotkeys-data.mjs --check` runs inside vitest; baseline audit/runs/baseline/ui-test.txt:171 `✓ src/screens/settings/areas/hotkeys/hotkeysData.test.ts (2 tests)`), and the 7 runtime-dispatched keys (F, Esc, I, Ctrl+T, Ctrl+B, Ctrl+E, Ctrl+A) match this file's rows verbatim (read of keymap.ts:68-94 excerpt strings + PlayerScreen.tsx:191-206).
— python3 counts (ran): 150 bindings, 36 `command = ` rows, 135 distinct label-msgids, 8 same-mode key collisions.

## 3. Findings
F-001 [S2]
Where: keymaps/default.toml:1-281 vs ui/src/screens/player/keymap.ts:3-15,68-94
Code (keymap.ts:5-11 — the consumer that does not exist):
```
// mode = "player-focused", … } }). The keymap LOADING pipeline has not landed
// (doc 05 §8.1 vs MP §4 path mismatch is open question Q4 in
// docs/inventory/ui-inventory.md §9) — [PROPOSAL] adaptation, recorded in
// ui/NEXT-STEPS.md: this module is the full consumption side (parser +
// matcher + shell recipe dispatch), seeded from a verbatim excerpt of the
// three shell-relevant bindings from keymaps/default.toml lines 154-156.
```
What is wrong: 143/150 bindings are wired to nothing. Runtime dispatch = F (fullscreen:154), Esc (exit-fullscreen:155), I (show-controls-fullscreen:156), Ctrl+T (jump-to-time), Ctrl+B (bookmarks-dialog), Ctrl+E (effects-and-filters), Ctrl+A (advanced-controls) = 7 rows. Space/S/N/P/./,/arrows/Up/Down/M/V/B/A/D/Z/W/L/Q/Ctrl+L/Ctrl+P/bookmarks F1-F10 — all dead. The two variant toggles that DO fire at runtime (B miniplayer, G music-mode) come from variantKeymap.ts, not this file. Why it matters: the file presents itself as the default keymap seed for the player; every surface that renders it (ShortcutsScreen, HotkeysEditor, MenuBar accelerators) advertises 150 shortcuts of which 95% do nothing. What correct looks like: the loader card landing, or the UI marking unimplemented rows disabled (bindings.ts has a `disabled` field — always false today).
Confidence: HIGH — Verification: read (rg matchBindings/matchModifierBindings call sites: PlayerScreen.tsx:792/811/821 only; App.tsx passes no onBinding).

F-002 [S2] (Wave 3 finding — confirmed and extended)
Where: keymaps/default.toml:68,94-95 vs crates/commands/src/catalog/spec.rs:240 vs apps/cli/src/dispatch.rs:47-63 vs apps/desktop/src (no "step" anywhere)
Code (default.toml:94):
```
frame-next = { key = ".", label-msgid = "hotkeys.frame-next", command = "playback.step-frame", args = { direction = "next" }, mode = "player-focused", when = "has-media", source = "design-package", note = "Onboarding-2 'Next Frame — .' (design adds a second frame-step key; VLC default is e, superseded)" }
```
and (dispatch.rs:47-63, the SERVED table) ends:
```
    CommandId::ConfigGetSetting,
    CommandId::ConfigSetSetting,
];
```
What is wrong: three rows (`.` frame-next, `,` frame-prev, BrowserNext media-key-frame-next) bind `playback.step-frame`. The catalog defines it (spec.rs:240 `CommandId::PlaybackStepFrame`, class A, target Engine), but the CLI's 15-id SERVED table omits it (typed `unsupported-for-host` refusal), and the desktop shell has no step handling at all (rg "step" in apps/desktop/src -> zero hits). The HotkeysEditor still lists frame-next/frame-prev as bindable actions. What correct looks like: serve the command or mark the rows disabled.
Confidence: HIGH — Verification: read of all four files.

F-003 [S2]
Where: ui/src/screens/settings/areas/hotkeys/overrides.ts:1-4 vs PlayerScreen.tsx:792-821
Code (overrides.ts:1-4):
```
// Hotkey override persistence (card 4.E2) — same shape and honesty rules as
// the settings persistence seam (persistence.ts, card 4.E1): a MOCK
// localStorage round-trip today, the real write path (keymap config over the
// client bridge) is the app layer's handoff.
```
What is wrong: user rebinding is a dead end. The editor persists overrides under `lumen.hotkeys.overrides.v1`; the runtime matchers (PlayerScreen.tsx:792/811/821) read only the hardcoded seeds + generated defaults — no load() of the override store anywhere outside the editor (rg hotkeyOverrides -> HotkeysEditor.tsx only). Apply "commits" bindings nothing will ever dispatch. What correct looks like: the runtime keymap resolving `draft ▸ persisted override ▸ keymap default` (the editor's own documented precedence) at match time.
Confidence: HIGH — Verification: read.

F-004 [S3]
Where: keymaps/default.toml:101-102,134-137,144-145
Code (:134-135 and :144):
```
nav-up = { key = "Up", label-msgid = "hotkeys.nav-up", ui_action = "nav-up", mode = "player-focused", source = "vlc-3.0.23-default", note = "key-nav-up=Up; needs in-disc-menu gating (Q4) — confl
volume-up = { key = "Up", label-msgid = "hotkeys.volume-up", recipe = "volume-up", mode = "player-focused", source = "design-package", note = "C-02: VLC default key-vol-up=Ctrl+Up; design wins per
```
What is wrong: 8 same-mode collisions (python3, ran): Up/Down (nav vs volume), Left/Right (seek vs nav) collide in `mode = "player-focused"` on the SAME platform, and the macOS overrides collide with base rows (F/G/H/J) because `mode` carries no platform dimension. VLC disambiguates by DVD-menu context; this seed just double-binds. Latent today (no consumer), a guaranteed conflict-resolution mess for the loader card. Correct: distinct modes (e.g. menu-nav-focused) or distinct keys.
Confidence: HIGH — Verification: ran collision script (§2).

F-005 [S3]
Where: ui/src/screens/player/variants/variantKeymap.ts:9-11 vs keymaps/default.toml:147,189
Code (variantKeymap.ts:9-11):
```
// and this seed is deleted. Keys avoid every key claimed in
// keymaps/default.toml's player-focused mode (Space/S/N/P/M/E/F/I/Esc and
// the media keys); final keys belong to the keymap data file, not code.
```
vs default.toml:147 `audio-track-cycle = { key = "B", … mode = "player-focused" …}` and :189 `subtitle-delay-down = { key = "G", … mode = "player-focused" …}`.
What is wrong: the claim is false twice over: B and G ARE claimed in player-focused mode, and the parenthetical avoid-list (Space/S/N/P/M/E/F/I/Esc + media keys) is a stale subset — the file claims ~40 player-focused keys (., ,, T, V, A, D, Z, O, W, R, X, Q, L, C, H, J, K, …). At runtime B/G win (variantKeymap is what PlayerScreen matches), so pressing B toggles miniplayer while keymaps/default.toml, ShortcutsScreen and HotkeysEditor all say B cycles the audio track.
Confidence: HIGH — Verification: read + collision script.

F-006 [S3]
Where: keymaps/default.toml (36 command rows) vs apps/cli/src/dispatch.rs:47-63; locales/en-US.toml; ui/src/i18n/messages.ts
What is wrong: (a) 18 of the 36 command-bearing rows name commands the CLI host refuses: playlist.next/prev (6 rows), playback.step-frame (3), video.set-zoom (4), playback.set-rate, video.take-screenshot, subtitle.set-scale, playlist.set-shuffle, playlist.clear (1 each) — every one answers `unsupported-for-host`. (b) All 135 distinct `hotkeys.*` label-msgids are absent from locales/en-US.toml (grep count 0): doc 07 §6.4 rule 4 (reference-completeness) lists keymaps/* as a msgid-ref source, but seed_integrity.rs only extracts refs from product.toml (2 refs) — the keymap's 135 refs are never audited and would fail rule 4 if they were. (c) The UI's own i18n covers 133/135; `hotkeys.seek-backward-short`/`hotkeys.seek-forward-short` (macOS-only rows) are missing there too. Confidence: HIGH — Verification: ran python3 counts + greps.

F-007 [S4]
Where: keymaps/default.toml:28-32 (FORMAT FLAG) and 266-270 ([platform_overrides.macos] header)
Code:
```
# FORMAT FLAG (for the Lead): doc 05 §8.1 specifies `config/keymaps/default.toml`
# with `[[binding]]` rows (`schema = "keymap.v1"`); this seed follows card 0.B3
# and D-003 (`keymaps/*.toml`, section-grouped `binding-id = { … }` entries with
```
What is wrong: the file openly deviates from the doc's specified format/locations, and the macOS overrides are sourced from a wiki page the file itself dates as "OLDER macOS VLC (0.8.6a / 3.0.12-era text)" with every entry `[VERIFY]` against hardware that was never pinned. Honest labeling, but 8 rows are folklore, not clean-room data. Confidence: HIGH — Verification: read.

## 4. Per-section inventory (data file)
| Section | Bindings | Runtime-wired | Notes |
|---|---|---|---|
| [media] | 19 | 0 | 7 command rows incl. step-frame and playlist.next/prev (refused) |
| [navigation] | 34 | 0 | Space/S/N/P/./,/seeks/rates dead; 8 same-mode collision members |
| [audio] | 8 | 0 | B collides with variantKeymap miniplayer |
| [video] | 21 | 2 (F, I) | fullscreen F + show-controls I |
| [subtitles] | 11 | 0 | G collides with variantKeymap music-mode |
| [playlist] | 6 | 1 (Ctrl+A) | via CUSTOMIZE_DIALOG_KEYMAP |
| [window] | 12 | 3 (Ctrl+T, Ctrl+B, Ctrl+E) | via DIALOG_KEYMAP excerpt |
| [advanced] | 22 | 0 | 20 bookmark rows dead |
| [platform_overrides.macos] | 8 | 0 | no consumer; 4 collide with base keys |

## 5. Test honesty
No tests in this file (data). The drift test hotkeysData.test.ts (baseline PASS) only proves generated/bindings.ts == this file — nothing about dispatch. keymap.test.ts (18 tests, baseline PASS) tests the excerpt seeds' parser/matcher, not this file's rows.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "default keymap seed (v1)" for the player | default.toml:1 | SHALLOW | 7/150 consumed |
| doc 05 §8.1 `config/keymaps/default.toml` `[[binding]]` format | doc 05 | FALSE vs this file | F-007 format flag |
| doc 07 §6.4 rule 4 covers keymaps/* msgid refs | localization refs.rs doc | NOT ENFORCED | 0/135 resolve in locales/en-US.toml |

## 7. Dead / duplicated / unreachable code
143/150 bindings unreachable at runtime; HOTKEY_PLATFORM_OVERRIDES (bindings.ts:174) exported with zero non-test consumers; PlayerScreen's "forward" branch (actions.play/pause/toggle/stop, PlayerScreen.tsx:850-855) is unreachable — no command-bearing row exists in the runtime keymap maps.

## 8. Dependencies & flags
None (data). Generated consumer ui/scripts/gen-hotkeys-data.mjs (dev-time only).

## 9. Open questions for the next agent
- ui keymap loader card (4.C2): owner and ETA; until then the HotkeysEditor is a UI over dead data.
- Should conflicts.ts flag the seed's own 8 collisions (F-004) or does it only check user drafts?
- locales/en-US.toml: who backfills the 135 hotkeys.* msgids — or does rule 4 get scoped to exclude keymaps?
