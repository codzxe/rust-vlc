# AUDIT: tools/bench-vs-vlc/src/md5.rs
| Field | Value |
|---|---|
| Auditor | W19-A07 — RECOVERED by W19-R2 |
| Date | 2026-11-24 |
| Lines / functions | 237 / 7 (+3 tests) |
| File verdict | REAL (verified algorithmically; Rust-side execution UNVERIFIED — no cargo in this sandbox) |
| Reachable from shipped entry point? | NO — standalone tool workspace (its Cargo.toml `[workspace]` is deliberately NOT a root-manifest member); reachable from the bench harness: run.py -> `cargo test --test lumen_pcm` -> `bench_vs_vlc::md5::md5_hex` (tests/lumen_pcm.rs:21,344,395) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 port-verification (exit 0); sed/rg quotes verified |

## 1. One-paragraph verdict
A clean-room RFC 1321 MD5 for the differential harness, correctly implemented: I ported
`compress`/`update`/`finalize` line-for-line from this source into Python and it
reproduces ALL SEVEN RFC 1321 §A.5 vectors and a 1000-byte streaming case byte-exactly
(vs `hashlib.md5`), and the K table matches `floor(abs(sin(i+1))·2^32)` on 64/64
constants. It is genuinely dependency-free (Cargo.toml [dependencies] = 7 path deps, no
digest crate), lints-clean by policy (no unwrap/expect/panic/index in the file), and its
output has real-world cross-validation: the committed vlc-diff-report's Lumen-side
digests — computed by this code — matched the ffmpeg/oracle-side digests (hashlib) on
35/35 rows. The only criticism is a byte-at-a-time padding loop (hygiene).

## 2. What actually works (verified)
- python3 faithful port of this file's compress/update/finalize -> `d41d8cd98f00b204e9800998ecf8427e` (empty), `0cc175b9…` ("a"), `90015098…` ("abc"), `f96b697d…`, `c3fcd3d7…`, `d174ab98…`, `57edf4a2…` — all 7 RFC vectors OK; 1000-byte pattern message OK vs hashlib.md5.
- K-table formula check: 0 mismatches over 64 constants; SHIFTS table matches the RFC S-table pattern (verified by the passing digests — S values feed every round).
- Consumers: tools/bench-vs-vlc/tests/lumen_pcm.rs:21 `use bench_vs_vlc::md5::md5_hex`; :75/:89 pin two fixture digests; :343-344/:394-395 compare report digests to freshly computed ones.

## 3. Findings
F-001 [S4]
Where: tools/bench-vs-vlc/src/md5.rs:104-106
Code: (verbatim)
```rust
        self.update(&[0x80]);
        while self.buffered != 56 {
            self.update(&[0x00]);
        }
```
What is wrong: padding is fed one byte per `update` call — each call re-does the buffered>0 branch, length accounting, and slice work; up to 64 calls to append <=63 zero bytes. Why it matters: pure hygiene (test tool, not a hot path; measured digests are over whole files via bulk `update`). What correct looks like: compute `padlen = 56 - buffered mod 64` and append a single zeros slice. Confidence: HIGH — Verification: read (algorithm verified by port).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SHIFTS` const | 20-24 | REAL | — | RFC S-table; all rounds verified via passing vectors. |
| `K` const | 28-37 | REAL | — | 64/64 match `floor(abs(sin(i+1))·2^32)` (python3 check). |
| `Md5::new` | 59-66 | REAL | — | canonical IV `0x67452301/0xefcdab89/0x98badcfe/0x10325476` (:61). |
| `Md5::update` | 70-96 | REAL | — | buffering correct incl. take<want partial + split_at bulk path; ported and verified. |
| `Md5::finalize` | 99-117 | REAL | S4 | `bit_len` captured BEFORE padding updates (:100) so the appended length is correct; F-001 hygiene only. |
| `finalize_hex` | 120-122 | REAL | — | delegates to to_hex. |
| `Md5::compress` | 125-151 | REAL | — | F/G/H/I + g-index formulas (i, 5i+1, 3i+5, 7i mod 16) are the RFC's; wrapping adds; rotate_left; state adds — verified end-to-end by port. |
| `to_hex` | 155-163 | REAL | — | lowercase hex, 16-byte input. |
| `md5_hex` | 166-170 | REAL | — | one-shot; consumed by tests/lumen_pcm.rs. |
| tests `rfc1321_vectors` | 180-202 | REAL (honest) | — | 7 canonical vectors; would fail on ANY round/constant regression. |
| tests `streaming_matches_one_shot` | 208-220 | REAL (honest) | — | 1000 bytes one-at-a-time vs one-shot — kills buffer bugs. |
| tests `chunked_boundaries_agree` | 225-235 | REAL (honest) | — | 7 chunk sizes incl. 63/64/65 — partial-block edges. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rfc1321_vectors | YES | Spec truth; one wrong constant/shift/g-index breaks every vector. |
| streaming_matches_one_shot | YES | The port reproduced the equivalent invariant; buffer state bugs diverge immediately. |
| chunked_boundaries_agree | YES | Non-64-aligned chunks exercise exactly the partial-buffer branch. |
Execution: UNVERIFIED in this sandbox (no cargo/rustc on PATH — load discipline); the baseline cargo-test-workspace run does NOT cover this crate (standalone workspace), so no orchestrator baseline row exists for these 3 tests. Algorithm-level verification done by faithful port (§2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "MD5 from RFC 1321 in-tree — zero new dependencies" | docs/PERF-VS-VLC.md:51 | REAL | Cargo.toml [dependencies] = path deps only (read); no digest crate. |
| "the fresh Lumen digest equals the ffmpeg 7.1.5 decode (md5 + byte count)" on 35/35 rows | docs/PERF-VS-VLC.md:50-55 | REAL (committed evidence) | vlc-diff-report-20261023.md Oracle-run table: 35 comparable rows with Lumen digests from this code matching the oracle-side digests; my PERF-VS-VLC ledger (claim #3) recounts 29 MATCH-prefix + 6 f32 rows. |
| "never enters engine crates" (test-tool only) | md5.rs:7-11 | REAL | Only consumer is tools/bench-vs-vlc (rg `bench_vs_vlc::md5` -> tests/lumen_pcm.rs). |

## 7. Dead / duplicated / unreachable code
None. `Default` impl (:51-55) is idiomatic, not dead. No duplicate MD5 elsewhere in
tools/ (rg for `fn compress` in bench-vs-vlc -> only this file).

## 8. Dependencies & flags
No external dependencies (the whole point); crate is a standalone workspace with its own
lint mirror ([lints] workspace = true against its own [workspace.lints] table — read in
Cargo.toml). `unsafe_code = deny` — file has zero unsafe blocks.

## 9. Open questions for the next agent
None. If a cargo-enabled session occurs, `cargo test --manifest-path
tools/bench-vs-vlc/Cargo.toml` would convert the three tests from REAL-UNPROVEN to
REAL at the execution level (the port already proves the algorithm).
