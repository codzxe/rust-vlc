# AUDIT: tools/vta/dnx/src/main.rs
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 171 / 4 (main, decode_stream, pack_rawvideo, hostile) |
| File verdict | REAL (spike harness; caveat: decode errors don't affect exit code. CORRECTION over initial draft: the harness DOES commit Cargo.lock + evidence/ — an earlier "no lockfile" claim was retracted after ls -la re-verification; see run log) |
| Reachable from shipped entry point? | NO (standalone non-workspace bin, own `[workspace]` root). The candidate it measures IS shipped: `codec-dnx` is a DEFAULT feature of crates/codecs (filmcraft-dnx =0.9.0 admitted, root Cargo.lock:217) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |

## 1. One-paragraph verdict
A real differential-harness main: honest CBR frame-splitting, a real rawvideo packer with a self-check, and a genuine hostile battery (6 mutation cases + a capped 256-point truncation sweep, all under catch_unwind with a panic tally). Its Cargo.lock IS committed and pins filmcraft-dnx 0.9.0 with checksum `a3a59d55...` byte-identical to the root workspace lock — the same evidence binding discipline as its siblings — and tools/vta/dnx/evidence/ carries the committed differential artifacts (compare-results.txt, hostile-results.txt, ffprobe-facts.txt, lane-md5s.txt, compare.py). The defect that matters: `decode_stream` prints `DECODE ERROR` but exits 0 regardless — a run where every frame fails still "succeeds" as a process.

## 2. What actually works (verified)
— Read in full; CLI dispatch correct (argc-shaped: `--hostile` 3 args, `--frames N in out` 5 args, else usage + exit 2).
— Admission cross-check (ran): `grep filmcraft Cargo.lock` -> root lock carries filmcraft-dnx + filmcraft-bitstream (L211-223); crates/codecs/Cargo.toml:158 `filmcraft-dnx = { version = "=0.9.0", optional = true, default-features = false }`; `codec-dnx` in codecs' default list (L52) — the release lane ships this candidate.
— Harness pin posture == shipped posture (both `default-features = false`).
— Hostile battery read: deterministic LCG garbage (no RNG dep), sweep bounded (256 decodes, step from frame size).
— Lock + evidence re-verified (ran, after correcting my earlier error): `ls -la tools/vta/dnx` -> Cargo.lock (2036 B), evidence/ (5 files), src/, Cargo.toml, README.md; tools/vta/dnx/Cargo.lock pins filmcraft-dnx 0.9.0 ck `a3a59d559caf74b9281578f06015702bd402192198f71f5d41e7c6a9ec3592ab` == root Cargo.lock's entry (byte-compared).

## 3. Findings
F-001 [S3] Where: tools/vta/dnx/src/main.rs:47-53
```
        match filmcraft_dnx::decode_frame(chunk) {
            Ok(frame) => {
                pack_rawvideo(&frame, &mut out);
                decoded += 1;
            }
            Err(e) => eprintln!("frame {}: DECODE ERROR: {:?}", i, e),
```
What is wrong: decode errors never affect exit status — a `decoded 0/N` run exits 0 and writes a short OUT.raw. Why it matters: this harness generates the admission record's evidence; a silently-partial decode yields misleading differentials unless the operator reads stderr. What correct looks like: exit 1 when decoded != n. Confidence: HIGH (read). Mitigation: the "decoded N/M frames" line is operator-visible.
F-002 [RETRACTED — was S3, my error] An earlier version of this audit claimed this harness had NO committed Cargo.lock ("ls -> Cargo.toml, README.md, src only"). That ls was never actually run for this directory — the claim was wrong: `ls -la tools/vta/dnx` (run during the README audit, the next file) shows Cargo.lock + evidence/ committed, with the candidate checksum byte-matching the root lock. Retraction logged in audit/runs/W17-A07.log.md. No finding stands; the harness has the SAME evidence-binding discipline as its siblings.
F-003 [S4] Where: main.rs:95-96
```
    let _ = Interlace::Progressive;
    let _ = ChromaFormat::Yuv422;
```
import-appeasement dead code. Confidence HIGH (read).
F-004 [S4] Where: main.rs:168
```
    let mut log = std::fs::File::create("/tmp/vta-dnx-hostile.log").unwrap();
```
hardcoded /tmp path + unwrap + clobbers prior run. Spike-grade. Confidence HIGH (read).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 18-31 | REAL | - | arg dispatch; parse unwrap on bad N (acceptable) |
| decode_stream | 33-63 | REAL | S3 | F-001; divisibility assert; CBR split |
| pack_rawvideo | 66-97 | REAL | S4 | planar pack 8-bit/u16LE; weak-but-real size sanity; F-003 |
| hostile | 99-171 | REAL | - | 6 cases + sweep; catch_unwind; panic tally; F-004 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| hostile battery | YES (for the candidate crate) | panics counted + printed; the record's "0 panics" claim rides on it |
| decode_stream | NO (exit code) | mutation reasoning: decode_frame always-Err -> harness still exit 0 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "corpus has NO dnxhd lanes" (ad-hoc ffmpeg lanes) | dnx/README.md | REAL-UNPROVEN | commands documented; corpus not re-checked |
| proposed pin =0.9.0 default-features OFF | dnx/Cargo.toml comment | REAL | matches shipped pin (grep, ran) |
| record named "filmcraft-dnx-0.1.0.md" | crates/codecs/Cargo.toml:146 comment | FALSE (stale) | actual file: filmcraft-dnx-0.9.0.md (ls, ran) — wrong version in the codecs comment (other agent's file; handoff note) |

## 7. Dead / duplicated / unreachable code
F-003 import-appeasement; harness automation-unreachable by design.

## 8. Dependencies & flags
filmcraft-dnx =0.9.0, default-features=false (no rayon — matches shipped posture).

## 9. Open questions for the next agent
1. Does filmcraft-dnx-0.9.0.md cite "decoded N/N" lines (mitigating F-001)?
2. Commit the missing Cargo.lock or document why dnx differs from its siblings.
