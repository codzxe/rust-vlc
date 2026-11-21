# AUDIT: crates/demux/fixtures/adts/0059-zero-range.bin
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 2,048-byte binary / n/a (fixture) |
| File verdict | REAL (test asset; the adts family's honestly-named, genuinely hostile member) |
| Reachable from shipped entry point? | N/A (test asset) — consumed by `crates/demux/src/adts/tests.rs:630-641` → `committed_mutation_fixtures_are_typed_or_recover_never_panics`, executed ok in baseline (orchrun-test-workspace-nofailfast.log.txt:1513). The ADTS lane itself IS shipped-reachable (containers/mod.rs:65-66, per the mp3.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | cmp -l vs 0059-trunc-4k.bin (398 differing bytes, all →0) · python3 range analysis (mutation == zero [1000..1400) exactly; parent[1163]==parent[1211]==0 explains the 2 non-diffs; outside the range byte-identical) · python3 ADTS frame walk of parent + derivative (mutation destroys the syncword at offset 1116, the start of parent frame #4; 3 frames parse, then the dead zone) · rg `0059-zero-range` (single reference, tests.rs:636) |

## 1. One-paragraph verdict
The family's one honest hostile asset: the bytes are the first 2,048 bytes of corpus `0059-audio-only.aac` (== the committed parent prefix `0059-trunc-4k.bin`) with exactly the byte range [1000..1400) zeroed — 400 bytes, 398 visibly changed (the other two were already 0x00), everything outside the range parent-identical, proven by cmp + range analysis. The name describes the mutation accurately (unlike sibling `0059-flip-idx15.bin`, whose name matches nothing in its bytes). The mutation is genuinely hostile: it destroys the ADTS syncword at offset 1116 (the start of the parent's 4th frame) plus 399 bytes of frame data, so any walk parses 3 frames and then faces a 400-byte zero dead zone mid-stream — a real resync/refusal exercise on committed bytes. Two hygiene defects, both shared with the family: no committed generator/recipe (provenance is only reconstructible relative to the committed parent prefix), and its sole consumer asserts nothing beyond exists+non-empty+no-panic.

## 2. What actually works (verified)
- Byte-accountable derivative: cmp against the committed parent prefix shows 398 differing bytes, ALL zero; python3 proves the mutation is exactly "zero [1000..1400)" (bytes before 1000 and from 1400 on are parent-identical; the 2 non-diffs inside the range are parent-zero bytes at 1163/1211).
- The ADTS header chain stays legal for the first 3 frames (`ff f1 44 40 …` — LC, 88.2 kHz, mono, frame lens 564/272/280, identical to the parent), then the syncword at 1116 is `00 00 00 00 00 00 00` — the hostile dead zone.
- Consumed by an executed test: baseline log 1513 ok.

## 3. Findings

### F-001 [S4]
Where: crates/demux/fixtures/adts/0059-zero-range.bin (provenance)
Code:
```
$ cmp -l 0059-trunc-4k.bin 0059-zero-range.bin | wc -l
398      # all values → 0; range [1000..1400) exactly, 2 bytes were already 0x00
```
What is wrong: No in-repo generator or recipe documents the derivative; the `xtask fixtures --check` gate covers only `tests/fixtures/` (55 files — gate-fixtures___check.log.txt: "no drift — 55 file(s) verified under …/tests/fixtures"), so byte drift in this committed artifact is invisible to every gate. (Mitigation vs. the ASF family: the parent prefix IS committed in-tree, so the derivative is at least locally reconstructible — but nothing asserts the relationship.)
Why it matters: A hostile-input fixture whose bytes can silently drift loses its meaning: a future "fix" that regenerates or re-truncates the family would change the exercised hostile surface with no failing check.
What correct looks like: Committed recipe (parent SHA-256 + "zero [1000,1400)" spec) or a generator + a drift assert (the asf family's sha-pin pattern).
Confidence: HIGH — Verification: cmp + python3 (exact bytes); gate log read; rg for references (single consumer).

### F-002 [S4]
Where: crates/demux/src/adts/tests.rs:630-641 (the sole consumer)
Code:
```
        let path = std::path::Path::new(dir).join(name);
        let bytes = std::fs::read(&path).must();
        assert!(!bytes.is_empty(), "{name} exists");
        super::fuzz::fuzz_demux_adts(&bytes);
```
What is wrong: The only assertions are "file exists, non-empty" plus the no-panic sweep. Nothing pins WHAT the walk does with this file (typed refusal? recovery? how many frames?). If the demuxer's resync behavior regressed from "typed truncation after 3 frames" to "silently skip 400 zero bytes and resync at 1400", the test still passes — the outcome class is invisible.
Why it matters: The fixture's whole value is the mid-stream dead zone; an unpinned outcome means that value is unverifiable. (Same family defect as the 0110-trunc-8k and 0059-flip-idx15 audits' equivalent findings.)
What correct looks like: One differential assertion per committed derivative: demux the parent prefix (3 frames + truncation class) and assert the derivative's walk is the same up to offset 1000, then the recorded typed outcome.
Confidence: HIGH — Verification: consumer read in full (tests.rs:630-641); baseline execution cited (log 1513).

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 2,048 B | Consistent with the family's 2 KiB derivative size |
| Structure matches the name | YES | "zero-range" == a zeroed byte range; exact range [1000..1400) proven |
| Referenced by code/docs | YES (one site) | adts/tests.rs:636 |
| Content sanity | YES | Parent-identical outside the range; legal ADTS for 3 frames; 400-byte zero dead zone at 1116..1399 (syncword of frame #4 destroyed) |
| Determinism / drift gate | NO | F-001 |
| Hostile value | REAL | Mid-stream syncword destruction — exercises resync/refusal on committed bytes (contrast: flip-idx15's framing-irrelevant flag bits) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `committed_mutation_fixtures_are_typed_or_recover_never_panics` (adts/tests.rs:630-641) | ONLY for panics | The sweep guards panics (real value: the zero dead zone is exactly the shape that tempts slicing panics); it cannot detect outcome-class regressions (F-002). Executed ok (baseline log 1513). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| Parent lane "0059-audio-only.aac | 346 pkts, aac/LC 88 200 Hz mono" | docs/formats/container-adts.md:45 | REAL | Byte-verified header chain (LC, sfi 1 = 88.2 kHz, mono); corpus lane test ok in baseline |
| The file name's implied mutation ("zero a range") | crates/demux/fixtures/adts/0059-zero-range.bin (name) | REAL | Bytes: exactly range [1000..1400) zeroed — the family's accurately-named member |

## 7. Dead / duplicated / unreachable code
Not applicable. Family: parent prefix = `0059-trunc-4k.bin` (first 2,048 bytes), siblings = `0059-flip-idx15.bin`, `0110-trunc-8k.bin` (different corpus parent 0110).

## 8. Dependencies & flags
None (data file).

## 9. Open questions for the next agent
- If the family gets recipes (F-001), note the zero range's round boundaries (1000/1400) suggest a scripted `data[1000:1400] = 0` — a worklog or script history may hold the generator.
- Pair F-002's outcome-pinning with the sibling audits' equivalent findings — one differential helper covers all four derivatives.
