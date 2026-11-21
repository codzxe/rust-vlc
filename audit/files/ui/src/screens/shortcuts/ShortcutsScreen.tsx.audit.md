# AUDIT: ui/src/screens/shortcuts/ShortcutsScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 511 / 16 (useShortcutsMsg, ShortcutRow/Group types, SECTION_TO_GROUP, macosSectionGroup, groupOfHotkey, shellRow, GROUP_LABELS/ORDER, KEYCAP_LABELS, keycapText, keyChips, keycapCombo, buildShortcutGroups, labelPrefix, trailingNumber, sortRowsNumericAware, rowMatches, filterGroups, FILTER_OPTIONS, ShortcutsScreen) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (App.tsx:306 `/keyboard-shortcuts` → lazy ShortcutsScreen; app-menu row "keyboard-shortcuts" navigates there, ControlBar.tsx:1024-1029). UI bundle builds (baseline #22); product caveat: no desktop binary exists (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "hotkeys" src/screens/settings/areas/index.tsx` (276: `hotkeys: "controls"` redirect registry) + Read SettingsScreen.tsx:287-294 (retired-slug canonicalisation — the pencil link /settings/hotkeys resolves, never 404s) + rg controls.tsx:43 (HotkeysEditor really hosted there); `rg -o 'section: "[a-z-]+"' generated/bindings.ts \| sort \| uniq -c` (8 sections: advanced 22, audio 7, media 18, navigation 35, playlist 8, subtitles 12, video 27, window 12 — all 141 rows mapped, 0 disabled); `rg -n '"shortcuts.group' src/i18n/messages.ts` (2117-2120: group headings "Playback/Navigation/Volume/Player variants"); `rg -n '"shortcuts.filter.all"\|"shortcuts.save"\|…' messages.ts` (866-871, 2346+: b16 lane keys ARE registered); Read w26.b16.ts header ("until then the screen resolves these via its local seam-first resolver"); `rg -n 'laneCatalog\[key\]' src` (17 copies of the same seam pattern repo-wide); `rg -n 'labelMsgid' scripts/gen-hotkeys-data.mjs` (50: default `hotkeys.${id}`, no registration validation); ShortcutsScreen.test.tsx read (17 tests, 30s timeouts corroborating the perf comment). Baseline: ui-vitest.log.txt 136/136 files 1705 passed; hotkeys:check PASS (INDEX #24). |

## 1. One-paragraph verdict
This is a genuinely data-driven reference screen: all 146 rows come from the generated keymaps mirror (HOTKEY_BINDINGS, 141 rows, all sections mapped) plus 5 shell keymap seed rows, the pencil edit-links resolve through a real retired-slug redirect to the actual HotkeysEditor, and 17 behavioral tests pin dialog semantics, friendly keycaps, scoping, numeric sort and all dismissal paths (executed in baseline). The hostile findings: the grouping map files 39 keymaps rows (the entire `video` section — aspect-ratio/crop/zoom/deinterlace — and all 12 `window` rows) under the group heading "Player variants", which the file's own header defines as the shell variant/fullscreen seed rows — a misleading reference categorization; and the `useShortcutsMsg` lane seam is dead code whose fallback can never fire (the b16 keys are already registered in messages.ts; useMsg is total), the same post-registration seam-rot pattern replicated in 17 files. Minor: the `labelMsgid as MessageKey` cast has no registration gate, and the macOS section-mapping branch is dead against current data.

## 2. What actually works (verified)
— verified by `rg -o 'section: …' generated/bindings.ts | sort | uniq -c` → 8 sections, 141 bindings, `rg -c 'disabled: true'` → 0: every keymaps row renders; the "146 pencil links" header claim = 141 + 5 shell rows (SHELL_BINDING_LABELS has 5 entries) — arithmetic checks out.
— verified by Read areas/index.tsx:275-291 + SettingsScreen.tsx:287-294 → `/settings/hotkeys` (the pencil href, asserted by ShortcutsScreen.test.tsx:132-139) redirects to the `controls` area which imports and hosts HotkeysEditor (controls.tsx:43) — the edit affordance reaches a real editing surface.
— verified by ShortcutsScreen.test.tsx:89-131 (no raw `BrowserRefresh`/`VolumeMute` spill; caps contain "Browser Refresh"/"Mute"/"←") + baseline ui-vitest.log.txt (136/136, 1705 passed) — keycap mapping executes and is pinned.
— verified by `rg '"shortcuts.group' src/i18n/messages.ts` → group headings exist and match the test's asserted titles.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/shortcuts/ShortcutsScreen.tsx:107-117
Code:
```ts
/** keymaps section → group id (see the module header for the policy). */
const SECTION_TO_GROUP: Readonly<Record<string, ShortcutGroupId>> = {
  media: "playback",
  playlist: "playback",
  subtitles: "playback",
  navigation: "navigation",
  advanced: "navigation",
  audio: "volume",
  video: "variants",
  window: "variants",
};
```
What is wrong: The `variants` group is titled "Player variants" (messages.ts:2120) and the module header (lines 8-9) defines it as "the shell-owned variant/fullscreen seed rows" — but SECTION_TO_GROUP also routes the ENTIRE keymaps `video` section (27 bindings: aspect-ratio-cycle, crop-cycle, crop-one-px-*, deinterlace-mode-cycle, autoscale, zoom…) and all 12 `window` bindings into it. 39 of 44 rows in that group are not player-variant actions. Likewise the 22 `advanced` bookmark rows (play/set-bookmark-1..10) sit under "Navigation".
Why it matters: This is a reference/lookup screen — its entire value is findability. A user looking for "Crop" or "Aspect ratio" scans Playback/Navigation/Volume and finds nothing; the labels lie about the taxonomy the data follows. The header comment and the map also disagree with each other (comment: variants = shell seed rows; map: variants = video+window+seeds).
What correct looks like: A "Video" group (or "View") for the video section, "Window" under navigation or its own group, and the variants group holding only the 5 shell seed rows — or at minimum a header comment that matches the map.
Confidence: HIGH — Verification: read the map, the group labels (messages.ts:2117-2120), and the section census (27 video + 12 window ids listed from generated data).

### F-002 [S3]
Where: ui/src/screens/shortcuts/ShortcutsScreen.tsx:78-84
Code:
```ts
function useShortcutsMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}
```
What is wrong: The `?? laneCatalog[key]` fallback is unreachable: `useMsg` is total (`return (key) => catalog[key] ?? messages[key]`, messages.ts:3016 — "a missing entry can never render `undefined`") and every b16 lane key is already registered in messages.ts (866-871 type union; 2346-2351 catalog entries). The seam's reason to exist ("Lane keys win while they are unregistered", line 74-75) is gone — the Lead merge already happened. Same post-registration seam-rot as ControlBar.tsx's k07Resolver; `rg 'laneCatalog\[key\]'` finds 17 copies of the pattern across the codebase.
Why it matters: Dead indirection whose doc comment still claims pre-registration liveness; each copy is a place where a future reader must re-derive that the fallback never fires. Systemic (17 sites), and the file's own comment (lines 72-77) is now false in the present tense.
What correct looks like: Delete the seam (call `m` directly) once keys are registered — the exact cleanup the ControlBar header documents having done for f13/gfixC/i03/i16; or a single shared utility instead of 17 hand-rolled copies.
Confidence: HIGH — Verification: read messages.ts:3011-3016 + registration sites; w26.b16.ts header read (its "until then" premise is expired).

### F-003 [S4]
Where: ui/src/screens/shortcuts/ShortcutsScreen.tsx:252-256
Code:
```ts
    rows[group].push({
      id: binding.id,
      labelKey: binding.labelMsgid as MessageKey,
      key: binding.key,
    });
```
What is wrong: `as MessageKey` is an unchecked cast over generated data. The generator (gen-hotkeys-data.mjs:50) defaults `labelMsgid` to `hotkeys.${id}` when keymaps/default.toml omits `label-msgid` and validates nothing against the i18n catalog; an unregistered msgid makes `m(row.labelKey)` return `undefined`, rendering an empty action label with no error (the known failure mode ControlBar.test.tsx:1029 explicitly tests for elsewhere: "an unregistered key would render empty").
Why it matters: The type system's guarantee (MessageKey ⇒ catalog entry) is silenced exactly where untrusted generated strings enter; a keymaps edit that drops a label-msgid ships a blank row in the reference screen silently.
What correct looks like: A gate assertion (l10n:check or hotkeys:check) that every emitted labelMsgid exists in messages.ts, or a runtime fallback to the raw msgid string instead of undefined.
Confidence: HIGH on the mechanism (read generator + cast + messages.ts totality); UNVERIFIED whether l10n:check already covers it (script not read — no counter-evidence found in gen-hotkeys-data.mjs).

### F-004 [S4]
Where: ui/src/screens/shortcuts/ShortcutsScreen.tsx:120-126
Code:
```ts
/** Platform-variant rows (section "<base>-macos") map by their base action. */
function macosSectionGroup(section: string): ShortcutGroupId | undefined {
  const base = section.replace(/-macos$/u, "");
  if (base.startsWith("audio")) return "volume";
  if (base.startsWith("subtitle") || base.startsWith("seek")) return "playback";
  if (base.startsWith("fullscreen")) return "variants";
  return undefined;
}
```
What is wrong: Dead-in-data defensive mapping: the generated bindings contain zero `-macos` sections (section census: advanced/audio/media/navigation/playlist/subtitles/video/window only), so every branch after the `replace` never fires. Also inconsistent with SECTION_TO_GROUP's exact-match policy (this one prefix-matches, so a hypothetical `seek-macos` maps to playback while a non-mac `seek` section would fall through SECTION_TO_GROUP to undefined → silently dropped).
Why it matters: Untestable-by-data code whose policy (prefix match) contradicts the primary map (exact match) — if macOS sections ever appear, rows will group differently than the same base section on other platforms.
What correct looks like: One mapping function (strip `-macos`, then SECTION_TO_GROUP) or a test fixture with a `-macos` row pinning the behavior.
Confidence: HIGH — Verification: section census over the generated file (0 `-macos`); read both maps.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useShortcutsMsg` | 78-84 | REAL | S3 | F-002 dead fallback seam. |
| types `ShortcutRow`/`ShortcutGroupId`/`ShortcutGroup` | 91-105 | REAL | — | Exported, consumed by tests. |
| `SECTION_TO_GROUP` | 108-117 | REAL | S3 | F-001 misgrouping (video+window → "Player variants"). |
| `macosSectionGroup` | 120-126 | REAL | S4 | F-004 dead-in-data + policy inconsistency. |
| `groupOfHotkey` | 128-131 | REAL | — | Dispatch only; total over current data. |
| `SHELL_BINDING_LABELS`/`shellRow` | 134-145 | REAL | — | 5 seed rows from DEFAULT_KEYMAP+VARIANT_KEYMAP; label keys registered. |
| `GROUP_LABELS`/`GROUP_ORDER` | 147-154 | REAL | — | Headings registered. |
| `KEYCAP_LABELS` | 163-212 | REAL | — | 40-entry artwork map; unknown spellings pass through verbatim (honest); pinned by test 89-118. |
| `keycapText`/`keyChips`/`keycapCombo` | 215-235 | REAL | — | Pure; exported and unit-pinned (test 120-131). |
| `buildShortcutGroups` | 238-263 | REAL | S4 | Pure, default-arg test-pinnable; disabled rows skipped (0 today); `labelMsgid as MessageKey` cast F-003. |
| `labelPrefix`/`trailingNumber` | 266-272 | REAL | — | Regex-based; trailing-number -1 sentinel handled by sort. |
| `sortRowsNumericAware` | 280-301 | REAL | — | Surgical run detection (consecutive same-prefix only); pinned by tests 140-172 (numeric 1..10; non-numbered untouched). |
| `rowMatches`/`filterGroups` | 307-340 | REAL | — | Scope-aware, case-insensitive, matches raw combo AND friendly caps; pure + pinned (tests 173-227). |
| `FILTER_OPTIONS` | 343-347 | REAL | — | Three scopes; label keys registered. |
| `ShortcutsScreen` (component) | 349-510 | REAL | — | Latest-ref `m` wrapper (362-366) is a documented, correct perf pin (useMsg identity instability, j16 #4); memos now effective; initial focus on search; Dialog trap + scrim/Escape/✕/Cancel all exit to /player (documented pre-rebuild contract — note: always to /player even when arrived from elsewhere, deliberate); Save disabled by design (read-only reference, frame-verbatim); Global Hotkey column renders empty keycaps + decorative pencil — honest (VLC 3.0.23 ships global-key-* empty; verified 0 `global` matches in generated data). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :46 "renders the four reference groups" | PARTIALLY | Asserts 4 sections exist and are non-empty — pins the group SET but not the grouping POLICY (F-001's misgrouping passes: video rows make "Player variants" non-empty). |
| :59 dialog grammar + footer | YES | role=dialog/aria-modal/placeholder/scope value/headers/Save-disabled all asserted. |
| :89 friendly keycaps | YES | Negative assertion on 12 raw spellings + positive on mapped ones. |
| :132 pencil link | YES (href only) | Asserts href="/settings/hotkeys" + aria-label; the redirect resolution is NOT covered here (verified statically by this audit instead). |
| :140/:157 numeric sort | YES | Asserts 1,2,3…10 order and untouched curated order. |
| :173/:192/:214 search + scopes | YES | Row-level match assertions across scopes. |
| :260/:266/:272 dismissals | YES | scrim/✕/Escape → /player navigation asserted. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "built from REAL keymap data" | ShortcutsScreen.tsx:4-9 header | REAL | HOTKEY_BINDINGS generated from keymaps/default.toml (bindings.ts:1-2), 141 rows + 5 shell rows = 146. |
| variants group = "the shell-owned variant/fullscreen seed rows" | ShortcutsScreen.tsx:8-9 header | FALSE | F-001: 39 non-seed keymaps rows also land in that group. |
| "Lane keys win while they are unregistered" | ShortcutsScreen.tsx:74-75 | FALSE (stale) | Keys registered (messages.ts:866-871); fallback unreachable — F-002. |
| "the keymaps model has no global-hotkey bindings (VLC … ships every `global-key-*` empty)" | ShortcutsScreen.tsx:36-38 | REAL | `rg -c 'global' generated/bindings.ts` → no matches; column honestly empty. |
| "Save … disabled — this reference screen is read-only by design" | ShortcutsScreen.tsx:32-34 | REAL | Button disabled at 504; pinned by test 59-86. |

## 7. Dead / duplicated / unreachable code
Dead: `?? laneCatalog[key]` fallback (F-002); macosSectionGroup branches (F-004). Duplicated: the lane-seam hook pattern (17 repo copies, this is one). Unreachable: nothing beyond the above; screen reachable via route + app menu.

## 8. Dependencies & flags
Consumes: HOTKEY_BINDINGS (generated, drift-guarded by hotkeys:check — baseline PASS #24), DEFAULT_KEYMAP/VARIANT_KEYMAP (player keymap data), Dialog/Dropdown/Button/Icon components, i18n b16 lane keys. No feature flags; no transport involvement (pure reference screen — correctly no dead backend wiring to audit).

## 9. Open questions for the next agent
- Who owns the 17-site lane-seam cleanup (F-002 family)? A single sweep deleting all post-registration seams is one PR.
- Does l10n:check assert msgid registration for generated labelMsgid values (F-003)? If yes, downgrade F-003 to a cast-hygiene note.
- F-001's taxonomy: is there a frame (Keyboard Shortcuts.svg) section order that dictates the 4 groups? If the frame shows different headings, the map is a code defect against the frame, not just a naming choice.
