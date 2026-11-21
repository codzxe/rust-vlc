# AUDIT: crates/subtitles/src/pgs/rle.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-21 |
| Lines / functions | 232 / 10 (1 const, RleDamage+any, decode_object, test helper px + 8 tests) |
| File verdict | UNREACHABLE (real, library-tested code; no shipped entry point reaches the crate) |
| Reachable from shipped entry point? | NO — crate-level proof (only `fuzz/Cargo.toml` depends on `subtitles`). In-crate callers: `pgs/mod.rs:550-551` (compose) and its own tests; `pgs/fuzz.rs` via the decoder. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full read of pgs/rle.rs; read of pgs/mod.rs:448-571 (sole production caller); constant re-derivation by hand (see §6); baseline `orchrun-test-workspace-nofailfast.log.txt:5269+` (267/0 incl. all `pgs::rle::tests`). |

## 1. One-paragraph verdict
A small, disciplined RLE decoder: every write is min-clamped into a
pre-sized `w×h` index buffer, every hostile shape (short line,
crossing run, truncated tail, trailing garbage) lands in a typed
damage summary, and the tests assert exact pixels for every code
form. I traced every arithmetic and indexing site and found no
panic, no overflow, and no off-by-one that survives its own tests.
The only defects are cosmetic: one provably dead branch and one
unverifiable claim about the reference decoder's behavior that the
module's honesty notes present as settled. Nothing shipped ever runs
this code.

## 2. What actually works (verified)
— Panic-free by trace: `out[pos..pos + take]` with
`take = run.min(n - pos)` (114-119) is always in-bounds; the EOL snap
`pos += w - col` (107) can never exceed `n` because `n = w·h` is a
multiple of `w` and `pos < n` on entry; `pos % w` (103) executes only
inside the loop, which requires `n > 0`, hence `w ≥ 1`.
— Baseline execution: `orchrun-test-workspace-nofailfast.log.txt:5269`
→ 267/0 subtitles unit tests including all 8 `pgs::rle::tests`
(literal/run/long-run/EOL/clip/snap/truncate/zero-dim), plus the PGS
compose paths that call `decode_object` (`pgs::tests::*` green).
— Code table self-consistency: `nn` literal (71-73), `00 00` EOL
(101-111), `00 0L`/`00 8L cc`/`00 4L LL`/`00 CL LL cc` runs (80-98)
behave exactly as the module table documents; a computed run of 0
from ANY form (incl. `00 40 00`) uniformly reads as EOL.

## 3. Findings

### F-001 [S4] Provably dead branch: the in-loop "canvas full" EOL exit
Where: crates/subtitles/src/pgs/rle.rs:101-111
Code:
```rust
        if run == 0 {
            // End of line.
            let col = pos % w;
            if col > 0 {
                damage.short_lines += 1;
                // Snap to the next row boundary (see module docs).
                pos += w - col;
            } else if pos == n {
                break;
            }
            continue;
        }
```
What is wrong: the `else if pos == n` arm is unreachable: the loop
guard at 68 (`pos < n`) is the invariant at line 101 — `pos` is only
mutated after this block — so when `col == 0` holds, `pos < n` always
holds too. The break can never fire.
Why it matters: dead control flow that reads as a real exit path
("EOL after a full canvas exits early") misleads maintainers into
thinking the loop can terminate there; it terminates via the guard.
What correct looks like: delete the arm (a comment "loop guard exits
at pos == n" in its place).
Confidence: HIGH — Verification: read (invariant argument above).

### F-002 [S4] Leftover bytes past a full canvas are silently ignored with no damage, justified by an unverifiable "reference behavior" claim
Where: crates/subtitles/src/pgs/rle.rs:125-127 (claim), 18-27 (docs)
Code:
```rust
    // Leftover bytes past a full canvas (typically the legal final
    // EOL after the last row) are ignored — the reference behavior.
```
What is wrong: two things. (a) The no-damage choice is defensible for
the legal trailing EOL, but a hostile object that pads kilobytes of
garbage after filling the canvas produces ZERO damage signal — the
damage ledger's contract ("every hostile byte sequence lands in the
returned damage summary", 55-56) is not met for this class. (b) The
"reference behavior" attribution (like the "fetched 2026-10-25"
ffmpeg claims in this module's header) cannot be re-verified: no
ffmpeg source exists in the sandbox and no network is allowed.
Why it matters: the module's core promise is typed damage for hostile
input; trailing-garbage objects are indistinguishable from clean
ones. The unverifiable provenance matters at wave level: several PGS
"DIVERGENCES from the reference" claims rest on it.
What correct looks like: count leftover bytes past a small tolerance
(e.g. > one EOL) into `RleDamage` (a `trailing_bytes` field), and mark
the ffmpeg-comparison claims UNVERIFIED-in-audit rather than settled.
Confidence: HIGH for the behavior (read), UNVERIFIED for the
reference claim.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_OBJECT_PIXELS` | 29-32 | REAL | — | 4 M px cap; enforced by caller (mod.rs:469) |
| `RleDamage` + `any` | 36-52 | REAL | — | 3-field summary; `any()` aggregates |
| `decode_object` | 60-128 | REAL | F-001, F-002 | bounded alloc `vec![0u8; n]`; all writes clamped; EOL snap; truncated flag; no panics (§2); O(n + data) |
| tests: `px` helper | 134-137 | REAL | — | |
| `literal_pixels_and_color_runs` | 139-150 | REAL (strong) | — | exact pixels both rows, clean-damage assert |
| `eol_and_short_color_runs` | 152-162 | REAL | — | short_lines == 1 asserted |
| `long_runs_both_colors` | 164-180 | REAL | — | all four run forms, exact values |
| `overrun_run_is_clipped_and_typed` | 182-192 | REAL | — | clipped_runs == 1, canvas full ⇒ NOT truncated (subtle and correct) |
| `short_line_snaps_to_next_row` | 194-205 | REAL | — | pins the anti-smear divergence with exact pixel positions |
| `truncated_data_pads_transparent` | 207-213 | REAL | — | truncated flag + zero pad + empty-data arm |
| `trailing_bytes_past_full_canvas_are_ignored` | 215-223 | REAL | — | pins F-002's no-damage behavior as intended |
| `zero_dimensions_decode_to_empty` | 225-230 | REAL | — | vacuous assert (`dmg.truncated \|\| !dmg.any()` is a tautology — accepts anything); harmless given n=0 exits before any damage could be set, but the assertion proves nothing |

## 5. Test honesty (tests in this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| literal_pixels_and_color_runs | YES | exact index values |
| eol_and_short_color_runs | YES | exact values + damage count |
| long_runs_both_colors | YES | exact values for 4 code forms |
| overrun_run_is_clipped_and_typed | YES | damage count + fill |
| short_line_snaps_to_next_row | YES | exact positions distinguish snap from smear |
| truncated_data_pads_transparent | YES | flag + content |
| trailing_bytes_past_full_canvas_are_ignored | YES | no-damage assert |
| zero_dimensions_decode_to_empty | NO | tautological assertion (`truncated \|\| !any()` always true); only the `img.is_empty()` half can fail |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "RLE 0x00 escapes" as part of subtitle-pgs "implemented" ×3 platforms | docs/feature-matrix.toml subtitle-pgs (983-995) | REAL (library) / UNREACHABLE (shipped) | code traced + tests green in no-fail-fast baseline; crate has no shipped dependent |
| "The code table, verified against the reference decoder (ffmpeg pgssubdec.c decode_rle, fetched 2026-10-25)" | pgs/rle.rs:3-4 | REAL-UNPROVEN | table internally consistent and matches the standard PGS code forms (re-derived: `L<<8\|LL` long form, 0x80 color flag); the ffmpeg cross-check itself is UNVERIFIABLE in this sandbox (no ffmpeg source, no network) |
| "ffmpeg … lets a short line shift every following line sideways (it only logs); … here a short line SNAPS … each recorded once" (deliberate divergence) | pgs/rle.rs:18-27 | REAL (implemented+typed) / UNVERIFIED (the ffmpeg smear claim) | snap implemented at 103-107 with short_lines damage; ffmpeg claim not checkable here |
| "Never panics; every hostile byte sequence lands in the returned damage summary" | pgs/rle.rs:54-56 | FALSE (narrowly) | F-002: trailing-garbage objects land in NO damage field |

## 7. Dead / duplicated / unreachable code
- F-001's dead arm. The `zero_dimensions` tautological assert (§5).
- Whole module unreachable from shipped binaries (header).

## 8. Dependencies & flags
- None: pure std slice arithmetic, no imports beyond nothing (self-
  contained); no feature gates. Caller contract: pgs/mod.rs compose
  (part1 audit) passes full reassembled fragments — verified there.

## 9. Open questions for the next agent
- pgs/mod.rs#part2 / pgs/fuzz.rs owner: does the fuzz harness ever
  exercise `decode_object` with leftover-garbage shapes (F-002)? The
  survival corpus (pgs_fuzz_survival.rs) should be checked for a
  trailing-bytes seed.
- Wave level: the "fetched 2026-10-25" ffmpeg provenance claims
  (pgs/rle.rs, pgs/palette.rs, and dvb segment/rle headers) should be
  consolidated into one UNVERIFIED-provenance ledger entry — no agent
  in this sandbox can re-check them.
