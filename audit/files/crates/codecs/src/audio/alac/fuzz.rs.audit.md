# AUDIT: crates/codecs/src/audio/alac/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-21 |
| Lines / functions | 128 / 3 items (`CountingSink` + FrameSink impl, `fuzz_codec_alac`) + 2 tests |
| File verdict | REAL — fuzz infrastructure, executed in baseline; one documented-honesty gap on its stable-coverage claim |
| Reachable from shipped entry point? | NO (by design — fuzz infrastructure): callers are the in-crate tests (baseline log:355-356) and the registered libFuzzer shim `fuzz/fuzz_targets/fuzz_codec_alac.rs` (exists, calls `codecs::audio::alac::fuzz::fuzz_codec_alac(data)`; target count 35 matches baseline #18 `fuzz_registry_check.sh` PASS "35 targets, diff empty"). The DECODER it exercises is shipped-reachable (pipeline_host.rs:293-297 `"alac"`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full Read · `cat fuzz/fuzz_targets/fuzz_codec_alac.rs` (the shim is real, not paper) · `ls fuzz/fuzz_targets/ \| wc -l` (35 — matches baseline #18) · `sed -n '183,250p'` + `sed -n '129,172p'` + `sed -n '281,287p'` + `sed -n '513,517p'` alac/decoder.rs (config gate: channels ≤ 8, frame_length ≤ 65536, bit_depth ≤ 32 → `max_output_len` ≤ 2 MiB — the bounds claim verified) · Read tests/alac_fuzz_survival.rs (where the real decode-under-mutation coverage lives) · baseline test-workspace.log.txt:348-357 (alac suites incl. both fuzz tests ok) |

## 1. One-paragraph verdict
The harness does exactly what its contract says — three passes (cookie walk, free decode with an input-derived split, contract configure→decode→flush), everything swallowed, panic/hang/allocation-freedom by construction — and its bounds claim is real (the config gate caps any decode output at ≤ 65536 × 8 × 4 = 2 MiB, verified in decoder.rs). The one defect is a coverage gap its "honest" wiring note papers over slightly: the committed deterministic sweeps can never drive passes 2 and 3 past the typed config refusals (no sweep input contains a valid ALACSpecificConfig or an `frma`/`alac`-prefixed cookie), so the contract-path decode-under-hostile-input stage of THIS harness is untested on stable — the actual decode-under-mutation coverage lives in a different file (tests/alac_fuzz_survival.rs, real cookies via the real MP4 demuxer, 512+256 LCG rounds, anti-vacuity assertion) that does not pass through `fuzz_codec_alac` at all.

## 2. What actually works (verified)
- Both unit tests execute in baseline — test-workspace.log.txt:355-356 (`hostile_wide_inputs_survive`, `empty_and_short_inputs_survive` — ok).
- Bounds claim verified against the decoder: `parse_alac_config_inner` (decoder.rs:189-250) rejects channels > 8, frame_length > 65536, bit_depth > 32 — so `max_output_len` (decoder.rs:284-287) ≤ 65536 × 8 × 4 = 2 MiB; three fixed passes per call; no unbounded loop in the harness (all slices input-bounded: pass 2 needs `len ≥ 48` with `cookie_len ∈ 24..=35`; pass 3's `cut = data[0] % (len+1) ≤ len`).
- Panic-freedom of the harness body: every indexing site checked (`data[0]` gated by `len ≥ 48`; `data[..cut]` with `cut ≤ len`; `data.first().copied().unwrap_or(0)` — empty-safe); results swallowed per the documented contract (15-18).
- The libFuzzer target is real, not paper: `fuzz/fuzz_targets/fuzz_codec_alac.rs` calls this harness; the target set (35 files) matches baseline #18's "35 targets, diff empty" PASS.
- The real decode-under-mutation coverage exists elsewhere: tests/alac_fuzz_survival.rs feeds REAL cookies (extracted through the real `Mp4Demuxer` from committed .m4a fixtures) with 512 rounds of 4 mutation classes + 256 cookie mutations, asserts typed-or-Ok, determinism, and anti-vacuity (`ok > 0 && typed > 0`) — and ran in the baseline suite.

## 3. Findings

### F-001 [S4]
Where: crates/codecs/src/audio/alac/fuzz.rs:20-25 (the "honest" wiring/coverage note) vs the committed sweeps at 108-126 and the parse gate at decoder.rs:193-202
Code:
```rust
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_alac.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's ALAC differential suite.
```
What is wrong: "proven … by the deterministic sweeps below" overstates what the sweeps can reach. Pass 2 feeds `data[..cookie_len]` with `cookie_len = 24 + data[0] % 12 ∈ 24..=35` — the config gate accepts only 24- or 48-byte bodies (decoder.rs:193), so 11/12 of splits die at `alac.config.length`, and the length-24 case needs those first 24 bytes to be a fully valid ALACSpecificConfig (version 0, bit_depth 1..=32, channels 1..=8, frame_length 1..=65536, rate ≠ 0) — impossible in the committed sweeps (all-0x00 fails `frame_length=0`, all-0xFF/0xa5 fail the length gate). Pass 3 feeds `data[..36]`, which validates only after `frma`/`alac` prefix stripping lands on 24/48 — also impossible with the uniform sweep bytes. Consequence: on stable CI, `fuzz_codec_alac`'s decode/flush stages execute only as typed refusals; the decode-under-hostile-input coverage that exists (tests/alac_fuzz_survival.rs) bypasses this harness entirely, and the contract path (pass 3) is never decoded under mutation anywhere until the Lead's nightly campaign is wired.
Why it matters: the note's "honest" framing is the file's main quality claim; a reader concludes the harness logic — including its decode stages — is proven on stable. It is half-proven (refusal paths + the separate survival suite's free-surface coverage). Blast radius: none for shipped behavior (fuzz infra), hence S4.
What correct looks like: add one deterministic sweep case embedding a valid 24-byte cookie (the survival suite already extracts real ones) with mutated packet bodies, so pass 2/3 decode stages run on stable; or narrow the note to "the refusal paths are proven on stable; the decode stages await the nightly campaign".
Confidence: HIGH — Verification: read (split arithmetic + parse gate traced; sweeps' byte patterns checked against the gate).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CountingSink` + impl | 40-50 | REAL | — | saturating counter, always-Ok sink — the panic-free emission target |
| `fuzz_codec_alac` pass 1 | 56-57 | REAL | — | whole-buffer cookie walk, result swallowed per contract |
| `fuzz_codec_alac` pass 2 | 59-64 | REAL | S4 | input-derived 24..=35-byte split; only the length-24 case can pass the gate (F-001); slices in-bounds (len ≥ 48) |
| `fuzz_codec_alac` pass 3 | 66-102 | REAL | S4 | 36-byte cookie rides `extra_data` (the sanctioned channel); `cut` bounded; decode/flush swallowed; configure-success unreachable in committed sweeps (F-001) |
| test `empty_and_short_inputs_survive` | 108-116 | REAL | — | survival-only assertions (the documented contract); 42 inputs |
| test `hostile_wide_inputs_survive` | 118-126 | REAL | — | all-FF/all-00 4096 B + 0xa5 512 B; survival-only |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_short_inputs_survive | ONLY on panic/hang (the stated contract) — a logic regression in decode cannot fail it (refusal paths only) |
| hostile_wide_inputs_survive | ONLY on panic/hang/unbounded allocation — same limit (F-001) |
| (out of file) mutated_packets_never_panic_or_hang / mutated_cookies_are_typed_not_panics / empty_and_edge_packets_are_typed (tests/alac_fuzz_survival.rs) | YES | real cookies + real packets, typed-kind assertion, determinism re-check, anti-vacuity `ok>0 && typed>0` / `rejected>0` — these carry the actual decode-mutation coverage |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:15-18 | REAL | read: all results swallowed, bounds verified (§2) |
| "Memory is bounded by the input: … at most samples-per-packet × channels × 4 bytes" | fuzz.rs:29-31 | REAL (and meaningful — the config gate makes it small) | decoder.rs:229-242 caps channels ≤ 8, frame_length ≤ 65536 → ≤ 2 MiB |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below and the crate's ALAC differential suite" | fuzz.rs:22-25 | SHALLOW (overstated for the decode stages) | F-001: sweeps reach only refusal paths; survival suite bypasses the harness; differential suite is clean-input |
| "The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_alac.rs`" | fuzz.rs:22-23 | REAL | file exists, calls the harness; 35 targets match baseline #18 |

## 7. Dead / duplicated / unreachable code
- None within the file. (Cross-file note: the survival suite's `try_decode` duplicates the harness's pass-2 shape with real inputs — the two could share a seeded driver; hygiene, not counted.)

## 8. Dependencies & flags
- Uses only codec-api/media-types + the alac decoder surface. `pub mod fuzz` under `codec-alac` (default set; baseline #1 CLEAN). The libFuzzer shim package (fuzz/Cargo.toml) is workspace-external Lead-owned wiring — compiles on stable per its own README claim (UNVERIFIED — no build run allowed; registry-check PASS is the committed evidence).

## 9. Open questions for the next agent
- W07-A05/A08 (alac/decoder.rs): the `find_alac_atom` scan cap (first 40 bytes) and the 12-byte strip — pass 3 of this harness depends on that prefix arithmetic for ANY input to configure; if the atom shapes ever change, pass 3 goes fully dead. Worth a cross-check from your side.
- Whoever owns fuzz wiring (Lead): the note promises nightly campaigns "once wired" — the registry check proves registration only, never execution; no baseline evidence shows any libFuzzer campaign has ever run.
