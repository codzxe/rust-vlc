# AUDIT: crates/demux/fixtures/adts/0059-trunc-4k.bin (binary fixture)
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Size / kind | 4,096 B (exact 4 KiB truncation point) · corrupted ADTS/AAC stream (corpus-v4 draw 0059, truncated) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture, read at test time only (adts/tests.rs:631 `concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/adts")`). The ADTS demuxer it exercises IS reachable (`containers/mod.rs:54` `"adts" => adts::open`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `od -A d -t x1` (header decoded by hand below); `ls fixtures/adts/` (4 mutation derivatives); `grep -n 'bin"' crates/demux/src/adts/tests.rs` (consumer :633); `git log --diff-filter=A` (commit 543e9a29, card 5.C2b); `sed -n '41,50p' docs/formats/container-adts.md` + `od -c` raw-bytes check of the corpus table (draw 0059 truth); baseline `orchrun-test-workspace-nofailfast.log.txt:1513` (`adts::tests::committed_mutation_fixtures_are_typed_or_recover_never_panics ... ok`) |

## 1. One-paragraph verdict
A genuine corrupted-media derivative with clean provenance: the bytes are real ffmpeg 7.1.5 AAC output (the `Lavc61.19.101` encoder string is embedded in the first frame; the ADTS header `ff f1 44 40` decodes by hand to AAC-LC / 88,200 Hz / mono / no-CRC — exactly the documented truth of corpus-v4 draw 0059 in `docs/formats/container-adts.md:46`), truncated at exactly 4,096 bytes per the name. It is consumed by exactly one test, which ran green in the baseline. The one defect is integrity discipline: unlike the ASF fixture family (SHA-256-pinned per file) this mutation family has **no hash pin and no expected-outcome assertion** — the test only asserts non-empty + no-panic, so any accidental byte change to this file would keep passing, silently changing what is being "tested."

## 2. What actually works (verified)
— Header decoded by hand from the od dump: byte0-1 `FF F1` = syncword 0xFFF + MPEG-4 + layer 00 + protection_absent 1; byte2 `0x44` = profile 01 (AAC LC) + sf_index 0001 (88,200 Hz) + private 0; byte2-3 channel_config = 001 (mono). This matches the corpus record's "0059-audio-only.aac | 346 pkts, aac/LC 88 200 Hz mono" row byte-for-byte (raw-bytes verified — an earlier terminal display mangled `0059-` to `n-`; od settled it).
— Truncation point: file is exactly 4,096 B (`-trunc-4k`), mid-stream cut of the 346-packet parent — the class of input the ADTS walk must answer with typed truncation/recovery, never a panic.
— Consumed by `committed_mutation_fixtures_are_typed_or_recover_never_panics` (adts/tests.rs:630-643) through the `fuzz_demux_adts` survival harness; executed ok in the baseline (:1513).
— Provenance chain: corpus-v4 (ffmpeg 7.1.5 / Lavf61.7.103) draw 0059 → documented parent truth (ffprobe lane table, payload md5 9c5542e8…, roundtrip PASS) → in-tree mutation derivative committed with card 5.C2b (commit 543e9a29).

## 3. Findings
### F-001 [S4]
Where: crates/demux/src/adts/tests.rs:630-643 (the consumer), exemplified by this fixture and its three siblings
Code:
```rust
        let path = std::path::Path::new(dir).join(name);
        let bytes = std::fs::read(&path).must();
        assert!(!bytes.is_empty(), "{name} exists");
        super::fuzz::fuzz_demux_adts(&bytes);
```
What is wrong: the committed mutation fixtures (0059-trunc-4k.bin, 0110-trunc-8k.bin, 0059-flip-idx15.bin, 0059-zero-range.bin) carry no SHA-256 pins and the test asserts nothing about their content beyond non-emptiness and survival. A regenerated, truncated-again, or accidentally-edited fixture passes identically — the "committed derivatives" can silently rot, and with them the only committed hostile-input coverage of the ADTS walk (the in-crate fuzz-lite has no scrambles of its own; these four files ARE the committed mutation surface).
Why it matters: the ASF family (my file-9 audit) pins every fixture's SHA and asserts per-file structural truth; the ADTS family — same wave vintage, same crate — pins nothing. House inconsistency plus a real rot risk for exactly the inputs whose value is their exact bytes.
What correct looks like: an `expected_sha`-style pin per file (the asf/tests.rs:1484-1499 pattern) plus, where cheap, the expected terminal code per mutation (`demux.adts.truncated` for this file).
Confidence: HIGH — Verification: read (test body), sha256sum absent-by-design (no pin to compare against), baseline :1513.

## 4. Per-function table
Not applicable (binary asset). Consumer: the single test above; harness `fuzz_demux_adts` (adts/fuzz.rs — another agent's file).

## 5. Test honesty
| Test consuming this fixture | Would it fail if the fixture were broken? | Reason |
|---|---|---|
| committed_mutation_fixtures_are_typed_or_recover_never_panics | PARTIALLY | Fails if the file is missing/empty or if the demuxer panics/hangs on it — but NOT if the bytes silently change (F-001). A survival-only contract is honest for a fuzz-lite lane; the missing pin is the gap, not the survival scope. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "0059-audio-only.aac | 346 pkts, aac/LC 88 200 Hz mono | 346 pkts, every pts = i·1024 | PASS" (the parent) | docs/formats/container-adts.md:46 | REAL | od-decoded header of this derivative matches the parent's documented profile/rate/channels exactly; baseline adts corpus lanes ok. |
| "fuzz_target = fuzz_demux_adts (in-crate adts/fuzz.rs + fuzz-lite corpus sweeps …)" | docs/formats/container-adts.md:14-16 | REAL | This fixture is one of the committed corpus sweeps; consumer verified; baseline :1513 ok. |
| "dirty-lane 350-vs-351 divergence recorded" | feature-matrix container-adts-aac evidence | REAL (recorded divergence — not this file) | The record documents the known dirty-lane divergence; this truncation derivative is a different lane. |

## 7. Dead / duplicated / unreachable code
Not applicable. All 4 derivatives in the directory are consumed by the same test (verified by name list at tests.rs:633-636).

## 8. Dependencies & flags
None (test data). No README/manifest in fixtures/adts/ — the mutation semantics live only in the filenames (trunc-4k / trunc-8k / flip-idx15 / zero-range) and the corpus-v4 docs for the parents. License: corpus-v4 media is ffmpeg self-build (lavfi), same clean provenance class as the ASF fixtures.

## 9. Open questions for the next agent
- The adts/fuzz.rs owner should confirm whether the harness has its own scramble sweep or whether these four files are the entire committed hostile surface for ADTS (if the latter, F-001's pin fix is higher priority).
- 0110-trunc-8k.bin is the 96,000 Hz draw truncated at 8 KiB — same audit conclusions apply (audited here as a family sample; it is not in my assignment list but shares the finding).
