# AUDIT: crates/codecs/examples/color_bench.rs#part1
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-21 |
| Lines / functions | 1–510 of 1022 (chunk part1 of 2; part2 ≈511–1021 W06-A05) / 18 items (module doc, `mod legacy` with 10 members, `mod now` include, `RowComposer` + 2 impls, `Fill`, `Sampling`, `Planes`, `Rng`, `build_planes`, `compose_frame`) |
| File verdict | REAL-UNPROVEN (genuine benchmark harness, compiles clean in baseline clippy `--all-targets`; no recorded run in baseline — docs themselves mark the re-run as "owed at merge") |
| Reachable from shipped entry point? | NO — a `cargo --example` target; the release workflow builds `-p lumen-cli -p desktop` + the tauri manifest only (release.yml:193,474,627,763,911). It is dev tooling, correctly not shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "^(fn…)" color_bench.rs` (structure map); `rg -n "compose_frame\|checksum\("` → all 7 call sites discard the u64; `sed -n '661,790p'` (main's equality mechanism, part2 context); `rg -n "seed\|Rng\("` → seed literal duplicated 443/634; `rg -n "color_bench" docs/ worklog.md` → no committed bench output; `rg -c "Checking codecs" audit/runs/baseline/clippy-workspace.log.txt` → 1 (all-targets covers examples) |

## 1. One-paragraph verdict
Part1 is the benchmark's fixed half: the frozen pre-optimization YCbCr→RGBA kernel (`mod legacy`), the `#[path]` include of the live `color.rs` (`mod now`), the plane generator with decoder-mirroring MCU geometry, a deterministic xorshift64* RNG, and the shared timed-loop body. The geometry claim is real — I verified `build_planes`'s `mcu_w·h_max·8` / `mcu_w·8` strides and `⌈w·h/h_max⌉` valid extents match `decoder.rs:736-746` exactly — and main's cross-arm bit-exactness check (part2) is a full byte compare, not the weak checksum. Two hygiene defects: the documented purpose of `compose_frame`'s checksum return ("DCE guard + cross-arm equality evidence") is false — every call site discards it; and the RNG seed literal exists in two independent copies. The "legacy kept verbatim @ merge 62c48b6" provenance claim is UNVERIFIABLE under this session's no-git rule.

## 2. What actually works (verified)
— Compiles clean: baseline `cargo clippy --workspace --all-targets -- -D warnings` → exit 0 (INDEX.md #5; `Checking codecs` present in clippy-workspace.log.txt) — `--all-targets` includes this example.
— Geometry mirror claim verified: `build_planes` (433-438) `y_stride = mcu_w·h_max·8`, `c_stride = mcu_w·8` == `decoder.rs:736-738` `block_cols = mcu_w·comp.h`, `stride = block_cols·8`; `valid_w` shapes match `decoder.rs:746`.
— Cross-arm equality is a REAL full compare: main (part2, 741-747) runs each fixed-point arm into `scratch` and asserts `scratch == reference` byte-for-byte, printed as `bitexact_vs_legacy` — verified by reading part2's `main`.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/examples/color_bench.rs:496-499
Code:
```
/// Compose one full frame through a module's `RgbComposer` (the exact
/// `assemble_rgba_into` loop shape: one composer, one row call per
/// output row). Returns a checksum over the output (DCE guard +
/// cross-arm equality evidence).
```
What is wrong: The returned checksum is discarded at every one of its call sites (`rg -n "compose_frame"` → 694, 701, 708, 716, 724, 963 — all statement-position calls; `checksum(out)` inside `compose_frame_scalar`/`compose_frame_simd` (902, 913) likewise feeds only the discarded return). It is neither a DCE guard (an unconsumed return value guards nothing) nor the cross-arm equality evidence (that is main's `scratch == reference`). The only thing keeping the timed work from theoretical dead-store elimination is the accidental `Box<dyn FnMut>` boundary in `Arm`, not this checksum.
Why it matters: The comment documents a measurement-integrity mechanism that does not exist; a reader trusting it could remove the dyn boundary believing the checksum protects them. The dead checksum loop also runs inside the timed region (uniform across arms, ~µs — negligible but nonzero).
What correct looks like: Either consume the checksum (e.g. `black_box(compose_frame(...))`) and delete the false comment, or drop the return and rely on main's byte-equality, documented as such.
Confidence: HIGH — Verification: read + rg of all call sites.
### F-002 [S4]
Where: crates/codecs/examples/color_bench.rs:443
Code:
```
    let mut rng = Rng(0x5eed_0000_0000_0603);
```
What is wrong: The seed literal is duplicated: the actual seed here (443) and the printed provenance `println!("# seed=0x5eed000000000603 …")` in `print_machine_header` (634, part2's range) are two independent hard-coded copies.
Why it matters: Changing one without the other makes the run's printed reproducibility claim (`# seed=…`, the honest-numbers mandate) silently false — the printed seed would no longer describe the data.
What correct looks like: One `const SEED: u64` used by both, or pass the seed into the header printer.
Confidence: HIGH — Verification: read + rg (only two `Rng(`/seed sites).

## 4. Per-function table (part1 range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-60 | REAL | — | Protocol claims (21 reps, 2 warmup, interleaved, medians, TSV) all match part2's main (REPS=21, WARMUP=2, interleaved `for arm` inner loop); "re-run owed at merge" honestly recorded in docs/perf/v1-input-readpath-20261024.md:112-114. |
| `mod legacy` (frozen kernel) | 66-328 | REAL-UNPROVEN | — | Internally consistent pre-W6-03 kernel: per-sample `.min` clamps, sign-branch `shr16_round`, per-sample checked fallback — exactly the "old loop" the live color.rs docs (1-50) describe replacing. Provenance "@ merge commit 62c48b6, verbatim" UNVERIFIED — no git commands permitted this session. |
| `legacy::ycc_to_rgb` | 75-87 | REAL | — | Q16 fixed point, clamp 0..=255; same constants as live color.rs:72-75 (identical values — bit-exactness of the comparison arms is credible). |
| `legacy::shr16_round` | 90-96 | REAL | — | Round-half-away-from-zero, sign branch — the pre-branchless form. |
| `legacy::PlaneView`/`ComponentGeom` | 100-116 | REAL | — | Mirror of live types (part2's build_* maps both). |
| `legacy::RowSampler::{new,sample,step}` | 135-180 | REAL | — | Checked row proof once per row; exact incremental `floor(x·h/h_max)` stepper — no division; cold-path `None` documented. |
| `legacy::RgbComposer::{new,compose_row}` | 194-242 | REAL | — | Hot/cold dispatch: out.len() check + row-bound proof; cold path for degenerate geometry — honest structure. |
| `legacy::compose_color_row` / `compose_gray_row` | 246-281 | REAL | — | Direct indexing after proof; per-pixel stepper carries. |
| `legacy::compose_row_cold` / `sample_up` | 289-326 | REAL | — | Checked `.get_mut`/`.unwrap_or(&128)` — the documented degenerate-read semantics. |
| `#[path] mod now` | 332-333 | REAL | — | Includes the exact live source; compile-verified (clippy all-targets). |
| `trait RowComposer` + impls | 337-353 | REAL | — | `#[inline]` name-split so trait dispatch can't shadow the kernel — sound. |
| `Fill` + `tag` | 356-369 | REAL | — | Trivial mapping. |
| `Sampling` + `tag`/`hv` | 372-397 | REAL | — | gray/420/422/444 → (h_max,v_max) pairs correct. |
| `Planes` | 400-411 | REAL | — | Plain data holder. |
| `Rng::next_u64` | 414-425 | REAL | — | Standard xorshift64*; nonzero fixed seed. Doc "seed printed by the run" true only via the duplicated literal (F-002). |
| `build_planes` | 430-494 | REAL | — | Geometry verified against decoder.rs:736-746; smooth fill stays in u8 range (max 198+15=213; chroma clamped); gray clears chroma (consumers pass `None`). |
| `compose_frame` | 500-511 | REAL | F-001 | Loop shape matches `assemble_rgba_into`; row slicing exact (no panic); checksum return dead (F-001). |

## 5. Test honesty
No tests in this example (it is a measurement tool). Its output is not committed anywhere as evidence (`rg color_bench docs/ worklog.md` → only "owed at merge" notes) — no fabricated evidence risk from this file today.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "legacy … kept verbatim … @ merge commit 62c48b6" | color_bench.rs:26-28 | REAL-UNPROVEN (provenance UNVERIFIED) | Cannot diff against the merge commit without git (session-2 deviation). Internal consistency: matches the live color.rs docs' description of the pre-optimization loop. |
| "Plane geometry mirrors the decoder's padded planes … same padding overhead as a real decode" | color_bench.rs:20-22 | REAL | build_planes (433-438) == decoder.rs:736-746 stride/extent math, verified term by term. |
| "Returns a checksum … (DCE guard + cross-arm equality evidence)" | color_bench.rs:496-499 | FALSE (mechanism) | Checksum discarded at all call sites; equality done by main's byte compare (F-001). |
| "medians, N≥15 recorded per-rep raw data, load conditions printed" | color_bench.rs:4-5 | REAL | REPS=21, per-rep TSV `REP` lines, loadavg header/footer (625-626, 644-648) — in part2, read for consistency. |

## 7. Dead / duplicated / unreachable code
- Dead: `compose_frame`'s checksum computation (F-001).
- Duplicated: seed literal (F-002); `build_legacy`/`build_now` are deliberate per-module twins (documented as the `#[path]` price, part2).

## 8. Dependencies & flags
Std-only (`std::time::Instant`); no crate deps; not feature-gated; not part of any shipped build. Relies on `/proc/cpuinfo` + `/proc/loadavg` (Linux) for machine context — degrades to "unknown" on other platforms.

## 9. Open questions for the next agent
- W06-A05 (part2): `median` (609-612) takes index `len/2` of 21 sorted samples = 11th value — confirm REPS stays odd if ever tuned; even N would make "median" an upper-mid element silently.
- Whoever runs the merge re-run: the recorded numbers should include the `bitexact_vs_legacy` column all-true; a false cell means the optimization changed pixels and the whole comparison is invalid.
