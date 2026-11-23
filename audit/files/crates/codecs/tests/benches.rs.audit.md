# AUDIT: crates/codecs/tests/benches.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 256 / 7 fns + 2 `#[ignore]`d tests |
| File verdict | REAL (both probes executed by me directly) — but the dense-4K probe FAILS its own documented debug sanity floor on this host, and neither probe runs in any gate |
| Reachable from shipped entry point? | N/A (benchmark tests; `#[ignore]`d — baseline cargo-test-workspace.txt:1004-1005 shows both "ignored"; opt-in via `cargo test -p codecs --release --ignored`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | ./target/debug/deps/benches-0eede66acee15e63 --ignored --nocapture --test-threads=1 -> 1 passed, 1 FAILED (contract 755.6 fps; dense-4K 0.4 fps < 1.0 floor); rg worklog/archive for ED-CORPUS-1 numbers; stat binary-vs-source timestamps |

## 1. One-paragraph verdict
The two throughput probes are real code through the real contract path — I executed the built binary directly and got real numbers: the 320x240 fixture probe decodes at **755.6 frames/s in a DEBUG build** (58.0 Mpx/s, 600 frames), passing trivially, and its synthetic dense-4K JPEG construction is verified correct (decodes cleanly at 3840x2160, zero concealment, I420 layout — the sanity asserts passed before the floor fired). But the second probe then FAILS its own documented debug sanity floor on this host: **0.4 frames/s (2.486 s/frame) < the 1.0 frames/s debug floor** — a panic I captured at benches.rs:251. Since both probes are `#[ignore]`d (a documented Lead decision ED-a, "acceptable, but not gate-enforced" per the M13 audit row in the archive worklog), nothing in CI would ever notice: the module's release claim "Asserts ≥ 60 frames/s (realtime 4K60)" — the answer to design question D-10 "will this freeze on a 4K 60fps stream?" — has ZERO enforcement anywhere in the repo, and this host's debug measurement (0.4 fps; typical debug→release speedups of 10-20x project to ~4-12 fps on this 2-core machine) suggests the release floor is not safe either (UNVERIFIED — release build out of budget). Separately, the "numbers are committed in the worklog" citation is still broken: the referenced ED-CORPUS-1 entry lives in docs/worklog/archive-2026-10.md (not worklog.md), and its AFTER-report holds the D-11 corpus-lane fps numbers — not these two benches' outputs. No record of these benches' actual measurements exists in-tree.

## 2. What actually works (verified)
- Executed `./target/debug/deps/benches-0eede66acee15e63 --ignored --nocapture --test-threads=1` (binary 18:07 newer than source 17:35):
  - `test mjpeg_decode_throughput_contract_path ... fixture 320x240 4:2:0 (largest committed fixture, 60 frames): 600 frames in 794068912 ns — 1323448 ns/frame, 755.6 frames/s, 58.0 Mpx/s (320x240)` — PASS (even the release floor of 60 fps is exceeded in debug on this host for the small fixture).
  - `test mjpeg_decode_throughput_dense_4k_budget ... synthetic dense-AC 3840x2160 4:2:0 (worst case): 1 frames in 2486148084 ns — 2486148084 ns/frame, 0.4 frames/s, 3.3 Mpx/s (3840x2160)` — then `panicked at crates/codecs/tests/benches.rs:251:5: 0.4 frames/s < 1.0 frames/s floor`.
  - Prior asserts in that test passed: the synthetic frame decoded (`decode_image`), `(3840, 2160)` asserted, `!image.concealed` asserted, I420 size asserted — the hand-built worst-case JPEG (two 1-bit Huffman codes, 0x55 fill, 194,400 blocks × 129 bits) is a VALID, fully-dense 4:2:0 JPEG. The construction comment's arithmetic (blocks = 240×135×6, 129 bits/block) checks out.
- Baseline: both probes listed `ignored` (cargo-test-workspace.txt:1004-1005) with descriptive reasons.

## 3. Findings
F-001 [S2]
Where: benches.rs:29-31, 248-255 (and 143-144, 222-223)
Code:
```
//! Debug builds assert only a sanity floor (≥ 1 frame/s): the
//! measurements and floors are release-profile claims, and pretending
//! an unoptimized build carries them would be dishonest.
```
and
```
    let floor = release_floor(60.0, 1.0);
    assert!(
        fps >= floor,
        "{fps:.1} frames/s < {floor:.1} frames/s floor (ns/frame {:.0})",
```
What is wrong: EXECUTED FAILURE. On this host the dense-4K probe measures 0.4 frames/s in debug — below even the 1 fps sanity floor — and panics. Because both probes are `#[ignore]`d and no gate, workflow, or CI lane runs `--release --ignored`, the module's headline claim ("Asserts ≥ 60 frames/s (realtime 4K60) in release builds" — the D-10 "will this freeze on a 4K 60fps stream?" answer) is enforced nowhere. Why it matters: the only machine-verifiable signal available (this debug run) contradicts the claim's margin, and nothing would catch a release regression either — the 4K60 budget answer rests on an unexecuted assertion. What correct looks like: a slow-lane CI job (release, --ignored, per-machine baselines with ratios rather than absolute floors, or Mpx/s-normalized floors) or a recorded release measurement committed next to the probe. Confidence: HIGH — Verification: ran (output pasted above); release floor itself UNVERIFIED (cargo-build-budget — no release test binary, heavy build forbidden).
Mitigation: the doc is candid that floors are release-profile claims; the M13 audit row already flagged "not gate-enforced". This audit adds the stronger fact: even the debug floor is false on this host.

F-002 [S3]
Where: benches.rs:3-8
Code:
```
//! `cargo test -p codecs --release --ignored`; the numbers are
//! committed in the worklog's "ED-CORPUS-1 + ED-MERGE-WAVE" entry,
//! AFTER-report bullet — W2-06 citation fix: the earlier text pointed
```
What is wrong: the pointer still does not lead to these benches' numbers. (a) The current worklog.md contains no ED-CORPUS-1 entry — it lives in docs/worklog/archive-2026-10.md:1675 (the citation was written before archival and never updated). (b) That entry's AFTER-report (archive:1693) records the D-11 corpus harness fps ("tiny16 111k->200k … uhd4k 2.2x (4->9)") — measurements of gen_corpus.py lanes, NOT of these two benches' outputs. No in-tree document records what these two probes ever measured. The earlier phantom-citation was already caught once (M13 audit row, gap (b)); the "fix" re-pointed at numbers that are adjacent but not these. Confidence: HIGH — Verification: rg over worklog.md (0 hits) and archive (entry found, contents read).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DroppingSink | 48-57 | REAL | — | counts, drops (throughput shape) |
| configured | 59-79 | REAL | — | mjpeg descriptor contract path |
| packet_for | 81-92 | REAL | — | |
| warm_up | 96-106 | REAL | — | asserts every frame decoded (not a silent warm-up) |
| timed_passes | 109-122 | REAL | — | single decoder reused; Instant-based |
| report | 124-133 | REAL | — | ns/frame, fps, Mpx/s — output captured in my run |
| release_floor | 135-141 | REAL | — | debug/release split (the debug half is what failed) |
| mjpeg_decode_throughput_contract_path | 143-175 | REAL | — | EXECUTED: PASS at 755.6 fps (debug) |
| dense_4k_jpeg | 182-220 | REAL | — | EXECUTED indirectly: produced a valid 3840x2160 dense JPEG that decodes cleanly, no concealment; header arithmetic verified (SOF height-before-width, 0x22/0x11 sampling, 129 bits/block) |
| mjpeg_decode_throughput_dense_4k_budget | 222-256 | REAL | S2 | EXECUTED: FAILED — 0.4 fps < 1.0 debug floor on this host |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| contract_path | YES when run | frame count, warm-up count, fps floor — but it NEVER runs in the gate (#[ignore]) |
| dense_4k_budget | YES when run — and it DOES fail here | my run: floor violation at 0.4 fps; but it NEVER runs in the gate — the failure is invisible to CI |
Both tests' assertions are genuine; the honesty problem is operational (never executed), not tautological.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Asserts ≥ 60 frames/s (realtime 4K60) in release builds" | benches.rs:26-27 | REAL-UNPROVEN / UNENFORCED | no gate runs it; debug measurement on this host is 0.4 fps (F-001); release UNVERIFIED |
| "Debug builds assert only a sanity floor (≥ 1 frame/s)" | :29-31 | FALSE on this host | executed: 0.4 fps < 1.0 floor → panic (F-001) |
| "the numbers are committed in the worklog's 'ED-CORPUS-1 + ED-MERGE-WAVE' entry, AFTER-report bullet" | :4-6 | FALSE (misdirected) | entry is in archive-2026-10.md; its numbers are the D-11 corpus lanes', not these benches' (F-002) |
| "external-writer-authored by the 2.A3 generator with committed per-frame sha256" | :16-17 | REAL | fixture + expectation verified in my pcm_fixtures audit (same AVI parent) |
| synthetic frame "every block carries 63 non-zero ACs, defeating the zero-AC fast path" | :23-24 | REAL | decode ran with `!image.concealed` and full I420 size asserted before the floor fired |

## 7. Dead / duplicated / unreachable code
None within the file. Both tests are unwired from every gate (see F-001) — reachable only by the explicit opt-in command in their ignore messages.

## 8. Dependencies & flags
Test-only, std-only (Instant). `#[ignore = "…"]` with descriptive reasons (good practice). No criterion/dev-deps (deliberate, per Lead decision ED-a). Fixture: the shared AVI parent (workspace tests/fixtures, verified in pcm_fixtures audit).

## 9. Open questions for the next agent
- Someone with release-build budget should run `cargo test -p codecs --release --ignored benches` on the reference hardware and commit the output next to the probe — that would close F-001's UNVERIFIED release half and repair F-002's evidence trail.
- The D-11 corpus "uhd4k 2.2x (4→9 fps)" AFTER number: which profile (debug/release)? If release, it contradicts the 60fps floor by an order of magnitude; if debug, it is consistent with my 0.4 fps dense measurement on slower hardware. The archive does not say.
