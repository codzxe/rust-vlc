# AUDIT: tools/corpus/corpus_vocab.py
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / symbols | 455 / ~45 module-level vocab tables + 2 factory fns (vfr_expr, draw_gop) |
| File verdict | REAL — the adversarial vocabulary of the randomized corpus generator (v4) |
| Reachable from shipped entry point? | NO — test infra; consumed by tools/corpus/{gen_corpus,corpus_builders}.py (the randomized corpus is dev-side, corpus-big/ gitignored per CORPUS.md:108-114) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; python3 import + consistency checks; rg consumers |

## 1. One-paragraph verdict
A pure data-boundary module (no decisions, all draw bounds — as its own
header claims and the code confirms: no hardcoded feature values, only
pools/weights/caps). It is genuinely adversarial: VFR timestamp-chaos
setpts factories, anamorphic SAR/rotation pools, B-frame storms up to 16,
chapter bombs (1000-3000), 16-32-stream AVI storms, metadata bombs capped
at 64 KiB, subtitle text with NUL bytes/control chars/RTL/emoji, and 22
garbage archetypes. All vocabulary is consumed by the real generator —
zero dead tables (verified below).

## 2. What actually works (verified)
- python3 consistency checks (imported the module, ran):
  - RES_WEIGHTS covers all 29 RESOLUTION_LADDER names, no extras (missing=[] extra=[]).
  - FPS_POOL/FPS_WEIGHTS lengths match (21/21).
  - 84 STRATIFICATION pins ⊆ 36 CLASS_WEIGHTS classes (pins not in class weights: set()); every class pinned; V4_CLASSES (15) ⊆ classes; weights sum 125.
- Consumers (rg): tools/corpus/gen_corpus.py:85-89 imports the core vocab and uses CORRUPTION_OPS (line 1065), GARBAGE_ARCHETYPES (1162), plus the v3-replay filter (812-817) honoring V4_CLASSES — the deterministic replay contract documented at corpus_vocab.py:444-448.
- tools/corpus/corpus_builders.py:35-48 imports and calls `draw_gop(rng, codec, fps)` (:214), `vfr_expr` (:48 import), SUB_TEXT_POOL (:281,294), VBV pools — the factories are exercised, not decorative.
- Factory honesty: `vfr_expr` keeps PTS strictly monotonic except the explicitly-marked `chaos-nonmono` mode (lines 168-171, 200-203) — rejections are ledger data, matching the header's "honest-rejects ledger" claim.

## 3. Findings
None. Checked and verified clean: weight/pool arity consistency (would raise
at import/draw time if broken — python3 import ran clean), coverage closure
of stratification pins, no dead symbols (all imported by the two real
consumers), caps present for every bomb class (BOMB_TAG_MAX 65536,
MAXTRACKS_RANGE bounded, CHAPTER_BOMB_RANGE bounded).
UNVERIFIED: end-to-end generation (ffmpeg not run in sandbox; requires
the ffmpeg probing pass the header describes — capability-probed pix_fmts).
No committed hash ledger covers corpus_vocab.py itself (it is source, not
data — correct).

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| RESOLUTION_LADDER / RES_WEIGHTS / RES_BY_NAME | 43-64 | REAL | 29 entries incl. portraits; weights complete |
| FPS_POOL / FPS_WEIGHTS | 67-69 | REAL | 0.5..240 fps |
| VIDEO_SOURCES / SOURCE_COST / NO_DURATION_SRCS | 71-78 | REAL | cost model for timeouts; allyuv exclusion documented (75) |
| VIDEO_CODECS / CAP_PROBED_PIX / GOP_KNOBS | 82-104 | REAL | 15 codecs; 10-bit/4:2:2 probed not assumed |
| CONTAINERS / RAW_VIDEO_TARGETS | 106-121 | REAL | 8 containers + raw ES (no timestamps — demuxer timeline synthesis target) |
| AUDIO_CODECS / AUDIO_CONTAINERS / rate pools | 123-157 | REAL | 17 codecs, odd rates 5512..384000 |
| ANAMORPHIC_SAR / ROTATIONS | 160-164 | REAL | real DVD/BD values |
| vfr_expr | 172-203 | REAL | 6 modes, monotone-bounded except marked chaos |
| draw_gop | 209-224 | REAL | measured x265 bf cap honesty (comment 213-217) |
| SUBTITLE_CODEC_FOR / SUB_TEXT_POOL / ASS_FONTS | 230-243 | REAL | hostile text incl. \x00, control chars, RTL |
| CORRUPTION_OPS / GARBAGE_ARCHETYPES | 249-260 | REAL | 12 ops + 22 archetypes, applied to copies |
| CLASS_WEIGHTS / STRATIFICATION / MANDATORY_RESOLUTIONS | 266-348 | REAL | coverage floor under randomness |
| PRESETS / RES_ORDER / bounds | 353-367 | REAL | smoke/standard/big/insane budgets |
| v4 escalation vocab | 374-454 | REAL | all measured-on-ffmpeg-7.1.5 per header/comments |

## 5. Test honesty
N/A (vocabulary, not assertions) — but the stratification floor means the
generated corpus provably covers every pinned family at least once per run.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing ... is a decision — it is the BOUNDS of what is drawable" | corpus_vocab.py:4 | REAL | only constants/factories; rng drawn per-file by consumers |
| "unsupported draws are recorded in the honest-rejects ledger" | :9-10 | REAL-UNPROVEN | ledger mechanism lives in gen_corpus.py (rejected_draws); not executed here (no ffmpeg in sandbox) |

## 7. Dead / duplicated code
None — every symbol imported by gen_corpus.py or corpus_builders.py (rg verified).

## 8. Dependencies & flags
Stdlib only (annotations future). No external deps — module import ran clean.

## 9. Open questions
- gen_corpus.py's use of STRATIFICATION 3/4-tuple pins (forced_res, forced_features) — check tuple arity handling once (it is exercised per pin; a wrong arity would KeyError at generation time).
