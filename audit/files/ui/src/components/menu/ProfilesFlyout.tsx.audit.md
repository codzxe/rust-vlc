# AUDIT: ui/src/components/menu/ProfilesFlyout.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 214 / 11 (2 persistence objects + coerceProfile, hydrate, commit, subscribe, getSnapshot, getProfilesState, useMediaProfiles, selectProfile, nextId, createProfile, resetProfilesForTests, buildProfileFlyout) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as UI code (call chain: PlayerScreen → ControlBar → AppMenuButton (`useMediaProfiles` :903, `buildProfileFlyout` :1016) → MenuPopup → MenuItem; /player route; baseline ui-build.log.txt builds it, ui-vitest.log.txt runs its tests). NO engine reachability — by design (no profile command in the catalog; local localStorage only) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | Read full file + ProfilesFlyout.test.tsx (172 lines) + MenuItem.tsx + ControlBar.tsx:890-1114 (the only production consumer) · `rg -n "useMediaProfiles\|buildProfileFlyout\|selectProfile\|createProfile\|getProfilesState\|resetProfilesForTests" ui/src` (consumers: ControlBar + tests + menu/index.ts barrel only) · `rg -n "selectedId" ui/src` (no reader outside this module + ControlBar's flyout build) · `rg -n "selected\|__check\|icon-accent" ui/src/components/menu/menu.css` (check paints via `.lumen-menu__item--selected > .lumen-menu__check::before` :165 — the docstring's paint contract verified) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
A competently built local store: honest corruption handling at the JSON boundary (parse failure ≡ absent ≡ seed), a documented SSR guard on the module-import hydrate (w26.k02), correct empty-list-is-valid semantics, collision-skipping quick-create names, and a real useSyncExternalStore wiring whose row grammar (check slot, selected face, separator, +New) is test-pinned down to the MenuPopup class names. The hostile question cuts deeper than the code quality: a "profile" here is a name and a check mark and NOTHING else — the persisted `selectedId` and the profile list have zero consumers beyond the flyout's own rendering (verified by exhaustive grep), there is no media library, no collection, no scoping anywhere in the UI, so selecting or creating a profile is a button whose entire effect is repainting its own check. The header is honest that "no row claims engine capability", but the shipped app-menu row still presents frame-seeded fixture rows (Movies/Shows/Music/Work) as if they were user-meaningful state. Two smaller defects: the docstring claims the selected row renders first (frame grammar row 1) while the builder preserves store order — true only in the untouched seed state, and the test named "selected first" cannot catch it; and hydrate silently drops malformed profile records, a loss the next commit persists permanently.

## 2. What actually works (verified)
— Store contract is real and test-pinned: seed/`selectedId` "movies" (:29-33), select-persists + unknown-id no-op (:35-45), quick-create "Profile 5"→"Profile 6" skipping (:47-62), empty-list-never-resurrects (:64-68) — all green in baseline ui-vitest.log.txt (1705 passed).
— Corruption honesty at the boundary: `JSON.parse` in try/catch returns null ≡ absent (:62-70); non-object/array rejected (:66); every stored record coerced field-by-field with name emptiness rejected (:91-96).
— SSR guard on the module-import hydrate (:118-119): windowless imports ride the memory seam instead of crashing — the w26.k02 regression is documented and the guard is present.
— The row grammar contract is verified against its consumers: MenuItem.tsx:79 renders the empty check slot when `entry.icon === undefined`, and menu.css:165-166 paints the check only on the selected face — both ends of the docstring's paint contract exist.
— Integration is test-pinned twice: ProfilesFlyout.test.tsx:131-161 (aria-current, no aria-checked, check slot, +New icon) and ControlBar.test.tsx:1167-1195 (app-menu select persists, +New creates "Profile 5" and moves the selection) — the flyout fires the real seams end-to-end.

## 3. Findings

### F-001 [S2]
Where: ui/src/components/menu/ProfilesFlyout.tsx:25-35, 142-171
Code:
```
/** One local media-library profile (a named collection marker). */
export interface MediaProfile {
```
```
export function useMediaProfiles(): ProfilesState {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
```
What is wrong: the entire feature is a dead end. `useMediaProfiles` is called exactly once in production (ControlBar.tsx:903) and only to build the flyout's own rows; `selectedId` has no reader outside this module and ControlBar's `selected: profile.id === selectedId` row flag. Selecting "Music" instead of "Work" changes nothing anywhere in the app: no library view, no collection membership, no scoping, no engine command (the header admits this). A "named collection marker" that marks no collection.
Why it matters: the shipped app menu presents a Profile flyout whose rows are seeded fixtures (Movies/Shows/Music/Work — frame data, not user data) and whose only user actions (select, +New) persist state that nothing consumes. This is the §11 "UI settings wired to nothing" pattern — same class as the write-only hotkey overrides (W15-A04 HotkeysEditor F-001) and the onboarding dead-end stores, here with the aggravation that the seed fabricates four profiles the user never created.
What correct looks like: either a consumer (a library/browse surface filtered by the active profile) or an honest inert marker — and at minimum the seed should be empty, not four fabricated profiles (the "Open Recent" row in the same menu shows the correct honest-empty pattern, ControlBar.tsx:924-927).
Confidence: HIGH — Verification: read + exhaustive rg of every export; consumers are ControlBar, menu/index.ts barrel, and tests only.

### F-002 [S3]
Where: ui/src/components/menu/ProfilesFlyout.tsx:181-183, 203-208
Code:
```
 * The flyout's rows as menu DATA (Profiles.svg §A10, comfortable density):
 * selected row first with the resting selected face + check in the icon slot
```
```
  const rows: MenuEntry<string>[] = profiles.map((profile) => ({
    id: `profile-${profile.id}`,
    labelKey: profile.name,
    selected: profile.id === selectedId,
    onSelect: () => onSelect(profile.id),
  }));
```
What is wrong: the docstring says the selected row renders FIRST (the frame grammar: "row 1 (the selected profile)", header :5-6); the builder maps store order and never reorders. In the seed state the claims coincide (Movies is both first and selected); after `selectProfile("work")` the check sits on row 4 and row 1 is an unselected Movies.
Why it matters: the doc-vs-code divergence is invisible to the test suite: ProfilesFlyout.test.tsx:95-116 is literally named "rows: selected first with the resting face" but asserts only the seed ordering — it passes unchanged under the divergent behavior (mutation test: reorder rows in the builder and the test still passes, because it never selects a non-first profile before asserting order).
What correct looks like: either sort the selected row first in the builder (matching the documented frame grammar) or fix the docstring to "rows in store order; the selected row carries the resting face + check wherever it sits" and rename the test to what it asserts.
Confidence: HIGH — Verification: read of builder + test; frame grammar quoted from this file's own header.

### F-003 [S3]
Where: ui/src/components/menu/ProfilesFlyout.tsx:98-105
Code:
```
/** Absent/corrupt store folds back onto the seed rows; a stored EMPTY list is
 * valid user state and stays empty — it never resurrects. */
function hydrate(): ProfilesState {
  const loaded = persistence.load();
  if (loaded === null || !Array.isArray(loaded.profiles)) {
    return { profiles: SEED, selectedId: SEED_SELECTED_ID };
  }
  const profiles = loaded.profiles.map(coerceProfile).filter((p): p is MediaProfile => p !== null);
```
What is wrong: asymmetric corruption handling with silent permanent loss. A store whose `profiles` is not an array folds to the seed; a store whose `profiles` IS an array of malformed records silently drops every malformed record and yields a shorter (possibly empty) list — which the comment then treats as "valid user state". The next `commit()` persists the filtered list back to localStorage, destroying the malformed originals. The comment's first sentence ("Absent/corrupt store folds back onto the seed rows") is false for exactly the corrupt-array case a partial write or schema drift produces.
Why it matters: user-created profiles (the one real write path, "+ New") can be silently discarded on a partially-corrupt read with no diagnostic, and the loss is then made permanent by the very next save.
What correct looks like: if ANY record fails coercion, treat the store as corrupt (fold to seed, or better: keep in-memory data, log a console diagnostic, and skip the save-back until the user acts) — the all-or-nothing rule the comment already promises.
Confidence: HIGH — Verification: read; corruption paths traced through load/coerce/commit.

### F-004 [S4]
Where: ui/src/components/menu/ProfilesFlyout.tsx:81-88, 173-178
Code:
```
export const memoryProfilesPersistence: ProfilesPersistence = {
  load() {
    return null; // test double: always the seed fixture
  },
```
What is wrong: test doubles and the test-reset seam (`resetProfilesForTests`, `memoryProfilesPersistence`) are exported from the production module and ship in the bundle; the module-global `persistence` variable is mutable production state that tests reassign.
Why it matters: hygiene/blast-radius only — no production caller mutates the seam (verified by grep: test files and the barrel only), but the pattern grows the public surface of every store module (the playlistsStore precedent carries the same shape).
What correct looks like: gate the test seam behind a `import.meta.vitest` check or a separate `./ProfilesFlyout.test-support.ts` module re-exported by the barrel under a vitest condition.
Confidence: HIGH — Verification: read + rg of all exports' consumers.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MediaProfile / ProfilesState / ProfilesPersistence (types) | 26-41 | REAL | — | shapes match the persisted JSON exactly |
| localStorageProfilesPersistence.load | 61-71 | REAL | — | try/catch ≡ absent on corrupt JSON; non-object rejected |
| localStorageProfilesPersistence.save | 72-78 | REAL | — | quota/private-mode failures swallowed to in-memory (documented) |
| memoryProfilesPersistence | 81-88 | REAL | — | test double; shipped in bundle (F-004) |
| coerceProfile | 91-96 | REAL | — | field-wise validation; empty name rejected |
| hydrate | 100-110 | REAL | S3 | F-003 asymmetric corrupt-array handling; selectedId validated against the filtered list |
| module init (persistence/state/listeners) | 118-121 | REAL | — | SSR-safe seam pick (w26.k02); hydrate at import |
| commit | 123-127 | REAL | — | save-then-notify; save failure non-fatal |
| subscribe / getSnapshot | 129-134 | REAL | — | correct useSyncExternalStore pair, stable identity |
| getProfilesState | 137-139 | REAL | — | non-hook read for tests/hosts |
| useMediaProfiles | 142-144 | REAL | S2 | the only production read — feeds the flyout that renders it (F-001) |
| selectProfile | 147-151 | REAL | S2 | persists a selection nothing consumes (F-001); unknown id no-op; same-id no-op |
| nextId | 154-156 | REAL | — | time+random, collision-safe per device |
| createProfile | 163-171 | REAL | S2 | name-skip logic correct (test-pinned); created profile joins the dead-end store (F-001) |
| resetProfilesForTests | 174-178 | REAL | S4 | test-only export, mutates module state (F-004) |
| buildProfileFlyout | 189-214 | REAL | S3 | row data correct for the check/separator/+New grammar; F-002 no selected-first ordering despite docstring |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :29 hydrates frame seed, Movies active | YES | asserts exact names + selectedId |
| :35 select persists; unknown id no-op | YES | asserts both transitions |
| :47 quick-create "Profile 5"/"Profile 6" skip | YES | name-collision skip is behaviorally pinned |
| :64 stored empty list stays empty | YES | pins the never-resurrect contract |
| :70 corrupt store folds to seed | PARTIAL | exercises only `load()=>null` (absent); the corrupt-ARRAY case (F-003) is untested |
| :95 "rows: selected first…" | NO (for the ordering claim) | asserts seed order; a builder that never reorders passes — the "selected first" claim in the name is untested (F-002) |
| :118 wire-up: row activation selects, +New creates | YES | callback contracts asserted |
| :131 MenuPopup integration (aria-current, check slot, +New icon) | YES | DOM-level assertions incl. absence of aria-checked |
| :163 MenuItem selected-check grammar | YES | class + aria assertions |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped). ControlBar.test.tsx:1167-1195 additionally pins the app-menu end-to-end path.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "profiles are LOCAL device data persisted under `lumen.profiles.v1` … no row claims engine capability" | header :13-18 | REAL | localStorage seam :60-79; no engine command touched (catalog grep via ControlBar audit) |
| "the seed never resurrects once the user edits" | header :15-16 | REAL (list-level) / FALSE (record-level) | empty-list case test-pinned :64; malformed-record case drops data instead (F-003) |
| "selected row first with the resting selected face" | :181-183 | FALSE (post-seed) | builder preserves store order (:203-208); F-002 |
| "MenuItem renders the empty check slot when a row has no icon, and menu.css paints the ✓ on the selected face" | :183-186 | REAL | MenuItem.tsx:79; menu.css:165-166 |
| "One local media-library profile (a named collection marker)" | :25 | FALSE (no library/collection exists to mark) | F-001: zero consumers beyond the flyout |

## 7. Dead / duplicated / unreachable code
- No dead code in the module. The FEATURE is dead-ended (F-001): every export is reachable, the store state itself reaches nothing.
- `memoryProfilesPersistence` / `resetProfilesForTests` are test-only but shipped (F-004).

## 8. Dependencies & flags
- react (useSyncExternalStore), ./types (MenuEntry). No engine/bridge dependency — deliberately local. No feature flags. Storage: `window.localStorage` under `lumen.profiles.v1`, same key scheme as `lumen.playlists.v1`.

## 9. Open questions for the next agent
- If a library/browse surface ever lands, it should consume `useMediaProfiles` — until then F-001 stands as the feature verdict.
- menu/index.ts barrel re-exports this module's test seam (F-004) — barrel auditors should check whether other stores duplicate the pattern or gate it.
