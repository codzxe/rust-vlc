# AUDIT: tools/vta/mpeg4/src/main.rs
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 332 / 6 (demux_lane, frame_to_i420, run, run_case, hostile, main) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — tools/vta/mpeg4 is a standalone non-workspace binary (its Cargo.toml pins an empty `[workspace]`); not in the lumen-cli/desktop build |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | sed/rg on source + Cargo.toml/Cargo.lock/README; attempted `cargo build` in tools/vta/mpeg4 — BLOCKED (no cargo/rustc toolchain in this sandbox; /tmp/my-project/bin contains only lumen/lumen-release) |

## 1. One-paragraph verdict
This is a genuinely implemented differential-harness CLI: it demuxes AVI/MP4
through the repo's own demuxers, decodes MPEG-4 Part 2 with the
crates.io crate `oxideav-mpeg4video =0.1.8` (registry-verified in
tools/vta/mpeg4/Cargo.lock:76-79), crops macroblock-padded planes to VOL
display dims into ffmpeg-rawvideo-layout I420, prints per-frame `FRAME`
lines, and runs a real hostile battery (truncate sweep, zero-length, garbage,
VOP-header corruption, garbage-then-real recovery, truncated mid-stream) under
catch_unwind. It is NOT the source of any "bit-exact" claim — the README
snapshot shows 0 identical frames on 7/8 lanes (max delta 3) and the crate was
BLOCKED; the oracle side is external ffmpeg 7.1.5 driven by diff.py. I could
not execute it (no Rust toolchain in the sandbox), so REAL-UNPROVEN.

## 2. What actually works (verified)
— `rg -n 'members' -A12 Cargo.toml` + `tools/vta/mpeg4/Cargo.toml` ([workspace]
empty) -> confirms standalone binary outside the shipped build.
— `ffmpeg -f lavfi ... -c:v mpeg4 -y /tmp/vta-audit/lanes/g_base.mp4` ->
LANE_OK, ffprobe: `stream,mpeg4,320,240` (lane generation works; harness
execution blocked by missing toolchain).
— Static read of all 332 lines: every CLI mode (`--out`, `--ecosystem`,
`--extradata`, `--hostile`) is wired to real code paths; no todo!/stub.

## 3. Findings
F-001 [S3]
Where: tools/vta/mpeg4/src/main.rs:70
Code:
```
    assert!(dw <= pw && dh <= ph, "display {dw}x{dh} > padded {pw}x{ph}");
```
What is wrong: decoder/VOL inconsistency aborts the whole harness with a panic
message instead of a typed error naming the lane; every other failure in the
file is a `Result<String>`. Hostile inputs intentionally hit this region.
What correct looks like: return Err with lane context.
Confidence: HIGH — Verification: read.

F-002 [S3]
Where: tools/vta/mpeg4/src/main.rs:300-307
Code:
```
            let clean: String = hex
                .chars()
                .filter(|c| c.is_ascii_hexdigit() || *c == ' ' || *c == '\n')
                .collect::<String>()
                .replace([' ', '\n'], "");
            let clean = clean.trim_start_matches("0x").to_string();
            if clean.len() % 2 != 0 {
                return Err(format!("odd hex length in {p}"));
            }
```
What is wrong: any non-hex garbage char (e.g. `g`, `x` mid-string, `:` ok but
`0x0g` variants) is silently DROPPED, so a malformed extradata file is
mis-parsed into different bytes with exit 0 — the harness then primes the
decoder with corrupted config and the differential silently measures the wrong
thing. Only odd total length is caught. `trim_start_matches("0x")` also strips
repeated leading "0x0x..." pairs.
What correct looks like: reject any char outside [0-9a-fA-F \n] with the
 offending position.
Confidence: HIGH — Verification: read; mutation reasoning (no toolchain to run).

F-003 [S4]
Where: tools/vta/mpeg4/src/main.rs:287-298
Code:
```
    let out = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned();
```
What is wrong: `--out` (and `--extradata`) with a missing operand is silently
ignored (None) instead of a usage error — a run intending to write raw output
proceeds and prints "0-file" results with exit 0.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: tools/vta/mpeg4/src/main.rs:15
Code: `use demux::avi::test_support::mem_stream;`
What is wrong: production harness reaches into the demuxer's `test_support`
module — layering smell; if AVI test-support changes, the harness breaks.
Confidence: HIGH — Verification: read (crates/demux/src/avi/mod.rs:121 `pub mod test_support;`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| demux_lane | 29-64 | REAL-UNPROVEN | — | full read+demux loop, picks first Video track, collects packets/pts/keyframes; swallows Recovered/Discontinuity (intended); pts error -> None silently |
| frame_to_i420 | 68-84 | REAL-UNPROVEN | S3 | tight crop; assert panic (F-001); chroma stride pw/2 assumes even padded width (holds: MB-aligned) |
| run | 86-182 | REAL-UNPROVEN | — | emit closure writes raw + FRAME lines, monotone-pts check, decode-error counter (max 3 printed), flush pass; correct |
| run_case | 184-210 | REAL-UNPROVEN | — | catch_unwind hostile case; counts panics; honest |
| hostile | 214-276 | REAL-UNPROVEN | — | 6 real cases incl. full 0..len truncate sweep on keyframe; corrupt-byte flip after first start code; honest counters |
| main | 278-332 | REAL-UNPROVEN | S4 | arg parsing; silent missing operands (F-003); hex filter defect (F-002) lives here |

## 5. Test honesty
No tests in this file (it is itself a test harness). Mutation reasoning: the
README's claimed hostile outputs (`0/9595, 0/2872, 0/6617` prefixes) could not
be re-executed — no cargo toolchain. UNVERIFIED.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Hostile: 0 panics ... Decode errors: 0 packets across all lanes" | tools/vta/mpeg4/README.md | REAL-UNPROVEN | code paths exist (L232-274); no toolchain to re-run |
| Results table: 0 identical frames on 7/8 lanes, max Δ 3 (NOT bit-exact) | tools/vta/mpeg4/README.md | REAL-UNPROVEN | honest negative result; diff.py measures md5/pixel deltas per frame |
| "Verdict: BLOCKED — tolerance-class differential ±1..3 ... NOT a dependency of any workspace crate" | tools/vta/mpeg4/README.md | REAL | rg 'oxideav' in crates/*/Cargo.toml -> only tools/vta; record docs/dependencies/records/oxideav-mpeg4video-0.1.8.md R10 confirms BLOCKED |

## 7. Dead / duplicated / unreachable code
None found; every function is called (run_case only from hostile; emit only
from run). File is unreachable from shipped binaries by design (spike tool).

## 8. Dependencies & flags
oxideav-mpeg4video =0.1.8 (crates.io, checksum in lock), path deps
demux/plugin-api/media-types from this repo. No features. dev profile
opt-level=1. No unsafe in this file.

## 9. Open questions for the next agent
Can any agent with a toolchain re-run one mpeg4 lane + hostile battery to
convert the README snapshot to REAL? Is the evidence table (2026-10-26)
regenerable from the committed diff.py alone?
