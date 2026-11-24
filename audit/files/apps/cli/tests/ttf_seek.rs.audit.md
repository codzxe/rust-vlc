# AUDIT: apps/cli/tests/ttf_seek.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2025-06-27 |
| Lines / functions | 788 / 14 (harness fns + 3 tests + 2 helper types) |
| File verdict | SHALLOW — real through-the-binary harness with strong structural assertions, but LIVE-PROVEN to execute ZERO assertions in this environment and in every repo CI lane (corpus never provisioned): all 3 tests pass in 0.01 s as no-ops |
| Reachable from shipped entry point? | N/A (integration test). Runs under `cargo test -p lumen-cli` — executed by me: `3 passed; 0 failed ... finished in 0.01s`, each printing `ttf-seek: corpus absent — measurement no-op`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `~/.cargo/bin/cargo test -p lumen-cli --test ttf_seek -- --nocapture` → 3 passed, all no-ops (output pasted in §2); rg over .github/workflows for LUMEN_CORPUS/corpus; ls /tmp/my-project/corpus-v4/media → absent; sed quote verification; baseline cargo-test-workspace.txt:3237-3239 (all "ok", same no-op) |

## 1. One-paragraph verdict
This is a well-built measurement harness: it spawns the REAL `lumen` binary (`CARGO_BIN_EXE_lumen`), timestamps every stdout line, and pins structural truths (seek lands in ±10% of half-media, exactly one `Idle → Opening`, typed refusal substrings, exit codes 0/65) rather than wall-clock magnitudes. But the whole file is gated on `corpus_media()`, and the corpus does not exist in this sandbox, in the orchestrator baseline, or in any CI workflow that runs these tests — so every "ok" this file has ever produced in a repo lane executed zero assertions. I ran it: three no-op passes in 0.01 s. The module header's "measured this session" landscape (µs-level landing values) is therefore unfalsifiable from the repo itself; only a manually-provisioned corpus (none is checked in, no regen path targets these lanes) can exercise the assertions. Two of the lanes (aiff fixture, ivf codec fixture) use in-repo files and could run unconditionally, proving the all-or-nothing gate is a choice.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p lumen-cli --test ttf_seek -- --nocapture` →
```
ttf-seek: corpus absent — measurement no-op
test start_time_pure_seek_open_path_watermarks ... ok
ttf-seek: corpus absent — measurement no-op
test in_session_seek_per_family_class_through_the_binary ... ok
ttf-seek: corpus absent — measurement no-op
test ttf_first_output_per_family_through_the_binary ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```
This verifies the harness compiles, spawns nothing, and passes vacuously. The with-corpus assertion strength is UNVERIFIED (no corpus; regenerating one is out of budget).

## 3. Findings
F-001 [S2]
Where: apps/cli/tests/ttf_seek.rs:166-176, 385-388, 479-482, 675-678
Code:
```
fn corpus_media() -> Option<PathBuf> {
    let root = std::env::var("LUMEN_CORPUS")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let default = PathBuf::from("/tmp/my-project/corpus-v4");
            default.is_dir().then_some(default)
        })?;
```
and
```
    let Some(media) = corpus_media() else {
        eprintln!("ttf-seek: corpus absent — measurement no-op");
        return;
    };
```
What is wrong: all three tests return before a single assertion when the corpus is absent — and it is absent everywhere this repo runs tests: `/tmp/my-project/corpus-v4/media` does not exist here, `LUMEN_CORPUS` is unset, no workflow in .github/workflows sets `LUMEN_CORPUS` or provisions the corpus for a `cargo test` lane (rg: only audit.yml/compat-matrix.yml mention "corpus"; compat-matrix regenerates its own corpus for a different sweep and runs no ttf test). The baseline's `ok` at cargo-test-workspace.txt:3237-3239 and my live run both prove the vacuous pass. Why it matters: these "ok" lines are counted in the 4744-passed total and could be cited as evidence that in-session seek and ttf work per family; they prove only that a directory is missing. Checklist 6.4 ("tests that never run"). What correct looks like: run the fixture-backed lanes (aiff, ivf) unconditionally, and gate only the corpus lanes — or provision the corpus in the test lane. Confidence: HIGH — Verification: ran (output above).

F-002 [S3]
Where: apps/cli/tests/ttf_seek.rs:112-123
Code:
```
    let stdout = child.stdout.take().must();
    let mut lines = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.must();
        lines.push((line, t0.elapsed().as_secs_f64() * 1000.0));
    }
    let status = child.wait().must();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
```
What is wrong: stderr is not drained until stdout reaches EOF; a child that writes more than the OS pipe buffer (~64 KiB) to stderr while the harness reads stdout deadlocks both processes (the child blocks on stderr write, never finishing stdout). Current lanes emit little stderr, so it is latent — but this harness is explicitly the tool for debugging pathological lanes (the s24le/u8 lanes that "never reported a terminal" would have been exactly such a case if the engine had chattered on stderr). What correct looks like: drain stderr on a thread (the standard two-pipe pattern). Confidence: MEDIUM — Verification: read.

F-003 [S4]
Where: apps/cli/tests/ttf_seek.rs:150-161
Code:
```
            let digits: String = tail[open..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse::<i64>().ok()
```
What is wrong: the parser accepts only ASCII digits — a negative timestamp would silently parse its absolute value (`-1234` → `1234`), and an empty digit run yields `None` (handled) but a payload like `Timestamp(1_000)` would parse `1`. Not reachable with the current wire text (`position: Timestamp(<µs>)`), pure hygiene. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Timed (struct) | 85-89 | REAL | — | transcript + timestamps + exit code |
| timed_session | 97-129 | REAL | S3 | real binary, piped stdin script; stderr drain hazard (F-002) |
| Timed::contains | 133-135 | REAL | — | substring probe |
| Timed::first_ms | 139-144 | REAL | — | first-needle wall offset |
| Timed::first_int_after | 150-161 | REAL | S4 | digit-only parse (F-003) |
| corpus_media | 166-176 | REAL | — | env-or-default corpus probe; the no-op gate |
| fixture | 179-184 | REAL | — | in-repo fixture path helper |
| codec_fixture | 188-193 | REAL | — | codecs-crate fixture helper |
| SeekClass (enum) | 200-215 | REAL | — | 4 honest seek classes |
| Lane (struct) | 219-228 | REAL | — | family/path/plays/refusal/seek |
| lanes | 231-371 | REAL | — | 17 curated lanes; refusals named per lane |
| ttf_first_output_per_family_through_the_binary | 383-457 | VACUOUS here | S2 | real asserts (receipt ordering, typed refusals) but no-op without corpus (F-001) |
| in_session_seek_per_family_class_through_the_binary | 477-638 | VACUOUS here | S2 | strongest asserts (±10% landing, no reopen, one Opening); no-op without corpus |
| start_time_pure_seek_open_path_watermarks | 673-763 | VACUOUS here | S2 | open-path seek landing + ts refusal exit 65; no-op without corpus |
| transcript | 766-773 | REAL | — | assertion-message carrier |
| MustOr / impl | 777-788 | REAL | — | Option bridge with context; panic carries context |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ttf_first_output_per_family_through_the_binary | NO in every repo environment (live-proven no-op); YES with a corpus | asserts receipt-before-output ordering, exit 0, typed refusal substrings, never-claims-first-output for refusing lanes |
| in_session_seek_per_family_class_through_the_binary | NO here; YES with corpus | ±10% landing neighborhood, exactly one Idle→Opening, no fabricated failure/completion, named refusal stages |
| start_time_pure_seek_open_path_watermarks | NO here; YES with corpus | landing near target window (−0.05 s, +1.0 s — loose upper bound, honest for keyframe-safe actuals), ts refusal + exit 65 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Through the REAL binary the playable families are narrower than the pipeline-layer evidence... h264 is not dispatched" | ttf_seek.rs:47-54 | CONSISTENT (UNVERIFIED live) | matches baseline engine-info "decoders: mjpeg, pcm_s16le" (AGENT-SPEC §13); no corpus to re-derive |
| "Timing is RECORDED... never asserted: the pins are structural" | ttf_seek.rs:34-43 | REAL (design) | verified in code: only ordering/containment asserts, no ms thresholds |
| "measured this session" landscape with µs values (e.g. 3 486 078 µs) | ttf_seek.rs:45-69 | UNVERIFIABLE from repo | corpus absent everywhere; historical session numbers, not reproducible in CI |

## 7. Dead / duplicated / unreachable code
- None dead per se; the entire assertion suite is unreachable in repo-provided environments (F-001).

## 8. Dependencies & flags
- testkit::Must; no feature gates; uses `CARGO_BIN_EXE_lumen` (dev-profile binary — the doc's honesty contract about DEV profile is upheld by construction).

## 9. Open questions for the next agent
- Does any non-CI environment (soak-nightly? local dev docs) provision corpus-v4 for these lanes? soak-nightly.yml had no "corpus" hit — so no.
- If the corpus is ever provisioned, re-run and re-score this file: the assertions look strong enough to be REAL.
