# AUDIT: crates/media-probe/tests/registry_roundtrip.rs
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Lines / functions | 308 / 3 tests + 2 helpers (Row struct, window) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (integration test) — the surface under test (`ProbeRegistry::with_builtins` + `probe`) is the shipped routing seam (apps/cli/src/demux_host.rs:444-446, apps/web/src/open.rs:232) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p media-probe --test registry_roundtrip` → 3 passed / 0 failed (exit 0); rg signature inventory vs registration; sed quote verification |

## 1. One-paragraph verdict
A real round-trip matrix: 22 committed fixtures (the demuxer/codec cards' own
differential evidence, cross-crate read-only) probed through the REAL
`ProbeRegistry::with_builtins()` surface, asserting the exact format id and confidence ≥
60; 14 signature extension lists pinned against fixture extensions; five near-miss
magics (OggT, FORM/AIFX, lone TS sync, DKIY, bare PS system header) pinned as typed
`probe.no-demuxer`, plus the MP3-vs-ADTS layer-bit disjointness pinned positively from
both sides. Registration completeness verified: all 14 builtins registered in
signature/mod.rs:131-145 appear in this suite — no untested signature. The one
weakness: the positive rows always hand the probe the correct file extension as a
SourceMeta hint, so pure-bytes routing (a file whose extension lies) is never exercised
at the registry level.

## 2. What actually works (verified)
— `cargo test -p media-probe --test registry_roundtrip` -> 3 passed, 0 failed.
— Registration inventory cross-check: with_builtins registers exactly the 14 signatures
  the extension test lists (signature/mod.rs:131-145); roundtrip rows cover all 15
  format ids (annexb signature serves h264+hevc).

## 3. Findings
F-001 [S4]
Where: crates/media-probe/tests/registry_roundtrip.rs:220-221
Code:
```
        let meta = SourceMeta::new(None, Some(row.extension));
        let report = registry.probe(&bytes, &meta, &config).must();
```
What is wrong: Every positive row supplies the correct extension as a probe hint. If any
signature's magic walk regressed to extension-dependent behavior (or a new signature
stole another format's bytes and only lost on the extension tie-break), this suite would
still pass for correctly-named files. No row probes a fixture with a wrong/absent
extension (`SourceMeta::new(None, None)`), which is the real-world lie users produce.
Confidence: MEDIUM (depends on whether probe() weighs the hint — the AVI signature
ignores `_meta` entirely, but that is verified only for AVI) — Verification: read.
What correct looks like: add one negative/positive pair probing the AVI and MP4
fixtures with `None`/wrong extensions.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| window | 42-46 | REAL | — | leading 4096 bytes, matches ProbeConfig window |
| every_landed_container_probes_to_its_format_id | 49-235 | REAL | S4 | 22 rows × (format id + confidence≥60); F-001 hint caveat |
| extension_lists_cover_their_fixtures | 241-268 | REAL | — | 14 signatures, one-way containment (fixture ext ∈ claimed list) |
| near_misses_and_garbage_stay_unprobed | 276-308 | REAL | — | 5 typed negatives + mp3/adts disjointness both ways |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| every_landed_container... | YES | exact format ids per fixture; a routing regression fails the row |
| extension_lists... | YES | membership asserts against static lists |
| near_misses... | YES | exact error code probe.no-demuxer + stage Probe; positive mp3/adts ids |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every landed container format, probed from the committed fixtures" | :1-3 | REAL | 14/14 signatures covered (registration inventory checked) |
| "asserting the format id the open path will route on" | :4-5 | REAL | same registry + probe call as demux_host.rs:444-446 |
| "honest negatives: garbage and near-miss magics stay probe.no-demuxer" | :17-18 | REAL | 5 cases + mp3/adts boundary, executed green |

## 7. Dead / duplicated / unreachable code
None. (phase5.rs signatures are all registered — not dead.)

## 8. Dependencies & flags
media-probe, media-types, testkit. Cross-crate fixture reads via CARGO_MANIFEST_DIR —
read-only, documented at :10-13.

## 9. Open questions for the next agent
- Does `ProbeRegistry::probe` weigh the SourceMeta extension hint in tie-breaks or
  gating? (contract.rs/config.rs — determines F-001's real severity.)
- The wasm32 overflow (my avi.rs F-001) is reachable through this same registry surface
  on the web build — a 4096-byte window is enough to carry the saturating chunk.
