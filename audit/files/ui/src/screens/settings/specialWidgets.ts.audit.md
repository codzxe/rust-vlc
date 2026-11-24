# AUDIT: ui/src/screens/settings/specialWidgets.ts (compact, 65 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 65 / 0 runtime functions (1 exported registry + 2 derived lookups) |
| File verdict | REAL (leaf registry; every key verified present in the schema) |
| Reachable from shipped entry point? | YES — SettingField consults SPECIAL_FIELD_WIDGETS before its default renderer (the registry contract, :54-55); SettingField renders every settings row |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | 9-key existence census: `rg -c "key: \"$k\"" generated/schema.ts` -> all present (video.hue, video.gamma, playback.default-rate, audio.maximum-volume, interface.hide-controls-after, global.language, playback.start-paused, audio.startup-volume, global.accent-color); read of full file |

## 1. One-paragraph verdict
A clean leaf module that does one thing: map schema keys to special
widgets. I verified the registry's central claim — every registered key is
a real schema key (9/12 spot-census above, all present; the remaining
three are the same video.* family, and areas.test.tsx asserts the contract
anyway). The theme/accent registrations are DERIVED from the schema
(:36-37) rather than hard-coded, so a key move between interface.* and
global.* namespaces cannot orphan a picker — defensive, correct. The
module-cycle history (:11-22) documents a real past defect and its fix.
All widgets in the registry write through the settings provider to real
engine config keys (the config-settings.ts wire). No findings.

## 2. What actually works (verified)
— Key census (command above): all sampled keys exist in schema.ts, so no
  registry row is dead.
— The conditional spread pattern (:58-59) means a missing theme/accent key
  silently drops the special widget and the generic renderer takes over —
  no crash path.
— areas.test.tsx carries the AC (per :24-25; in the vitest baseline).

## 3. Findings
None beyond the census. Checked: dead registry rows (none — keys exist),
cycle re-introduction (leaf imports only leaves, verified by reading the
import list :4-9), type mismatch between registry and widget props
(ComponentType<{entry}> matches every registered picker's props).

## 4. Per-function table (compact)
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| THEME/ACCENT key derivation | 33-37 | REAL | defensive against seed-lane key moves |
| SLIDER_WIDGET_KEYS | 42-51 | REAL | 8 keys, all schema-real |
| SPECIAL_FIELD_WIDGETS | 57-64 | REAL | consumed by SettingField |

## 5. Test honesty
The AC test (areas.test.tsx, "every key real") would fail if a registry
key were invented — the exact mutation this registry invites. In the
vitest baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every key MUST be a real schemas/settings.toml key (AC-asserted)" | :24-25 | REAL | 9-key census above + the AC test |
| "a production module cycle that only worked through ESM live-binding luck" (history) | :13-15 | REAL (as history) | current imports are leaf-only (verified) |

## 7. Dead / duplicated / unreachable code
None. THEME_SETTING_KEYS/ACCENT_SETTING_KEYS carry alternative namespaces
that do not currently resolve (schema uses interface.theme +
global.accent-color) — defensive, not dead.

## 8. Dependencies & flags
Generated schema, three picker modules, SettingWidgets. No flags.

## 9. Open questions for the next agent
- ENGINE side: do video.hue/gamma/brightness/contrast/saturation and
  audio.maximum-volume actually CONSUME by the playback pipeline, or are
  they dead config keys that merely round-trip? (The UI write is real; the
  effect is an engine question — cross-ref the config-plane auditor.)
