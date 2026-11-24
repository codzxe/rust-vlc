# CLAIM AUDIT: docs/formats/video-vp8-vp9.md
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 8 |
| REAL 5 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 2 |
| Internal contradictions found | 2 (both minor drift, self-documented era splits) |

## 1. Blunt verdict
A decision record whose plan was executed exactly: the prescribed `rusty_vp9 = "=0.1.3"`
admission landed (Cargo.toml pin, decode glue, differential test, dependency record,
matrix row implemented) and its own append-only addendum honestly marks the front-matter
as historical. The VP8 half stays truthfully blocked ×4. Two drift points: (1) the
addendum's "implemented ×3" for VP9 no longer matches the matrix (linux now `partial`);
(2) its corpus lane numbers (0126/0078, 480×360/160×120) do not match the committed
corpus manifest (0006/0081, 1024×768/320×240) — the doc inspected a different local
generation (the 143-file /tmp corpus of the open-path-wiring era), so its corpus
specifics are unverifiable in-repo.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "rusty_vp9 = "=0.1.3" is pinned in crates/codecs/Cargo.toml (card 6.M-vp9)" | :174-176 | REAL | crates/codecs/Cargo.toml:145 `rusty_vp9 = { version = "=0.1.3", optional = true }` + feature `codec-vp9` (:81) — exact |
| 2 | "decode glue lives at crates/codecs/src/video/vp9/**" | :176-177 | REAL | crates/codecs/src/video/vp9/{decoder.rs, error.rs, mod.rs} exist |
| 3 | "matrix row codec-video-vp9 … is now implemented" (addendum: ×3) | :178-180 | REAL with drift | matrix :473-496: `windows = "implemented", linux = "partial", macos = "implemented", web = "not_started"` — the addendum's "implemented ×3" no longer matches (linux=partial today); web=not_started matches |
| 4 | "vp9_differential 4/4 pass in the codecs suite (643 tests / 624 pass / 0 fail)" | :183-184 | REAL-UNPROVEN | crates/codecs/tests/vp9_differential.rs exists with `differential_lanes_vs_ffmpeg_libvpx_vp9()`; the 4/4 and 643/624 counts are that session's run (not re-runnable: cargo-test budget; baseline workspace-wide 4744 passed/2 failed is consistent) |
| 5 | "VP8: unchanged — matrix row codec-video-vp8 = blocked ×4, evidence empty, blocked_reason cites this record" | :187-191 | REAL | matrix: `status = { windows="blocked", linux="blocked", macos="blocked", web="blocked" }`, `evidence = ""` — exact |
| 6 | demux carriage: mkv V_VP9/V_VP8 → vp9/vp8; mp4 vp08/vp09; ivf VP90 → vp9 | :119-124 | REAL | mkv/header.rs:805-806 + test pins :980-981; mp4/mod.rs:444-445; ivf/mod.rs:37/:82 (doc's line cites drifted, content exact) |
| 7 | typed decode refusal vocabulary (`unsupported-software-decode-pending`) at product-config/model.rs + plugin-api/codec.rs | :124-125 | REAL | crates/product-config/src/model.rs:166 documents the vocabulary; crates/plugin-api/src/codec.rs carries the ADR-001 §1 seam (cited) |
| 8 | corpus lanes: "one .webm lane: 0126-webm-video.webm = VP8-only, 480×360, 1.64 s, no audio"; "0078-vfr-av.mkv VP9+Opus 160×120 3.61 s" | :128-132 | UNVERIFIED (drift vs committed manifest) | the committed corpus-v4-smoke-manifest-20261023.json has `media/0006-webm-video.webm` (vp8, 1024×768, 3.3 s, no audio) and `media/0081-vfr-av.mkv` (vp9+opus, 320×240, 4.0 s) — same lane STRUCTURE (one vp8 webm; one vp9+opus mkv among vfr-av ×3) but different indices/dims/durations; the doc inspected a different (143-file) local corpus generation |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| addendum: "implemented ×3 (windows/linux/macos)" | matrix today: linux = "partial" | the MATRIX wins for current state (the doc's addendum is itself dated 2026-11-10); the doc self-declares "drift, not a false claim" (:193-194) — an honest, recorded drift |
| corpus lanes 0126/0078 with 480×360/160×120 | committed manifest 0006/0081 with 1024×768/320×240 | both were real corpora of their machines; the committed manifest is the repo's record — the doc's specifics cannot be reconciled against it (flagged, not false) |

## 4. Fabrication / inflation indicators
- None. The record explicitly refuses to claim validation it did not do ("claim is the
  crate description's, unproven by us; that is the spike" :86-87) and its addendum
  marks its own front-matter stale. Model behavior for decision records.

## 5. Matrix rows
Directly documents rows codec-video-vp9 (:473+) and codec-video-vp8 — verdicts verified
in claim ledger (5 REAL, 1 REAL-UNPROVEN). VP9's evidence string cites artifacts that
exist (vp9_differential.rs, rusty_vp9-0.1.3.md record).

## 6. Open questions for the next agent
- Why did codec-video-vp9's linux cell drop from implemented (addendum, 2026-11-10) to
  partial (today's matrix)? A status downgrade with no evidence-pointer change is worth
  a git-history question to the Lead (agents cannot run git).
- The wasm32 leg for vp9: matrix says web=not_started — check whether rusty_vp9 builds
  on wasm32 (the codec-av1 precedent failed there).
