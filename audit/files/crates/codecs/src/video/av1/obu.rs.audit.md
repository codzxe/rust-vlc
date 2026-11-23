# AUDIT: crates/codecs/src/video/av1/obu.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 558 / 3 prod fns + 2 types + 11 tests |
| File verdict | REAL — executed, reachable, honestly scoped framing validator |
| Reachable from shipped entry point? | YES: apps/cli/src/pipeline_host.rs:304 `"av1" \| "av01" => Av1Decoder::new()` (codecs default features incl. `codec-av1`, apps/cli/Cargo.toml:51) → Av1Decoder::decode → `obu::parse_temporal_unit` (decoder.rs:860); census gates feeding at decoder.rs:872/:880 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 av1::obu` → 11 passed; 0 failed, exit 0; rg (callers, rav1d constraints) |

## 1. One-paragraph verdict
This is a genuinely good file: an honestly-scoped OBU framing validator
("framing validator, not a bitstream parser", :7) with typed damage codes,
a bounded OBU census (4096 cap), correct leb128 windowing (≤8 bytes, ≤u32
value, checked_add on the payload end — the exact pattern the HEVC
length-prefixed splitter lacks), and a 3-bit frame-header peek whose
limitations are flagged in-line. All 11 unit tests ran live and pass, and
the module is reachable from the shipped CLI decode path. Hostile-focus
answer for the rav1d split: obu.rs never calls rav1d — it only frames and
censuses TUs before the engine sees bytes; rav1d is called exclusively by
av1/decoder.rs (dav1d C-ABI, unsafe extern "C", rav1d 1.1.0 pinned
asm-off, features bitdepth_8+bitdepth_16), and the wrapper's output gate
is 8-bit 4:2:0 only: decoder.rs:537-541 refuses `pic.p.layout !=
DAV1D_PIXEL_LAYOUT_I420` (`av1.layout`) and `pic.p.bpc != 8`
(`av1.bitdepth`) AFTER the engine decoded the picture. Only two nits: a
dead can-never-fire branch and its write-only flag.

## 2. What actually works (verified)
— verified by `./target/debug/deps/codecs-bae0f1756cc0a809 av1::obu` →
`test result: ok. 11 passed; 0 failed` (exit 0), covering leb128
boundaries (incl. 9-byte continuation and >u32::MAX values), the OBU count
guard (MAX+1 OBUs → `av1.obu.count_limit`), payload truncation geometry,
extension reserved/truncated paths, and the frame-type peek truth table.

## 3. Findings

F-001 [S4]
Where: crates/codecs/src/video/av1/obu.rs:333-335
Code:
```rust
    if !may_produce_picture && has_frame_header && has_tile_group {
        may_produce_picture = true;
    }
```
What is wrong: dead branch. `has_frame_header` is only ever set at
:300-303, which sets `may_produce_picture = true` in the same arm — so
`!may_produce_picture && has_frame_header` is unsatisfiable. Whatever
shape this patch (frame-header + tile-group combination handled
separately) was written for, the match arms absorbed it; the tail check
never fires and no test can reach it.
Confidence: HIGH — Verification: read (all writers of both flags traced).

F-002 [S4]
Where: crates/codecs/src/video/av1/obu.rs:182, 304-307, 333
Code:
```rust
    let mut has_tile_group = false;
...
            OBU_TILE_GROUP => {
                has_tile_group = true;
                may_produce_picture = true;
            }
```
What is wrong: `has_tile_group` is write-only state whose only read is the
dead branch of F-001. It is computed for every TU and never consulted —
misleading bookkeeping suggesting a rule that does not exist.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| OBU_* consts (9) | 41-58 | REAL | - | spec §6 type ids |
| MAX_OBUS_PER_TU | 65 | REAL | - | guard verified by test (fires at 4097th) |
| ObuInfo / TemporalUnit | 68-116 | REAL | - | census consumed by decoder.rs:860-916 |
| obu_error | 121-127 | REAL | - | single error class, Corrupt kind |
| read_leb128 | 134-158 | REAL | - | ≤8 bytes, typed truncation/too-long |
| parse_temporal_unit | 169-343 | REAL | S4 | typed guards; checked_add :263-270; peek :310-323; F-001/F-002 |
| tests (11) | 388-557 | REAL | - | all ran live, would fail if broken (exact codes/geometry) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 11 | YES | exact error-code + payload-range assertions; leb128 edge vectors; count guard exercised at boundary |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "framing validator, not a bitstream parser" | obu.rs:7 | REAL | no bitstream fields decoded; peek is 3 bits |
| "bounded-OBU-count resource guard" | obu.rs:20-21 | REAL | :189-196 + test `obu_count_guard_fires_typed` ran |
| "3-bit peek ... never a mis-decode" | obu.rs:107-109 | REAL-UNPROVEN | plausible; no adversarial stream executed here; reduced-still-picture limitation honestly flagged [VERIFY-P1] :110-112 |
| "the structure rav1d needs delivered intact (one IVF packet = one TU)" | obu.rs:9-11 | REAL (contract) | demuxer side not audited by me — UNVERIFIED |
## 7. Dead / duplicated / unreachable code
F-001 dead branch; F-002 write-only flag. No unreachable-from-shipped code
(this module IS shipped via the CLI av1 row).
## 8. Dependencies & flags
media-types only (no rav1d import — the split is clean: obu.rs has zero
engine knowledge). `codec-av1 = ["dep:rav1d"]` gates the module
(video/mod.rs:22); on by default → in the release lane for native targets;
wasm32 never (rav1d doesn't compile there, mod.rs:9 note + video/mod.rs:18-21).
## 9. Open questions for the next agent
- decoder.rs (not assigned): the 8-bit/4:2:0 refusal happens AFTER rav1d
  decodes — 10-bit streams pay full decode cost then refuse; confirm the
  rav1d bitdepth_16 feature actually decodes them (else the refusal may be
  preceded by an engine error path instead).
- The `[VERIFY-P1: reduced-still-picture ...]` marker — is there a card
  tracking it, or is it doc-only debt?
