# AUDIT: ui/scripts/l10n-pseudo-probe.mjs
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 197 / 4 (main flow, probeUrl, waitForServer, stopServer) + routes table |
| File verdict | REAL-UNPROVEN (runnable probe with documented hardening; zero committed run evidence — outputs gitignored, no report anywhere in the tree) |
| Reachable from shipped entry point? | NO — dev tool (`npm run l10n:pseudo-probe`, package.json). Not part of any shipped binary; requires Playwright chromium (absent in sandbox — baseline tooling gaps). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `grep -rn "locale" src/main.tsx` (:49 reads `?locale=pseudo` — the probe's premise is real); `head -60 src/i18n/pseudo.ts` (deterministic transform: brackets + accented glyphs + ≥1.5× filler + placeholder masking); `grep -rn "ellipsizedCount" docs ui audit` (report signature exists ONLY in the script — no committed output); `grep -n -A5 "l10n-pseudo-probe" ui/eslint.config.js` (:66-79 browser-globals block present — the P4-GATE fix landed); `grep -rn "pseudo-probe" docs/gates/phase4-exit.md` (the "clean per H3's committed evidence" claim cites 9b56338 with no artifact in-tree); `.gitignore:7-8` (pseudo-probe/ ignored); baseline #21-24 (probe NOT in the run set) |

## 1. One-paragraph verdict
A well-hardened Playwright probe — poll-not-sleep server readiness, fail-fast on dead process, finally-guarded teardown with process-group kill, onboarding-gate seeding so the walk photographs real screens instead of the wizard, explicit 127.0.0.1 binding after a live sandbox failure — whose premise is genuinely wired (`?locale=pseudo` is read by main.tsx:49 and backed by a deterministic pseudo.ts transform that brackets every string, which doubles as a hardcoded-literal detector). The problem is evidence, not engineering: the probe's output directory is gitignored, its JSON report signature (`ellipsizedCount`) exists nowhere outside the script, and the phase-4 gate's "pseudo-locale clean per H3's committed evidence" cites a commit hash whose artifact is not in the tree. The l10n clipping proof the gate claims as MET is therefore unverifiable, and the probe was never in the baseline run set.

## 2. What actually works (verified)
— verified by `grep -rn "locale" src/main.tsx:44-49` → the query param is read and routed to the i18n seam; `src/i18n/pseudo.ts` implements the bracket/accent/filler transform with placeholder masking (U+E000 sentinels) — the probe's target behavior exists.
— verified by `ui/eslint.config.js:66-79` → the browser-globals block for this exact file exists (the 6 no-undef errors phase4-exit.md:51 recorded on main are fixed at HEAD).
— verified by read → the five hardening claims in the header match the code: waitForServer (96-110) polls 250ms/60s with dead-process detection (:99-103); the walk is wrapped try/finally with `stopServer` (192-196); `stopServer` guards both kill paths (113-126); the onboarding seed is path-conditional (:147-155); spawn passes `--host 127.0.0.1 --strictPort` (:128-132).
— verified by read of :43 → `UI` is computed from `import.meta.url` — the p4-h3 hardcoded-path wart (phase4-exit.md §4.7) is fixed.

## 3. Findings

### F-001 [S3]
Where: ui/scripts/l10n-pseudo-probe.mjs:43-45 + .gitignore:7-8 vs docs/gates/phase4-exit.md:33
Code:
```
const PROBE_DIR = resolve(UI, "pseudo-probe");
mkdirSync(PROBE_DIR, { recursive: true });
```
What is wrong: the probe's outputs (24 PNG screenshots + the JSON report on stdout) go to a gitignored directory, and no committed artifact of any past run exists — `rg "ellipsizedCount"` (the report's field name) matches only the script itself. The phase-4 gate records "pseudo-locale clean per H3's committed evidence" (phase4-exit.md:33, commit 9b56338) — evidence that is a hash citation with no in-tree artifact, while the baseline run set (#21-24) never executes this probe.
Why it matters: the gate's l10n "MET" row leans on a clipping proof nobody can re-inspect; the probe could never have run (or could have reported 40 clipped strings) and the committed record would look identical. For an audit built on evidence-only, an unverifiable MET claim is a defect of the evidence chain, not necessarily of the probe.
What correct looks like: commit one representative JSON report (small, textual) under docs/gates/logs/ next to the phase-4 log, or paste the summary table into phase4-exit.md; keep PNGs ignored.
Confidence: HIGH — Verification: three greps (report signature, gitignore, gate doc) + baseline INDEX table read.
Cross-ref: W11's localization findings (orphaned crate, en-US hardcode) — this probe is the one mitigation lane that exists, and it too is evidence-less.

### F-002 [S4]
Where: ui/scripts/l10n-pseudo-probe.mjs:50-52
Code:
```
/** The full routable inventory (w26.k20): 12 non-settings renderable routes
 * (dev fixture excluded) + all 13 settings areas — the same truth the
 * visual manifest and the a11y sweep walk. */
```
What is wrong: the comment's arithmetic is wrong — the list below it contains 11 non-settings routes (player, player/music, miniplayer, playlist, media-info, discovery, convert, onboarding, keyboard-shortcuts, open-network-stream, no-such-route) + 13 settings areas = 24 walked. "12 non-settings renderable routes (dev fixture excluded)" would make 25; the visual manifest's 25-route truth INCLUDES /dev/tokens (baselines prove it: dev-tokens-*.png exist), so excluding the dev fixture leaves 11, not 12.
Why it matters: the file's self-description is the contract for "did the walk cover everything"; a miscount invites the next route to be silently skipped.
What correct looks like: "24 routes = 11 non-settings (dev fixture and redirect-only paths excluded) + 13 settings areas", or derive the list from App.tsx's router like the manifest does.
Confidence: HIGH — Verification: counted the array (:53-78); cross-checked against e2e/visual/baselines route census (dev-tokens PNGs exist).

### F-003 [S4]
Where: ui/scripts/l10n-pseudo-probe.mjs:175, 182
Code:
```
      for (const el of document.querySelectorAll("main *, [role] *")) {
```
What is wrong: two heuristic blind spots: (a) the clipping scan covers only leaf elements under `main` or under any `[role]` element — text rendered in portals/dialogs outside `main` (the app's Dialogs render into the app tree, but any body-level portal escapes) is never scanned; (b) `bracketed` is `document.body.textContent.includes("[")` — one literal `[` anywhere (a keyboard-shortcut legend, a "1 [of] 10" string) flags the whole page as containing leftover-untranslated text.
Why it matters: the report is explicitly "data, not failures" (:38-40), so a maintainer triaging from it can conclude "no clipping in dialogs" without the scan ever having looked there.
What correct looks like: scan `[aria-modal]`/portal roots too; for brackets, search for the pseudo sentinel pair (pseudo.ts wraps with specific bracket glyphs — match those exactly, not any `[`).
Confidence: HIGH for the code shape (read); MEDIUM for whether dialogs actually escape `main` (AppShell structure not traced — UNVERIFIED).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `routes` table | 53-78 | REAL | S4 | 24 walked; comment miscount F-002; 404 route included (good) |
| `probeUrl` | 81-93 | REAL | — | any <500 counts ready; 2s timeout; error→false — robust |
| `waitForServer` | 96-110 | REAL | — | 250ms poll / 60s cap / dead-process fail-fast — the k20 fix verified in code |
| `stopServer` | 113-126 | REAL | — | process-group kill, direct-pid fallback, both guarded |
| main flow (top-level) | 128-197 | REAL-UNPROVEN | S3 | spawn detached + strictPort + 127.0.0.1; onboarding seed conditional on path; networkidle + 700ms settle; screenshot per route; exitCode 1 only on load failures; F-001 evidence gap; F-003 heuristics |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (none) — no test covers the probe | — | dev tool; would fail loudly itself on infra breakage (exit 1 contract), but nothing pins that contract; the `npm run lint` pass (baseline via eslint config block) is the only automated touch |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "pseudo-locale clean per H3's committed evidence" | docs/gates/phase4-exit.md:33 | REAL-UNPROVEN (evidence untraceable) | no committed report; outputs gitignored (F-001) |
| "the probe now seeds lumen.onboarding.v1 … and walks the full 24-route inventory" | l10n-pseudo-probe.mjs:26-29 | REAL | code verified (:146-156, 24 routes) — modulo the 12-vs-11 comment (F-002) |
| "clipping findings are REPORT data, not failures; 1 = infra failure" | l10n-pseudo-probe.mjs:38-41 | REAL | exitCode set only from loadFailures (:188-191) |
| eslint: 6 pre-existing no-undef errors fixed by P4-GATE | phase4-exit.md:51 + eslint.config.js:66-79 | REAL | config block present at HEAD |

## 7. Dead / duplicated / unreachable code
None in-file. The `/dev/tokens` exclusion is deliberate and documented (:28-29).

## 8. Dependencies & flags
`@playwright/test` (chromium) — devDependency; node stdlib only otherwise. Not run in baseline (no browsers in sandbox — INDEX tooling gaps).

## 9. Open questions for the next agent
- REQUEST-ORCH-RUN (logged): `cd ui && npm run l10n:pseudo-probe` on a host with Playwright chromium — would either produce the missing F-001 evidence or expose live clipping findings the phase-4 gate recorded as clean.
- Does any portal/dialog in AppShell render outside `<main>`? If yes, F-003(a) is S3, not S4 — the AppShell wave can answer cheaply.
