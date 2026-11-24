# AUDIT: ui/src/client/frame-channel.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A01 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 499 / 24 (3 describes) |
| File verdict | REAL (baseline-verified green; mutation-hard) |
| Reachable from shipped entry point? | YES (test for ui/src/client/frame-channel.ts, consumed by ui/src/components/player/MediaSurface.tsx in the desktop player surface) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline (ui-test.txt:43); rg source + consumers |

## 1. Verdict
A genuinely strong unit suite: a hand-written golden LUF1 record (byte-by-byte, independent of
the in-test encoder), typed refusal checks on every malformed input class, and a shim suite that
pins message shapes, ordering, dedup, the fetch-dance invoke shape, idempotent close, and the
end marker. Baseline proves it runs and passes: ui-test.txt:43
`✓ src/client/frame-channel.test.ts (24 tests) 416ms`.

## 2. What actually works (verified)
- Baseline vitest run green (cite above), 24 tests, no skips recorded.
- Golden independence: GOLDEN_2X2 (:82-103) spells magic/header/geometry/payload literally;
  the encoder helper (:37-57) is a second mirror — a decoder regression cannot hide behind it.
- Refusals assert the TYPED kind, not message text (:106-113 `expectRefusal` -> FrameWireError.kind),
  and every kind asserted (bad-magic, unsupported-version, bad-header-len, unknown-format-code,
  reserved-not-zero, bad-geometry, payload-length-mismatch, truncated, trailing-bytes,
  unsafe-integer) exists in the source (frame-channel.ts:115-155 throws match).
- Mutation reasoning: breaking header validation, BGRA passthrough, reorder buffer, duplicate
  drop, close idempotency, or the end marker each fails a dedicated test. The close test even
  checks late-message-after-close delivery is dropped (:454-460).
- decodeLuf1Record delegates to parseLuf1Header (frame-channel.ts:192) — the 10 refusal tests
  exercise the exported parser's validation indirectly.

## 3. Findings
F-001 [S4]
Where: ui/src/client/frame-channel.test.ts:242-246
Code:
describe("parseLuf1Header (the exported header parser)", () => {
  it("parses the golden header alone", () => {
    expect(parseLuf1Header(GOLDEN_2X2).frameId).toBe(7);
  });
});
What is wrong: the exported parseLuf1Header has exactly ONE direct assertion (frameId only);
all its validation branches are covered only through decodeLuf1Record. Anyone calling
parseLuf1Header directly (its own export implies such callers) gets untested-at-the-edge
behavior coverage. Not a lie — a thin spot.
Confidence: HIGH — Verification: read (rg 'parseLuf1Header' over the test file: 1 hit).
F-002 (observation, no severity): the tauri shell message shapes ({message,index},
{message,id}, {end:true}) are pinned from tauri 2.1.x docs, never observed against a real
webview (test header cites INFRA-BLOCKED.md; no webview in sandbox). Shell-shape fidelity is
UNVERIFIED — the mock mirrors documented shapes, not the shell itself.

## 4. Per-function/per-block table (>100 LOC)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| encodeRecord fixture | 37-57 | REAL | — | asserts payload/geometry consistency |
| GOLDEN_2X2 / RGBA_2X2 | 60-103 | REAL | — | hand-written bytes |
| expectRefusal | 105-113 | REAL | — | typed kind assert |
| decodeLuf1Record suite | 119-240 | REAL | — | 12 tests incl. 10 refusals |
| parseLuf1Header suite | 242-246 | SHALLOW | S4 | 1 happy-path assert |
| FakeInternals | 254-291 | REAL | — | scripted seam mock, queues answers |
| FrameChannelShim suite | 304-473 | REAL | — | 11 tests: order/dedup/fetch/throw/close/end |
| detectFrameChannelBackend suite | 475-498 | REAL | — | save/restore global probe |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| golden decode | YES | full header + payload equality |
| all refusal tests | YES | toThrow + kind equality |
| reorder/dedup/fetch-dance | YES | vi.waitFor on exact arrays / pinned invoke shape |
| throwing onRecord | YES | seen==[1,2] + diagnostics==1 |
| close idempotency | YES | unregisterCallback called exactly once |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mirror of wire.rs's own encoder, byte-for-byte" | :22-24 | UNVERIFIED | wire.rs not in my assignment; cross-check next agent |
| "no webview exists in this sandbox (INFRA-BLOCKED.md)" | :5-6 | REAL | consistent with baseline (no playwright visual run) |

## 7. Dead / duplicated / unreachable code
None. All imports used; FakeInternals implements the exported FrameChannelBackend interface.

## 8. Dependencies & flags
vitest only; no cfg/feature gating; runs in the standard `npm run test` lane (baseline ui-test.txt).

## 9. Open questions for the next agent
Verify the LUF1 wire layout against the Rust producer (wire.rs) field offsets 4/6/8/9/10/12/16/20/24/32/40 —
the golden is the only spec of record in ui/.
