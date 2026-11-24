# AUDIT: tools/corpus/gen_corpus.py
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 1,338 / 20 top-level (incl. Generator class with 8 methods) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev/test tooling (drives ffmpeg out-of-tree into gitignored /corpus*, per .gitignore:96-107). Not part of lumen-cli/desktop builds. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 4 |
| Commands run | `python3 tools/corpus/gen_corpus.py --out audit/runs/W16-A04.corpus-smoke --preset smoke --jobs 2 --seed 20261124` -> exit 0, 133 files, 73 MiB, wall 29s, 29 capability probes printed, 1 rejected draw; second run same seed -> exit 0; manifest/plan diff via python3 |

## 1. One-paragraph verdict
A genuinely engineered adversarial corpus generator (plan freeze, capability probing, dirty/garbage passes, honest rejected-draws ledger) that RUNS end-to-end in this sandbox (executed twice, exit 0). Its central determinism contract is nevertheless FALSE in execution: two runs with the identical seed produced byte-identical plan.json but 36/89 differing clean files, because audio draws use ffmpeg `anoisesrc` without a `seed=` parameter, and the dirty/garbage passes additionally depend on thread-pool completion order. Everything else (box walkers, post-ops, budget gate, manifest) checks out under inspection and execution.

## 2. What actually works (verified)
- Ran `--preset smoke --seed 20261124` (twice): exit 0 both times; 92 planned entries, 133 files (41 dirty/garbage), 73 MiB, 29 capability probes (counted: exactly 29, matching the argparse help text), 1 rejected draw recorded honestly (`{'idx': 58, 'family': ['mp4','prores'], 'reason': 'unsupported-draw'}`). Probes honestly reported `rotate_meta=N` and `zeroframe:mp4=N` on this ffmpeg build. Manifests kept at audit/runs/W16-A04.corpus{,2}-smoke/manifest.json.
- plan.json freeze verified: both runs' plan.json are 92/92 entry-identical -> the plan-freeze claim is real.
- post_tkhd_rotate offsets independently derived correct: v0 matrix at body+40, v1 at body+52 (recomputed against ISO 14496-12 tkhd field layout); hdlr handler_type at o4+16 correct.

## 3. Findings
F-001 [S2]
Where: gen_corpus.py:11-14 (contract) vs corpus_builders.py:142-151,1092,1386 (cause)
Code:
```
    * RANDOM by default — seed from OS entropy; the full randomness is frozen
    into plan.json before any encoding starts, so a reaper kill mid-run
    loses nothing and `--seed S` (or replaying the recorded seed)
    reproduces the exact corpus.
```
What is wrong: "`--seed S` … reproduces the exact corpus" is false. corpus_builders.py builds `anoisesrc=color=…:duration=…:sample_rate=…` with no `seed=` (ffmpeg anoisesrc is entropy-seeded per process). Executed proof: two same-seed runs -> plan.json 92/92 identical but 36/89 clean files differ; e.g. media/0014-sr-chaos.wav (argv identical across runs) first differs at byte 102 inside the PCM data (`\x00\x00Lavf61.7.103` muxer tag identical, payload diverges). Why it matters: manifests record per-file sha256 that a replay cannot reproduce, so "replay with --replay manifest.json" (docs/corpus/report-*.md) cannot regenerate the recorded corpus for regression confirmation. What correct looks like: pass `seed=<erng-drawn>` into every anoisesrc/entropy-based lavfi source. Confidence: HIGH — Verification: ran (both runs + byte-level diff).
F-002 [S3]
Where: gen_corpus.py:1015-1030, 1056-1062
Code:
```
        done = {f["rel"] for f in self.files}
        ...
        with ThreadPoolExecutor(max_workers=self.jobs) as ex:
            futs = {ex.submit(self.execute_entry, e): e for e in todo}
            for fut in as_completed(futs):
                rec = fut.result()
                ...
                    self.files.append(rec)
```
What is wrong: self.files is appended in completion order; dirty_pass does `self.rng.sample(clean, …)` over that order, so dirty/garbage derivative selection varies run-to-run even with a fixed seed (observed: run A picked dirty source 0077-avi-video, run B 0082-multivideo-av; 2 dirty files present in only one run each). Why it matters: compounds F-001; replay of the dirty layer is impossible. What correct looks like: sort self.files by rel (plan order) before the dirty/garbage passes. Confidence: HIGH — Verification: ran.
F-003 [S3]
Where: tools/corpus/README.md:8-10, 26 vs reality
Code:
```
| `corpus_builders.py` | the 12 per-class ffmpeg command builders |
```
What is wrong: README says "17 tiny real encodes are probed" and "the 12 per-class ffmpeg command builders"; actual: 29 probes (verified by execution), 27 builder kinds (imported BUILDERS). The v3-era numbers were never updated for v4. Why it matters: ops docs misstate the tool's coverage. Confidence: HIGH — Verification: ran (probe list printed) + imported BUILDERS.
F-004 [S4]
Where: gen_corpus.py:935-937
Code:
```
            self.rejected.append({"idx": entry.get("idx", -1),
                                  "family": entry["family"], "reason": err,
```
What is wrong: plan entries built in draw_plan (:880-885) carry no "idx" key, so every execution-time rejection records idx -1 — the manifest cannot say which plan slot failed. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: gen_corpus.py:523-535
Code:
```
        if size < 8 or off + size > end:
            return
        yield typ, off, size
        off += size + (size & 1)
```
What is wrong: `size & 1` pad advance is a RIFF convention, not ISO-BMFF (boxes advance exactly `size`); an odd-size box would misalign the walk. Bounded (refuses on overrun), and generator-authored boxes are even today. Confidence: HIGH — Verification: read.
F-006 [S4]
Where: gen_corpus.py:469
Code:
```
            idx_entries.append((cid, rel, len(piece), 0x10 if k == 0 else 0))
```
What is wrong: every split piece of the first input chunk (j>0 included) is flagged AVI_KEYFRAME 0x10 — a silent semantic lie beyond the documented rechunk lie. Minor: it is a dirty-world file by design. Confidence: HIGH — Verification: read.
F-007 [S4]
Where: gen_corpus.py:772-773
Code:
```
    if params.get("class") == "drift-av":
        v = int(v * 1.0)
```
What is wrong: dead no-op. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| sha256_file | 132-137 | REAL | — | streamed, correct |
| run | 140-150 | REAL | — | timeout/OSError handled; returns last stderr line |
| probe_commands | 159-190 | REAL | — | 15 probe cmds |
| probe_caps | 193-378 | REAL | — | 29 probes total; verified live (2/29 honestly N) |
| available_encoders | 381-389 | REAL | — | parses `-encoders` table |
| _avi_boxes | 399-408 | REAL | — | bounded RIFF walk, pad-aware |
| _avi_find_movi_idx1 | 411-418 | REAL | — | first movi/idx1 only |
| _avi_movi_chunks | 421-438 | REAL | — | rec-descend, JUNK skip |
| post_avi_rechunk | 441-497 | REAL | S4 (F-006) | repatches movi+RIFF sizes (verified logic); stale header fields left deliberately |
| post_riff_size_lie | 500-520 | REAL | — | oversize/max/tiny/zero lies, masked to u32 |
| _mp4_boxes | 523-535 | REAL | S4 (F-005) | odd-pad advance non-ISO |
| post_tkhd_rotate | 538-587 | REAL | — | offsets independently re-derived correct; quadrant+flip composition correct |
| draw_family | 601-719 | REAL | — | full class->family mapping, raises on unknown |
| random_features | 722-745 | REAL | — | dice rolls; stratification may override |
| est_bytes | 748-774 | REAL | S4 (F-007) | plan-time budget estimate; est_hint override honored |
| Generator.__init__ | 778-807 | REAL | — | replay_v3 gating |
| Generator.draw_plan | 810-886 | REAL | — | verified deterministic (plan.json identical across runs) |
| Generator.execute_entry | 889-982 | REAL | S4 (F-004) | argvs, aux decode, post-ops, harvest; __TMP__/__OUT__ substitution |
| Generator._validate | 984-989 | REAL | — | ffprobe format-name check |
| Generator.run | 991-1053 | REAL | S3 (F-002) | resume support; bad_clean only warns (exit stays 0 — honest-ledger by design) |
| Generator.dirty_pass | 1056-1156 | REAL | S3 (F-002) | 12 corruption ops verified present in vocab |
| Generator.garbage_pass | 1159-1233 | REAL | — | 22 archetypes verified in vocab |
| Generator.write_manifest | 1236-1257 | REAL | — | atomic tmp+replace |
| main | 1260-1334 | REAL | — | refuses non-empty out; replay + resume seed recovery |

## 5. Test honesty
No tests target gen_corpus.py itself (no pytest/unittest in tools/corpus/). Its "test" is execution — performed: exit 0 twice. The tool's own honest-ledger behaviors (rejected_draws, ffprobe_ok=False recorded not fatal) verified in manifest.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "--seed S … reproduces the exact corpus" | gen_corpus.py:14-19 docstring | FALSE | two same-seed runs -> 36/89 clean shas differ (executed; manifests kept) |
| "29 capability probes" | gen_corpus.py:1268 argparse help | REAL | counted 29 in live probe output |
| "22 garbage archetypes / 12 byte-level ops" | gen_corpus.py:37-39, README | REAL | len(GARBAGE_ARCHETYPES)=22, len(CORRUPTION_OPS)=12 (imported) |
| "17 tiny real encodes are probed" | tools/corpus/README.md:26 | FALSE | 29 probed (executed) |
| "12 per-class ffmpeg command builders" | tools/corpus/README.md:10 | FALSE | 27 BUILDERS kinds (imported) |
| corpus generated into gitignored dir, not committed | .gitignore:96-107, CORPUS.md | REAL | /corpus/, /corpus-smoke/, /corpus-big/, corpus-v4/ ignored |
## 7. Dead / duplicated / unreachable code
est_bytes drift-av no-op (F-007); rotate_meta probe is a known-dead route on ffmpeg 7.x (documented in-code at :276-277, superseded by rot_copy probes). No dead files: all 7 companion modules exist (corpus_vocab.py, corpus_builders.py, minimize.py, fidelity.py, speed_bench.py, run_harness.py, README.md).
## 8. Dependencies & flags
External: ffmpeg/ffprobe (present in sandbox at /usr/bin; env override FFMPEG/FFPROBE respected via --ffmpeg/--ffprobe args, defaults read at import — fine for a dev tool). No cargo/npm coupling.
## 9. Open questions for the next agent
- Who generated the 85 committed fixture files under tests/fixtures/generated/ outside the avi/mp4_mjpeg_pcm families (aiff/wav/ogg/mkv/webm/3gp/qt/ivf/frag/chapters/edts + 59 derivatives)? No in-tree generator references them; CORPUS.md claims fixtures.rs authored "every file under generated/" (see CORPUS.md:28-33) while fixtures.rs only handles 26 media files. Provenance UNVERIFIED.
- Whether mkv SegmentUID randomness also contributes to F-001 beyond anoisesrc (likely; not isolated).
