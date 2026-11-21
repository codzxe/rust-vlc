# AUDIT: crates/demux/fixtures/adts/0059-flip-idx15.bin
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 2048 bytes binary / n/a (fixture) |
| File verdict | REAL (bytes) with a FALSE name — the mutation the name describes is not the mutation the bytes carry |
| Reachable from shipped entry point? | N/A (test asset) — consumed by `crates/demux/src/adts/tests.rs:635` → `committed_mutation_fixtures_are_typed_or_recover_never_panics`, executed ok in baseline (log line 1513) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | cmp -l vs 0059-trunc-4k.bin (the unmutated parent prefix — exactly ONE differing byte) · python3 xor analysis (`0x40 ^ 0x78 = 0x38`, three bits, offset 3) · od header dumps (ff f1 44 40 parent / ff f1 44 78 derivative) · rg `flip-idx15` (single reference) · rg `flip` in tools/xtask/src/cmd/fixtures.rs (the repo's flipN convention) |

## 1. One-paragraph verdict
The bytes are a real derivative of corpus `0059-audio-only.aac` (AAC-LC 88.2 kHz mono, ffmpeg-encoded): exactly 2048 bytes of the parent with ONE byte changed — offset 3, `0x40 → 0x78`, an XOR of `0x38` setting the ADTS header's `original/copy`, `home`, and `copyright_id` flag bits. Three problems, all hygiene-grade but real: the name "flip-idx15" describes nothing in the file (byte 15 is unchanged; three bits flip, not one or fifteen; the repo's own flipN convention — "flip1's bit set is a subset of flip4's … of flip16's", fixtures.rs:2464-2465 — is not this); the flag bits that actually flip are framing-irrelevant (no demuxer decision reads them), so the fixture exercises the demuxer identically to the clean parent — its hostile value is near zero; and like its siblings it has no committed recipe and no drift gate.

## 2. What actually works (verified)
- The derivative is byte-accountable: cmp against the committed parent prefix `0059-trunc-4k.bin` shows exactly one differing byte (offset 3), everything else identical for the full 2048 bytes — the mutation is surgical and the parent/child relationship is provable in-tree.
- The ADTS header stays structurally valid (sync `ff f1`, LC, sfi 1 = 88.2 kHz, mono) — the file is a legal stream whose walk behaves like the parent's.
- Consumed by an executed test: baseline log 1513 ok (see the 0110-trunc-8k audit for the run citation).

## 3. Findings

### F-001 [S4]
Where: crates/demux/fixtures/adts/0059-flip-idx15.bin (the asset's name vs bytes)
Code:
```
$ cmp -l 0059-trunc-4k.bin 0059-flip-idx15.bin
    4 100 170        # 1-based byte 4 = 0-based offset 3: 0x40 -> 0x78
$ python3: 0x40 ^ 0x78 = 0x38   # bits 3,4,5 — orig/home/copyright_id
```
What is wrong: The name claims a flip at "idx 15"; the bytes carry a 3-bit flag change at offset 3. Byte 15 (`0x31`) is identical to the parent. Neither of the repo's two plausible conventions matches: the generated-fixtures convention is flipN = N seeded bits in a subset chain (fixtures.rs:2464-2465), and a naive byte-index reading fails. No committed recipe explains the name (see F-002).
Why it matters: A hostile-input fixture whose label misdescribes its content cannot be reasoned about: an auditor or fix-wave selecting "the index-15 flip case" gets flag-bit noise instead; correlating future failures against fixture names becomes guesswork.
What correct looks like: Rename to the actual mutation (e.g. `0059-flip-hdr-flags.bin` or `0059-flip-idx3-3bit.bin`), or regenerate with the documented flipN convention.
Confidence: HIGH — Verification: cmp + python3 (exact bytes), fixtures.rs convention read.

### F-002 [S4]
Where: crates/demux/fixtures/adts/0059-flip-idx15.bin (provenance)
What is wrong: No in-repo generator or recipe; the `xtask fixtures --check` gate covers only `tests/fixtures/` (55 files — gate log quoted in the 0110-trunc-8k audit F-001); the parent lives in the external corpus-v4.
Why it matters: Same as the sibling: byte drift in this committed artifact is invisible to every gate; regeneration requires out-of-tree knowledge.
What correct looks like: Committed recipe (parent SHA-256 + mutation spec) or generator coverage.
Confidence: HIGH — Verification: rg + gate log.

### F-003 [S4]
Where: crates/demux/fixtures/adts/0059-flip-idx15.bin:3 (the mutated byte)
Code:
```
ADTS fixed header, byte 3: [channel_config lo(2)][orig(1)][home(1)][copyright_id(1)][copyright_start(1)][frame_length hi(2)]
parent 0x40 = flags all 0; derivative 0x78 = orig|home|copyright_id set
```
What is wrong: The three flipped bits are informational flags no framing decision reads — the demuxer's parse, walk, and timing over this file are bit-identical to the clean parent's. As a "mutation fixture" it adds no hostile surface beyond the parent prefix; the no-panic sweep over it is nearly a duplicate of sweeping the parent.
Why it matters: Test value inflation: the fixture list looks like four distinct hostile shapes (trunc/flip/zero) but this one is functionally the clean parent; effort spent regenerating/pinning it buys nothing.
What correct looks like: Flip a bit that matters (syncword, sampling index, frame length, or a mid-payload byte) — the family's apparent intent.
Confidence: HIGH — Verification: read of the ADTS header semantics + the byte diff (the flags are not read anywhere in the adts parse — the demuxer's header fields are documented in container-adts.md and none include orig/home/copyright).

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 2048 B | Consistent with the family's 2 KiB derivative size |
| Structure matches the name | NO | F-001 — one 3-bit flag change at offset 3, nothing at "idx 15" |
| Referenced by code/docs | YES (one site) | adts/tests.rs:635 |
| Content sanity | YES | Parent-identical except the one byte; legal ADTS throughout |
| Determinism / drift gate | NO | F-002 |
| Hostile value | NEAR ZERO | F-003 — framing-irrelevant bits |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `committed_mutation_fixtures_are_typed_or_recover_never_panics` (adts/tests.rs:630-641) | NO for this asset's specific value | Because the mutated bits are unread (F-003), the sweep is equivalent to sweeping the clean parent; it only guards panics. Executed ok (baseline log 1513). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| Parent lane "0059-audio-only.aac | 346 pkts, aac/LC 88 200 Hz mono" | docs/formats/container-adts.md:45 | REAL | Byte-verified header (LC, sfi 1 = 88.2 kHz, mono); corpus lane test ok in baseline |
| The file name's implied mutation ("flip at index 15") | crates/demux/fixtures/adts/0059-flip-idx15.bin (name) | FALSE | F-001: the flip is 3 bits at offset 3 |

## 7. Dead / duplicated / unreachable code
Not applicable. Note the family relationship: parent prefix = `0059-trunc-4k.bin` (first 2048 bytes), sibling = `0059-zero-range.bin`.

## 8. Dependencies & flags
None (data file).

## 9. Open questions for the next agent
- Was "idx15" ever an LCG seed index in a removed generator? If a worklog records the recipe, attach it to the fixture family (retires F-001/F-002's provenance half).
- If the family is regenerated with meaningful mutations, fold the outcome-pinning suggestion from the 0110-trunc-8k audit (F-002 there) into the same pass.
