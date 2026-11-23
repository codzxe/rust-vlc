# AUDIT: crates/codecs/tests/adpcm_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 378 / 5 fns + 1 impl + 4 tests |
| File verdict | REAL (executed: 4/4 pass, direct binary run, exit 0; corpus absent in sandbox → fixtures-only mode confirmed by the run's own message) |
| Reachable from shipped entry point? | N/A (integration test; baseline cargo-test-workspace.txt:937-946, 4/4 ok in 0.71s; I re-ran the built binary: 4/4 in 1.09s) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | ./target/debug/deps/adpcm_fuzz_survival-4bcf0cfa52427a86 --nocapture -> 4/4 ok EXIT=0; test -d corpus path (ABSENT); ls fixture existence; sed quote checks |

## 1. One-paragraph verdict
A real, reproducible fuzz-lite suite: a seeded LCG (no dev-dependency) mutates committed ADPCM WAV payloads (byte flips, truncations, wild header bytes, bursts — 512 rounds per input) and enforces the survival contract — every decode returns Ok with a typed `Emitted`/`Recovered` outcome, never a panic, never an untyped Err — plus fmt-header mutations that must fail `Unsupported`-typed, empty/tiny-payload honesty, and a 3× determinism check with MD5 pinning. Anti-tautology liveness asserts (`recovered > 0`, `decoded_total > 0`, `typed_unsupported > 0`) prevent a too-weak fuzzer from passing vacuously. I executed it: 4/4 pass. Two hygiene defects: the "real corpus" half of the substrate is silently inert whenever `/tmp/my-project/corpus-v4/media` (or `$LUMEN_CORPUS`) is absent — this sandbox has no corpus, so coverage ran fixtures-only while still counting green; and the determinism test's comment claims it guards state leaks "between packets" when no test here ever decodes more than one packet into one decoder.

## 2. What actually works (verified)
- Direct run `./target/debug/deps/adpcm_fuzz_survival-4bcf0cfa52427a86 --nocapture` → `test result: ok. 4 passed; 0 failed ... finished in 1.09s`, EXIT=0; output includes `corpus(adpcm-fuzz): "/tmp/my-project/corpus-v4/media" absent — fixtures only` — the fallback branch executed exactly as documented.
- Baseline corroborates: cargo-test-workspace.txt:939-946, 4/4 in 0.71s.
- Fixtures exist: crates/codecs/tests/fixtures/adpcm_ima_mono.wav, adpcm_ms_mono.wav (ls-verified).
- Corpus dir check: `test -d /tmp/my-project/corpus-v4/media` → ABSENT in this sandbox (fixtures-only mode is the real CI shape here unless a corpus is mounted).

## 3. Findings
F-001 [S4]
Where: adpcm_fuzz_survival.rs:124-131
Code:
```
fn corpus_inputs() -> Vec<(String, Vec<u8>)> {
    let dir = std::env::var("LUMEN_CORPUS").map_or_else(
        |_| PathBuf::from("/tmp/my-project/corpus-v4/media"),
        PathBuf::from,
    );
    if !dir.is_dir() {
        eprintln!("corpus(adpcm-fuzz): {dir:?} absent — fixtures only");
        return Vec::new();
    }
```
What is wrong: the module doc advertises "the real corpus's first ADPCM blocks" as part of the hammer (lines 2-3), but with the corpus absent (the default state of this sandbox and any machine without the mount) that half silently vanishes — an eprintln, not a failure — and the suite still reports 4/4 green on the 2 fixtures alone. Coverage narrows without any red signal. Honest (the message + "when present" doc), but pass-counting. What correct looks like: a `--ignored` opt-in corpus lane or a printed row summary stating fixtures-only mode in the test NAME-visible output. Confidence: HIGH — Verification: executed (the message printed in my run).
Blast radius: test coverage only; shipped code unaffected.

F-002 [S4]
Where: adpcm_fuzz_survival.rs:350-354
Code:
```
    // Guards the fuzz substrate itself: the same bytes decode to the
    // same PCM every run (the pooled-slot rotation must not leak
    // state between packets or runs).
```
What is wrong: `decode_mutation` (168-194) constructs a FRESH `AdpcmDecoder` per call and submits exactly ONE packet; no test in this file decodes multiple packets into one decoder. "State between packets" is never exercised — the claim describes coverage that does not exist (only fresh-decoder run-to-run determinism is tested, which the 3× MD5 pin does genuinely verify). Confidence: HIGH — Verification: read (every decode path in the file routes through decode_mutation).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lcg::next_u32 / below | 27-42 | REAL | — | Knuth-style LCG, u32 out of high bits; `% n` bias irrelevant for fuzz |
| Sink | 47-59 | REAL | — | concatenates PCM; video frames silently ignored (Ok) — acceptable, decoder emits audio only |
| fmt_of | 63-88 | REAL | — | WAV walker; bounds-checked (`body + size > data.len()` → None); tags 0x02/0x11 only |
| data_of | 91-107 | REAL | — | walker, bounds-checked |
| fixture_inputs | 110-120 | REAL | — | `.ok()` swallow is fine (assert!(!inputs.is_empty()) guards) |
| corpus_inputs | 124-166 | REAL | S4 | F-001; trim-to-3-blocks rewrite of the size field is correct (bounds + pad preserved) |
| decode_mutation | 168-194 | REAL | — | fresh decoder + one packet (see F-002 note) |
| mutated_blocks_survive… (test) | 196-278 | REAL | — | EXECUTED; 512 rounds/input; typed-outcome contract; liveness asserts |
| mutated_fmt_headers… (test) | 280-325 | REAL | — | EXECUTED; Unsupported-kind assertion; liveness assert |
| empty_and_tiny_payloads… (test) | 327-348 | REAL | — | EXECUTED; empty→NeedMoreInput, stray byte→Recovered |
| unmutated_substrate… (test) | 350-378 | REAL | S4 | EXECUTED; 3× MD5 determinism real; comment overstates (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mutated_blocks_survive… | YES | any panic fails the test by construction; any decode Err hits unreachable!; liveness `recovered > 0`/`decoded_total > 0` catch a vacuous mutation engine |
| mutated_fmt_headers… | YES | wrong error kind fails the assert; `typed_unsupported > 0` liveness |
| empty_and_tiny_payloads… | YES | outcome shapes pinned exactly |
| unmutated_substrate… | YES | cross-run MD5 equality |
Note: the "no hang" half of the contract (module doc :4) has no explicit watchdog — a hang manifests as a never-finishing test (CI timeout), not an assertion. Acceptable; implicit only.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no panic, no hang, no untyped failure — every decode call returns Ok with a typed outcome" | :3-6 | REAL (panic/typed; hang implicit) | executed 4/4; Err paths hit unreachable! |
| "the corpus is bounded to the first few blocks per file so the committed inputs stay small" | :7-9 | REAL | corpus trim logic :154-158 (3×1024); corpus absent here, fixtures-only confirmed |
| "the two committed fixtures are ~6 KiB each" | :9 | REAL-UNPROVEN | fixtures exist; sizes not re-measured (trivial) |
| determinism comment "must not leak state between packets or runs" | :352-354 | INFLATED | no multi-packet decode exists in the file (F-002) |

## 7. Dead / duplicated / unreachable code
None. All helpers consumed. `use common::md5_hex` used only by the determinism test — fine.

## 8. Dependencies & flags
Test-only; no dev-dependencies (LCG hand-rolled). Env coupling: `LUMEN_CORPUS` or hardcoded `/tmp/my-project/corpus-v4/media` (F-001). Target: `codecs::audio::adpcm::AdpcmDecoder` (adpcm codecs declared landed in the engine's registry claims — other agents' files).

## 9. Open questions for the next agent
- Does any suite decode MULTIPLE packets through one AdpcmDecoder (state-rotation coverage)? If not, the "pooled-slot rotation" comment's concern is untested repo-wide for ADPCM.
- Where does the corpus live in CI (is LUMEN_CORPUS set in any workflow)? rg .github for LUMEN_CORPUS if the fuzz corpus coverage matters to the claims.
