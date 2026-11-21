# AUDIT: crates/codecs/src/video/mjpeg/decoder.rs#part2 (lines 651–1300)
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-11-21 |
| Lines / functions | 650 / 21 (incl. 6 tests; chunk of 1948-line file, equal 3-way split: part1=1–650, part2=651–1300, part3=1301–1948) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `MjpegDecoder::decode` mod.rs:362–375 → `context.decode_scan` → `frame_len` → `decoder::pooled_frame_slot(… assemble_frame)`; MJPEG is the shipped first-vertical video decoder — baseline `lumen engine-info` → `decoders: mjpeg, pcm_s16le`; rc-smoke 6/6; AVI 60/60 frames, audit/runs/baseline/INDEX.md §contrast) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg -n` structure map of decoder.rs; `sed -n '1,1300p'` full read of part; `rg -n "pooled_frame_slot|take_output|decode_scan|frame_len|assemble_frame"` in mjpeg/mod.rs + examples (callers found: mod.rs:362–375, stage_bench.rs:248–324,398); `sed -n '545,612p' parser.rs` (sof_index/table validation — proves indexing safety); `sed -n '1,120p' huffman.rs` + bitreader.rs `read_bits`/acc width (F-001 feasibility); `rg` codecs/Cargo.toml, apps/*/Cargo.toml, release.yml (feature trace) |

## 1. One-paragraph verdict
This chunk is the real MJPEG decode engine's back half — plane/table reuse, the pooled-output handoff, block entropy decode, and both output assemblies — and it is the code the shipped `lumen` binary actually runs for its only working video lane. The engineering is unusually defensive (typed errors on every fallback, saturating DC prediction, parser-proven indexing invariants), and the reuse/pooling claims in the module docs match the code I traced. One genuine hostile-input defect survives: `receive_extend` performs unchecked i32 arithmetic on a Huffman-symbol-derived size that no layer validates, so a crafted DHT (DC symbol category 31) overflows i32 — panic in overflow-checked builds, silent garbage-coefficient output reported as a *clean* (non-concealed) frame in the shipped release build. Plus two error-metadata defects in the `LimitExceeded` paths and two doc/robustness nits.

## 2. What actually works (verified)
— `MjpegDecoder::decode` → `decode_scan`/`frame_len`/`pooled_frame_slot`/`assemble_frame` call chain verified by `rg` (mod.rs:362–375); executed end-to-end by baseline `decode-bench` on the AVI fixture: `v_frames=60 v_decoded=60` (audit/runs/baseline/INDEX.md §contrast, rc-smoke 6/6).
— `crop_plane_into` neutral-128 + padding-trim semantics pinned by tests at 1222–1251 (see §5).
— Pixel cap enforced in `scan_into` (part1) *before* `prepare_planes`/`frame_total_for`, so the unchecked `w * h` in `assemble_i420_into:1120` cannot overflow on 64-bit (w·h ≤ MAX_PIXELS = 8 MiP proven at decoder.rs:531–538).
— Parser pre-validates everything `scan_into` indexes: `sof_index` via component-position lookup, scan-component uniqueness, `td/ta ≤ 3`, Huffman and quant-table presence (parser.rs:575–606) — so `self.planes[index]`/`self.cursors[index]`/`dc_pred[scan_comp.sof_index.min(3)]` cannot panic on any parser-accepted input.

## 3. Findings
### F-001 [S2] Hostile DHT DC size category 31 → i32 overflow in `receive_extend` (debug panic; release: garbage diff, frame still "clean")
Where: crates/codecs/src/video/mjpeg/decoder.rs:967-982
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
What is wrong: `s` is the raw Huffman symbol from the DC table (`size = dc_table.decode(reader)?` at 987). No layer validates it: `HuffmanDecoder::new/rebuild` (huffman.rs:65–110) checks only code-space structure, and the parser stores HUFFVAL verbatim. JPEG bounds DC categories to 0..=11, but a crafted DHT can carry 17..=255. The bit reader (acc u32, nbits ≤ 32, bitreader.rs:49–51,195–204) returns `None` for s > 32 (→ concealment, fine), but for s in 17..=32 it happily returns bits. For s = 31, `1 << 31` is `i32::MIN`, and `v - (1 << 31) + 1` = `v + 2147483648 + 1` overflows i32 for every v ≥ 0 satisfying `v < (1 << 30)` — panic in debug/overflow-checked builds (including `cargo test`/fuzz profiles), silent two's-complement wrap in the shipped release build. For s = 17..=30 and s = 32 the extension formula produces out-of-spec garbage (s=32 returns `v` un-extended), which is then emitted as a **non-concealed** frame: `concealed` stays false, the contract reports `DecodeOutcome::Emitted`, contradicting the module's own concealment policy (lines 7–16: entropy damage must be "bounded, observable, never silent").
Why it matters: The module's headline guarantee — "no panics, damage is concealed and observable" — is broken by a 1-table craft. The repo's own bitreader doc (bitreader.rs:192: "a hostile DC size category beyond the baseline") proves the threat was anticipated, but only the reader side was hardened, not the arithmetic.
What correct looks like: validate `s` before extending — `if s > 15 { return Err(Stop::InvalidCode); }` (or clamp to the spec band 0..=11 for DC / 1..=10 for AC), turning the craft into the documented concealment path. libjpeg rejects S > 15 at table-build time.
Confidence: HIGH — Verification: read (traced huffman.rs build validation, bitreader.rs acc width; arithmetic checked by hand: s=31, v∈[0,2³⁰) → `v − i32::MIN + 1` ≥ 2³¹+1 > i32::MAX). Not executed (no cargo runs per rules) — REQUEST-ORCH-RUN candidate: `cargo test -p codecs mjpeg` with a DHT DC symbol 31 added to `tiny_jpeg` would panic.

### F-002 [S3] `prepare_planes` overflow error reports a fabricated `attempted` value and the wrong `budget`
Where: crates/codecs/src/video/mjpeg/decoder.rs:748-756
Code:
```rust
            let size = stride
                .checked_mul(rows)
                .ok_or_else(|| MjpegError::LimitExceeded {
                    limit: media_types::error::LimitKind::Memory,
                    budget: MAX_PIXELS,
                    attempted: u64::try_from(rows).unwrap_or(u64::MAX).saturating_mul(1),
                })?;
```
What is wrong: On `stride * rows` overflow, the error claims the decoder attempted `rows * 1` bytes — not `stride * rows`. The `.saturating_mul(1)` is a no-op that looks like an unfinished edit (probably meant `saturating_mul(u64::try_from(stride)…)`). Also `budget: MAX_PIXELS` misstates the bound that actually fired (usize multiplication overflow), while the real memory budget applied 3 lines later (758: `MAX_PIXELS * 4`) is a different number again.
Why it matters: `LimitExceeded` is the typed error surfaced to users/engine telemetry for OOM-class refusal; a wrong `attempted`/`budget` pair makes the diagnostic lie about what was rejected and why.
What correct looks like: `attempted: u64::try_from(stride).unwrap_or(u64::MAX).saturating_mul(u64::try_from(rows).unwrap_or(u64::MAX))` and a budget that names the actual failing bound.
Confidence: HIGH — Verification: read (arithmetic above; the branch is unreachable on 64-bit because stride,rows ≤ ~33.6 M — hence S3 not S2).

### F-003 [S3] Error paths recompute the overflowing expression with unchecked arithmetic
Where: crates/codecs/src/video/mjpeg/decoder.rs:927-931 (and same pattern 1063, 1064)
Code:
```rust
        OutputFormat::Rgba8 => w
            .checked_mul(h)
            .and_then(|px| px.checked_mul(4))
            .ok_or_else(|| limit_error(w * h * 4))?,
```
What is wrong: The `ok_or_else` closure exists *only* when `w*h*4` overflowed usize — and then recomputes `w * h * 4` unchecked. Same at `i420_total`: `w.checked_mul(h).ok_or_else(|| limit_error(w * h))?` (1063) and `cw.checked_mul(ch).ok_or_else(|| limit_error(cw * ch))?` (1064). On a 32-bit target (w,h ≤ 65535 → w·h·4 ≈ 17.2·10⁹ > u32-sized usize) the error path itself panics/wraps instead of returning the typed limit error.
Why it matters: Defensive code that crashes in the exact case it was written to report is worse than no check: it converts a typed refusal into an arithmetic panic on 32-bit builds. On the shipped 64-bit target the pixel cap (8 MiP, enforced in `scan_into` before these run) makes the closures unreachable, which is why this is S3, not S2.
What correct looks like: compute the attempted value with saturating arithmetic inside the closure: `limit_error(w.saturating_mul(h).saturating_mul(4))`.
Confidence: HIGH — Verification: read (branch reachability reasoned: `frame_total_for` only called on headers that passed the 8 MiP cap in `scan_into`).

### F-004 [S4] Stale doc on `take_output`: "the one allocation per frame on the contract path"
Where: crates/codecs/src/video/mjpeg/decoder.rs:688-694
Code:
```rust
    /// Move the decoded frame's pixel bytes out (the one allocation
    /// per frame on the contract path: the frame's own buffer).
    pub(crate) fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }
```
What is wrong: The contract path (`MjpegDecoder::decode`, mod.rs:362–379) has not called `take_output` since W6-02 — it fills a pooled `SharedBytes` slot via `pooled_frame_slot`/`assemble_frame` (the module's own docs at lines 71–81 describe this replacement). The only remaining callers are the stateless `decode()` (decoder.rs:1037) and tests (decoder.rs:1571).
Why it matters: The doc directs auditors/maintainers to a handoff that no longer exists; on the real contract path `take_output` is dead code (kept only for the stateless/fuzz entry — legitimate, but the comment must say so).
What correct looks like: "Move the decoded frame's pixel bytes out (used by the stateless `decode` entry and tests; the contract path fills pooled slots instead)."
Confidence: HIGH — Verification: read + `rg -n "take_output"` across `crates/codecs/src` — callers: decoder.rs:1037 (stateless decode), decoder.rs:1571 (test). None in mod.rs.
### F-005 [S4] `crop_plane_into` fast path panics on non-multiple destination lengths that the slow path tolerates
Where: crates/codecs/src/video/mjpeg/decoder.rs:1092-1103
Code:
```rust
    let rows = dst.len() / out_w;
    if plane.valid_w >= out_w
        && plane.stride == out_w
        && plane.valid_h >= rows
        && out_w > 0
        && rows > 0
    {
        let end = out_w
            .checked_mul(rows)
            .and_then(|len| plane.data.get(..len).map(|_| len));
        if let Some(len) = end {
            dst.copy_from_slice(&plane.data[..len]);
```
What is wrong: If `dst.len()` is not an exact multiple of `out_w`, `len = out_w * rows < dst.len()` and `dst.copy_from_slice` panics on the length mismatch. The general path right below (1105–1116) handles the same input gracefully with `chunks_exact_mut`. Today both callers (`assemble_i420_into`:1121–1137) pass exact multiples (luma `w*h`/`w`, chroma `cw*ch`/`cw`), so this is a latent contract trap: any future caller with a ragged destination gets a panic in the "fast" branch instead of the documented typed/cropped behavior.
Why it matters: Same function, two different input contracts depending on which branch runs — a maintenance hazard, not a live bug.
What correct looks like: Add `&& dst.len() == out_w * rows` to the fast-path condition (or use `dst[..len].copy_from_slice`).
Confidence: HIGH — Verification: read (callers traced; current inputs always exact multiples).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `scan_into` (concealment tail + `DecodedMeta`) | 660–680 | REAL | — | gray fill from per-component cursors after `break 'scan`; indexing safe (parser.rs:575–584 guarantees sof_index bounds+uniqueness); cause/`concealed` propagate to contract (`Recovered{ConcealedFrame}` mod.rs:386–390) |
| `DecodeContext::output` `#[cfg(test)]` | 686–688 | REAL | — | test-only accessor, honestly labeled |
| `DecodeContext::take_output` | 692–694 | REAL | S4 | F-004 stale doc; correct body |
| `DecodeContext::prepare_planes` | 701–765 | REAL | S3 | geometry-key reuse verified (early return 717–719); rebuild path zero-fills via `resize(size,0)` (759–760) — conservative, fine; F-002 error metadata |
| `DecodeContext::prepare_tables` | 779–817 | REAL | — | DHT structural compare + in-place rebuild; missing tables → `None` (matches parser pre-validation, defense-in-depth); Stop detail discarded by `map_err(\|_\| …)` — acceptable (context string kept) |
| `refresh_table` | 824–833 | REAL | — | trivial Some/None dispatch |
| `h_max_of` / `v_max_of` | 838–840 / 842–844 | REAL | — | trivial max; `unwrap_or(1)` for empty (unreachable: components ≥ 1 enforced by parser) |
| `FRAME_SLOTS` const | 856 | config key | — | 4; bounded-memory claim consistent (pool re-keys on length) |
| `pooled_frame_slot` | 870–917 | REAL | — | warm path (`len` match + `get_mut` exclusivity), cold path bounded at 4 with eviction; `*next` can exceed `slots.len()` mid-warmup but every use is modulo `count` — traced all count values 0..4, correct; fresh-slot `get_mut` failure handled as typed error, not panic |
| `frame_total_for` | 919–942 | REAL | S3 | F-003 recompute; totals arithmetic checked |
| `assemble_frame_into` | 944–965 | REAL | — | loud typed error on length mismatch before touching `out`; dispatch I420/RGBA |
| `receive_extend` | 967–982 | REAL (defect) | S2 | F-001; correct for spec-band s ≤ 15 (test at 1262 pins s=2) |
| `decode_block` | 984–1028 | REAL | — | DC predictor `saturating_add` + i16 clamp (documented); AC `k += run; k > 63 → InvalidCode` bounds ZRL correctly; `block.get_mut(ZIGZAG[k])` in-bounds; hostile `size` is the F-001 hole |
| `decode` (stateless entry) | 1030–1043 | REAL | — | fresh context per call — fuzz/tests entry; one alloc per frame by design here |
| `choose_format` | 1046–1058 | REAL | — | 4:2:0 + grayscale → I420, everything else RGBA; matches doc 01 §3.3 claim |
| `i420_total` | 1060–1072 | REAL | S3 | F-003 (1063, 1064); formula `w·h + 2·⌈w/2⌉·⌈h/2⌉` correct |
| `crop_plane_into` | 1081–1117 | REAL | S4 | F-005; slow path neutral-128 fill verified by tests 1222–1251 |
| `assemble_i420_into` | 1119–1140 | REAL | — | grayscale → chroma fill 128 (documented neutral representation); `planes[2]` relies on the 1\|3 component gate (518–524) — safe, but a comment would help |
| `assemble_rgba_into` | 1145–1176 | REAL | — | all row access via `out.get_mut(..)`, breaks on ragged buffer (no panic); composer seam in color.rs (other agent's part) |
| `limit_error` | 1193–1200 | REAL | — | trivial constructor |
| tests `plane_of`, `crop_into_trims_padding`, `crop_into_fills_neutral_past_the_data`, `gray_blocks_fills_only_the_tail`, `receive_extend_decodes_negative_and_positive`, `pixel_cap_rejects_oversize_frames` | 1209–1300 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `crop_into_trims_padding` (1222) | YES | asserts every output byte is the valid-region value; padding bytes in the plane are 255 — any padding leak fails the `all(==9)` assert |
| `crop_into_fills_neutral_past_the_data` (1241) | YES | pins both the 128 fill and exact byte ranges |
| `gray_blocks_fills_only_the_tail` (1253) | YES | pins untouched prefix + gray tail (exercises part1's `Plane::gray_blocks`, called from my range at 665) |
| `receive_extend_decodes_negative_and_positive` (1262) | PARTIAL | pins the sign-extension for s=2 (−3, 2, 3 — a sign-flip mutation fails it), but covers only the spec band; the hostile-s band of F-001 is untested |
| `pixel_cap_rejects_oversize_frames` (1271) | YES | hand-builds a complete valid-header JPEG at 8193×8193 and asserts `LimitExceeded{attempted: 8193*8193}` — removing the cap at 531–538 makes it fail on allocation/panic instead |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "undecodable blocks from the failure point on are filled with mid-gray … concealed = true … bounded, observable, never silent" | decoder.rs module docs 7–16 | FALSE for hostile DHT symbols | F-001: s=17..=32 yields garbage coefficients with `concealed=false` (release) or an overflow panic (debug) — neither gray fill nor observability |
| "steady-state handoff allocates nothing" (pooled slots) | module docs 71–81, 84–90 | REAL | code traced: `pooled_frame_slot` warm path fills in place via `get_mut`; only callers mod.rs:374 |
| "the padded per-component planes are reallocated only when the MCU geometry changes" | module docs 29–40 | REAL | `prepare_planes` early-return on `plane_key` equality (717–719); `plane_builds` counter observable by tests |
| MAX_PIXELS "keeps the worst-case decode footprint … under the corpus' 64 MiB rss budget `[PROPOSAL: the number; the invariant is the bound itself]`" | decoder.rs:100–104 (part1's line range, noted for the part1 auditor) | REAL-UNPROVEN | the bound itself is enforced (531–538 + 758) but the 64 MiB figure carries a literal `[PROPOSAL: …]` placeholder — no measurement cited |

## 7. Dead / duplicated / unreachable code
- `take_output` is production-dead (stateless `decode` + tests only) — see F-004.
- `output()` is `#[cfg(test)]` — intentional, labeled.
- The `no_dc_table`/`no_ac_table` error arms (612–625, part1 range but feeding my part) are unreachable behind parser.rs:597's pre-validation — defense-in-depth, acceptable.

## 8. Dependencies & flags
- MJPEG module is **unconditional** — no feature gate: crates/codecs/Cargo.toml:26–27 ("The IN-HOUSE decoders (mjpeg, pcm, adpcm, h264, hevc, mpeg12 — zero external deps) stay unconditional"). So this file compiles in every build configuration, including `--no-default-features`.
- **Wave-6 hostile question (answered for this file's lane):** H.264 decode IS reachable from the shipped `lumen` CLI: apps/cli/Cargo.toml:51 `codecs = { path = "../../crates/codecs" }` (default features on) → crates/codecs/Cargo.toml:56 `default = [… "codec-h264-sw" …]`, :73 `codec-h264-sw = ["dep:rust_h264"]` → h264/mod.rs:55/61 `#[cfg(feature = "codec-h264-sw")] pub use sw::H264SwDecoder;` → apps/cli/src/pipeline_host.rs:346 `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` (unconditional). release.yml builds `cargo build --release --locked -p lumen-cli -p desktop` (lines 193, 474, 627, 763, 911) and the tauri crate (`--manifest-path apps/desktop-tauri/Cargo.toml`, lines 219, 499, 652, 803, 954) — no `--no-default-features` anywhere; tauri → apps/desktop/Cargo.toml:40 `lumen-cli` → same defaults. Baseline proof: sample-420.mp4 (h264 CB 4:2:0) PLAYS via `lumen open` (audit/runs/baseline/INDEX.md CORRECTION v2). Caveats: release.yml has never executed in CI (billing blocked, release.yml:22–30 honesty note) and `lumen engine-info`'s `decoders: mjpeg, pcm_s16le` is a hard-coded literal that under-reports the real registry (INDEX.md standing defects).

## 9. Open questions for the next agent
- part1 auditor: verify `Plane::write_block`/`store` stage hooks write within padded geometry (I traced callers only); the `[PROPOSAL]` placeholder at 104 and the "Concelment" typo at line 5 are in your range.
- part3 auditor: the stale-slot differential tests (referenced at 84–90) live in your range — check they actually poison slots (pre-filled sentinel) rather than reuse zeroed buffers.
- Whoever owns `RgbComposer` (color.rs): my range hands it `valid_w/valid_h` defaults of 1 for missing planes (1153–1158) — confirm the composer treats those as neutral, not as an off-by-one luma geometry.
