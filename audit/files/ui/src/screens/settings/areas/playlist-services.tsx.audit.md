# AUDIT: ui/src/screens/settings/areas/playlist-services.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 37 / 1 (PlaylistServicesArea) |
| File verdict | REAL (an honest empty seam — renders nothing, discloses why, mounted in the library area) |
| Reachable from shipped entry point? | YES in the web UI (library.tsx:28 imports + :56 mounts `<PlaylistServicesArea />` inside LibraryArea; route /settings/library; its own test pins the route renders the library area root). NO desktop binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `playlist\|service\|upnp\|podcast` over schemas/settings.toml → **zero matches** (the header's "NO entries exist" claim is TRUE); rg `PlaylistServicesArea` over ui/src → library.tsx:28/:56 (the mount) + playlist-services.test.tsx (the only other consumer); Read playlist-services.test.tsx (3 tests: zero rows / no invented options / routed); Read library.tsx:21-28,53-56 (the merge home + mount). Baseline: ui-vitest 136 files / 1705 passed. |

## 1. One-paragraph verdict
The most honest file of the assignment: a 37-line seam that renders an empty marked div because the schema it awaits has zero playlist/service keys (verified: no playlist, service, upnp, or podcast entry exists in schemas/settings.toml), says exactly that in its header, mounts inside the library area as documented, and is pinned by three tests asserting it renders no rows, invents no options, and sits behind the routed library area. The "CONFIG-NEVER-INVENT AUDIT" header's claims are all true. Nothing is wrong; the only observation is that the component's sole runtime output is an invisible div with data markers — a deliberate mount point, not a defect.

## 2. What actually works (verified)
— verified by playlist-services.test.tsx:14-29 (zero .settings-field rows, no [data-setting], no empty-area note, no area-root claim; no "discovery"/UPnP/podcast inventions) + :31-42 (the /settings/library route renders the library area root) + baseline ui-vitest (136 files, 1705 passed).
— verified by rg over schemas/settings.toml: zero playlist/service/upnp/podcast keys — the header's central claim.
— verified by library.tsx:53-56: the fold is mounted inside LibraryArea exactly as the header describes ("LibraryArea hosts this fold").

## 3. Findings
None. Checked and clean: header claims (all three verified — no schema keys, LibraryArea hosts it, one area root per route), mount wiring, no invented rows, no dead-looking controls, no imports beyond React types.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PlaylistServicesArea` | 25-36 | REAL | — | Returns an empty div with data-settings-fold + data-testid markers; deliberately nothing (documented seam); no props, no state, no transport. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :14 zero rows, no area-root claim, no empty note | YES | Negative DOM assertions — would fail if anyone added an invented row or re-claimed the area root. |
| :24 invents no service-discovery options | YES | Negative text assertions. |
| :31 routed at /settings/library | WEAK | Asserts the LIBRARY area root renders, not the fold itself — the fold's mount is only structurally verified via library.tsx (read), not by this test. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Service discovery toggles …: NO entries exist in schemas/settings.toml → rendered as NOTHING" | playlist-services.tsx:5-8 | REAL | rg over settings.toml: zero matches; test :14. |
| "LibraryArea hosts this fold" | playlist-services.tsx:2-3 | REAL | library.tsx:28/:56. |
| "the fold no longer claims the area root … exactly ONE [data-settings-area='library'] renders per route" | playlist-services.tsx:13-15 | REAL | Component carries no data-settings-area (read); test :16. |
| "the area-level empty note is gone: the library screen now carries the §C9 staged rows" | playlist-services.tsx:15-17 | REAL | library.test/library.tsx carry the staged §C9 rows (library.tsx:51 data-advanced-staged family — read). |

## 7. Dead / duplicated / unreachable code
None. The empty div is the designed seam output, not dead code.

## 8. Dependencies & flags
React types only — zero runtime imports. No tokens, no transport, no flags.

## 9. Open questions for the next agent
- When playlist-services schema rows land, this seam's consumers (SettingsSections via LibraryArea) should replace the empty div; the routing test should then assert the fold's own presence, not just the area root (see the weak-test note).
