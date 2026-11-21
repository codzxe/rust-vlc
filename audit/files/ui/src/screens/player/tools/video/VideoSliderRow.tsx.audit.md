# AUDIT: ui/src/screens/player/tools/video/VideoSliderRow.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 44 / 2 (VideoSliderRowProps, VideoSliderRow) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the dev/test shell — 2 production consumers (EssentialTool.tsx:137, ColorsTool.tsx:~256 for saturation/similarity/sepia rows) → ToolsDialog.tsx:134-137 (mounted player tools dialog) and the settings effects area; exported via tools/video/index.ts. No shipped binary exists (W14 pin). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | tools/shared/SliderRow.tsx read in full (memo:144, tick math:104-133, resolveLabel generic); EssentialTool.tsx:40-145 + ColorsTool.tsx:259-261 read (inline sinks, zero useCallback — rg -c: no matches); dynamics.css:168-250 read (track/fill/thumb/tick grammar vs header claims); `rg -o 'video\.set-[a-z-]+'` over catalog ids.rs (aspect-ratio/crop/zoom/delay/fullscreen/screenshot — NO set-colors/set-essential/set-hdr/set-adjust) + feature-matrix.toml (video.set-adjust ×3, D-012); Grep VideoSetCrop across all .rs → only the 3 commands-catalog files, zero executors; messages.ts:2429 staged note copy; EssentialTool.test.tsx row pins read |

## 1. One-paragraph verdict
An honest 3-line composition of the shared SliderRow — but its header makes one false claim and its users make one false assumption. False claim: "React.memo'd — a drag on one row no longer re-renders the card's other rows (j11 P3-5)" — both production consumers pass an inline arrow `onChange`, a fresh function identity on every card render, so the memo's shallow compare fails for EVERY row on EVERY drag tick; the optimization is dead weight and the comment asserts behavior the code does not deliver. False assumption (admitted in the prop doc): "Live + commit sink … no engine yet" — the sliders are interactive-but-inert local state, and the commands the staged markers await (video.set-essential / video.set-colors) exist in NO catalog, while the matrix's actually-planned command is video.set-adjust and even the one existing adjacent id (video.set-crop) has no executor anywhere. The visual grammar itself is excellent and fully verified.

## 2. What actually works (verified)
- Composition real: `<SliderRow row={row} value={value} onChange={onChange} resolveLabel={m} />` with `m = useVideoMsg()` (copy.ts:23) — the generic label-key seam the header claims (SliderRow.tsx:63-81); the former 1:1 copy of ToolSliderRow is gone.
- Grammar claims verified against dynamics.css (the shared sheet both lanes ride): track 16px height on 328px-wide scope, rx = --radius-control (frame rx6), fill inset 2px/12px band rx4 = --color-accent-tinted (frame #371F06), thumb 8×24 rx4 = --color-accent-strong (frame #AB6314), flush thumb/fill edges (dynamics.css:191-207 with three frame citations), ticks 2×8 round-cap pills on the centerline, covered tick visibility:hidden (frame z-order).
- i07 tick domain verified: SliderRow.tsx:128 `left: calc(2px + (100% - 4px) * i/(n+1))` matches the comment's frame measurement ("x = 38 + 324·i/11 on the x36–364 track": 36+2=38, 328−4=324) — the corrected domain this header says the old video copy lacked.
- "NO value column" verified: video.css hides the shared value span inside .video-tool (pinned by harness.test.tsx's css regex, cross-ref file-17 audit); aria-valuetext still reports via Slider formatValue (SliderRow.tsx:101).
- Staged honesty markers real: VideoCard renders `data-staged-tool`, `data-pending-command`, and a user-visible role="note" — "Staged — controls activate with the video effects command card" (messages.ts:2429).
- Tests: EssentialTool.test.tsx pins all 8 label-only slider rows by accessible name (FRAME_ROWS loop, :43-48) plus collapsed/expanded disclosure state; ColorsTool.test.tsx carries the aria-pin family (i08 log). Baseline vitest 1705/1705 (INDEX #23).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/tools/video/VideoSliderRow.tsx:9-11 (claim) + EssentialTool.tsx:140-143, ColorsTool.tsx:259-261 (cause)
Code:
```tsx
// is React.memo'd — a drag on one row no longer re-renders the card's other
// rows (j11 P3-5).
```
```tsx
          onChange={(next) => {
            setValues((prev) => ({ ...prev, [row.id]: next }));
          }}
```
What is wrong: The memo claim is false at HEAD. Both consumers pass an inline arrow sink, so every re-render of the card produces a NEW `onChange` identity; `memo(SliderRowInner)` (SliderRow.tsx:144) does a shallow prop compare, the `onChange` prop differs for every row, and the compare fails — all rows re-render on every drag tick (each running its i18n seam call and tick math, exactly the cost the comment says was eliminated). Neither consumer uses useCallback (rg: zero matches in both files).
Why it matters: A perf fix that does not hold is worse than no claim — the comment is cited to a review ruling (j11 P3-5) and will be trusted. On a live drag (onChangeLive fires per pointer move) this re-renders the whole card's rows per move.
What correct looks like: In each consumer, one `useCallback`'d sink keyed by row id (`const setRow = useCallback((id, v) => setValues(p => ({...p, [id]: v})), [])` bound per row via a stable wrapper), or a single sink passed `row.id` — then the memo actually engages.
Confidence: HIGH — Verification: read; memo implementation and both consumers' sink construction traced; no useCallback anywhere in either file.

### F-002 [S3]
Where: ui/src/screens/player/tools/video/VideoSliderRow.tsx:32-33 (admission) + catalog/matrix evidence
Code:
```tsx
  /** Live + commit sink (both drive the same local state — no engine yet). */
  readonly onChange: (value: number) => void;
```
What is wrong: Every slider this row renders is interactive-but-inert: values live in card-local useState and are never dispatched anywhere. The awaited commands do not exist: `video.set-essential` / `video.set-colors` appear in NO catalog (crates/commands/src/catalog/ids.rs enumerates video.set-{aspect-ratio, crop, zoom, delay, fullscreen, take-screenshot} only); the feature matrix's planned seam is `video.set-adjust` (feature-matrix.toml, D-012 filter cards; engine-side adjust.rs with CPU reference + 48 tests EXISTS but nothing wires it); even `video.set-crop` — the one marker name that IS a catalog id — has zero executors (Grep VideoSetCrop across all .rs: only the 3 commands-catalog definition files). Deeper: the GUI mounts no canvas (Wave-0 pin), so there is no video display these sliders could ever adjust.
Why it matters: A user drags gamma/hue/saturation, sees values and degree ticks move, and nothing changes anywhere — the classic dead-control-that-looks-alive. Kept at S3 (not S2) ONLY because the UI is honestly marked: the visible "Staged — controls activate with the video effects command card" note + data-staged-tool/data-pending-command markers (VideoCard.tsx:73-101), the same honesty grammar as the staged settings (cross-ref W15-A08 file 11) and the W8 audio.set-eq pin.
What correct looks like: Land the D-012 command (video.set-adjust) against the existing adjust.rs parameter model and rewire the sinks; until then the markers at least keep the contract visible.
Confidence: HIGH — Verification: read; catalog ids enumerated; matrix + D-012 read; VideoSetCrop executor search over the whole tree.

### F-003 [S4]
Where: ui/src/screens/player/tools/video/VideoSliderRow.tsx consumers — EssentialTool.tsx:133, ColorsTool.tsx:224, HdrTool.tsx (data-pending-command values)
Code:
```tsx
      pendingCommand="video.set-essential"
```
What is wrong: The forward-declaration contract ("data-pending-command naming the forward-declared command they await", VideoCard.tsx:29-30) names commands that are declared NOWHERE: set-colors, set-essential, set-hdr match no catalog id and no matrix row; the matrix's planned id is video.set-adjust. Only CropTool's video.set-crop matches a real (unexecuted) id.
Why it matters: The marker exists to make the eventual wiring traceable; pointing at phantom names means a Lead landing video.set-adjust leaves every marker stale and grep-for-wiring finds nothing.
What correct looks like: Either register the intended ids in the catalog spec (ids.rs carries planned ids already) or align the markers on video.set-adjust per the matrix.
Confidence: HIGH — Verification: read; ids.rs + matrix + ROADMAP D-012 cross-checked.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `VideoSliderRowProps` | 27-34 | REAL | S3 | honest "no engine yet" admission (F-002); row/value/onChange shape matches shared SliderRowProps minus the generic (VideoSliderRowDef fixes TKey = VideoLabelKey — the lane freeze types.ts:9 documents) |
| `VideoSliderRow` | 37-44 | REAL | S3 | pure composition, no local state; the memo behavior it inherits is defeated by consumers (F-001) |

## 5. Test honesty (if tests exist in/for this file)
No dedicated VideoSliderRow.test; covered through the two tool tests.
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| EssentialTool.test.tsx:43-48 "renders the eight label-only slider rows, in frame order" | YES | role=slider queries by frame label, count pinned at 8 |
| EssentialTool.test.tsx disclosure tests (:26-38) | YES | aria-expanded/aria-controls wiring through VideoCard |
| ColorsTool.test.tsx aria pins (per i08 log: "aria pins + jsdom contract") | YES | same grammar for the 3 colors rows |
| Mutation check (reasoned): strip `resolveLabel={m}` (render raw keys) | YES | slider name queries would fail (labels become msgid strings) |
| Mutation check (reasoned): defeat the memo (already defeated — F-001) | NOT caught | no test asserts render counts; the false perf claim is invisible to the suite |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the row itself now lives in ONE place — tools/shared/SliderRow … generic over the label key with a resolveLabel prop" | VideoSliderRow.tsx:2-4 | REAL | SliderRow.tsx:42-81,144; ToolSliderRow composes the same |
| "React.memo'd — a drag on one row no longer re-renders the card's other rows (j11 P3-5)" | VideoSliderRow.tsx:9-11 | **FALSE at HEAD** | inline sinks in both consumers defeat the shallow compare (F-001) |
| tick-domain drift numbers (gamma −1.6px, grain ±1.6, banding ±1.4, sharpen ±1, 2-tick ±0.7) | VideoSliderRow.tsx:7-9 | UNVERIFIABLE (historical) | the pre-k10 copy is deleted; the CORRECTED domain is verified (§2) |
| "track 328×16 rx6 #090909, flat #371F06 fill 12px rx4 inset 2, thumb 8×24 rx4 #AB6314, optional degree ticks" | VideoSliderRow.tsx:17-20 | REAL | dynamics.css:175-250, token-mapped equivalents |
| "video.css hides the shared value span; the slider's aria-valuetext still reports every value" | VideoSliderRow.tsx:15-16 | REAL | video.css rule pinned by harness.test.tsx regex; formatValue wired (SliderRow.tsx:101) |
| (adjacent, matrix) "UI sliders render inert-with-reason until the command lands" | docs/feature-matrix.toml | REAL | staged note + markers verified |

## 7. Dead / duplicated / unreachable code
- No duplication left (the dedup is real). Dead weight: the memo wrapper is currently ineffective in both production consumers (F-001) — not dead code, defeated code.
- The value span (`dynamics-row__value`, SliderRow.tsx:90) renders into a display:none context for every video row — kept deliberately for the DOM contract (documented, tested).

## 8. Dependencies & flags
Imports: tools/shared SliderRow, ./copy useVideoMsg, ./types VideoSliderRowDef. No engine/bridge imports — nothing to dispatch to (F-002). No feature flags. The i18n seam resolves VideoLabelKey through the lane catalog (copy.ts:23-27).

## 9. Open questions for the next agent
- The dynamics lane's ToolSliderRow likely suffers the same memo-defeat (same inline-sink pattern) — worth checking when its file comes up; the j10/j11 review rulings assumed the memo worked.
- If D-012 lands video.set-adjust, the three phantom marker names (F-003) need a sweep: rg 'data-pending-command="video.set-' across ui/src.
