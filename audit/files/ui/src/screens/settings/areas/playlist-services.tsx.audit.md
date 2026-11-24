# AUDIT: ui/src/screens/settings/areas/playlist-services.tsx
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A05) |
| Date | 2026-11-24 |
| Lines / functions | 36 / 1 (PlaylistServicesArea) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — App.tsx:39-40 lazy SettingsScreen, route /settings/:area (App.tsx:108) → areas/index.tsx:258 `library: LibraryArea` → library.tsx:56 `<PlaylistServicesArea />` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (importers, schema keys) -> hits shown; `rg -c "upnp|podcast|discover" schemas/settings.toml` -> exit 1 (zero); `rg 'screen = "(library|playlist-services)"' schemas/settings.toml` -> exit 1 (zero); sed -n '25,36p' -> quote verified |

## 1. One-paragraph verdict
An honest empty seam. The component renders exactly one empty div with stable test hooks and claims nothing else — and every factual claim in its comments is independently verified true: schemas/settings.toml contains zero upnp/podcast/discover keys and zero `screen = "library"`-placed settings, so "renders nothing today (no playlist-services [[setting]] keys exist)" is accurate, not an excuse. This is the anti-pattern the audit usually flags (dead-looking controls) done correctly: no fake toggles, no disabled rows pretending a backend exists. The one defect is cosmetic-naming: it reuses the `.settings-sections` container class for a fold containing zero sections.

## 2. What actually works (verified)
— verified by `rg -n "playlist-services" audit/runs/baseline/ui-test.txt` -> line 167: `✓ src/screens/settings/areas/playlist-services.test.tsx (3 tests) 145ms` (routing + zero-rows assertions pass).
— verified by `rg -c "upnp|podcast|discover" schemas/settings.toml` -> no matches (exit 1): the "schema has no service-discovery entries" claim is true.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/playlist-services.tsx:25-35
Code: (verbatim, sed-verified)
```
export function PlaylistServicesArea(): ReactElement {
  return (
    <div
      className="settings-sections"
      data-settings-fold="playlist-services"
      data-testid="playlist-services-area"
    >
      {/* service discovery / playlist behavior: absent from the schema and the
          feature exposure list → deliberately nothing (never dead controls). */}
    </div>
  );
}
```
What is wrong: the entire runtime deliverable of this routed file is an empty div; it also reuses `.settings-sections` (settings.css:349 — the area-level flex container for `.settings-section` children) while containing zero sections, nested inside library.tsx's own `.settings-sections` root. Why it matters: a class name that lies about its contents; the file's whole value is placeholder scaffolding. What correct looks like: a neutral fold class (e.g. `.settings-fold`) or folding the seam into library.tsx until schema rows exist. Confidence: HIGH — Verification: read (CSS rule + nesting verified).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PlaylistServicesArea | 25-36 | REAL | S4 | Renders the empty fold container; matches its documented contract exactly |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| playlist-services.test.tsx:13-22 (zero rows, no area root) | YES | queries `.settings-field`, `[data-setting]` — a smuggled control would break `toHaveLength(0)` |
| playlist-services.test.tsx:24-28 (no invented discovery options) | YES | text queries for /discovery/i, UPnP, podcast, SAP |
| playlist-services.test.tsx:31-43 (routed at /settings/library) | YES only for LibraryArea mounting | asserts `data-settings-area="library"` non-null; the fold's own mounting is pinned by library.test.tsx:46-48 (baseline:108) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO entries exist in schemas/settings.toml" (service discovery) | lines 6-8 | REAL | rg over schemas/settings.toml -> zero matches |
| "renders nothing today (no playlist-services [[setting]] keys exist)" | line 23 | REAL | `rg 'screen = "(library|playlist-services)"'` -> zero; rg discovery keys -> zero |
| "exactly ONE [data-settings-area="library"] renders per route" | lines 13-15 | REAL | area root is on library.tsx:49; test :18 asserts fold carries none |

## 7. Dead / duplicated / unreachable code
The component itself is reachable (chain in header) but contributes zero visible UI — by design and honestly disclosed. No unreachable code.

## 8. Dependencies & flags
None. Pure React element, no imports beyond `ReactElement` type, no i18n, no client calls, no feature gates.

## 9. Open questions for the next agent
When library [[setting]] rows land, does this fold get deleted or does E1's generated SettingsSections mount inside it? (Comment lines 8-11 promises the latter — currently UNVERIFIABLE, no rows exist.)
