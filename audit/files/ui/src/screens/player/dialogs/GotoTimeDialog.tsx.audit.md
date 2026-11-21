# AUDIT: ui/src/screens/player/dialogs/GotoTimeDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 156 / 1 (GotoTimeDialog) + reviewed its direct dependencies: parseTimeInput.ts (46), useDialogsMsg.ts (104), secondsToUs (wire-time.ts), formatSeekTime (slider/formatTime.ts) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain 1: PlayerScreen.tsx:1245 `{gotoOpen ? <GotoTimeDialog …>}` ← Ctrl+T hotkey match PlayerScreen.tsx:790-793 ← keymap.ts:87 `jump-to-time` DIALOG_KEYMAP, pinned by keymap.test.ts:174; call chain 2: ControlBar.tsx:404 tools-dialog edit lane. Both mount per-open) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read full file + parseTimeInput.ts + parseTimeInput.test.ts + GotoTimeDialog.test.tsx (211 lines) + useDialogsMsg.ts · `rg -n "GotoTimeDialog\|goto-time\|jump-to-time" ui/src` (both mount sites + keymap + i18n keys) · `rg -n "seek\|position\|durationSeconds" ui/src/screens/player/context.tsx` (actions.seek is documented ABSOLUTE :91-92, sends `playback.seek` µs target :378-387; seek-completed/seek-failed handling :458-463) · `rg -n "player.dialog.goto" ui/src/i18n` (all 9 keys registered central, strings identical to lane fallback) · `rg -n "order: -1" dialogs.css` (:77 — the header-paints-first claim verified) · `rg -n "formatSeekTime" ui/src` (M:SS/H:MM:SS, negative-safe, test-pinned Seekbar.test.tsx:154-159) |

## 1. One-paragraph verdict
This dialog is what the whole UI claims to be and rarely is: a control wired end-to-end to the one real engine lane. The input grammar (bare seconds / mm:ss / hh:mm:ss) is a pure, exhaustively-tested parser; submit dispatches `playback.seek` with an ABSOLUTE µs target through the same seam the seekbar uses; the duration cross-check uses only engine-reported truth and deliberately sets no ceiling when duration is unknown (live) — that exact case is test-pinned at the wire-payload level, as is the invalid-input refusal, the beyond-end refusal, the µs conversion (90 s → 90_000_000), the initial-focus landing on the input (with the CSS `order:-1` trick verified in dialogs.css:77), and both close affordances. The prefill-captured-at-mount contract is documented and honored by both production hosts. The only defects are hygiene: the parser accepts unbounded digit counts, so an absurd live-stream target crosses the wire and relies on the engine's i64 deserializer to refuse it (the dialog has already closed by then; the failure surfaces later as a typed error), and the component's `open` prop invites a keep-mounted usage pattern that would silently break the mount-time prefill contract that only prose documents.

## 2. What actually works (verified)
— The seek funnel is real: GotoTimeDialog.test.tsx:124-139 submits "1:30" and asserts the exact wire args `{ target: { kind: "absolute", time: 90_000_000 } }`; :141-154 pins the primary Go button through the same form ("1:02:05" → 3_725_000_000 µs). context.tsx:378-387 confirms `actions.seek` sends exactly that command shape (secondsToUs at the send edge).
— Honesty of refusal: "banana" → field error, ZERO `playback.seek` sent, dialog stays open (:156-165); "700" with a 600 s engine duration → beyond-end error carrying "10:00", no seek (:167-180); unknown duration → no invented ceiling, "9999" seeks (:182-195). All pinned against a RecordingTransport that records every send.
— Prefill contract: mount-per-open host test :100-122 emits `timing.time-tick` time=90_000_000 BEFORE opening → input prefills "1:30" (seam seconds → formatted, the µs→s conversion at context.tsx:441).
— The hotkey lane exists end-to-end: keymap.ts:87 `jump-to-time = { key = "Ctrl+T", ui_action = "goto-time", … when = "has-media" }`, matched at PlayerScreen.tsx:790-793, pinned by keymap.test.ts:133-176 and PlayerScreen.test.tsx:729.
— i18n seam: all 9 `player.dialog.goto.*` keys are registered in messages.ts:1472-1480/:2954-2962 with strings identical to the w26.k09 lane fallback — no hardcoded English in the component itself (repo-wide en-US-only limitation is the standing W11 finding, not this file's).
— parseTimeInput unit tests (7 cases) cover every grammar rule the header documents, including the negatives-as-invalid rule and "90:00" = 5400 s.

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/player/dialogs/parseTimeInput.ts:25-31 (consumed by GotoTimeDialog.tsx:67-84)
Code:
```
  if (parts.length === 1) {
    // Bare seconds — decimals allowed, sign never (a goto target is a
    // position, not a delta).
    if (!/^\d+(\.\d+)?$/.test(parts[0] ?? "")) return null;
    const seconds = Number(parts[0]);
    return Number.isFinite(seconds) ? seconds : null;
```
What is wrong: no magnitude bound. A 25-digit input parses to a finite float (e.g. 1e20 s); on a live stream (`durationSeconds === null`) the dialog's own cross-check is disabled by design, so `actions.seek(1e20)` dispatches `secondsToUs(1e20)` = 1e26 µs — two orders of magnitude beyond i64 range (~9.2e18 µs).
Why it matters: the dialog closes immediately (`onClose()` at :85) and the refusal arrives asynchronously as a transport/typed error in the player's ErrorSurface — the user who typed a far-future time on a live stream gets a closed dialog and a delayed opaque engine error instead of the same honest field error the invalid-grammar path produces. Engine-side rejection of the oversized i64 is UNVERIFIED in this sandbox (no runnable GUI host; the engine's serde behavior on `1e+26` for `Timestamp` is inferred, not observed).
What correct looks like: bound the parse (`seconds > 1e12` → the same `player.dialog.goto.invalid` field error, or a dedicated too-large message) — the bound belongs in the parser next to the 0..59 field rules it already enforces.
Confidence: MEDIUM — Verification: read + wire-path trace (context.tsx:378-387, wire-time.ts:26-28); engine-side overflow behavior UNVERIFIED.

### F-002 [S4]
Where: ui/src/screens/player/dialogs/GotoTimeDialog.tsx:45-56, 60
Code:
```
 * The Go-to-Time dialog. Mount per-open (fresh state — the input prefills
 * with the engine-reported position at open time, VLC parity): render as
 * `{open ? <GotoTimeDialog open onClose={…} /> : null}`.
```
```
  const [value, setValue] = useState(() => formatSeekTime(state.position));
```
What is wrong: the mount-time prefill (and the error state) is correct only under the mount-per-open contract, which exists solely in this docstring. The component's public shape (`open: boolean` prop) is the standard toggle-dialog shape; a host that renders `<GotoTimeDialog open={open} …/>` unmounted-toggle style gets a prefill frozen at first mount and stale error state across opens. Both current hosts comply (PlayerScreen.tsx:1245, ControlBar.tsx:404 — both conditional-mount), so this is latent, not live.
Why it matters: the API invites the misuse; nothing in the component (e.g. re-prefill on `open` transitions) or in types enforces the contract.
What correct looks like: either drop the `open` prop entirely (the Dialog already no-ops when closed) or reset value/error in an effect keyed on `open` transitions.
Confidence: HIGH (latent contract gap; both hosts verified compliant) — Verification: read + rg of both mount sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| GotoTimeDialog | 50-156 | REAL | S4 | prefill-at-mount (F-002 contract); parse → duration cross-check (engine truth only, null = no ceiling) → absolute seek → close; error cleared on input change; focus order via DOM order + CSS order:-1 (verified dialogs.css:77) |
| — dependency: parseTimeInput | parseTimeInput.ts:19-45 | REAL | S4 | grammar correct + 7-case test file; F-001 unbounded magnitude; precision loss beyond 2^53 silently accepted (sub-case of F-001) |
| — dependency: useDialogsMsg | useDialogsMsg.ts:38-44 | REAL | — | central-first with lane fallback; all goto keys central-registered (verified) |
| — dependency: secondsToUs | wire-time.ts:26-28 | REAL | — | the single documented µs conversion factor |
| — dependency: formatSeekTime | slider/formatTime.ts:8 | REAL | — | M:SS / H:MM:SS, negative-safe; test-pinned |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :76 initial focus on input, not header ✕ | YES | focus assertions (the w26.i09 body-first DOM trick) |
| :88 renders with position prefilled "0:00", inputmode text, "--:--" | YES | value + attribute assertions |
| :100 nonzero prefill via per-open mount after engine tick | YES | pins the mount-time capture semantics |
| :124 submit mm:ss → wire `{absolute, 90_000_000}` + closes | YES | exact wire-args assertion incl. the µs conversion |
| :141 Go button → same form → 3_725_000_000 | YES | footer button wired via form id |
| :156 unparseable → error, no seek, stays open | YES | transport recorded, zero playback.seek |
| :167 past duration → error with formatted duration, no seek | YES | engine duration pushed through the event seam |
| :182 unknown duration → no ceiling, 9999 seeks | YES | the live-stream design decision is pinned |
| :197 Escape closes; :204 quiet Close closes without seeking | YES | both affordances asserted |
| parseTimeInput.test.ts (7 its) | YES | each grammar rule asserted incl. rejections |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped). Tests mock the TRANSPORT, not the subject — the dialog+seam is the exercised code; the engine's seek execution is separately evidenced by baseline rc-smoke (AVI lane seeks work).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "submits … `playback.seek` with an ABSOLUTE target … the same funnel the seekbar uses" | header :4-9 | REAL | context.tsx:91-92, :378-387; test :124-139 |
| "seek truth arrives via `timing.seek-completed` / `timing.seek-failed` (the pending contract lives in the seam)" | header :8-9 | REAL | context.tsx:458-463 handles both events |
| "HOTKEY: keymaps/default.toml's `jump-to-time` (Ctrl+T …)" | header :11-15 | REAL | keymap.ts:87; PlayerScreen.tsx:790-793; keymap.test.ts:174-176 |
| "input prefills with the engine-reported position at open time, VLC parity" | :46-48 | REAL | test :100-122 (mount-time capture) |
| "no ceiling is invented" for unknown duration | :72-74 | REAL | test :182-195 |
| "minutes may exceed 59: '90:00' is 5400 s" | parseTimeInput.ts:5 | REAL | parseTimeInput.test.ts:14+ |
| "VLC rejects '1:75' too" | parseTimeInput.ts:34 | REAL-UNVERVENGED (code rejects it) / VLC-parity claim UNVERIFIED | parseTimeInput.test.ts:49 pins the rejection; VLC's actual behavior not runnable here |

## 7. Dead / duplicated / unreachable code
- None. Every rendered element has a role in the grammar; no dead props beyond the latent `open` shape (F-002).

## 8. Dependencies & flags
- Button/IconButton/TextField/Dialog primitives, slider formatSeekTime, player context (usePlayerControls), dialogs.css, i18n lane seam. No feature flags. Wire dependency: `playback.seek` in the generated command catalog (same command the seekbar sends).

## 9. Open questions for the next agent
- Engine-side: what does `playback.seek` do with an absolute target beyond media end (live/no-duration) — clamp, error, or no-op? Determines whether F-001's failure mode is an honest typed error or a silent no-op with `pendingSeek` stuck until the tolerance clear (context.tsx:447).
- BookmarksDialog shares this grammar partially (its jump path) — same magnitude question applies there.
