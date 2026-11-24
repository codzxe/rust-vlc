# AUDIT: apps/web/src/render/present.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-24 |
| Lines / functions | 875 / 12 prod fns (incl. Display) + 3 test helpers + 9 tests |
| File verdict | REAL (unit level, executed) — but the lane's screen-side consumer does not exist in-tree |
| Reachable from shipped entry point? | NO for the release lane (lumen-cli/desktop do not build lumen-web); the wasm lane is built by scripts/rc-smoke.sh §6 but never loaded by any page |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `cargo test -p lumen-web --lib render::present` -> 9 passed, 0 failed (0.29s); `rg -ln 'presentFrame|lumen_web|worker\.js' ui/src` -> no hits (exit 1); sed verification of all quotes (exit 0) |

## 1. One-paragraph verdict
The YUV→RGBA conversion and latest-wins mailbox are genuinely implemented and
genuinely tested: checked plane arithmetic, per-colorimetry BT.601/709/2020 ×
limited/full coefficients, typed refusals for every malformed shape, and 9 unit
tests including an independent f64 reference model — I executed them, all pass.
The hostile-focus question "how frames reach the screen" has a hostile answer:
they do not, in-tree. The chain is present.rs → host.rs mailbox → abi.rs raw
frame lane → worker.js postMessage — and it dead-ends there. The module doc
promises "the JS glue's canvas draw"; worker.js contains no canvas code and no
page in the repo consumes `presentFrame` (the shipped UI's real canvas path is
the desktop shell's separate LUF1 frame channel, ui/src/components/player/
MediaSurface.tsx, which does not use this crate).

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p lumen-web --lib render::present` ->
"test result: ok. 9 passed; 0 failed" (all 9 present.rs tests, incl.
conversion_matches_the_reference_within_one_lsb and mailbox_is_latest_wins_with_counted_drops).
— wiring verified by reading host.rs:401 (`store`), host.rs:1145-1163
(`present_one` → `WorkerToHost::PresentFrame`), abi.rs:241-247 (`stage_frame`),
abi.rs:385-400 (`lumen_web_frame_addr/len`), worker.js:96-105 (reads frame lane,
postMessages rgba).

## 3. Findings
F-001 [S2]
Where: apps/web/src/render/present.rs:1-9 (module doc); apps/web/worker.js:102-108; ui/src (absence)
Code:
```
//! The CPU-staged present lane: decoded CPU frames → RGBA (the
//! single allowed CPU→GPU-adjacent copy of doc 03 §4.11 e / doc 09
//! §4.2) → the **latest-wins present mailbox** → one present per
//! pump slice → the JS glue's canvas draw.
```
and (worker.js:102-108, the lane's actual terminus):
```
  for (const message of messages) {
    if (message.type === "presentFrame" && frame !== null) {
      message.rgba = frame;
      self.postMessage(message, [frame.buffer]);
    } else {
      self.postMessage(message);
    }
  }
```
What is wrong: The doc claims frames reach "the JS glue's canvas draw". The JS
glue never draws: worker.js only postMessages the RGBA bytes to a host page.
`rg -ln 'presentFrame|lumen_web|worker\.js' ui/src ui/*.ts ui/*.html` returns
nothing — no page in the repository instantiates this worker or consumes
`presentFrame`. The only real canvas blit in-tree is the desktop shell's LUF1
frame channel (MediaSurface.tsx), a different lane that does not involve
lumen-web. rc-smoke's "browser-load" proves only that the SPA renders an h1
(scripts/rc-smoke-web.mjs:70-75) — it never loads the wasm module. Why it
matters: the entire present lane's user-visible outcome — pixels on a browser
screen — has no implementation; "one present per pump slice → canvas draw" is a
wiring claim the tree contradicts. What correct looks like: either a page-side
consumer (the doc'd "future @lumen/wasm-client") or a doc that says the lane
currently terminates at the Worker boundary.
Confidence: HIGH — Verification: ran (rg absence proof + read of all five files in the chain)

F-002 [S3]
Where: apps/web/src/render/present.rs:377
Code:
```
        if layout.rgba_bytes(frame.size)? > MAX_UPLOAD_BYTES {
            self.refused += 1;
            return Err(UploadError::TooLarge);
        }
```
What is wrong: `store`'s contract (line 77-79: "The typed upload refusals
(every one counts, none kill the session)") is that every refusal increments
`refused`. The `?` on `rgba_bytes` propagates its `Err(TooLarge)` (u64 multiply
overflow) without counting it, silently skewing PresentStats. Why it matters:
the stats are the lane's "honest present evidence" — one path bypasses the
counter. Practically unreachable (overflow needs w×h > 2^62 while the frame's
actual bytes already passed `bytes.len() == layout.total`), hence S3 not S2.
What correct looks like: match on the Result and count both arms.
Confidence: HIGH — Verification: read

F-003 [S4]
Where: apps/web/src/render/present.rs:202-205
Code:
```
    let pts_ns = frame
        .pts
        .map_timestamp(|stamp| stamp.convert(&NANO_BASE).ok().map(|ns| ns.ticks))
        .flatten();
```
What is wrong: a pts whose base conversion overflows is silently flattened to
`None` — a swallowed error; the upload then reports "no pts" instead of a
conversion failure. Same pattern duplicated at host.rs:390-394. Limited blast
radius (pts is observability/timing metadata here). What correct looks like:
count or log the failed conversion.
Confidence: MEDIUM — Verification: read

F-004 [S4]
Where: apps/web/src/render/present.rs:222-223
Code:
```
                    let (r, g, b) = convert_yuv(y_value, cb_value, cr_value, &coefficients);
                    rgba.extend_from_slice(&[r, g, b, 255]);
```
What is wrong: one `extend_from_slice` call per pixel (capacity is
pre-reserved, so no realloc, but a per-pixel bounds-check + memcpy call). For a
1080p I420 frame that is ~2M calls; 4K ~8.3M. Bounded by MAX_UPLOAD_BYTES so
not a defect in the D-2 sense, but a chunked write (as_chunks_mut cursor)
would halve the cost. `debug_assert_eq!` (line 237) also means a short-buffer
bug would ship silently in release. What correct looks like: write 4 bytes
through a `&mut [u8]` cursor.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| UploadError::as_str | 97-108 | REAL | — | five typed sentences; matches plane_layout refusals |
| Display for UploadError | 110-116 | REAL | — | delegates to as_str |
| plane_layout | 119-159 | REAL | — | checked mul/add throughout; ZeroExtent; NV12/YUYV refused typed |
| PlaneLayout::rgba_bytes | 171-177 | REAL | — | checked 4×pixels |
| rgba_from_video | 185-250 | REAL | — | exact-length check, bound check before loop; I420/Rgba8/Bgra8 paths; F-003, F-004 |
| yuv_coefficients | 271-293 | REAL | — | 601/709/2020 Kr/Kb × limited/full; matches test's f64 reference |
| convert_yuv | 295-316 | REAL | — | clamp+round; deterministic |
| PresentMailbox::new | 344-346 | REAL | — | Default |
| PresentMailbox::store | 356-391 | REAL | — | full upfront validation; F-002 `?` bypass |
| PresentMailbox::take | 392-410 | REAL | — | converts at take time; failure counted, no panic |
| PresentMailbox::is_pending | 411-414 | REAL | — | |
| PresentMailbox::stats | 415-424 | REAL | — | copy of counters |
| tests::i420_frame / limited / reference_rgb | 439-527 | REAL | — | honest fixtures; f64 independent model |
| 9 #[test] fns | 530-875 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| endpoints_are_exact_for_every_matrix | YES | asserts exact 0/255 black/white per matrix |
| conversion_matches_the_reference_within_one_lsb | YES | f64 reference computed independently in-test; any coefficient bug shifts >1 LSB for at least one of 7 samples × 6 matrix/range combos |
| chroma_siting_is_top_left_420 | YES | pins siting via reference for two luma values |
| odd_dimensions_round_chroma_up | YES | 3×3 → 17 bytes layout check |
| rgba_passthrough_and_bgra_swizzle | YES | exact byte vectors |
| upload_refusals_are_typed | YES | asserts each distinct UploadError variant |
| pts_rides_the_upload | YES | round-trips 123_456_789 ns |
| mailbox_is_latest_wins_with_counted_drops | YES | displaced==1, converted==1, second frame wins |
| refused_offers_count_and_keep_the_staged_frame | YES | refusal leaves staged frame intact |
All 9 executed by me: 9 passed, 0 failed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one present per pump slice" | present.rs:8 | REAL | host.rs:1130 `self.present_one()` after tick loop |
| "the JS glue's canvas draw" | present.rs:9 | FALSE | worker.js:102-108 posts only; no canvas code; ui/src has zero `presentFrame` consumers (rg exit 1) |
| "32 MiB covers a 4K RGBA frame (3840×2160×4 = 33,177,600) with margin" | present.rs:63-66 | REAL | 33,177,600 < 33,554,432 |
| "displaced frames are COUNTED (never silent)" | present.rs:33-35 | REAL | store():381-382 `self.displaced += 1`; exception F-002 |

## 7. Dead / duplicated / unreachable code
None dead in-file. The crate-level dead end is F-001. Duplicated pattern: pts
convert-and-flatten appears here and host.rs:391-393 (F-003).

## 8. Dependencies & flags
media_types (frame/time), testkit. No feature gates in this file; crate is
`crate-type = ["cdylib","rlib"]`, publish=false. Release lane (-p lumen-cli
-p desktop) does not include it.

## 9. Open questions for the next agent
- Does any doc (docs/09-web-wasm-target.md) claim browser playback works
  end-to-end? If yes, F-001 escalates to S0 for that doc row.
- Who owns the "future @lumen/wasm-client" (worker.js:127-129)? If it was
  promised for this milestone, the lane is an unfinished deliverable.
