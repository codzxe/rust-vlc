# AUDIT: crates/codecs/src/video/mjpeg/decoder.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 1948 / 38 (+16 unit tests, 3 test helpers) |
| File verdict | REAL (one hostile-input panic hole: F-001) |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:242-243 `"mjpeg" => codecs::video::mjpeg::MjpegDecoder::new()` → `MjpegDecoder::decode` (mod.rs:334) → `DecodeContext::decode_scan` → `scan_into` → `decode_block` → `receive_extend`; also apps/web/src/host.rs |
| Findings | S0: 0 S1: 1 S2: 1 S3: 4 S4: 1 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 video::mjpeg` → 91 passed, 0 failed (0.26s). Standalone probes against prebuilt `libcodecs-8512239cee599a96.rlib` (rustc +1.99.1, `-C debug-assertions=on`): hand-built hostile DC-symbol-31 JPEG → **PANIC** `decoder.rs:976:12 attempt to subtract with overflow` (backtrace: fuzz_codec_mjpeg → decode_image → decode → decode_scan_with → scan_into → decode_block → receive_extend); 7500-iter mini-fuzz (1500 random + 6000 structured mutations of a valid JPEG) → 2 more panics, same site. `rustc` semantics probe of `v - (1 << s) + 1` (s=31, v=0): debug panic `attempt to subtract with overflow`, release wraps |

## 1. One-paragraph verdict
This is a genuinely implemented, heavily hardened baseline-JPEG scan decoder: real marker walk, real Huffman entropy decode with an 8-bit LUT fast path, real dequant+IDCT (scalar default, SSE4.1 opt-in), real restart-marker handling, real 4:2:0/gray→I420 and 4:2:2/4:4:4/4:4:0→RGBA assembly, allocation-discipline reuse (planes/DHT keyed) that the in-file tests pin bit-exactly, and concealment that is observable. It is the flagship decoder and the engineering is above the repo's average by a wide margin. It is NOT panic-free: a hostile DC Huffman symbol of size 31 reaches `receive_extend` where `v - (1 << s) + 1` overflows i32 — a reproduced panic (3 independent inputs) in any debug-assertions build (cargo test, dev lane, the registered `fuzz_codec_mjpeg` campaign), directly contradicting the module's own "no panic on any input" guarantee; in release it silently wraps to a wrong DC value with no concealment. Secondary latent issues: a `copy_from_slice` length-mismatch panic in `crop_plane_into`'s defensive slow path (unreachable under current format routing), an index panic in `choose_format`/`frame_len` on a never/failed-walked context (pub(crate) misuse surface), and doc drift around the SIMD default.

## 2. What actually works (verified)
- All 91 mjpeg unit tests pass via the prebuilt test binary: `./target/debug/deps/codecs-bae0f1756cc0a809 video::mjpeg` → `test result: ok. 91 passed; 0 failed; 0 ignored` (0.26s).
- The concealment, context-reuse, pooled-slot, restart, AVI1, and odd-geometry behaviors asserted in the in-file tests are executed there (context reuse bit-exactness, plane/table rebuild counters, stale-slot poisoning, RST boundaries, 87×63 4:2:0).
- Orchestrator baseline corroborates end-to-end: rc-smoke headless-decode (AVI/MJPEG/PCM) PASS, engine-info decoders: mjpeg, pcm_s16le (audit/runs/baseline, AGENT-SPEC §13).
- Negative proof of the no-panic claim FAILED: reproduced panic (F-001) through the repo's own `fuzz::fuzz_codec_mjpeg` harness.

## 3. Findings
F-001 [S1]
Where: crates/codecs/src/video/mjpeg/decoder.rs:967-980 (panic at 976)
Code:
```rust
fn receive_extend(reader: &mut BitReader<'_>, s: u32) -> Result<i32, Stop> {
    let Some(bits) = reader.read_bits(s) else {
        return Err(Stop::Exhausted);
    };
    let v = bits as i32;
    if s == 0 {
        return Ok(0);
    }
    if v < (1 << (s - 1)) {
        Ok(v - (1 << s) + 1)
```
What is wrong: `s` is the raw DC HUFFVAL symbol (0..=255) — nothing clamps it to the baseline 0..=11 band. For s=31 with decoded bits < 2^30, `1 << s` = i32::MIN and `v - i32::MIN + 1` overflows i32: debug-assertions builds panic ("attempt to subtract with overflow"); release builds wrap to a wrong DC difference with `concealed = false`. `read_bits` only returns `None` for n > 32 (accumulator cap), so sizes 16..=32 are fed through. Reproduced 3× through the repo's own fuzz harness (`fuzz_codec_mjpeg` → `decode_image`), including twice from random structured mutations of a valid JPEG (7500-iter mini-fuzz). Why it matters: it falsifies the module guarantee `mod.rs:73-74` "bounds-checked and overflow-checked: no panic on any input (the `fuzz_codec_mjpeg` target's guarantee)" and `fuzz.rs:56` "Run one fuzz pass over `data` ... Never panics", and it falsifies `bitreader.rs:186-190` ("a hostile DC size category beyond the baseline 0..=15 band simply consumes the tail and reports exhaustion ... no panic in any build") — read_bits returns `Some` for n ≤ 32. A hostile MJPEG frame in any debug/dev lane aborts the process; in the release lane it silently produces a wrong pixel column with no Recovered outcome. What correct looks like: in `decode_block`, reject/clamp `size > 15` as `Stop::InvalidCode` (conceal), matching libjpeg's DC-category bound; compute the extension in i64 or with `checked_sub`. Confidence: HIGH — Verification: ran (3 reproductions + backtrace + isolated rustc semantics probe of the exact expression).
Note: this is one root cause; my fuzz campaign found no other panic site in 7500 iterations.

F-002 [S2]
Where: crates/codecs/src/video/mjpeg/decoder.rs:1101-1112
Code:
```rust
        let start = y * plane.stride;
        let end = start + out_w.min(plane.valid_w);
        match plane.data.get(start..end) {
            Some(row) => dst_row.copy_from_slice(row),
```
What is wrong: `row.len()` is `out_w.min(plane.valid_w)` but `dst_row.len()` is `out_w`; when `valid_w < out_w`, `copy_from_slice` panics on length mismatch (the `None` arm cannot fire — `get(start..end)` succeeds whenever valid_w < out_w because end < start+out_w ≤ data extent). Currently unreachable: `crop_plane_into` is only called from `assemble_i420_into`, and `choose_format` selects I420 only when valid_w == out_w for every plane (luma h=2/h_max=2 ⇒ valid_w=w; chroma 1/2 ⇒ ceil(w/2)=cw). Why it matters: this is the "defensive" path — the one that exists to handle geometry surprises — and it is the only branch in the assembly that can panic; any future format-routing change (e.g. a planar 4:2:2 vocabulary) walks straight into a guaranteed panic. What correct looks like: copy the valid prefix and `fill(128)` the remainder (like the `y >= valid_h` arm). Confidence: HIGH — Verification: read (geometry unreachability proven from choose_format + prepare_planes formulas; my 7500-iter fuzz produced no hit, consistent with unreachability).

F-003 [S3]
Where: crates/codecs/src/video/mjpeg/decoder.rs:1046-1050
Code:
```rust
fn choose_format(info: &JpegInfo, h_max: usize, v_max: usize) -> OutputFormat {
    if info.components.len() == 1 {
        return OutputFormat::I420;
    }
    let luma = &info.components[0];
```
What is wrong: `choose_format` indexes `components[0]` when `components.len() == 0`. `frame_len()`/`assemble_frame()` are `pub(crate)` and callable on a fresh or failed-walk context (`decode_scan_with` restores the partially-walked `JpegInfo` even on Err — L501-504), where `components` can be empty ⇒ index panic. Not reachable from the contract path (`MjpegDecoder::decode` propagates `decode_scan`'s error before calling `frame_len`, mod.rs:361-369), so this is a latent pub(crate)-misuse panic, not a shipped crash. What correct looks like: return a typed `LimitExceeded`/corrupt error for the 0-component state instead of indexing. Confidence: HIGH — Verification: read (call-order proof mod.rs:361-371; JpegInfo::empty/reset leaves components empty, parser.rs:94-95, 112-119).

F-004 [S3]
Where: crates/codecs/src/video/mjpeg/decoder.rs:643-651
Code:
```rust
                                    cause = Some(match stop {
                                        Stop::Exhausted => match reader.stop() {
                                            Some(ReaderStop::Marker(code)) => {
                                                ConcealCause::PrematureMarker(code)
                                            }
                                            _ => ConcealCause::EndOfData,
```
What is wrong: when the reader stopped at a restart marker mid-MCU (damaged restart placement), `reader.stop()` is `Some(ReaderStop::Restart(_))` and the `_` arm mislabels the cause `EndOfData`; a pending RST is neither end-of-data nor a non-restart marker. The concealment contract's observability (doc 03 §4.5(d)) reports the wrong damage class. Cosmetic-to-minor: the frame is still concealed and flagged. What correct looks like: a `ConcealCause::RestartMismatch` arm for `ReaderStop::Restart`. Confidence: HIGH — Verification: read.

F-005 [S3]
Where: crates/codecs/src/video/mjpeg/decoder.rs:1046-1053 (with parser.rs:545-560)
Code:
```rust
    let luma = &info.components[0];
    let chroma_420 = info.components[1..].iter().all(|c| c.h == 1 && c.v == 1);
```
What is wrong: SOF component order is assumed to be Y,Cb,Cr (planes[0]=luma, [1]/[2]=chroma everywhere: choose_format, assemble_i420_into, assemble_rgba_into). The parser (parse_sos) matches scan components by ID but nothing validates that component 0 is Y. §B.2.2 does not mandate order (only JFIF A.1 does); a valid-but-exotic stream listing Cb first decodes to channel-permuted pixels silently (wrong colors, no error). Blast radius limited: MJPEG streams in the wild are JFIF-ordered; my mini-fuzz exercised order mutations without a crash. What correct looks like: reject non-JFIF component order as `Unsupported`, or key luma by the scan's first component. Confidence: MEDIUM — Verification: read (no execution of a permuted-order stream).

F-006 [S4]
Where: crates/codecs/src/video/mjpeg/decoder.rs:256-259 (vs idct_simd.rs:118-126, mod.rs:122-124)
Code:
```rust
    /// Dequantize + inverse-DCT one block into `out` samples
    /// (default: the real transform via [`idct_block_dispatch`] —
    /// SSE4.1 SIMD pass 1 where the CPU has it, scalar otherwise,
    /// bit-exact either way per `idct_simd`'s bounds proof).
```
What is wrong: two doc contradictions around the IDCT. (1) This comment claims SIMD runs "where the CPU has it", but `idct_simd.rs:123-125` makes **scalar the default** — the SSE4.1 kernel requires the `LUMEN_MJPEG_IDCT_SIMD` env opt-in (LEAD DECISION 2026-10-24: measured 1.021×/0.995× — SIMD on the slow side of noise). (2) mod.rs:122-124 still says "(idct_simd: card 7-b stub — SIMD plan + dispatch skeleton, not yet wired" while decoder.rs wires `idct_block_dispatch` as the production transform. Neither affects behavior; both mislead a reader auditing performance claims. Confidence: HIGH — Verification: read + the dispatch body executed by the 91 passing tests (scalar path).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Plane::write_block | 185-199 | REAL | — | bounds-checked via get_mut; silent return on OOB (defensive) |
| Plane::gray_blocks | 203-219 | REAL | — | raster-order tail fill; matches cursor order (proven) |
| ScanHooks::transform (default) | 260-263 | REAL | S4 | forwards to idct_block_dispatch; doc drift F-006 |
| ScanHooks::store (default) | 266-270 | REAL | — | forwards to Plane::write_block |
| FullStages impl | 276 | REAL | — | empty impl = all defaults; #[inline] seam, tested bit-exact (L1739-1784) |
| NoIdctStages::transform | 285-290 | REAL (profiling arm) | — | #[allow(dead_code)]; lib never stubs; used by stage_bench |
| NoWriteStages::store | 299-304 | REAL (profiling arm) | — | black_box anchor |
| NoIdctNoWriteStages | 311-321 | REAL (profiling arm) | — | parse+entropy only |
| DhtKey::matches | 339-343 | REAL | — | allocation-free content compare incl. HUFFVAL; pinned by L1452-1498 |
| DhtKey::of | 348-354 | REAL | — | snapshot on change only |
| DhtKey::update_from | 362-372 | REAL | — | in-place refill; V-3 |
| DecodeContext::default/new | 404-425 | REAL | — | no alloc until first decode |
| DecodeContext::decode | 438-457 | REAL | — | resize without zero-fill claimed safe; pinned by stale-slot tests |
| DecodeContext::decode_scan | 472-474 | REAL | — | production entry (contract path) |
| DecodeContext::decode_scan_with | 490-505 | REAL | — | scratch swap; restores even on Err (L501-504) — enables F-003 state |
| DecodeContext::frame_len | 514-516 | REAL | S3 | 0-component headers ⇒ F-003 panic (misuse) |
| DecodeContext::assemble_frame | 528-530 | REAL | — | length-validated (L954-958), tested L1787-1815 |
| DecodeContext::scan_into | 541-677 | REAL | S3 | core walk; component gate 1/3 (L552-559); MAX_PIXELS cap (L562-570); restart logic correct (RST mod 8, DC pred reset); F-004 mislabel |
| DecodeContext::output | 685-688 | REAL | — | #[cfg(test)] only |
| DecodeContext::take_output | 692-694 | REAL | — | mem::take; tested |
| DecodeContext::prepare_planes | 701-773 | REAL | — | checked mul; MAX_PIXELS*4 plane cap; geometry-keyed reuse |
| DecodeContext::prepare_tables | 779-818 | REAL | — | DHT-keyed rebuild; bad table ⇒ typed error |
| refresh_table | 824-836 | REAL | — | build-or-rebuild in place |
| h_max_of / v_max_of | 838-844 | REAL | — | unwrap_or(1) on empty (feeds F-003 path) |
| pooled_frame_slot | 870-912 | REAL | — | warm/cold/evict paths correct; bounded FRAME_SLOTS; tested L1584-1736 |
| frame_total_for | 919-933 | REAL | — | checked arithmetic; RGBA total |
| assemble_frame_into | 944-964 | REAL | — | exact-length gate |
| receive_extend | 967-980 | REAL with S1 hole | S1 | F-001: i32 overflow on hostile s=31; s=32 dead branch |
| decode_block | 984-1026 | REAL | S1(via F-001) | DC size unmasked; AC path masks to 4 bits (safe); k>63 guarded; ZRL tolerated past end |
| decode (stateless) | 1030-1042 | REAL | — | fresh context per call (fuzz + one-shot) |
| choose_format | 1046-1057 | REAL | S3 | F-003 (empty components index), F-005 (order assumption) |
| i420_total | 1060-1072 | REAL | — | checked; ⌈w/2⌉×⌈h/2⌉ chroma |
| crop_plane_into | 1081-1113 | REAL | S2 | F-002 latent copy_from_slice panic; fast path byte-identical (tested) |
| assemble_i420_into | 1119-1142 | REAL | — | gray ⇒ neutral-128 chroma; in-place offsets |
| assemble_rgba_into | 1145-1191 | REAL | — | RgbComposer; row loop bounds-checked |
| limit_error | 1193-1199 | REAL | — | typed LimitExceeded |
| tests (16) + helpers (plane_of, tiny_jpeg, with_avi1) | 1201-1948 | REAL | — | see §5 |

F-007 [S3]
Where: crates/codecs/src/video/mjpeg/decoder.rs:261-263 → idct_simd.rs:126-137
Code:
```rust
    fn transform(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
        idct_block_dispatch(block, qt, out)
    }
```
(with idct_simd.rs:131: `if std::env::var_os("LUMEN_MJPEG_IDCT_SIMD").is_some()`)
What is wrong: the SIMD opt-in check is a `getenv` executed **per 8×8 block** on the default scalar path (transform is called for every block of every plane). `var_os` walks the process environment each call (~100 ns); at 1080p 4:2:0 that is ~8 100 getenv calls per frame ≈ 0.8 ms/frame of pure dispatch overhead — measurable against a decode that the repo's own perf docs measure in the ms range. What correct looks like: resolve the flag once (lazy static / OnceLock, or at DecodeContext construction) and pass a bool. Confidence: MEDIUM (cost not measured here; mechanism read-verified) — Verification: read.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| context_reuse_is_bit_exact_and_reallocates_only_on_change (L1379-1449) | YES | compares pooled-context bytes vs fresh stateless decodes; asserts rebuild counters 6/3 exactly |
| dht_key_compare_is_content_sensitive... (L1452-1498) | YES | bits-only compare would pass symbol_five wrongly; asserts assert_ne on the two references + counter 3 |
| header_error_does_not_poison_the_persistent_scratch (L1501-1532) | YES | damaged-then-clean through one context vs fresh decode |
| concealment_does_not_leak_across_frames (L1535-1561) | YES | concealed tail then clean frame bit-exact |
| pooled_slot_refill_is_bit_exact_and_stale_byte_safe (L1585-1663) | YES | differing frames through warm slots; asserts pool growth/refill transitions |
| pooled_slots_rekey_on_geometry_and_stay_bounded (L1666-1736) | YES | 12 alternating rounds, FRAME_SLOTS bound asserted per round |
| staged_full_scan_is_bit_exact_vs_the_production_path (L1739-1784) | YES | FullStages generic vs production instantiation, 4 geometries |
| assemble_frame_rejects_a_wrong_length_destination (L1787-1815) | YES | ±1 length must Err |
| avi1_* (L1830-1878) | YES | polarity surfaced, pixels unchanged vs marker-free |
| restart_boundaries_crossing_mcu_rows_decode_bit_exact (L1881-1916) | YES | hand-built RST stream equals restart-free decode |
| odd_geometry_420... (L1919-1947) | YES | exact byte length + stale-byte poisoning via geometry flip |
| pixel_cap_rejects_oversize_frames (L1271-1309) | YES | asserts attempted == 8193*8193 |
| receive_extend_decodes_negative_and_positive (L1262-1268) | NO for hostile sizes | only s=2 — the >15 band (F-001) is untested in-file |
| crop_into_* / gray_blocks_* (L1222-1259) | YES | padding trim + neutral fill + tail fill |
Honesty note: the suite is strong but has a hole exactly at F-001 — no test feeds a DC symbol outside 0..=15; the bitreader test `oversized_read_consumes_the_tail_without_panicking` only covers n > available, giving false confidence in the composition.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no panic on any input (the fuzz_codec_mjpeg target's guarantee)" | mod.rs:73-74 | FALSE | reproduced panic decoder.rs:976 via fuzz_codec_mjpeg (F-001) |
| "Never panics, never hangs" | fuzz.rs:56 | FALSE | same reproduction |
| "a hostile DC size category beyond the baseline 0..=15 band simply consumes the tail and reports exhaustion ... no panic in any build" | bitreader.rs:186-190 | FALSE | read_bits(31) returns Some; overflow at decoder.rs:976 |
| "SSE4.1 SIMD pass 1 where the CPU has it" | decoder.rs:258 | FALSE | idct_simd.rs:123-126: scalar default, env opt-in only (F-006) |
| "(idct_simd: card 7-b stub ... not yet wired)" | mod.rs:122-124 | FALSE (stale) | decoder.rs:95,261-263 wires idct_block_dispatch |
| Reuse-without-clearing is bit-exact because Ns==Nf | decoder.rs:43-50 | REAL | parser.rs:591-599 enforces Ns == components.len() |
| Pooled handoff allocates nothing steady-state | decoder.rs:65-88 | REAL | tests L1585-1736; pooled_frame_slot logic verified |
| MAX_PIXELS 8 MiP bound | decoder.rs:99-105 | REAL | enforced L562-570 + plane cap L755-761; test L1271-1309 |

## 7. Dead / duplicated / unreachable code
- NoIdctStages/NoWriteStages/NoIdctNoWriteStages: #[allow(dead_code)] profiling arms — legitimately used by examples/stage_bench.rs (verified the file exists in this repo and references them — see huffman_bench/stage_bench audit); not dead in the workspace sense, unreachable from shipped entry points by design (documented).
- DecodeContext::output(): #[cfg(test)]-gated — test-only by declaration, not shipped dead code.

## 8. Dependencies & flags
- Depends only on sibling modules + media_types (SharedBytes). No external crates. `unsafe` appears nowhere in this file (the IDCT SIMD unsafe lives in idct_simd.rs, sanctioned).
- `LUMEN_MJPEG_IDCT_SIMD` env knob (idct_simd.rs:131) — read per block dispatch call... actually read per call via `std::env::var_os` on every block (`idct_block_dispatch` is per-block!): **per-block env-var lookup is a hot-path syscall-ish cost when unset** — but since scalar is the default and the check short-circuits on `var_os` (a libc getenv, not a syscall; ~100ns), the SIMD arm being opt-in means the shipped path pays `getenv` per block. That is a real (small) perf smell worth noting here: F-007-lite, S3. (Verified: idct_simd.rs:131 `std::env::var_os("LUMEN_MJPEG_IDCT_SIMD").is_some()` inside `idct_block_dispatch`, called per 8×8 block via ScanHooks::transform.)

## 9. Open questions for the next agent
- Does any registered libFuzzer campaign (fuzz/fuzz_targets/fuzz_codec_mjpeg.rs) run with debug assertions in CI? If yes, F-001 will crash the campaign the moment it synthesizes a DC symbol 31; if it runs release-mode, the panic is latent and the wrong-value path hides instead.
- tools/xtask fixture encoder (2.A3): does it ever emit DC categories > 11? (Determines whether the committed reference digests could ever have caught F-001.)
- idct.rs `shr32_round`/pass-2 bit-exactness is claimed, not audited here (out of assignment scope).
