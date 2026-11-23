# AUDIT: crates/subtitles/src/dvb/clut.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-23 |
| Lines / functions | 386 / 5 (+7 tests) |
| File verdict | REAL (conversion + banks verified); one S1 cross-file palette-bounds panic whose blast radius includes this module's `bank()` contract; crate UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (crate-level, see segment.rs audit). In-crate chain: `dvb/mod.rs:532` `Clut::from_entries` (clut_segment) and `dvb/mod.rs:812/826` `default_banks`/`bank` (compose) <- `decode_pes` <- `dvb/fuzz.rs:58` + tests. |
| Findings | S0: 0 S1: 1 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p subtitles --lib dvb::clut` -> 7 passed, 0 failed (exit 0); `sed -n` quote verification; Grep bounds-chain sweep (bank/bank256/paint paths). |

## 1. One-paragraph verdict
The color math is genuinely good: all five FIX constants numerically equal ffmpeg's `YUV_TO_RGB*_CCIR` (SCALEBITS=10) — I recomputed each — and the DVB quirks (T inversion `a = 255 - t`, Y==0 forces transparency) are implemented once, centrally, and pinned by tests that pass. `from_entries`'s own bounds handling is correct (2/4-bit banks guarded, 8-bit needs no guard since `u8` indexes `[Rgba; 256]` exactly). The hostile-focus result: the palette-bounds safety of the WHOLE chain is violated one level up — a region redefinition that shrinks depth keeps the old pixel buffer, and `compose`'s `bank[index]` indexes a 4/16-entry bank with stale 8-bit indices: a deterministic, attacker-authored, release-profile panic. clut.rs's `bank()` hands out the short slice; mod.rs owns the bug; this audit documents the chain.

## 2. What actually works (verified)
— `cargo test -p subtitles --lib dvb::clut` -> 7/7 pass (exit 0): studio white/black exact, Y=0-transparency override, T inversion (T=255→a=0, T=128→a=127), red/blue primaries, depth-flag routing incl. the 0xC0 dual-bank alias, default-bank spot checks, limited-range unpack values.
— Constants recomputed by hand: FIX_R_CR=round(1.402·255/224·1024)=1634, FIX_G_CB=round(0.34414·255/224·1024)=401, FIX_G_CR=832, FIX_B_CB=2066, FIX_Y=round(255/219·1024)=1192 — each equals lines 198-202 and ffmpeg's CCIR macros. i32 ranges cannot overflow ((255-16)·1192 + 1634·127 + 512 ≈ 493k << 2^31).
— Bounds: `bank4[entry_id]` behind `entry_id < 4` (line 47), `bank16` behind `< 16` (line 50), `bank256[e.entry_id as usize]` (line 54) safe for any u8.

## 3. Findings
F-001 [S1]
Where: panic site crates/subtitles/src/dvb/mod.rs:826-832; root cause crates/subtitles/src/dvb/mod.rs:462-470; bank contract crates/subtitles/src/dvb/clut.rs:170-177; value source crates/subtitles/src/dvb/rle.rs:283-288
Code (mod.rs:826-832):
```rust
        let bank = clut.bank(region.depth);
        let mut rgba = Vec::with_capacity(region.pbuf.len() * 4);
        for &index in &region.pbuf {
            let c = bank[index as usize];
```
and the root cause (mod.rs:462-470):
```rust
        match self.regions.get_mut(&region.region_id) {
            Some(existing) if existing.width == w && existing.height == h => {
                existing.depth = depth;
                existing.clut_id = region.clut_id;
                if region.fill {
                    existing.pbuf.fill(region.fill_color);
                    existing.dirty = false;
                }
```
What is wrong: re-defining a region with the SAME width/height but a SMALLER depth mutates `existing.depth` while keeping the pixel buffer painted at the old depth. `paint_line` set `dirty = true` (mod.rs:762) and the redefinition only clears it when `fill` is set — with `fill = false` the stale, still-dirty buffer survives. `compose` then renders it: `bank(depth=2)` is `[Rgba; 4]`, but pbuf still holds 8-bit values (rle.rs:285 `row[x] = v;` writes arbitrary 0-255). `bank[200]` panics — slice index out of bounds, in debug AND release profiles.
Deterministic attack (single PES payload, five segments): (1) page composition listing region 1; (2) region composition: region 1, WxH, depth_code 3 (8-bit), fill=0; (3) object data: 0x12 8-bit block painting value 200 (dirty=true); (4) region composition again: region 1, same WxH, depth_code 1 (2-bit), fill=0 — width/height match so the `Some(existing)` arm runs, depth becomes 2, pbuf untouched; (5) end-of-display-set → compose → `bank4[200]` → panic.
Why it matters: `fuzz_sub_dvb` (dvb/fuzz.rs:41-75) feeds arbitrary bytes through exactly this chain and its contract is "Never panics" (dvb/fuzz.rs:39-40); this input is one crafted stream away. Any future consumer wiring DVB PES into playback gets a remotely-triggerable crash. The committed survival tests pass only because their seeds never author the shrink-redefinition sequence.
What correct looks like: on depth change, clear/mask the pixel buffer (or reallocate), or make `region_image` bounds-check/mask indices (`bank.get(index)` + typed damage), or version the region state on depth like ffmpeg does (its region struct is keyed by depth-compatible state).
Confidence: HIGH — Verification: read (full call chain traced end-to-end: rle.rs:285 -> RegionState.pbuf -> mod.rs:462-470 -> compose -> mod.rs:831); execution NOT run (source read-only; no test can be added; standalone repro would require creating a project outside audit/ — UNVERIFIED-by-execution only).

F-002 [S4]
Where: crates/subtitles/src/dvb/clut.rs:369-385
Code:
```rust
        let b0 = 0b1010_0110u8;
        let b1 = 0b1101_0011u8;
        let b2 = 0b0110_1001u8;
        let y = b0 & 0xFC;
        let cr = (((b0 & 0x03) << 4) | (b1 >> 4)) << 2;
```
What is wrong: `limited_range_entries_unpack` re-types the production unpack formulas (identical to segment.rs:282-294) inline and asserts their outputs — a tautology. A systematic bit-layout error in the production unpack passes this test unchanged (same expressions, same belief). It never calls `parse_clut`/`entries()`.
What correct looks like: feed the 3 bytes through `segment::parse_clut(...).entries()` and assert the resulting `ClutEntry` fields (the segment.rs test does this; this one adds nothing beyond duplication).
Confidence: HIGH — Verification: read + ran (passes; proves nothing about layout correctness).
F-003 [S4]
Where: crates/subtitles/src/dvb/clut.rs:171-177
Code:
```rust
    pub fn bank(&self, depth: u8) -> &[Rgba] {
        match depth {
            2 => &self.bank4,
            4 => &self.bank16,
            _ => &self.bank256,
        }
    }
```
What is wrong: silent catch-all — any depth other than 2/4 (invalid values 1, 3, 5-255) silently yields the 256-entry bank instead of being rejected. Today's callers pass `RegionSegment::depth()`-sanitized values, but F-001 demonstrates depth invariants CAN be violated by stream-driven state; an exhaustive match (or typed error) would turn the next invariant break into a loud failure instead of a wrong-but-silent palette.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/subtitles/src/dvb/clut.rs:47-55
Code:
```rust
            if e.depth_2 && e.entry_id < 4 {
                clut.bank4[e.entry_id as usize] = rgba;
            }
```
What is wrong: an entry flagged 2-bit with `entry_id >= 4` (spec-invalid) is silently dropped — no damage event, unlike every other malformed-input path in this crate (UnknownSegment, RegionDepthInvalid, RegionSizeInvalid...). ffmpeg skips it silently too, so reference-consistent, but it deviates from the crate's own typed-damage philosophy and hides stream damage from callers.
What correct looks like: a `DvbDamage::ClutEntryIdOutOfRange`-style event (the ledger is capped anyway).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Clut::from_entries | 39-58 | REAL | F-004 | bounds-correct (guards 4/16; u8 fits 256); defaults pre-seeded so partial redefinition keeps entries |
| Clut::default_banks | 62-64 | REAL | - | version 0 |
| Clut::with_default_banks | 66-168 | REAL | - | ffmpeg-shaped table (see doc claims); 5/276 entries test-pinned |
| Clut::bank | 171-177 | REAL (defensive gap) | F-003 | silent catch-all |
| ycbcr_to_rgba | 211-228 | REAL | - | constants = ffmpeg CCIR exactly; Y=0 rule applied before alpha; clamp255; no overflow (analysis in §2) |
| clamp255 | 206-208 | REAL | - | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| black_is_opaque_white_is_white | YES | exact studio-range endpoints |
| y_zero_is_transparent_even_when_t_says_opaque | YES | pins the §7.2.4.2 override |
| alpha_is_inverted_and_clamped | YES | T=255/128 pins inversion |
| red_and_blue_primaries | PARTIAL | range assertions (>200/<60), not exact values — a mild matrix error would pass; acceptable for primaries |
| entry_depth_flags_route_to_their_banks | YES | pins bank isolation + aliasing incl. negative cases |
| default_banks_match_reference_shape | PARTIAL | 5 of 276 entries; the other 271 unpinned (see doc claims) |
| limited_range_entries_unpack | NO | tautology (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the SAME integer arithmetic the reference decoders use (ffmpeg YUV_TO_RGB*_CCIR, SCALEBITS=10), so the differential oracle is bit-exact" | clut.rs:5-8 | REAL | constants recomputed identical (§2); no ffmpeg differential harness exists in-tree, so "oracle is bit-exact" is proven only at the constant level, not by a differential run |
| "DVB alpha is inverted relative to straight alpha (0 = opaque, 255 = transparent) and a luma of 0 means fully transparent" | clut.rs:17-20 | REAL-UNPROVEN | implemented + tested (tests pass); matches my knowledge of ffmpeg/GStreamer behavior; no in-tree reference to diff — MEDIUM |
| "ffmpeg's default_clut" | clut.rs:60-61 | REAL-UNPROVEN | structural match (alpha 63 for i<8, 43/85/127/170/255 constants, 0x88 mask) to ffmpeg's generate_default_clut from my knowledge; bit-exactness of all 271 unpinned entries UNVERIFIED (no ffmpeg source in sandbox) |

## 7. Dead / duplicated / unreachable code
Nothing dead. `Clut` is `pub(crate)` with `pub fn`s (visibility capped by the type — cosmetic only). Crate unreachable from shipped binaries (header).

## 8. Dependencies & flags
Depends on `compositing::target::Rgba` only. No unsafe, no features, std-only.

## 9. Open questions for the next agent
- F-001's fix location is dvb/mod.rs (owned by whoever audits it): confirm the `Some(existing)` arm (mod.rs:462-470) and the `bank[index]` site (mod.rs:831); a one-line reproduction is sketched in F-001 — worth adding to the committed fuzz corpus.
- `map2to8`/`map4to8` default tables (mod.rs:633-638) — the 2-to-8 default here is `[0x00, 0x77, 0x88, 0xFF]`; ffmpeg's default is `[0, 0x77, 0x88, 0xff]`? Verify against EN 300 743 §7.2.5 defaults when that file is audited.
