# AUDIT: tools/corpus/run_harness.py
| Field | Value |
|---|---|
| Auditor | W16-A10 |
| Date | 2026-11-24 |
| Lines / functions | 1414 / 11 module-level + 18 nested-in-main |
| File verdict | REAL-UNPROVEN (would be REAL if executed; cargo/ffprobe absent in sandbox — logic fully read) |
| Reachable from shipped entry point? | NO — dev-tool; driven manually or by scripts/w4-measure.sh (which only ever uses `--skip-suites`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 2 |

## 1. One-paragraph verdict
This is the most honest piece of test infra audited so far: every Lumen-side cell that does not exist is written as an explicit gap marker (`not-emitted (suite gap)`, `no-decoder`, `no-path` — never fabricated), the oracle is always ffprobe/ffmpeg, persisted TSV reuse under `--skip-suites` is labeled as not-a-measurement, and ffprobe truth is checkpointed with resume. But the "judge" never convicts: judgement metrics (garbage rejection "must be 100%", PCM byte-exactness, packet-count parity) do not influence the exit code — a committed report shows `garbage rejection 10/11 (must be 100%)` with all suite rc=0, i.e. the violated requirement gates nothing. Plus one latent crash (`float("N/A")`), one dead CLI flag, and one KeyError on manifest entries lacking `family`.

## 2. What actually works (verified)
- `python3 -m py_compile tools/corpus/run_harness.py` -> COMPILES-OK (exit 0).
- Bounded RIFF walkers `avi_video_strh_rate` / `avi_audio_strf_block_align` (lines 258-345): read at most a 256 KiB prefix, clamp to declared RIFF end, honor word alignment, garbage size ends the walk — verified by reading; no unbounded loop reachable.
- Truth checkpoint + gap-fill logic (lines 438-469): damaged truth.json is discarded and rebuilt ("never trust it"), partial checkpoints upgraded.
- Gap-marker discipline: lines 134-136 define `GAP/NO_DECODER/NO_PATH`; decoded-rows section (lines 652-732) writes them instead of inventing Lumen cells; `rate()` prints `N/A — not-emitted (suite gap) (0/N comparable)` rather than a fabricated 100%.
- Committed full-run evidence exists: docs/corpus/report-20261022-1334.md (486 files, suite rc all 0) — the pipeline runs end-to-end on a corpus.
- Consumers: scripts/w4-measure.sh:5 (`python3 tools/corpus/run_harness.py --corpus corpus-big --skip-suites`), and the env-driven Rust suites it drives (crates/{media-probe,demux,codecs}/tests/corpus.rs, LUMEN_CORPUS env).

## 3. Findings
F-001 [S2]
Where: tools/corpus/run_harness.py:413
Code: (verbatim)
```
            rec["duration_ms"] = f"{float(d) * 1000:.1f}" if d else "-"
```
What is wrong: `d` is ffprobe's `format.duration` string; for TS/OGV-class files ffprobe emits `"N/A"` (a truthy string), so `float(d)` raises an uncaught ValueError and kills the whole harness run mid-probe. The corpus vocabulary explicitly contains `ts-video` and `ogv-video` classes (V3_CLASSES, line 122-128). Why it matters: one N/A duration aborts a 486-file measurement run with no checkpoint of the partial loop state (the incremental checkpoint at line 465 only fires every 50 probes). What correct looks like: `try: float(d) except ValueError: "-"`. Confidence: MEDIUM — Verification: read (ffprobe not runnable in sandbox; trigger UNVERIFIED at runtime).
F-002 [S3]
Where: tools/corpus/run_harness.py:1399-1411 (exit path) + docs/corpus/report-20261022-1334.md:14
Code: (verbatim)
```
    return 0 if all(rc == 0 for rc in (rc_probe, rc_demux, rc_mjpeg, rc_pcm)) else 1
```
and (report, committed):
```
| garbage rejection | **10/11** (must be 100%) |
```
What is wrong: the exit code depends only on the Rust suites' exit codes; every judgement row the harness computes — including the one it labels "must be 100%" and "any mismatch is a defect signal" (line ~1168) — is report-only. The committed 2026-10-22 13:34 report shows garbage rejection 10/11 (violating its own MUST) and PCM byte-exact 18/41, with the run judged successful. Why it matters: the docstring calls this "the judge and reporter"; the judge half cannot fail, so CI/pipeline consumers see green on red numbers. What correct looks like: `garbage_rejected != garbage_total` (and pcm mismatch files > 0) contribute to exit 1. Confidence: HIGH — Verification: read + committed report cited.
F-003 [S3]
Where: tools/corpus/run_harness.py:366-368
Code: (verbatim)
```
    ap.add_argument("--skip-gen-check", action="store_true",
                    help="compatibility flag: skip corpus freshness verification (this driver only "
                         "requires manifest.json to exist; nothing else is verified today)")
```
What is wrong: dead flag — `args.skip_gen_check` is never referenced anywhere else in the file (rg: zero uses); parsing it changes nothing. What correct looks like: remove it or implement the freshness check it implies once existed. Confidence: HIGH — Verification: ran (`rg -n 'skip_gen_check' tools/corpus/run_harness.py` -> only the add_argument lines).
F-004 [S3]
Where: tools/corpus/run_harness.py:426
Code: (verbatim)
```
            if entry["family"][0] == "avi":
```
What is wrong: direct indexing; a manifest entry lacking `family` raises KeyError, contradicting the file's own back-compat requirement (lines 78-81: "fields absent from old (pre-v3) manifests render '-', never crash"). Elsewhere the same data is read defensively (`entry.get("family") or []`, line 679). What correct looks like: `entry.get("family", ["?"])[0]` guarded for empty. Confidence: MEDIUM — Verification: read.
F-005 [S4]
Where: tools/corpus/run_harness.py:391 (and 11 more: 449, 463, 468, 501, 565, 620, 636, 730, 1155, 1403, 1407)
Code: (verbatim)
```
    print(f"[h[harness] corpus: {len(files)} files, seed {manifest.get('seed')}")
```
What is wrong: mangled log prefix `[h[harness]` in all 12 progress prints — should be `[harness]`. Cosmetic but systematic (a botched search-replace). Confidence: HIGH — Verification: ran (python count = 12) + Read tool (Bash display strips the sequence; verified via Read).
F-006 [S4]
Where: tools/corpus/run_harness.py:504-505
Code: (verbatim)
```
        mjpeg_entries = [e for e in avi_entries
                         if e["params"].get("codec") == "mjpeg" or e["params"].get("codec", "").startswith("mjpeg")]
```
What is wrong: the `== "mjpeg"` clause is dead — subsumed by `startswith("mjpeg")`. Confidence: HIGH — Verification: read.

## 4. Per-function table (module-level; nested functions live inside main())
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| pix_family | 145-160 | REAL | — | classifier, pure |
| sample_family | 163-179 | REAL | — | s32-padded s24 equivalence documented |
| ref_frame_cap | 182-200 | REAL | — | bounded disk; dirty min(cap,16) |
| sh | 203-204 | REAL | — | subprocess wrapper; no check=True (callers check rc) |
| ffprobe_json | 207-214 | REAL | — | returns None on rc!=0/JSON error |
| ffprobe_packets | 217-228 | REAL | — | int() guarded |
| ffprobe_audio_payload_bytes | 231-255 | REAL | — | non-numeric lines recorded as absent |
| avi_video_strh_rate | 258-299 | REAL | — | bounded walk, verified |
| avi_audio_strf_block_align | 302-345 | REAL | — | bounded walk, verified |
| percentile | 348-353 | REAL | — | nearest-rank |
| fmt_ms | 356-357 | REAL | — | NaN -> "-" |
| main | 360-1411 | REAL (defects F-001..F-006) | S2/S3 | nested: probe_truth, run_suite, read_tsv, reuse_results, video/audio_codec_intent, lumen_video/audio_cells, suite_note, avi_supported, num, mjpeg_case_meta, case_kind, t_of, dist, rate — all read; no stub among them |

## 5. Test honesty
Not a test file (a harness). Its honesty posture is unusually good: --skip-suites reports are suffixed and labeled; suite-gap cells are never fabricated (line 130-136 vocabulary). The one dishonesty is structural: nothing it "judges" can fail the run (F-002).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "garbage rejection (must be 100%)" | report table + code | UNENFORCED | exit path ignores it; committed report shows 10/11 with rc=0 |
| "this suite fails only on contract violations" (Rust side) | crates/demux/tests/corpus.rs:13-15 | REAL-UNPROVEN (not executed here) | suite rc=0 in committed report with 10/11 rejection |
| "never crash" on pre-v3 manifest fields | run_harness.py:81 | FALSE for `family` | F-004 KeyError |

## 7. Dead / duplicated / unreachable code
Dead flag `--skip-gen-check` (F-003); dead predicate clause (F-006); `__pycache__` committed under tools/corpus/ (hygiene).

## 8. Dependencies & flags
python3 stdlib only (argparse/json/os/shutil/statistics/struct/subprocess/sys/time). External deps at runtime: ffprobe/ffmpeg/cargo — none present in this sandbox (UNVERIFIED at runtime). Not wired into any .github workflow (rg: zero hits).

## 9. Open questions for the next agent
Does any script drive the FULL (non-skip-suites) mode besides manual dev runs? scripts/w4-measure.sh:5 only uses --skip-suites. Check the 2026-10-22 full reports' provenance (docs/corpus/report-20261022-{1229,1334,1610}.md) — they cite corpus/ which does not exist in-repo (external corpus, manifest copies in docs/corpus/).
