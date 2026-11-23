# AUDIT: crates/codecs/tests/flac_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 75 / 1 ignored test |
| File verdict | REAL bench design, but its evidence is environment-dead here: corpus-gated + #[ignore]d — measured live as SKIP → "ok" in 0.00s |
| Reachable from shipped entry point? | N/A (measurement test); drives the SHIPPED claxon-backed FlacDecoder (CLI "flac" row) on the contract path |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/flac_bench-67aa5f595d040a89 --ignored --nocapture` → 1 passed in 0.00s with "corpus ... absent — SKIP" (exit 0); baseline cargo-test-workspace.txt:1062 (ignored) |

## 1. One-paragraph verdict
The bench is correctly shaped per the pcm_bench convention: #[ignore]d
with the exact release command in the reason, 7 reps with median, a real
decoded-coverage assertion (decoded samples must equal the STREAMINFO
total), and an explicit performance floor (500 k samples/s release ≈ 5.7×
realtime at 88.2 kHz; 50 k debug sanity). But its sole input is the
/tmp-resident corpus FLAC 0046, which is ABSENT in this sandbox — I ran
it with `--ignored` and it printed "corpus ... absent — SKIP" and
returned "ok" in 0.00 seconds: zero decode work, zero measurement, the
performance floor never evaluated. The 5.A1 worklog's throughput number
is therefore UNVERIFIABLE in this environment, and any "passed" count
including this test proves nothing about FLAC performance.

## 2. What actually works (verified)
— verified by live run: the skip path itself behaves as documented
(message + clean exit), and the baseline records the test as ignored by
design (:24 reason string). Nothing else is executable here.

## 3. Findings

F-001 [S4]
Where: crates/codecs/tests/flac_bench.rs:26-29
Code:
```rust
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
```
What is wrong: a measurement test whose entire purpose is producing a
number silently passes when its only input is missing — and here it is
missing (verified: `test -d /tmp/my-project/corpus-v4` ABSENT). My
`--ignored` run: `test result: ok. 1 passed ... finished in 0.00s`.
The skip is messaged and the header documents the /tmp-residency, so it
is honest by this repo's convention — but the consequence is that the
card 5.A1 throughput evidence (worklog number) is un-reproducible in this
sandbox, and a CI green on this test communicates nothing.
What correct looks like: a committed downsized corpus excerpt (the
<200 KB fixture-budget pattern the mp3 reference uses) so the floor
assert always runs, with only the absolute number corpus-gated.
Confidence: HIGH — Verification: ran (skip observed live).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| flac_decode_throughput_corpus_0046 | 25-75 | REAL (design) / SKIP (here) | S4 | coverage assert + median + floor; corpus-gated + ignored; F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| flac_decode_throughput_corpus_0046 | NO in this env | skips before any decode; with corpus: YES for coverage mismatch and floor regression |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an ignored measurement test whose number goes into the card's worklog entry, never the normal gate run" | :1-3 | REAL | baseline :1062 shows it ignored |
| "when absent the test SKIPs with a message" | :9-10 | REAL | observed live |
| the 5.A1 worklog throughput number | (worklog, not this file) | UNVERIFIED | corpus absent; bench cannot re-derive it here |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
tests/flac.rs helper (corpus_dir, decode_all — contract-path decode),
testkit. #[ignore] by design.
## 9. Open questions for the next agent
- tests/flac.rs + flac_bitexact.rs (not assigned): does the bit-exact
  suite carry committed fixtures (not corpus-gated) so FLAC correctness
  (as opposed to throughput) has always-on coverage? Earlier rg showed
  flac_bitexact.rs:202 has a corpus SKIP message — check whether a
  committed-fixture lane exists there too.
