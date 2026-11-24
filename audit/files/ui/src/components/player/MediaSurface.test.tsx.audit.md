# AUDIT: ui/src/components/player/MediaSurface.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A10) |
| Date | 2026-11-24 |
| Lines / functions | 275 / 5 tests + 4 fixtures (RecordingTransport, FakeInternals, encodeRecord, mockCanvas2d) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:80 (5 tests, 244ms); subject mounted at PlayerScreen.tsx:97,1062,1098 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (mount points, encodeRecord dup), wc -l subjects, it-count 5 == baseline |

## 1. One-paragraph verdict
Small but hostile and honest: it proves the browser/desktop mode split (canvas NEVER mounts
without `__TAURI_INTERNALS__`, zero transport commands leak in browser mode), the shell bind
(`session.set-frame-channel` with the transformCallback-allocated id 41, window._41 lifecycle,
unregisterCallback on unmount), 1:1 RGBA rasterization, the BGRA→RGBA draw-side swizzle, and
malformed-record resilience (corrupt magic + truncated header → console diagnostic, next frame
still renders). Fakes sit exactly at the environment boundaries (TAURI internals injection,
jsdom's absent 2D rasterizer) and say so in the header. The only defect is a duplicated
byte-layout encoder.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:80 `✓ src/components/player/
MediaSurface.test.tsx (5 tests) 244ms` (1705-passed run); 5 it() blocks on disk match.

## 3. Findings
F-001 [S4]
Where: ui/src/components/player/MediaSurface.test.tsx:80-95 (vs src/client/frame-channel.test.ts:37)
Code:
function encodeRecord(header: HeaderSpec, payload: Uint8Array): Uint8Array {
  const implied = header.width * header.height * 4;
  const out = new Uint8Array(LUF1.MIN_HEADER_LEN + implied);
  out.set(LUF1.MAGIC, 0);
What is wrong: this is a verbatim re-implementation of frame-channel.test.ts:37's encoder (same
name, same layout) — the LUF1 wire layout is now encoded by hand in two test files. A layout
change must be mirrored in both or the suites silently test a stale format.
Why it matters: duplicated wire-format fixtures are drift-prone. What correct looks like: export
a shared test-only encoder from one place (e.g. frame-channel.test or a testutil).
Confidence: HIGH — Verification: read (rg encodeRecord hit in both files).
No S0-S3: no tautologies — every assertion binds to DOM presence, transport payloads, window
registration state, or recorded putImageData bytes.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| browser mode: no canvas, no bind (140) | REAL | — | honest empty surface; 0 commands |
| shell mode: canvas+bind+deregister (158) | REAL | — | channel 41, window._41 lifecycle |
| RGBA frames render 1:1 (191) | REAL | — | canvas sized 3×2, bytes equal |
| BGRA swizzles draw-side (216) | REAL | — | channel swap asserted byte-wise |
| malformed record → diagnostic, no crash (246) | REAL | — | 2 warns; next frame renders |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 5 | YES | mode-split, cmd envelope, registration ids, byte equality and swizzle are all component-output-bound; mutation examples: drop swizzle → test 216 fails; forget unregister → 188 fails |
Mocking posture: RecordingTransport records (does not fabricate responses beyond unit completion);
FakeInternals fakes the webview injection seam (documented as tauri-transport.test.ts mirror);
mockCanvas2d records putImageData at the rasterizer boundary jsdom lacks. No mock of the subject. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "jsdom ships no canvas rasterizer — asserted at the seam" | header 5-7 | REAL | mockCanvas2d + recorded puts |
| "surface mounts ONLY inside the desktop shell" | header 2-3 | REAL | browser-mode test 140-156 |

## 7. Dead / duplicated / unreachable code
F-001 encoder duplication. Subject reachable (PlayerScreen mounts).

## 8. Dependencies & flags
In-repo only (ClientProvider, transport types, LUF1 constants, MediaSurface). No new deps.

## 9. Open questions for the next agent
Does the desktop build's real invoke path (ArrayBuffer fetch) ever get covered outside
tauri-transport.test.ts? This file rejects it by design (line 54-56).
