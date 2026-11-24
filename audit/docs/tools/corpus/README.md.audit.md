# CLAIM AUDIT: tools/corpus/README.md
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Claims extracted | 22 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 11 REAL / 9 REAL-UNPROVEN / 0 / 0 / 0 / 1 FALSE-precision / 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
An unusually honest operational manual: it registers its own tool gaps
(§10) and records ffmpeg-behavior realities as measured facts. All
quantitative vocabulary claims (class/pin/pool counts) verify exactly
against corpus_vocab.py by import. The §9/§9a "measured" ffmpeg claims and
all performance numbers are un-re-runnable here (no ffmpeg in sandbox) —
cited as REAL-UNPROVEN. One false-precision claim: the registered
`--media-a/--media-b` gap describes flag names the docstring does not
actually contain (it says "auto-decoded").

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Everything here drives ffmpeg as the external oracle — Lumen never grades its own homework." | :6 | REAL (design) | fidelity.py uses ffmpeg psnr/ssim filters; audio scorer is pure-Python arithmetic (documented at :274-275) — not self-grading either |
| 2 | "~14–22% of clean files get a corrupted sibling" | :21 | REAL-UNPROVEN | dirty-pass rate is gen_corpus.py behavior; no ffmpeg run here |
| 3 | "Nothing is silently dropped ... recorded as data in manifest.json" | :28-30 | REAL-UNPROVEN | schema fields exist (corpus.rs RegenManifest + README §5); generation not run |
| 4 | "17 tiny real encodes are probed at run start" / "29 probes" | :34, :221-228 | REAL (count) / UNVERIFIED (results) | probe list arithmetic: 16 v3 + 13 v4 = 29 names listed; run not executed |
| 5 | "smoke: 115 files = 79 clean + 13 dirty + 1 extension-lie + 22 garbage; 21/21 classes" | :82-85 | REAL-UNPROVEN | committed-baseline-free; ffmpeg needed |
| 6 | Preset table (80/420/650/1000 files, budgets) | :56-61 | REAL | corpus_vocab.py PRESETS match exactly (python3 import check) |
| 7 | "36 clean classes + 3 derived" / "Stratification pins 84 (62 v3 + 22 v4)" | :100-102 | REAL | python3: CLASS_WEIGHTS=36, STRATIFICATION=84, 62+22=84 ✓ |
| 8 | "29 resolutions, 16×16 → 3840×2160, including 3 portrait" | :394 | REAL | RESOLUTION_LADDER len 29; portraits 360/720/1080 ✓ |
| 9 | "21 frame rates, 0.5 → 240 fps" | :397 | REAL | FPS_POOL len 21, ends 240 ✓ |
| 10 | "15 video codecs ... 8 video containers" | :398-400 | REAL | VIDEO_CODECS=15, CONTAINERS=8 ✓ |
| 11 | "17 audio codecs ... 12 audio containers; odd-rate pool" | :401-404 | REAL | AUDIO_CODECS=17, AUDIO_CONTAINERS=12, ODD_RATES ✓ |
| 12 | "7 anamorphic SARs ... 5-entry rotation pool" | :410-411 | REAL | ANAMORPHIC_SAR=7, ROTATIONS=5 ✓ |
| 13 | "12 corruption ops ... 22 garbage archetypes" | :425-428 | REAL | CORRUPTION_OPS=12, GARBAGE_ARCHETYPES=22 ✓ |
| 14 | "Full flag set (verified against argparse)" | :63-70 | REAL | rg add_argument in gen_corpus.py:1274-1288 — all 12 flags match |
| 15 | minimize.py flags/defaults (400/16/3/60) | :259-264 | REAL | minimize.py:122-135 matches exactly |
| 16 | speed_bench.py flags (60/1/3/180) | :324-326 | REAL | speed_bench.py:103-113 matches exactly |
| 17 | "the module docstring mentions a `--media-a/--media-b` auto-decode mode that argparse does not implement" | :299-301 | FALSE-precision | fidelity.py docstring says "(or two media files, auto-decoded)" (:5) — no flag names anywhere; argparse indeed lacks them (verified :176-187) |
| 18 | "PSNR per-frame series parses empty in fidelity.py summary mode" | :373-375 | REAL-UNPROVEN | registered gap; not reproduced (needs raw streams) |
| 19 | §9 items 1-11 "each verified by isolation" | :328-349 | REAL-UNPROVEN | no ffmpeg in sandbox; internally consistent with corpus_vocab code (e.g. #1 allyuv exclusion at vocab:75, #2 bf cap at :213-217) |
| 20 | §9a items 12-23 measured W5-16 | :351-369 | REAL-UNPROVEN | same; code matches (ODD_SOURCES :389, zeroframe container gating :364) |
| 21 | "Lumen decode-CLI runner for speed_bench.py awaits the decode-null CLI card" | :378-379 | REAL | speed_bench.py has --runner hook, no committed lumen runner config |
| 22 | "dirty pass rate 14–22% ... none of the 12 ops is currently pinned" | :385-387 | REAL | STRATIFICATION contains no dirty-op pins (python3 check: all pins are clean classes) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| docstring mentions `--media-a/--media-b` (README:300) | fidelity.py:5 mentions only "auto-decoded" | The GAP is real, the citation is wrong — README describes flags that the docstring never names |

## 4. Fabrication / inflation indicators
None found. Numbers all reconcile against the code; the two operational gaps
are self-reported. The "measured" performance numbers (217 B minimize, 75.7 s
smoke) have no committed logs in-tree — they are worklog-cited claims,
treat as REAL-UNPROVEN.

## 5. Matrix rows
Not a matrix chunk. Related: docs/feature-matrix.toml rows cite "fuzz runs"
and compat-matrix runs, not this README; CORPUS.md:106-121 cross-references
this suite as "the project's primary adversarial asset" — consistent.
