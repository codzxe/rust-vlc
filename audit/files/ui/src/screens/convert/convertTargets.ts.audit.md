# AUDIT: ui/src/screens/convert/convertTargets.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 64 / 2 (isConvertTargetId, convertTargetDef) + types/table |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — ConvertScreen.tsx:56,269 renders the picker; BUT the convert lane is disabled-from-data (no convert.* command exists in the wire union — the screen gates it) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg crates/transcode/src/convert.rs (wav target resolution + typed refusals, tests at :284-308); rg "convert." in generated/commands.ts (NO convert command); rg ConvertScreen wiring |

## 1. One-paragraph verdict
A closed two-entry target table (wav, mp4) that mirrors the engine's real
transcode capability set. The engine-side claims check out:
crates/transcode/src/convert.rs implements WAV conversion with typed
refusals for narrowing conversions (wav_conversion_unsupported, tested at
:284-308) and mp4mux.rs exists for the stream-copy path. Crucially for the
master question: NO convert command exists in the 1.D4 wire union, and
ConvertScreen honestly gates its queue lane ("disabled-from-data while no
conversion lane exists", ConvertScreen.tsx:289) — the picker renders real
capability data into an honestly-gated screen. No capability shown that is
not implemented: verified.

## 2. What actually works (verified)
- Capability mirror: engine has exactly the two operations (convert.rs WAV
  + mp4mux.rs); the table offers exactly those two.
- isConvertTargetId closes the set; convertTargetDef's fallback keeps render
  total.
- The acceptance criterion "no capability shown that is not implemented"
  holds — MP3/MKV/H.265 absent from the picker as claimed.

## 3. Findings
None. Checked: engine parity (verified above), type-guard correctness,
fallback totality, dead code (none — both consumers in ConvertScreen).

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| ConvertTargetId/ConvertTargetDef | 31-38 | REAL | closed union |
| CONVERT_TARGETS | 41-52 | REAL | engine-mirrored |
| isConvertTargetId | 55-57 | REAL | |
| convertTargetDef | 62-64 | REAL | total via fallback |

## 5. Test honesty
ConvertScreen.test.tsx / convertQueue.test.ts cover the screen; the target
set's closure is compile-enforced (exhaustive union).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the ENGINE's real capability set, nothing else" | convertTargets.ts:1-2 | REAL | crates/transcode/convert.rs + mp4mux.rs; no third operation found |
| "a narrowing or int↔float target is a typed refusal in the engine" | convertTargets.ts:8-9 | REAL | convert.rs:73-86 wav_conversion_unsupported + tests :284-308 |
| "[PROPOSAL] a third target 'raw elementary stream' ... implemented engine-side but NOT surfaced" | convertTargets.ts:21-26 | REAL-UNPROVEN | SinkSpec::RawEsCopy not verified by me (engine lane) |
| picker feeds a live conversion | (implied by screen) | FALSE if so read | no convert.* command in the wire union; screen is honestly gated (ConvertScreen.tsx:289-303) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
i18n MessageKey types. No gates (the gate lives in ConvertScreen).

## 9. Open questions for the next agent
- ConvertScreen.tsx owner: confirm the gate line copy names the missing lane
  (the convert.css audit deferred this).
