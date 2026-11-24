# AUDIT: tools/vta/ac3/src/main.rs
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / functions | 323 / 5 (frame_len_bytes, syncwalk, decode_file, md5, main) + 2 subcommands (hostile, cplprobe) |
| File verdict | REAL (harness does what it says; evidence outputs match the record; NOT production) |
| Reachable from shipped entry point? | NO — by design: standalone `[workspace]` (tools/vta/ac3/Cargo.toml:1), NOT in root Cargo.toml members (root lists crates/*, apps/*, tools/xtask, tests/integration/first-vertical, tools/mem-audit, tools/sweep). README: "ARCHIVED — NOT ADMITTED ... evidence only" |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | python3 T5_18 formula spot-checks; rg lane names (3 files only); sed Cargo.lock (oxideav-ac3 0.0.11 + oxideav-core 0.1.42 pinned); registry cache listing (oxideav tarballs present) |

## 1. One-paragraph verdict
A real differential-testing harness: syncwalker -> oxideav-ac3 decode ->
ffmpeg-reference compare (exact/differing/maxLSB + MD5) -> hostile inputs under
catch_unwind -> side-info census. The AC-3 frame-size table is numerically
correct (spot-verified against the A/52 closed forms: 48k words = 2*kbps, 32k
words = 3*kbps, 44.1k alternating 69/70 for 32kbps — all match), bounds in
syncwalk are safe (i+5 guard before data[i+4] read, next[0]/[1] guarded by
is_empty), and the harness's printed columns line up 1:1 with the evidence
record docs/dependencies/records/oxideav-ac3-0.0.11.md (8 lanes, 0 dec errors,
0 resyncs, 6 hostile cases, cplprobe) — the record's "BLOCKED" verdict (3/8
lanes 202-431 LSB off ffmpeg) was honestly produced from this code and the crate
was NOT admitted (no crates/codecs wiring — verified via root Cargo.toml
members). The harness is unreachable from shipped builds by construction. It
cannot be re-executed here (cargo build forbidden; /tmp/vta-ac3 lanes absent), so
the differential numbers themselves are REAL-UNPROVEN on my side, but the record
+ registry cache (oxideav tarballs downloaded in this sandbox) corroborate that
the session ran. One real doc lie: the README points lane-generation commands
to "record §D", which contains none.

## 2. What actually works (verified)
- T5_18: python3 — 32kbps@48k → 64 words (128 B) ✓, 640kbps@48k → 1280 words ✓, 44.1k@32kbps → 69/70 alternating ✓, 32k → 96 words ✓. Table rows 0/18 + closed-form comments are correct.
- Bound safety: `while i + 5 <= data.len()` guards `data[i+4]`; `next.is_empty()` guards `next[0]`; `i + len > data.len()` pushes trailing frame then breaks — no panic path in syncwalk.
- Cargo.lock pins oxideav-ac3 0.0.11 (checksum efb5f931…) + oxideav-core 0.1.42 — matches record R1/R2 exactly.
- Lanes/cases cross-check: main.rs lanes vector (8) == record §D rows (8); hostile() 6 cases == record §H ("0 panics / 6 cases"); cplprobe == record's census.
- Registry cache contains oxideav tarballs (downloaded in this sandbox) — the record's "measured in this build sandbox" is corroborated.
- `wait_with_output()` closes stdin (std docs) — the md5() piped-write pattern does not deadlock.

## 3. Findings
F-001 [S3]
Where: tools/vta/ac3/README.md:31-32 vs docs/dependencies/records/oxideav-ac3-0.0.11.md:90-133
Code:
```
# data: corpus lane + generated lanes live in /tmp/vta-ac3 (see
# record §D for the exact generation commands)
```
What is wrong: record §D contains the RESULTS table + characterization — ZERO
generation commands. Only ONE lane's ffmpeg command is in the README; the other
7 lanes' generation commands exist nowhere in the tree (rg: lane names appear in
only 3 files). Why it matters: the differential evidence is not reproducible
from the tree — a re-auditor cannot regenerate 7 of 8 lanes, so the BLOCKED
verdict's numbers cannot be independently re-derived. What correct looks like: a
`gen-lanes.sh` under tools/vta/ac3/ with all 8 ffmpeg invocations.
Confidence: HIGH — Verification: rg across docs/ worklog.md tools/ (3 files,
no commands).

F-002 [S4]
Where: tools/vta/ac3/src/main.rs:137
Code:
```
                    channels = (a.data[0].len() / 2 / a.samples as usize) as u32;
```
What is wrong: division by `a.samples` — an audio frame with samples=0 panics
(div by zero), and the formula assumes exactly one interleaved s16le plane.
Harness lanes are known-good so it never fires, and the hostile subcommand
exercises a DIFFERENT path (no channel computation there). Confidence: HIGH —
Verification: read.

F-003 [S4]
Where: tools/vta/ac3/src/main.rs:322
Code:
```
    println!("cplprobe {path}: {frames} frames, {cpl_blocks}/{total_blocks} cplinu {cpl_blocks}/{total_blocks}, dithflag {dith_blocks}/{total_blocks}");
```
What is wrong: cpl ratio printed TWICE (identical `{cpl_blocks}/{total_blocks}`
in both slots) — a copy-paste artifact; one slot presumably meant something
else. Pure-census noise; output still parses. Confidence: HIGH — Verification:
read.

F-004 [S4]
Where: tools/vta/ac3/src/main.rs:31,196,200
Code:
```
const DIR: &str = "/tmp/vta-ac3";
```
What is wrong: hard-coded absolute scratch path for lanes/outputs — re-runs stomp
prior outputs (out_*.s16 / ff_*.s16 keyed only by lane name, no run id);
evidence provenance relies on the record copying the numbers out.
Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory; file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| frame_len_bytes | 57-70 | REAL (algorithm verified numerically) | — | frmsizecod>=38 / fscod>2 guards; 44.1k row select by parity — correct |
| syncwalk | 75-107 | REAL (bounds verified by reading; not executed) | — | resync counting; truncated-tail push; no panic path found |
| decode_file | 110-150 | REAL-UNPROVEN | S4 | channels div-by-zero on samples=0 (F-002); error counting honest |
| md5 | 152-168 | REAL | — | shells to md5sum; stdin closed by wait_with_output (no deadlock) |
| main | 170-230 | REAL-UNPROVEN | — | 8 lanes; ffmpeg assert!(success) fail-loud; prints exact/diff/maxd + 16-char md5 prefix |
| hostile | 232-295 | REAL-UNPROVEN | — | 6 cases, catch_unwind, fresh decoder per case + warm-up — real hostile methodology |
| cplprobe | 299-323 | REAL-UNPROVEN | S4 | duplicate cpl ratio print (F-003); crate-parser API (syncinfo/bsi/audblk) UNVERIFIED (crate source not in tree) |

## 5. Test honesty
The harness IS the test rig for the record. Mutation reasoning: if the decoder
were broken vs ffmpeg, exact/diff/maxd would show it (record: 77-97% differing,
max 431 LSB — the harness FAILED its own gate and the verdict says BLOCKED —
honest negative evidence, the opposite of fraud). If syncwalk mis-synced,
resyncs>0 would print (record claims 0 on all lanes). The harness cannot
silently pass: any decode error increments and prints. md5 prefix truncation to
16 chars is display-only; the record independently states "raw MD5s differ
everywhere" — consistent.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "measured this session (2026-10-29) in the build sandbox" | record:7 | REAL-UNPROVEN-CORROBORATED | oxideav tarballs present in ~/.cargo/registry cache; number format matches harness output |
| 8 lanes, 0 dec err, 0 resyncs, 3 DIVERGENT (316/202/431 LSB) | record §D | REAL-UNPROVEN | lane set == main.rs lanes; re-run needs crate build (forbidden) |
| hostile: 0 panics / 6 cases | record §H | REAL-UNPROVEN | 6 cases in hostile() match by name |
| "see record §D for the exact generation commands" | README:31-32 | FALSE | §D has no commands (F-001) |
| "HARNESS ONLY — never production" | main.rs:2-4 | TRUE | standalone workspace; not in root members; no crates/codecs reference |

## 7. Dead / duplicated / unreachable code
No dead code found. cplprobe's duplicate print is the only sloppiness.
Reachability: NO by design — the harness never enters any shipped binary.

## 8. Dependencies & flags
oxideav-ac3 0.0.11 + oxideav-core 0.1.42, pinned with checksums in
tools/vta/ac3/Cargo.lock — matches the record's pin demands. No features.
External runtime deps: ffmpeg on PATH (asserted), md5sum, /tmp/vta-ac3 lanes.

## 9. Open questions for the next agent
- Re-run the differential when cargo-build budget allows; the BLOCKED verdict
  currently rests on one unreplicable session (lanes not regenerable, F-001).
- Does `oxideav_ac3::{syncinfo,bsi,audblk}` API exist as used (compile check)?
  UNVERIFIED — crate not in tree, build forbidden.
