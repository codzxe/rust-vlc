# AUDIT: crates/demux/fixtures/adts/0110-trunc-8k.bin
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 8192 bytes binary / n/a (fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test asset) — consumed by `crates/demux/src/adts/tests.rs:634` → `committed_mutation_fixtures_are_typed_or_recover_never_panics`, executed ok in baseline (log line 1513; 31 adts tests total, all ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | od -A d -t x1 (header bytes) · python3 byte analysis (syncword positions, tail shape) · cmp vs 0059-trunc-4k.bin (parent-prefix reference) · rg `0110-trunc-8k` (single reference, adts/tests.rs:634) · rg `xtask fixtures --check` coverage (gate log: 55 files under tests/fixtures only — this dir NOT covered) |

## 1. One-paragraph verdict
A faithful 8192-byte truncation of the corpus file `0110-audio-only.aac` (ffmpeg-encoded AAC-LC 96 kHz mono — verified in the bytes: ADTS sync `ff f1`, profile bits = LC, sampling-frequency index 0 = 96 000 Hz matching docs/formats/container-adts.md:47, channel bits = mono, and the `Lavc61.19.101` encoder string matching the baseline's ffmpeg 7.1.5). The content is a genuine multi-frame ADTS series (frame syncs at offsets 0, 115, 244, 324, 376, 429, 528, 581, 672, 748 …) cut mid-payload at the 8 KiB boundary — exactly the hostile shape a truncation derivative should be. It is referenced by exactly one test, which ran green in the baseline. Two hygiene findings: the derivative has no committed generator (the repo's determinism contract covers only `tests/fixtures/` — the `xtask fixtures --check` gate verified 55 files there, not this directory), and the consuming test's entire assertion for this asset is non-empty + no-panic (the fuzz driver's own docs say its contract is "absence-of-panic/hang/unbounded-allocation, nothing else").

## 2. What actually works (verified)
- Structure verified in the bytes: ADTS header `ff f1 40 40` = syncword + MPEG-4 + no-CRC + LC profile + sfi 0 (96 kHz, matching the documented 0110 lane) + mono; frame series present; tail (last 16 bytes `fd bf fa ff …`) is mid-payload data, i.e. a genuine mid-frame cut, not a padded file.
- Consumed by an executed test: baseline `orchrun-test-workspace-nofailfast.log.txt:1513` — `test adts::tests::committed_mutation_fixtures_are_typed_or_recover_never_panics ... ok`.
- The truncation actually exercises the demuxer's sticky-truncation/resync paths (the file ends inside a frame's declared extent — the exact `demux.adts.truncated` shape the adts suite pins elsewhere).

## 3. Findings

### F-001 [S4]
Where: crates/demux/fixtures/adts/0110-trunc-8k.bin (the asset); evidence: audit/runs/baseline/gate-fixtures___check.log.txt
Code:
```
result: no drift — 55 file(s) verified under /home/z/my-project/audit-workspace/tests/fixtures
```
What is wrong: The repo's fixture-determinism contract ("The whole corpus is a pure function of this source file", tools/xtask/src/cmd/fixtures.rs:31-40) does not extend to `crates/demux/fixtures/`: no generator, no recipe, no drift check covers these committed mutation derivatives. The parent (`0110-audio-only.aac`) lives in the external corpus-v4 set; regenerating this 8 KiB cut requires that external file plus knowledge that is not in the tree.
Why it matters: A committed hostile-input artifact whose provenance cannot be reproduced from the repo cannot be audited for accidental content changes (a byte drift here would be invisible to every gate) and depends on an out-of-tree corpus for its meaning.
What correct looks like: Either a small committed generator (parent hash + cut offset — the truncation recipe is one line) covered by `xtask fixtures --check`, or a manifest recording parent SHA-256 + mutation parameters next to the files.
Confidence: HIGH — Verification: rg over tools/xtask (no reference to demux/fixtures); gate log quoted.

### F-002 [S4]
Where: crates/demux/src/adts/tests.rs:630-641 (the sole consumer)
Code:
```
        let bytes = std::fs::read(&path).must();
        assert!(!bytes.is_empty(), "{name} exists");
        super::fuzz::fuzz_demux_adts(&bytes);
```
What is wrong: The only assertions over this asset are "file is non-empty" and "the fuzz driver does not panic". The driver's own module docs state the contract is "absence-of-panic/hang/unbounded-allocation, nothing else — malformed input is *expected* to produce typed errors, which are swallowed here" (adts/fuzz.rs:3-8). Nothing pins WHICH typed outcome this truncation produces (sticky truncation vs recovered-clean-end), so a regression that turns the typed truncation into a silent clean EOS — the exact class I filed as ogg F-002 — would pass this test unchanged.
Why it matters: The asset's forensic value (a known mid-frame cut) is not captured; the committed bytes could silently change demuxer behavior without any test failing.
What correct looks like: Pin the terminal outcome per fixture: drain the demuxer over the bytes and assert the specific terminal event/error (e.g. `Corrupt::Truncated`) alongside the no-panic sweep.
Confidence: HIGH — Verification: read (test + fuzz driver docs); baseline run cited.

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 8192 B exactly as named | Truncation length is the name's contract |
| Structure matches the name | YES | Valid ADTS series cut mid-payload at 8 KiB (byte-verified, §2) |
| Referenced by code/docs | YES (one site) | adts/tests.rs:634 |
| Content sanity | YES | ffmpeg 7.1.x encoder string; no suspicious embedded content; syncs at plausible frame intervals |
| Determinism / drift gate | NO | F-001 |
| Test pins behavior | NO (survival only) | F-002 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `committed_mutation_fixtures_are_typed_or_recover_never_panics` (adts/tests.rs:630-641) | NO for behavior, YES for panics | It proves the committed bytes never panic/hang the demuxer (a real regression guard against crash-class regressions); it cannot detect outcome-class regressions (F-002). Executed ok (baseline log 1513). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "0110-audio-only.aac | 376 pkts, aac/LC 96 000 Hz mono" (the parent this derives from) | docs/formats/container-adts.md:47 | REAL | Byte-verified header: LC + sfi 0 (96 kHz) + mono; the parent lane is corpus-pinned by `corpus_aac_lanes…` ok (baseline log 2023-2027 region) |
| The `+FB` committed-derivatives posture ("the committed `+FB` fuzz-lite derivatives") | adts/tests.rs:3 | REAL-UNPROVEN | The derivatives exist and are swept; their generation recipe is not committed (F-001) |

## 7. Dead / duplicated / unreachable code
Not applicable (binary asset). The sibling `0059-trunc-4k.bin` (not my assignment) is the parent-prefix reference; my two 0059 derivatives diff cleanly against it.

## 8. Dependencies & flags
None (data file). Consumed only via `std::fs::read` at test time — a missing file fails the test loudly (`must()`).

## 9. Open questions for the next agent
- Whoever audits `crates/demux/fixtures/adts/0059-trunc-4k.bin`: it is the de-facto unmutated parent prefix; my two 0059 audits diff against it — cross-link.
- If F-001's manifest lands, include the parent SHA-256 (the external corpus's 0110 file) so the derivative chain is verifiable without the corpus.
