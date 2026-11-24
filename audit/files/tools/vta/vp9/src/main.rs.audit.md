# AUDIT: tools/vta/vp9/src/main.rs
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Date | 2026-11-24 |
| Lines / functions | 230 / 7 (frame_to_i420, frame_to_i420_hbd, demux_packets, decode_all, main, try_feed, hostile) |
| File verdict | REAL (complete differential harness; execution recorded in docs/dependencies/records/rusty_vp9-0.1.3.md; cannot build here — no cargo in audit sandbox) |
| Reachable from shipped entry point? | NO (by design: standalone non-workspace VTA spike tool, own `[workspace]` in Cargo.toml; never linked into lumen-cli/desktop) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | sed/rg/cat (source + Cargo.toml + README + record) — exit 0; cargo ABSENT in sandbox (gate.sh run: "cargo not found on PATH") |

## 1. One-paragraph verdict
A genuine VP9 differential harness: IVF via OUR demuxer (`demux::ivf::IvfDemuxer`),
decode via `rusty_vp9::Vp9Decoder`, tightly-packed I420/u16LE rawvideo output plus
per-frame prints — the exact shape its README specifies, and its execution results
(frame counts, MD5 parity, hostile battery) are recorded in
docs/dependencies/records/rusty_vp9-0.1.3.md. The demux dependency is legal:
`demux::avi::test_support` is a deliberately public module (documented as such in its
own doc-comment), not a cfg(test) leak. The hostile battery is real (catch_unwind,
7 case classes + full off-by-one sweep). One robustness bug: `hostile()` itself
indexes `pkts[0]` unguarded — a valid EMPTY .ivf panics the no-panic battery.
Cannot be built or executed in this audit sandbox (no cargo toolchain) — execution
claims rest on the committed record.

## 2. What actually works (verified)
- Source review: `demux_packets` really drives `IvfDemuxer` (L64-77) with `mem_stream` from `demux::avi::test_support` — public-by-design module (crates/demux/src/avi/test_support.rs:1-8 "public on purpose"; crates/demux/src/avi/mod.rs:121 `pub mod test_support;`).
- `frame_to_i420` repack arithmetic: `w_uv = (w + subsampling_x) >> 1` etc. — correct I420 layout; byte layout matches ffmpeg rawvideo as the record's lanes claim.
- README honesty: digests computed EXTERNALLY "because a hand-rolled in-harness MD5 was tried first and was WRONG — the whole-stream md5sum cross-check caught it" (tools/vta/vp9/README.md) — an honest defect record; the record's table (rusty_vp9-0.1.3.md:165-175, "0 differing frames anywhere") is the executed evidence.
- Cargo.toml pins `rusty_vp9 = "=0.1.3"` + path deps on our demux/plugin-api; empty `[workspace]` table correctly pins it to its own workspace root.

## 3. Findings
F-001 [S3]
Where: tools/vta/vp9/src/main.rs:162-163
Code:
```
fn hostile(pkts: &[Vec<u8>]) {
    let first = &pkts[0];
```
What is wrong: the no-panic battery crashes on its own input contract — a valid .ivf
with zero packets (32-byte header, no frames) demuxes to `pkts == []` and this index
panics before ANY hostile case runs. Why it matters: the one tool whose job is to
prove panic-freedom panics on a legal input; a future run against a truncated/empty
lane dies confusingly. What correct looks like: an empty guard printing
"hostile: no packets". Confidence: HIGH — Verification: read (panic path trivially
reachable; no build possible to execute).
F-002 [S4]
Where: tools/vta/vp9/src/main.rs:196
Code:
```
            while !matches!(dec.next_frame(), Err(rusty_vp9::Error::Eof)) {}
```
What is wrong: the off-by-one sweep loop has no guard against a decoder that keeps
returning `Again` forever — catch_unwind catches panics, not hangs. UNVERIFIED
whether rusty_vp9 guarantees post-flush termination (registry source not in sandbox).
Confidence: MEDIUM — Verification: read.
F-003 [S4]
Where: tools/vta/vp9/src/main.rs:132-139
Code:
```
    if let Some(o) = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|o| args.get(o + 1))
```
What is wrong: `--out` as the last argument (missing filename) is silently ignored —
no raw file written, no error. Minor CLI contract softness. Confidence: HIGH — read.
F-004 [S4]
Where: tools/vta/vp9/src/main.rs:113-131 vs README comparator
Code:
```
    let mut dims = (0u32, 0u32);
    for (i, f) in frames.iter().enumerate() {
        dims = (f.width, f.height);
```
What is wrong: `DIMS` prints only the LAST frame's dimensions and the external MD5
comparator (README) chunks by a FIXED frame size — a VP9 stream that changes
resolution mid-stream (legal in VP9) would silently misalign every subsequent
per-frame digest comparison. All measured lanes were fixed-dim, so no recorded
result is affected. Confidence: HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| frame_to_i420 | 16-36 | REAL | — | 8-bit I420 repack, correct strides/subsampling math; assert guards bit_depth==8 |
| frame_to_i420_hbd | 39-56 | REAL | — | u16LE repack for 10/12-bit; chunks(2) copy is byte-faithful LE |
| demux_packets | 58-77 | REAL | — | real IvfDemuxer + mem_stream; Recovered/Discontinuity intentionally ignored; typed String errors |
| decode_all | 79-87 | REAL | — | push/flush/next_frame loop; Eof breaks, Again continues, other errors abort with frame-count context |
| main | 89-140 | REAL | — | usage exit 2; DEMUX/DECODE-ERROR exit 1; per-frame prints; --out (F-003) |
| try_feed | 142-160 | REAL | — | catch_unwind + outcome string; reports PANIC — FAIL on unwind |
| hostile | 162-230 | REAL | — | 7 case classes + per-packet off-by-one sweep + garbage-then-real recovery; F-001 empty-input panic, F-002 hang risk |

## 5. Test honesty
N/A (it IS the test harness). Mutation reasoning: the battery would FAIL loudly
(prints "PANIC — FAIL") if rusty_vp9 panicked on any case — the record's committed
"panic-free" line (rusty_vp9-0.1.3.md:198) is the executed output of exactly this code.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decodes ... through OUR ivf demuxer into rusty_vp9 0.1.3, writing tightly packed I420 rawvideo" + per-frame lines; digests external | tools/vta/vp9/README.md | REAL | source implements exactly this; digests-external note is an honest design decision with its own defect history |
| "Evidence lives in docs/dependencies/records/rusty_vp9-0.1.3.md" | README:9 | REAL | record carries the lane table + hostile battery results |
| harness runs: differential lanes + hostile | rusty_vp9-0.1.3.md:158-198 | REAL-UNPROVEN | committed record is the only execution evidence; no cargo in this sandbox to re-run |

## 7. Dead / duplicated / unreachable code
None dead. File is UNREACHABLE from shipped entry points by design (standalone
spike workspace; its own Cargo.lock).

## 8. Dependencies & flags
rusty_vp9 =0.1.3 (registry), demux + plugin-api (path). No features. No cargo
available in the audit sandbox (command -v cargo empty; gate.sh output "cargo not
found on PATH").

## 9. Open questions for the next agent
- rusty_vp9-0.1.3.md owner: cross-check the record's "0 differing frames" claims
  against this harness's output format (frame lines + external md5) — the wiring
  verified here makes the record's method claim coherent.
- If anyone re-runs the battery on an empty IVF, F-001 will fire — fix belongs to
  the VTA tools owner.
