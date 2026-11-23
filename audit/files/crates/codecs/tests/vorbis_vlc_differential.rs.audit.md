# AUDIT: crates/codecs/tests/vorbis_vlc_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 245 / 1 test + 5 helpers |
| File verdict | REAL Lumen half (ffmpeg-pinned digests, asserted) / **PENDING-INFRA VLC half** — and even with a VLC binary the non-silence lanes would compare nothing |
| Reachable from shipped entry point? | Test file (not shipped); code under test IS shipped (`"vorbis"` arm, apps/cli/src/pipeline_host.rs:271, symphonia vorbis engine) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
The test's honest label is in its own name: `vorbis_differential_vs_vlc_or_pending_infra`. The Lumen half is real and always-on: three fixtures + a corpus row decode through the product path (OGG demuxer → shipped VorbisDecoder) and their canonicalized f32 window MD5s are asserted against ffmpeg-derived pins — I ran it live, the pins held. The VLC half is a no-op in this sandbox (no vlc/cvlc; `which` → nothing) and the output says so plainly ("PENDING-VLC ... nothing is fabricated"). The deeper problem the file discloses only in a comment: **even when a VLC oracle exists, the lossy lanes perform no comparison** — the code prints the two digests and defers the "bounded delta" to a *different* oracle's suite (vorbis_bitexact vs ffmpeg), while the `compared` counter still counts those rows, so the final assertion `compared == rows.len()` would pass no matter what VLC decoded. Only the digital-silence lane would ever assert against VLC. Card 5.B2's "differential vs VLC" is therefore: one byte-exact silence row (potential), zero lossy rows, forever — unless someone adds the byte-level delta this file explicitly says it cannot do with digests.

## 2. What actually works (verified)
- `timeout 110 ./target/debug/deps/vorbis_vlc_differential-3a9245a4339ccea0 --test-threads=2 --nocapture` → `1 passed` in 0.13s. Output shows the three Lumen digests (each having passed the `assert_eq!` against `reference["lumen_window_md5"]` at :144-147 before printing), all VLC rows `PENDING-VLC`, corpus row skipped loudly.
- The skip/pend machinery never fabricates: `vlc_md5` is `Option`, `Err(reason)` from the oracle becomes a printed skip, not a fake digest.

## 3. Findings

F-001 [S3 — with an oracle present, lossy lanes compare nothing; the counter lies]
Where: vorbis_vlc_differential.rs:204-232 (esp. 216-224, 244)
Code:
```
                } else {
                    // A bounded-delta comparison needs the bytes, not
                    // just digests; the digest row is recorded and the
                    // delta corridor re-asserted by the live-ffmpeg
                    // differential (the same 1e-7 bound applies).
                    eprintln!(
                        "{}: Lumen {} vs VLC {} — bounded-delta lane (see vorbis_bitexact.rs)",
```
```
    assert_eq!(compared, if oracle.is_some() { rows.len() } else { 0 });
```
What is wrong: for non-silence rows the VLC digest is only *printed*; no assertion uses it (the comment admits a delta needs bytes, which `Row` discards). Yet `compared += 1` runs for them (:225), so with a working VLC the closing assert demands only that `compared == rows.len()` — trivially true — and the test passes even if VLC's decode diverges arbitrarily on every lossy lane. The only genuine VLC assertion in the file is the silence lane's byte equality (:209-214). The "differential vs VLC" of card 5.B2 is thus a one-lane byte check plus digest logging.
Why it matters: the day someone installs VLC 3.0.23 and points `$VLC` at it, this test will go green while proving nothing about lossy-lane parity — a green test used as AC evidence would be a false pass.
What correct looks like: keep the raw f32 bytes per row and implement the documented bounded-delta compare, or count only genuinely asserted rows in `compared`.
Confidence: HIGH — Verification: read + ran (no-oracle branch).

F-002 [S4 — lax oracle-version pin + non-executable `which`]
Where: vorbis_vlc_differential.rs:57-62, 102
Code:
```
    if !text.contains(PINNED_VLC) {
```
What is wrong: substring containment means a hypothetical "3.0.230" or "13.0.23" satisfies the "3.0.23" pin; and `which` accepts any *file* on PATH named vlc/cvlc without checking the executable bit (a data file named `vlc` would be "found", then fail at Command spawn with a confusing error). Cosmetic robustness gaps in plumbing that is currently dormant.
Confidence: HIGH — Verification: read.

## 4. Per-function table (test file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| vlc_binary / which | 48-62 | REAL | S4 | env override + PATH walk; F-002 |
| vlc_dump_args | 66-82 | REAL | — | the 3.E3 pinned headless float32 dump command |
| vlc_oracle_md5 | 96-122 | REAL | — | version gate, typed Err (no fabrication), temp-file cleanup |
| Row / vorbis_differential_vs_vlc_or_pending_infra | 86-90, 124-245 | SHALLOW | S3 | Lumen half real + asserted; VLC half PENDING (honest) / non-asserting (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| vorbis_differential_vs_vlc_or_pending_infra | YES for the Lumen/ffmpeg half (pins asserted per row); **NO for the VLC half** — nothing asserts VLC output except a hypothetical silence lane | F-001; the PENDING state is loudly printed, not hidden |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "PENDING INFRA ... the VLC half of every row is recorded as PENDING-VLC" | :5-11 | TRUE | live run output |
| "the VLC invocation plumbing is ... ready to run the moment a binary exists" | :16-18 | HALF-TRUE | plumbing runs, but lossy lanes assert nothing even then (F-001) |
| "the VLC comparison ... is a bounded per-sample delta on the granule-aligned window" | :20-24 | NOT IMPLEMENTED (here) | no delta code exists in this file; deferred to the ffmpeg suite |
| "the digital-silence fixture is the one byte-comparable lane" | :26-27 | TRUE | the only real VLC assertion |

## 7. Dead / duplicated / unreachable code
The entire VLC comparison branch is dormant (no oracle in sandbox, none in CI per the 5.A1 note). `Row.vlc_md5` is write-then-print only.

## 8. Dependencies & flags
Test-only; symphonia vorbis engine (shipped), ogg demuxer, testkit; external oracle vlc/cvlc 3.0.23 (absent).

## 9. Open questions for the next agent
- Doc/matrix auditors: does any row cite "Vorbis vs VLC parity" as verified evidence? The honest status is PENDING-VLC with a single-potential-lane assertion (F-001) — citing this test as VLC-parity proof would be FALSE.
- tools/bench-vs-vlc (3.E3 harness): same pending status claimed — worth an audit of its own.
