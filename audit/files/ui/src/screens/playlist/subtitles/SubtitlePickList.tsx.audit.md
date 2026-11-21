# AUDIT: ui/src/screens/playlist/subtitles/SubtitlePickList.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 190 / 1 component + 6 inner handlers |
| File verdict | REAL (code) — production reach: EMPTY STATE ONLY (see F-001) |
| Reachable from shipped entry point? | DEGENERATE: mounted via AddSubtitlesDialog ← PlaylistPanel.tsx:293 / PlaylistScreen.tsx:391/:523 — but all three production mounts pass NO `candidates` (default []) and NO `onSave`, so the only state ever rendered in a shipped configuration is the empty listbox (this component's :145-152). The interactive APG listbox is exercised exclusively by SubtitlePickList.test.tsx and the dev harness. No shipped binary at all (W14 E0425). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:3 |
| Commands run | rg AddSubtitlesDialog production mounts + props; sed AddSubtitlesDialog honesty header; SubtitlePickList.test.tsx suites read (ordering/selection/keyboard/candidate-changes); commands.ts catalog scan for folder-scan/OpenSubtitles services (absent — verified from the CommandRequest union). |

## 1. One-paragraph verdict
A correctly-built APG roving-tabindex listbox (arrows/Home/End/Enter/Space, wrap-around, multi/single semantics, focus-steal protection with a real two-repro test suite) — and in every shipped configuration it renders nothing but its empty state, because the three production mounts of its host dialog pass no candidates: the engine catalog has no folder-scan or OpenSubtitles service (verified against the generated CommandRequest union — no such commands exist), and `subtitle.add-file` needs a host path the web UI cannot produce (the dialog's own honesty header, AddSubtitlesDialog.tsx:25-38, discloses all of this and renders Save honest-disabled without onSave). So the picker is honest staging — but 190 lines of interactive widget ship as a never-populated husk. Beyond reachability, three small defects: the content-signature reset (length+first+last) misses middle-element swaps (focus dies to body), `key={name}` collides on duplicate filenames (the exact class PlaylistBrowse's J06-7 fix addressed — and the by-name selection model makes duplicates ambiguous), and the empty transition unmounts the listbox role entirely, dropping focus and the a11y widget.

## 2. What actually works (verified)
- Keyboard model: move() wraps (:106-109); Home/End; Enter/Space toggles the active row (:137-141); Tab correctly NOT trapped (:142 — the dialog trap owns it); rows get role=option/tabIndex roving/aria-selected (:176-185); onFocus updates activeIndex (:184).
- Focus discipline (w26.k15): the effect (:74-81) focuses the active row on mount and afterwards ONLY while focus already sits inside the listbox — verified by the two-repro suite: "filtering while the search field holds focus never steals the caret (repro A)" (:274-291) and "a non-memoized candidates array (same content) never resets roving focus (repro B)" (:293+). Both behavioral, both in the baseline vitest pass (row 23).
- Selection semantics: single replaces, multi append/remove (:96-104) — tested (:129-183).
- Host-order rendering (g19 P3-4): `ordered = candidates` verbatim, no silent re-sort (:67) — tested (:92).
- The staging is honestly disclosed at the dialog level (AddSubtitlesDialog.tsx:25-38): no fake candidates, no fake persistence, Save honest-disabled without onSave.
- i18n via useSubtitlesMsg; empty-state copy present (:148-149).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/playlist/subtitles/SubtitlePickList.tsx:28-31 (props contract) — impact across all shipped mounts
Code:
```
  /** Candidate subtitle file names, rendered in the GIVEN order (the
   * frame's host order — Folder-2.svg; sort at the producing seam). */
  candidates: readonly string[];
```
What is wrong: No shipped host can populate `candidates`: the three production mounts (PlaylistPanel.tsx:293-297, PlaylistScreen.tsx:391-396, :523-528) pass only open/onClose/mediaNames/initialDestination — candidates defaults to [] (AddSubtitlesDialog.tsx:96), so this listbox renders ONLY its empty state in every reachable configuration. The root cause is disclosed and real: the engine catalog has no folder-scan/OpenSubtitles command (verified — the generated CommandRequest union contains no directory-listing or subtitle-search service), and `subtitle.add-file` requires a host-filesystem path the web UI cannot produce.
Why it matters: The "Add Subtitles" affordance ships in three production surfaces (queue panel kebab, playlist toolbar F2, file context menu); a user who opens it gets a full-window dialog whose picker is permanently empty and whose Save is disabled. Honest staging — but the interactive widget is dead weight in the product until the engine seam exists, and nothing in THIS component marks it (the disclosure lives in the dialog header a maintainer might never open).
What correct looks like: Either gate the dialog's mounting on candidate availability (the host knows it passes none), or render an explicit staged notice in the empty state ("folder scan arrives with the engine seam") instead of plain "no candidates" copy that reads like a transient condition.
Confidence: HIGH — Verification: read all three mounts; commands.ts catalog scan; dialog honesty header.
### F-002 [S4]
Where: ui/src/screens/playlist/subtitles/SubtitlePickList.tsx:88-94
Code:
```
  const signature = `${ordered.length}|${ordered[0] ?? ""}|${ordered[ordered.length - 1] ?? ""}`;
  const prevSignature = useRef(signature);
  useEffect(() => {
    if (prevSignature.current === signature) return;
```
What is wrong: The content signature is length + first + last — a middle-element swap ([A,B,C] → [A,X,C]) produces the SAME signature: no reset fires, activeIndex stays, and the previously-focused row B unmounts, dropping focus to document.body (the focus effect only re-runs on activeIndex change, which did not happen).
Why it matters: The j15 P2-1 trade-off's blind spot: focus loss (and a stale activeIndex pointing at a different item than the user last interacted with) for any candidate change that preserves length and both endpoints — e.g., a search filter that swaps one middle filename. Only reachable once a real producer exists (F-001), but the component is the reusable picker.
What correct looks like: Include the full ordered content in the signature (join with a delimiter), or hash it — identity churn was the only reason to weaken it, and a content join is still identity-independent.
Confidence: HIGH on the mechanism (read); MEDIUM on real-world reachability (needs a live producer).
### F-003 [S4]
Where: ui/src/screens/playlist/subtitles/SubtitlePickList.tsx:166-172 (and the by-name selection model :68/:96-104)
Code:
```
      {ordered.map((name, index) => (
        <SubtitlePickRow
          key={name}
```
What is wrong: `key={name}` collides on duplicate filenames — the exact defect class the same wave fixed in PlaylistBrowse (w26.k06 J06-7: "a name-only key collides on duplicate filenames; the index keeps siblings distinct"). Worse here: selection is name-keyed (`selected.includes(name)`, toggle appends/removes by name), so duplicate names are fundamentally ambiguous — both rows would toggle together and render selected together.
Why it matters: The producer contract (arbitrary name arrays via props) does not guarantee uniqueness; the component's own sibling got the P2 fix for this class.
What correct looks like: Index-prefixed keys (PlaylistBrowse's fix) plus either a disclosure that names are assumed unique or an id-based selection model.
Confidence: HIGH on the mechanism; reachability gated on a real producer (F-001).
### F-004 [S4]
Where: ui/src/screens/playlist/subtitles/SubtitlePickList.tsx:145-152
Code:
```
  if (ordered.length === 0) {
    return (
      <div className="lumen-plsubs-empty" id={rootId}>
```
What is wrong: The empty state early-returns WITHOUT role="listbox" — when the last candidate disappears (filter to zero), the listbox widget unmounts from the a11y tree and any focus inside it dies to body. APG listbox practice keeps a focusable empty listbox (the widget persists; only options vanish).
Why it matters: In the Folder-3 online-search flow the user can type a query that matches nothing — at that instant the screen reader loses the widget and keyboard focus escapes to the dialog body (Escape still works via the dialog trap, but the widget contract breaks).
What correct looks like: Render the empty state INSIDE the role="listbox" container (aria-disabled or an inline empty option message).
Confidence: HIGH — Verification: read; the empty-state test (:121) asserts the copy but not role persistence.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SubtitlePickListProps (type) | 28-43 | REAL | — | Producer contract; uniqueness unstated (F-003) |
| SubtitlePickList | 45-190 | REAL | S3/S4 | F-001 reachability, F-002 signature, F-003 keys, F-004 empty unmount; core APG model correct + tested |
| └ focus effect | 74-81 | REAL | — | Mount-focus + subtree-guard; repro A/B tested |
| └ signature reset effect | 88-94 | REAL | S4 | F-002 blind spot |
| └ toggle | 96-104 | REAL | — | Multi/single semantics; name-keyed (F-003 ambiguity) |
| └ move / onKeyDown | 106-143 | REAL | — | Wrap-around; Home/End; Enter/Space; Tab left to the dialog trap (correct) |
| └ empty early-return | 145-152 | REAL | S4 | F-004 |
| └ listbox render | 154-189 | REAL | — | role/aria-multiselectable/roving tabIndex; label default via catalog |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ordering (:92), listbox exposure (:104), empty (:121) | YES | Structural assertions |
| selection states multi/single (:129-183) | YES | Interaction-level, callback spied |
| keyboard model (:185-250) | YES | Focus assertions per key (roving, wrap, Home/End, Enter/Space) |
| candidate changes repro A/B (:252-300) | YES | The exact regressions the k15 fix addressed — caret retention + identity churn |
| NOT covered: middle-swap signature (F-002), duplicate names (F-003), empty-transition role persistence (F-004) | — | Gap facts, not findings of dishonesty |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "aria-activedescendant was REMOVED … roving stays the one model" | :5-7 | REAL | No activedescendant in render; tabIndex roving :178 |
| "candidates render in HOST order … the previous natural-sort re-broke" | :9-14 | REAL | `ordered = candidates` (:67); test :92 |
| "the reset is keyed on a CONTENT SIGNATURE … identity churn … no longer resets" | :16-21 | REAL (with F-002 blind spot) | Signature :88; repro B test |
| "no shipped host can produce candidates or commit a selection yet" (dialog) | AddSubtitlesDialog.tsx:27-28 | REAL | All 3 mounts pass no candidates/onSave; catalog scan (no folder-scan/OpenSubtitles command) |

## 7. Dead / duplicated / unreachable code
- The interactive listbox is production-unreachable (F-001) — alive only in tests + subtitles/harness.tsx (dev vision harness).
- No duplication.

## 8. Dependencies & flags
SubtitlePickRow, useSubtitlesMsg, subtitles.css hooks. The engine seam gap (no folder-scan/OpenSubtitles; subtitle.add-file host-path constraint) is the reachability ceiling — cross-ref crates/commands catalog audit (W2/W5 already established the catalog's shape). No flags.

## 9. Open questions for the next agent
- AddSubtitlesDialog.tsx / FilenameMatchControl / MatchedPairsList belong to other agents — F-001's root wiring is theirs; my finding is the pick-list's degenerate production reach.
- When a folder-scan command lands, F-002/F-003/F-004 become live defects — they should be fixed before that seam ships.
