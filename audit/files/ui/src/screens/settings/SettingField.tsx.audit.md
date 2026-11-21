# AUDIT: ui/src/screens/settings/SettingField.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 434 / 10 (SettingField, SettingInfoDot, labelOf, numberStep, clampToSchema, secondsDuration, formatDurationMs, parseDurationMs, formatFloatValue, INFO_LABEL_KEY) |
| File verdict | REAL (code) — persistence dead-ends at HEAD (W11+W14); int branch drift |
| Reachable from shipped entry point? | YES in the web build (rendered by SettingsScreen.tsx:397 search results + SettingsSections.tsx:180 area rows; baseline rows 22-23). NO in any shipped product: the desktop shell that would carry writes to the engine (desktop-tauri → runtime.dispatch) cannot compile at HEAD (W14 SUMMARY l.19); the browser shell's demo transport answers `feature-not-exposed` for config.* (persistence.ts:4-6). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | rg `<SettingField` (2 production render sites + tests); read SettingsProvider.tsx:92-150 (setValue=local draft; apply()=persistence.save only); read persistence.ts:1-45 (MOCK seam, localStorage, W11 confirmation); read client/config-settings.ts:1-60 + SettingsScreen.tsx:449-490 (bridge path exists but gated on isDesktopShell()); rg `set-setting` apps/desktop* (engine handler exists in apps/desktop/src/shell.rs:438; desktop-tauri routes via runtime.dispatch); read NumberField.tsx:35-120 (steppers clamp, typed input does not); rg list<string> schema keys (2 language-list keys only) |

## 1. One-paragraph verdict
The row component itself is competently built: type→control mapping is total, the typing-draft discipline for duration/float wells is correct, hints are honestly absent when no text exists, and every option binds a schema key. Three real defects: the int branch omits the clamp + empty-string guard its sibling branches have (out-of-range and cleared-to-0 values commit); the path rows ship a permanently disabled Browse button because no file-dialog command exists anywhere in the command catalog; and the whole file's writes flow into a persistence seam that at HEAD can only ever reach localStorage — the engine-backed writer (config-settings.ts) requires a desktop shell binary that does not compile (W14), confirming W11's "settings never persist" at the seam level.

## 2. What actually works (verified)
— verified by render-site grep → live in SettingsScreen search results and SettingsSections area/advanced rows; baseline build+vitest green (INDEX rows 22-23).
— verified by reading SettingsProvider.tsx:101-128 → setValue writes draft only; apply() writes ONLY `persistence.save(overrides)` — no hidden engine push; the draft/dirty/apply/revert/resetField state machine matches this component's usage.
— verified by reading the duration/float handlers (l.259-292) → raw-draft hold, complete-parse commit with clampToSchema, blur snap-back: the j12 P3-10 contract is implemented as documented.
— verified by rg → deterministic `${entry.key}-hinttext` id cannot collide: search view and area view are mutually exclusive (`searching ? results : content`, SettingsScreen.tsx:382-398), advanced rows use disjoint keys.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/settings/SettingField.tsx:294-310
Code:
```
    control = (
      <NumberField
        className="settings-field__bare"
        label={rowLabel}
        labelHidden
        min={entry.min}
        max={entry.max}
        step={numberStep(entry)}
        value={typeof current === "number" ? current : entry.min ?? 0}
        onChange={(e) => {
          const next = Number(e.target.value);
          if (Number.isFinite(next)) setValue(entry.key, next);
        }}
      />
    );
```
What is wrong: The int branch commits `Number(e.target.value)` with NO clampToSchema and NO empty-string guard. Two failures the sibling branches explicitly handle: (1) float branch guards `e.target.value.trim() !== ""` (l.287) because `Number("") === 0` — clearing an int well therefore COMMITS 0, even when the schema min is > 0; (2) duration/float branches wrap commits in `clampToSchema` (l.264, 288) — a typed out-of-range int (e.g. 99999 with max 100) commits unclamped, because native `<input type=number>` does not clamp typed text (only NumberField's steppers clamp, NumberField.tsx:48-61). persistence.ts's own validation then declines to catch it: "min/max ranges are NOT enforced here … the wells clamp on edit" (persistence.ts:26-27) — but THIS well does not clamp on edit, so the stated invariant is false for int rows.
Why it matters: The settings store can hold out-of-range ints via both typing and clearing; the row re-renders the illegal value, marks itself dirty, and (in the localStorage seam) reloads it on next launch. The codebase's own comments promise clamping that this branch does not do — drift between documented invariant and behavior.
What correct looks like: `const next = Number(e.target.value); if (e.target.value.trim() !== "" && Number.isFinite(next)) setValue(entry.key, clampToSchema(entry, next));`
Confidence: HIGH — Verification: read all three branches + NumberField.tsx + persistence.ts:22-35.

### F-002 [S3]
Where: ui/src/screens/settings/SettingField.tsx:311-343
Code:
```
        <button
          type="button"
          className="settings-action settings-field__browse-action"
          disabled
          aria-describedby={browseNoteId}
          title={m("settings.capability-unavailable")}
        >
          {m("settings.field.browse")}
        </button>
```
What is wrong: Every path row (global.log-file, interface.skin-file, recording.directory) renders a Browse button that is PERMANENTLY disabled — the comment admits "the Browse ACTION has no file-dialog command in the catalog". No transport, no shell, no future lane in the current tree can ever enable it: the command does not exist anywhere (the catalog is generated; rg finds no file-dialog/pick-file command).
Why it matters: This is a shipped dead control — the exact "dead button" class this wave hunts. It is honest (disabled + title reason) rather than fake-active, which is the right way to ship a missing backend, but it is still three rows of permanently inert UI masquerading as a file picker affordance, and it will survive every future release until someone adds a command.
What correct looks like: either the file-dialog command (engine + catalog + transport) or hiding the button until it exists; a permanently disabled control is a placeholder shipped as product.
Confidence: HIGH — Verification: read; rg for file-dialog commands in schemas/commands.schema.json + apps (none found for path picking).
Note: severity held at S3 (honest inert control, no wrong data); would be S2 if it looked active.

### F-003 [S3]
Where: ui/src/screens/settings/SettingField.tsx:219, 244, 329, 373 (every `setValue` call) + persistence chain
Code:
```
      <Switch
        aria-label={rowLabel}
        checked={current === true}
        onChange={(checked) => setValue(entry.key, checked)}
      />
```
What is wrong: Every control in this file writes to `useSettings().setValue`, which is a LOCAL DRAFT (SettingsProvider.tsx:101-103); Apply writes to the persistence seam, whose only reachable implementation at HEAD is `localStorageSettingsPersistence` (persistence.ts:4-6: "MOCK persistence seam… Today it round-trips localStorage for development"). The real writer (createConfigSettingsPersistence → config.set-setting over the bridge) is mounted only when `isDesktopShell()` (SettingsScreen.tsx:459-461), and the desktop shell cannot be built (W14: desktop-tauri fails to compile at HEAD). In the browser dev shell the demo transport answers `feature-not-exposed` for config.* — so no user of any buildable artifact can persist a setting to the engine.
Why it matters: This is W11's "settings never persist" pin, verified at the seam: a fully-rendered, dirty-tracked, restart-badged settings UI whose values die with the webview. The UI layer is honest about it in comments, but the product claim (a settings screen) is hollow at HEAD.
What correct looks like: the desktop shell compiling (W14 fix) is the prerequisite; until then the screen is a UI mock over localStorage.
Confidence: HIGH — Verification: full chain read (SettingField → SettingsProvider → persistence/config-settings → SettingsScreen gate → W14 SUMMARY).
Filed at S3 for THIS file (the component correctly uses its context; the dead-end lives below it); the orchestration-level severity was already banked by W11/W14.

### F-004 [S4]
Where: ui/src/screens/settings/SettingField.tsx:239-242
Code:
```
        options={entry.enumeration.map((o) => ({
          value: o.value,
          labelKey: m(o.msgid as SettingsMsgKey) || o.value,
        }))}
```
What is wrong: `labelKey` receives an already-RESOLVED label string (m() output or raw enum value), not a key. Dropdown's default resolver is identity (Dropdown.tsx:123), so it works — but the prop name and its generic `K` type now mean "label" at this call site, and any future host that wires a real resolver (useMsg) into this Dropdown would double-resolve these strings.
Why it matters: Naming that lies is drift bait; the comment (l.225-226) even calls it "the labelKey + identity resolver seam" — a seam whose entire purpose is to defeat the prop's name.
What correct looks like: pass `label` alongside/instead of `labelKey`, or make Dropdown accept both.
Confidence: HIGH — Verification: read Dropdown.tsx:67-123.

### F-005 [S4]
Where: ui/src/screens/settings/SettingField.tsx:345-363
Code:
```
        value={list.join(", ")}
        onChange={(e) =>
          setValue(
            entry.key,
            e.target.value
              .split(",")
```
What is wrong: list<string> editing joins on ", " and splits on "," — any value containing a comma cannot round-trip. The only two list<string> keys are playback.preferred-audio-languages / preferred-subtitle-languages (language codes, comma-free), so today's data survives; the editing model itself is lossy by construction and would corrupt the first comma-bearing list the schema ever grows.
Why it matters: A value editor that silently destroys legal values of its own value type; fine only by accident of current keys.
What correct looks like: per-item editing (add/remove rows) or an escape-aware split.
Confidence: HIGH — Verification: read; schema grep shows exactly 2 list keys, both language lists.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `labelOf` | 60 | REAL | — | `entry.description as MessageKey` cast — schema contract, generated side audited elsewhere |
| `INFO_LABEL_KEY` | 63 | REAL | — | shared msgid |
| `numberStep` | 65-69 | REAL | — | float 0.05 / duration 50 / int 1 — matches header claim |
| `clampToSchema` | 72-76 | REAL | — | correct; NOT used by the int branch (F-001) |
| `secondsDuration` / `formatDurationMs` / `parseDurationMs` | 84-95 | REAL | — | strip-non-digits parse: minus sign stripped → negative durations unenterable (schema mins ≥0 — acceptable); "3.5.5" → NaN → null ✓ |
| `formatFloatValue` | 98-99 | REAL | — | one-decimal minimum per frame |
| `SettingInfoDot` | 124-162 | REAL | — | real button, aria-label + describedby twin + title; SR-perceivable note (j12 P2-3) — well done |
| `SettingField` (feature/group gates) | 184-188 | REAL | — | hidden+inert for unexposed features, grouped keys deduped |
| `SettingField` (hint resolution) | 190-199 | REAL | — | defensive seed-hint read; honest absence when empty |
| `SettingField` (special widgets) | 209-212 | REAL | — | registry replace, kind tagged |
| bool→Switch | 213-221 | REAL | S3 | F-003 chain |
| enum→Dropdown | 222-246 | REAL | S4 | F-004 labelKey lie; raw-value fallback tested (test:53) |
| duration-ms→TextField | 247-269 | REAL | — | draft+clamp+blur-snap correct |
| float→TextField | 270-293 | REAL | — | empty-guard + clamp correct (the int branch's missing template) |
| int→NumberField | 294-310 | REAL | S3 | F-001 no clamp, clear=0 |
| path→well+Browse | 311-344 | REAL | S3 | F-002 permanent dead button; typed path IS live (real value) |
| list<string>→TextField | 345-364 | REAL | S4 | F-005 comma split |
| string/url→TextField | 365-375 | REAL | — | plain well |
| row render (label/badges/reset/hint) | 378-431 | REAL | — | aria-hidden label (control carries name) ✓; restart badge logic ✓; reset only when ≠ default ✓ |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| test:25-51 enum dropdown + dirty marking | YES | renders, opens listbox, selects, asserts draft/dirty |
| test:53-76 unregistered msgid → raw value fallback | YES | the h09 regression is real and asserted |
| test:78-87 aspect labels end-to-end | YES | label resolution asserted through the row |
| test:89-125 label-left grammar + duration formatting | YES | asserts formatted well text ("3 s") |
| test:128-160 ⓘ absence/presence + glyph geometry | YES | absence case prevents fabricated hints |
| MISSING: int-branch coverage | — | no test types an out-of-range int or clears an int well — F-001's two failure modes are untested; the suite covers enum/duration/hint but not the int commit path |
Mutation check (reasoned): add `clampToSchema` to the int branch → no test fails (proves the gap); delete the raw-value fallback → test:53 fails.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every option is bound to a REAL schema key; gated-but-unexposed features render nothing" | l.36-37 | REAL | gates at l.184-188; options from entry.enumeration |
| "int/float/duration-ms → NumberField (schema min/max; step derived)" | l.25-27 | PARTIAL | int uses NumberField but does not enforce min/max on typed commits (F-001); float/duration use TextField |
| "the Browse ACTION has no file-dialog command in the catalog → disabled with the honest reason" | l.315-317 | REAL (self-indicting) | F-002; no such command exists |
| "w26.k12: the duration/float wells hold the RAW text while typing" | l.43-47 | REAL | l.259-292 draft logic |
| persistence "the REAL write path is the app layer's — config.set-setting over the client bridge" (persistence.ts:4-6) | seam doc | REAL code / UNREACHABLE at HEAD | bridge writer exists (config-settings.ts), mounted only in the desktop shell, which cannot compile (W14) — F-003 |

## 7. Dead / duplicated / unreachable code
No dead code in this file. Unreachable-at-HEAD: the entire write path's engine half (see F-003). Duplicated: the clamp+guard pattern exists in 2 of 3 numeric branches (F-001 is the missing third).

## 8. Dependencies & flags
Consumes Dropdown, Icon, NumberField, Switch, TextField, SPECIAL_FIELD_WIDGETS, WIDGET_GROUPED_KEYS, useSettings, useSettingsMsg, generated schema. `import.meta.env` not used here. CSS: setting-field.css (not in my list). No feature flags beyond entry.featureExposed (product.toml gating, honest).

## 9. Open questions for the next agent
- specialWidgets.ts / SettingWidgets.tsx (not in my list): do the slider/values widgets clamp their commits? F-001 suggests checking every widget's commit path.
- persistence.test.ts + config-settings.test.ts owners: confirm no test asserts range validation on save — persistence.ts explicitly declines to enforce ranges.
- If the file-dialog command ever lands, remember to enable three Browse buttons and the aria-describedby note disappears.
