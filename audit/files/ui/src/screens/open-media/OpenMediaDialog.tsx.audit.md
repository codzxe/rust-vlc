# AUDIT: ui/src/screens/open-media/OpenMediaDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 327 / 7 |
| File verdict | REAL (one genuine S2 ordering defect in the Recent list; the open flow itself is real code over the one funnel — but only the DEMO transport answers it in any buildable configuration) |
| Reachable from shipped entry point? | YES (web app build): AppShell.tsx:83 imports, :314 listens for `lumen:open-media`, :851 mounts `<OpenMediaDialog open=…>` — the shell-wide dialog every surface triggers (PlayerScreen empty-state CTA, canvas menu, File menu). Baseline #22 build OK, #23 vitest 1705 passed incl. OpenMediaDialog.test.tsx (20 cases). No GUI binary hosts it (W14-A01); the real-host transport lane (app-transport → tauri) does not compile. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `Read` OpenMediaDialog.tsx 1-327 (complete); `rg -n "recentOpenings" -B2 -A4 + sed -n '978,990p' demo-transport.ts` (getter doc "most recent first" vs prepend-then-reverse = oldest-first — F-001 root cause); `rg -n "describe|it(" OpenMediaDialog.test.tsx` (20 cases — NONE covers the Recent section/ordering); `rg -n "OpenMediaDialog|OPEN_MEDIA_EVENT" AppShell.tsx` (mount + listener chain); `rg -n "openPathRequest|openPathRoute" -A 12 open-path.ts` (the ONE funnel: session.open / playlist.load — real command envelopes); `sed -n` verification of every quoted range. |

## 1. One-paragraph verdict
This is the one "wired to a real backend seam" answer in my file list — and the wiring is genuine: every open (row click, Enter, pasted path, footer Open) goes through `openPathRequest` → `session.open` / `playlist.load` over the ClientProvider transport, with a correct one-in-flight guard, request supersession, honest blank-path invalidation, demo-clip gating that hides fake clips from non-demo hosts, and 20 behavioral tests. Two defects: the Recent section renders OLDEST-first because the demo transport's `recentOpenings` getter contradicts its own "most recent first" doc comment (the same defect PlayerScreen's seam works around with `.at(-1)` — the dialog has no such compensation, and no test covers the section); and the transport error is swallowed (`.catch(() => {})`, documented as delegated to the player's error surfaces). The standing caveat: in every buildable configuration the transport answering `session.open` is the DEMO transport synthesizing media info — the real host lane (tauri) cannot compile at HEAD (W14-A01), so "open media" in the browser shell opens a simulated session whose file sources mount a src-less video surface (PlayerScreen part2 F-001).

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): all 20 OpenMediaDialog.test.tsx cases — single-click open sends the real `session.open` envelope and routes to /player; playlist path sends `playlist.load` append:false and routes to /playlist; pasted URL opens through the same seam; blank path honestly invalid; close affordances; first-field focus (incl. re-open focus); demo-clip gating for non-demo browser hosts and the desktop shell; plain-activation rows; footer-Open submit association.
— Verified by read: `openSource` (142-162) — `if (opening) return` single-flight guard, per-activation `requestId` supersession check in `.finally` (a late resolution of a superseded open cannot clobber the newer one's close/route); state reset on completion.
— Verified by read: `parseSourceInput` (81-86) — scheme regex `^[a-z][a-z0-9+.-]*:\/\//i` correctly leaves `C:\Users\…` as a file path (no `://`), and empty input returns null (honest invalid).
— Verified by read: the ONE funnel is real — `openPathRequest`/`openPathRoute` (open-path.ts:58-75) classify and emit genuine catalog command envelopes.

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/open-media/OpenMediaDialog.tsx:124 (read), 264-273 (render, oldest-first); root cause: ui/src/client/demo-transport.ts:353-356 (getter) + 982-984 (prepend)
Code:
```tsx
  const recent = isDemoTransport(client) ? client.recentOpenings : [];
```
and (render):
```tsx
              <ul className="open-media__list">
                {recent.map((entry) => (
                  <li key={sourceLocation(entry.source)}>
```
What is wrong: the Recent list renders in the transport's array order, and the demo transport's `recentOpenings` getter returns OLDEST-first while its own doc comment — which names THIS dialog as its consumer — promises "The open-media dialog's 'Recent' section (most recent first)". Root cause verified: `rememberOpening` PREPENDS the newest entry (demo-transport.ts:982-984) and the getter `.reverse()`s (355), flipping newest-first storage into oldest-first output. The dialog does no compensation; PlayerScreen's media-surface seam compensates the same bug with `.at(-1)` (PlayerScreen.tsx:718-719, its ORDERING NOTE documents the defect), proving it is known and shipped anyway.
Why it matters: wrong behavior in a realistic case (S2): a browser-shell user who opens clip A then clip B sees "Recent" list A above B — the opposite of every Recent list convention and of the transport's own documented contract. No test covers the Recent section at all (20 cases, zero touch it), so the regression gate is blind to it.
What correct looks like: fix the getter (delete the `.reverse()`), flip PlayerScreen's `.at(-1)` seam to `[0]` per its own ORDERING NOTE, and add a Recent-ordering test to this dialog's suite.
Confidence: HIGH — Verification: read (both demo-transport sites and both consumers traced; PlayerScreen's compensation comment cross-verified in my part2 audit).

### F-002 [S4]
Where: ui/src/screens/open-media/OpenMediaDialog.tsx:81-86
Code:
```tsx
export function parseSourceInput(raw: string): SourceSpec | null {
  const value = raw.trim();
  if (value.length === 0) return null;
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(value)) return { kind: "url", url: value };
  return { kind: "file", path: value };
}
```
What is wrong: a `file:///path` input matches the scheme regex and becomes `kind: "url"` — the file:// scheme is a file by intent, but it is dispatched as a url source (`session.open { source: { kind: "url", url: "file:///…" } }`). Engine-side handling of a file:// URL through the url lane is unverified (and the demo transport merely synthesizes info for it).
Why it matters: an edge input class silently routes down the wrong source lane; in a real host this either errors at open or double-handles the path.
What correct looks like: special-case `file://` to `{ kind: "file", path: decodeURIComponent(new URL(value).pathname) }` or reject it with the honest invalid copy.
Confidence: MEDIUM — Verification: read (classification traced); the engine's url-lane file:// behavior UNVERIFIED (would need a run against a real transport — none exists, W14).
### F-003 [S4]
Where: ui/src/screens/open-media/OpenMediaDialog.tsx:147-152
Code:
```tsx
    void client
      .send(openPathRequest(source, requestId))
      .catch(() => {
        // Transport unreachable — the player screens surface their own
        // honest typed errors; the dialog still closes and routes.
      })
```
What is wrong: the transport error is swallowed with an empty catch; the dialog then closes and routes to the player unconditionally. The comment documents the delegation (the player's StateRouter renders the error overlay from engine/transport truth), so the error is not lost — but nothing in THIS component distinguishes "opened" from "failed to even send" on close.
Why it matters: a user whose open request never reached the transport sees the dialog close and the player appear — correct only because the player's empty/error states are honest; the component itself has no failure feedback path.
What correct looks like: if the delegation is the design, assert it in a test (no such test exists — the catch branch is untested); otherwise surface a non-blocking inline error in the dialog before close.
Confidence: HIGH — Verification: read (branch traced; test suite grepped — the `.catch` path has zero coverage).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `sourceLocation` | 62-63 | REAL | — | Trivial data selector. |
| `sourceIcon` | 71-73 | REAL | — | Two-glyph vocabulary; no invented folder kind; exported for contract test. |
| `parseSourceInput` | 81-86 | REAL | F-002 | Scheme/file split; blank → null; file:// misclassification noted. |
| `OpenMediaDialog` (component) | 88-295 | REAL | F-001, F-003 | See sub-rows. |
| — focus effect | 118-120 | REAL | — | Re-focus on every OPENING (AppShell keeps mounted); runs after the trap effect — the documented j16 P2-3 family fix. |
| — recent/showClips gating | 122-130 | REAL | F-001 | Recent = demo transport only; clips = browser+demo only (honest fixture gating, test-pinned). |
| — `openSource` | 142-162 | REAL | F-003 | One-funnel send; single-flight; requestId supersession; state reset; swallowed transport error (documented delegation). |
| — `onPathSubmit` | 166-174 | REAL | — | Honest blank invalid; same funnel. |
| — `clearAndClose` | 176-180 | REAL | — | State reset + close. |
| — render (Dialog/form/sections/footer) | 182-294 | REAL | F-001 | Dialog primitive chrome; form + footer-Open submit association (always live); Recent section oldest-first (F-001); full-location rows with title attr. |
| `SourceRow` | 300-326 | REAL | — | Plain activation button; full two-line location; no focus-as-selection. |

## 5. Test honesty (OpenMediaDialog.test.tsx, 20 cases, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parseSourceInput contract (86-106) | YES | Real classification cases incl. scheme'd and blank. |
| single-click/Escape/close/focus suites (108-260) | YES | Transport-spy assertions on the exact command envelope; routing assertions; focus placement on open AND re-open. |
| demo-clip gating (262-301) | YES | Three-way gate: non-demo browser host, demo browser, demo desktop — pins the honest-fixture rule. |
| plain-activation rows + live footer Open (303-350) | YES | No aria-pressed; footer submits the form; blank → invalid. |
| affordance glyphs (352+) | YES | Icon-name assertions. |
| NOT covered: Recent section (F-001), `.catch` failure path (F-003), file:// input (F-002) | — | Zero tests touch the Recent list or the failure branch — the gaps the findings live in. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "any surface … asks for the open-media flow by dispatching `lumen:open-media`; the AppShell owns the single listener and mounts this dialog" (header 4-8) | this file | REAL | AppShell.tsx:314 listener + :851 mount; PlayerScreen canvas menu and empty-state CTA dispatch it (part2 audit). |
| "opens the picked SourceSpec through the ClientProvider transport (`session.open` / `playlist.load` — the ONE funnel, client/open-path.ts), then routes to the player/playlist" (25-28) | this file | REAL | open-path.ts:58-75 verified; tests :127/:164/:182 assert the envelopes (baseline #23). |
| "a blank path input is honestly invalid — no fabricated source" (36) | this file | REAL | parseSourceInput null + test :201. |
| (implicit) "Recent" lists recent openings | the section title + demo-transport.ts:353 "most recent first" | FALSE (partially) | F-001: actual order is oldest-first; getter contradicts its own doc. |

## 7. Dead / duplicated / unreachable code
- No dead code. All exports consumed (OPEN_MEDIA_EVENT by AppShell + tests; parseSourceInput/sourceIcon by tests).
- Unreachable as a shipped binary (W14) — global UI condition; in the browser build the dialog is fully reachable and the demo transport answers.

## 8. Dependencies & flags
- Dialog/Separator (surface), TextField (form), Button/IconButton, Icon, ClientProvider + demo/app transport guards, open-path funnel, react-router. No feature flags.

## 9. Open questions for the next agent
- Whoever owns `client/demo-transport.ts` (not in my list): F-001's fix instructions are already written in PlayerScreen.tsx:710-717 (delete the getter's `.reverse()`, flip `.at(-1)` → `[0]`, update the player test) — add a Recent-ordering test here when doing it.
- Whoever owns `client/open-path.ts` (w15-g003 asset group, another agent): does `classifyOpenSource` handle `.m3u8`/`file://`? The dialog delegates all classification to it.
