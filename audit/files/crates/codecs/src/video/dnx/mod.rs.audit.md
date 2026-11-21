# AUDIT: crates/codecs/src/video/dnx/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-11-21 |
| Lines / functions | 46 / 3 items (crate-doc module map, 2 `mod` declarations, 1 re-export line) — no function bodies |
| File verdict | UNREACHABLE (module compiled + dispatch-wired, but no shipped entry point can produce a `dnxhd` track) |
| Reachable from shipped entry point? | NO — same verdict as my error.rs audit: `DnxDecoder` is compiled (default features) and wired at apps/cli/src/pipeline_host.rs:326, but zero demuxer/container/probe code mints the `dnxhd` descriptor (rg: no hits in crates/demux, apps/cli/src/containers, media-probe) and the CLI has no codec override. This module's own doc admits the gap (35-41: "Container / elementary-stream wiring … is the Lead's reconciliation"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | full `Read` (46 lines); `rg -n "lsb|tolerance"` on crates/codecs/tests/dnx_differential.rs (tolerance gate exists and matches the doc claim); baseline cite for the differential run (test-workspace.log.txt:934-943, 6/6 ok); `rg -n "filmcraft"` codecs/Cargo.toml:158 (pin matches doc); reachability rg set from my error.rs audit (zero `dnxhd` producers) |

## 1. One-paragraph verdict
A 46-line facade whose every factual claim checks out against code or committed runs: the engine pin quoted in the doc is byte-identical to the Cargo.toml pin (`=0.9.0`, `default-features = false`), the "±2 LSB bulk ±1, balanced signs" IDCT-tolerance story is exactly what the differential suite enforces (and that suite ran 6/6 in baseline), and the scope note honestly declares the container wiring as future work — which is the file's real defect: that future is why the entire module is dead code in shipped builds. One typo: "MKF" (line 38) is neither MKV nor MXF.

## 2. What actually works (verified)
— Module map honesty: `pub mod decoder; mod error;` matches the two sibling files; the re-export line (46) resolves (`decoder.rs` defines `DnxChroma`, `DnxColorVolume`, `DnxDecoder`, `DnxPicture` — seen in my error.rs cross-reads of decoder.rs:212-270).
— Engine pin: doc lines 18-23 quote `filmcraft-dnx = { version = "=0.9.0", default-features = false }` — identical to codecs/Cargo.toml:158; the rayon-free rationale matches the record's binding condition (already verified by the existing Cargo.toml audit against Cargo.lock).
— Tolerance claims: the doc's "±2 LSB (bulk ±1, balanced signs)" is enforced as hard gates in `tests/dnx_differential.rs` (per-plane max|Δ| ≤ 2 LSB, byte-count parity, sign-balance < 3:1, outliers < 1%) — and that suite executed: `tests/dnx_differential.rs … 6 passed` (test-workspace.log.txt:934-943).

## 3. Findings
### F-001 [S3] The module ships dead: no container lane exists to reach it (doc admits, binary still carries it)
Where: crates/codecs/src/video/dnx/mod.rs:35-41
Code:
```rust
//! This card is the codecs-crate decode surface. Container /
//! elementary-stream wiring (AVdn/AVdh sample entries, MKF, the
//! CBR ES splitter the raw-`.dnxhd` lanes assume) is the Lead's
```
What is wrong: The scope note names exactly the missing wiring (MOV/MP4 `AVdn`/`AVdh` sample entries, the Matroska/MXF mappings, a raw-ES lane) — none of it exists in the tree (my error.rs audit F-001 rg evidence: zero `dnxhd` descriptor producers). So the module is compiled into every shipped binary (default features) and registered in the pipeline host's dispatch, but no user input can ever construct a track that reaches it. Full detail and the fix options are in my error.rs audit F-001 (same root cause); repeated here because this is the module root making the admission.
Why it matters: See error.rs F-001 — dead-but-shipped decoder surface with descriptor-contract rot risk.
What correct looks like: Same as error.rs F-001: land one minting lane or feature-gate the module until one exists.
Confidence: HIGH — Verification: read + the rg set cited in error.rs audit.
### F-002 [S4] "MKF" — a typo naming a container that does not exist
Where: crates/codecs/src/video/dnx/mod.rs:38
Code:
```rust
//! elementary-stream wiring (AVdn/AVdh sample entries, MKF, the
//! CBR ES splitter the raw-`.dnxhd` lanes assume) is the Lead's
```
What is wrong: "MKF" is neither MKV (Matroska — what pipeline_host.rs:322 says owns the future mapping) nor MXF (the pro-video container DNxHD actually rides in the wild). One of the two was meant.
Why it matters: A scope note that misnames the future integration target sends the next implementer hunting a nonexistent format; the pipeline_host comment and this doc already disagree (mkv vs MKF).
What correct looks like: Pick the intended container (likely both: `MKV (V_MS/FOURCC)` and `MXF`) and spell it.
Confidence: HIGH — Verification: read + cross-ref pipeline_host.rs:319-324.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crate docs (module map, engine pin, tolerance, scope) | 1-41 | REAL | S3/S4 | F-001 (dead module admission), F-002 (MKF typo); every other claim verified against Cargo.toml + differential suite |
| `pub mod decoder;` / `mod error;` | 43-44 | REAL (unreachable) | — | error is private (pub(crate) types inside) — correct scoping |
| re-export line | 46 | REAL (unreachable) | — | all four names resolve in decoder.rs |

## 5. Test honesty
No tests in this file. The module's real evidence lives in `decoder::tests` (11 tests, baseline log 535-545) and `tests/dnx_differential.rs` (6/6, log 934-943) — both cited in §2.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`filmcraft-dnx = { version = \"=0.9.0\", default-features = false }` … keeps the lockfile rayon-free" | mod.rs:18-23 | REAL | codecs/Cargo.toml:158 identical; Cargo.lock check in the existing Cargo.toml audit |
| "bit-exactness vs ffmpeg is unreachable in principle … differential class is ±2 LSB (bulk ±1, balanced signs), Lead-ratified" | mod.rs:25-33 | REAL | dnx_differential.rs implements exactly those hard gates; ran 6/6 ok in baseline (:934-943). "Lead-ratified" itself is a process claim — UNVERIFIED beyond the gate's existence |
| "the packet contract here is 'one packet = one complete VC-3 frame'" | mod.rs:40-41 | REAL (unreachable) | decoder.rs decode path (other agent's audit); nothing in-tree produces such packets |
| "MKF" as a wiring target | mod.rs:38 | FALSE (typo) | no such format; MKV/MXF both plausible intents (F-002) |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable in shipped builds (F-001) — see error.rs audit for the full rg evidence.
- No duplication within the file; the 6.M-opus module shape matches the mjpeg/mpeg12 module conventions.

## 8. Dependencies & flags
- `codec-dnx = ["dep:filmcraft-dnx"]` IS in the default feature set (codecs/Cargo.toml:56 area) → compiled into every shipped build including the baseline `lumen` binary (engine-info's decoder list under-reports it — the hard-coded literal issue, INDEX.md standing defects).
- Wave-6 hostile question: dnx is default-enabled yet unreachable (no descriptor producer) — the mirror image of H.264 sw (default-enabled AND reachable via containers minting `h264`/`avc1`, pipeline_host.rs:346, baseline sample-420.mp4 plays). Full trace: mjpeg/decoder.rs#part2.audit.md §8.

## 9. Open questions for the next agent
- decoder.rs#part1/2 owners: the "full-depth DnxPicture seam" and the CBR gate promised here (lines 10-12) — your audit should confirm the gate's declared-size arithmetic and that `DnxPicture` is not a dead export (who consumes it besides tests?).
- Whoever lands the AVdn/AVdh lane: fix the MKF typo in the same PR (F-002) and mint exactly `dnxhd` (single spelling — see error.rs audit).
