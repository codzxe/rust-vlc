# AUDIT: ui/e2e/visual/manifest.ts
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / functions | 181 / 5 exports + slug/caseId + 2 grid builders |
| File verdict | REAL — the screenshot-diff matrix definition; math and route inventory verified by count |
| Reachable from shipped entry point? | NO — e2e test infra; consumed by ui/e2e/visual/visual.spec.ts:24 (`import { MANIFEST, caseId }`), driven by .github/workflows/ui.yml visual/visual-nightly lanes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | count ROUTES (25 ✓); rg consumers; rg CI workflow; baseline PNG dims cross-check |

## 1. One-paragraph verdict
A clean, countable matrix manifest: 25 routes (7 renderable + 13 settings
areas + onboarding/keyboard-shortcuts/open-network-stream + not-found +
/dev/tokens — hand-counted, matches the "full 25-route truth" header), ×2
viewports ×2 DPI ×2 themes = 200 grid cases + 2×8 accent samples = 216
nightly, and the `pr` flag picks 1280×720@1× dark+light = 50 PR-lane cases —
every count the header claims (L28-29) reproduces exactly. The accent anchor
story (grid anchored on `orange` == product default since W25) is documented
with its own past-discrepancy history (L98-108). Consumers and CI lanes
exist; baselines committed per-OS (linux present, windows/macos honestly
blocked per workflow comments).

## 2. What actually works (verified)
— Consumer: visual.spec.ts:24 imports MANIFEST/caseId; L32 filters
`MODE === "nightly" || c.pr`; L79-102 clips to #root's bounding box and calls
`toHaveScreenshot(`${id}.png`, { clip: box })`.
— Count checks (python-free hand count): ROUTES length = 25; GRID = 25×8=200;
ACCENT_SAMPLES = 16; MANIFEST = 216; pr-true subset = 25 routes × 2 themes =
50 — all match header claims.
— Name shape verified against disk: `settings-global-1280x720-1x-dark-violet`
== caseId({route:/settings/global,1280,720,1,dark,violet}) with the
default-accent empty suffix rule (L143).
— Capture-dims note verified: visual.spec.ts clips to #root, so PNG pixel
heights follow the app root's layout (e.g. onboarding baselines are 1280×704
at a 720 viewport — NOT a capture bug; see the onboarding PNG audits).

## 3. Findings
F-001 [S4]
Where: .github/workflows/ui.yml:29 vs ui/e2e/visual/manifest.ts:61-92
Code:
#     `visual` = PR-lite lane (1280x720 @1x, dark+light — 15 routes × 2
What is wrong: the workflow's lane comment still says "15 routes × 2" while
the manifest's w26.k20 expansion ships 25 routes (header L18-22: "the
inventory lagged the app by 9 routes … ROUTES is now the full 25-route
truth"); the PR lane actually runs 50 cases. Stale doc comment adjacent to
live code; the spec follows the manifest, so behavior is right and the
comment lies.
Confidence: HIGH — Verification: read (both files).
F-002 [S4]
Where: ui/e2e/visual/manifest.ts:112-127
Code:
/** The 9 accent ids (doc 08 §3.3), mirroring
 * src/tokens/generated/tokens.ts ACCENTS (kept in sync by that module's
 * committed-set test; duplicated here so the e2e tsconfig stays self-contained). */
What is wrong: deliberate cross-module duplication of the accent set — sync
is delegated to a test in the OTHER module ("kept in sync by that module's
committed-set test"), i.e. this list's correctness is only proven by a test
that does not live here. Self-declared and disclosed; the coupling is real
but pinned indirectly.
Confidence: HIGH — Verification: read (disclosure quoted).

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VisualCase | 33-46 | REAL | — | route×viewport×dpi×theme×accent×pr |
| VIEWPORTS/DPIS/THEMES | 48-57 | REAL | — | [PROPOSAL]-marked minimal set, honest |
| ROUTES | 61-92 | REAL | — | 25 counted; not-found caveat noted L88-90 |
| DEFAULT_ACCENT + notes | 94-110 | REAL | — | past discrepancy + resolution history in-file |
| ACCENTS / ACCENT_SAMPLE_ROUTES | 112-133 | REAL | S4 (F-002) | duplicated list |
| slug / caseId | 135-145 | REAL | — | Playwright name-sanitization rationale L140-141 |
| GRID / ACCENT_SAMPLES / MANIFEST | 147-181 | REAL | — | counts verified (§2) |

## 5. Test honesty
No tests in this file; it is the data source for visual.spec.ts (whose
filter logic is the consumer). The matrix itself is the honesty object —
counts reproduce.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Full matrix = 25 × 2 × 2 × 2 + 16 = 216 nightly cases; PR lane = 50" | manifest.ts:28-29 | REAL | hand count: 200+16=216; pr filter 25×2=50 |
| "ROUTES is now the full 25-route truth (…same inventory e2e/a11y.spec.ts walks)" | manifest.ts:18-22 | REAL-UNPROVEN | a11y.spec.ts:20 says "FULL routable inventory (App.tsx route tree: 24 product…)" — 24-vs-25 wording differs (a11y counts product routes only?); not reconciled line-by-line here. UNVERIFIED: exact a11y-list parity |
| "15 routes × 2" (PR lane) | ui.yml:29 | FALSE (stale) | manifest 25 routes |

## 7. Dead / duplicated / unreachable code
ACCENTS duplication (F-002). No dead exports (all consumed by visual.spec.ts).

## 8. Dependencies & flags
No imports at all (self-contained by design for the e2e tsconfig); consumed
by Playwright spec; CI: ui.yml visual + visual-nightly + visual-baseline-seed
(blocked on first real macOS/Windows runs — honest).

## 9. Open questions for the next agent
- visual.spec.ts owner: does the harness fail loudly if a baseline PNG dims
  drift from #root's current box (toHaveScreenshot handles it)? (My PNG
  audits show linux baselines present only — windows/macos lanes blocked.)
