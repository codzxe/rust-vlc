# AUDIT: tools/vta/mpeg4/diff.py
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 139 / 2 (run_lane, __main__ driver) |
| File verdict | REAL (spike differential driver; output-driven, no exit-code contract) |
| Reachable from shipped entry point? | NO — manual VTA driver for the oxideav-mpeg4video spike; the candidate is NOT admitted (zero oxideav in root Cargo.toml/Cargo.lock, verified in the oxideav record audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |

## 1. One-paragraph verdict
A real differential driver, and the machinery behind the mpeg4 record's §D lane table: ffprobe extradata hex-parsing (with the ffprobe hexdump ASCII-column correctly excluded via `[:8]`), harness run + FRAME-line walk, ffmpeg rawvideo + framehash reference, per-frame MD5 + byte-level max-delta + differing-sample counts, and a whole-stream MD5 cross-check. The frame-size math is correct yuv420p (`w*h + 2*ceil(w/2)*ceil(h/2)`). Defects are grade-appropriate: no exit-code contract (divergence still exits 0 — the human reads the table, nothing enforces), the mismatch detail is suppressed exactly when frame counts differ (the interesting lane C case), and a dead "per_frame" key filter for a key that is never stored.

## 2. What actually works (verified)
— Read in full; ran `python3 -m py_compile tools/vta/mpeg4/diff.py` -> exit 0 (parses).
— Frame-size math verified: `n = w*h + 2*((w+1)//2)*((h+1)//2)` == yuv420p bytes/frame (L87).
— Hexdump parse logic reasoned through: ffprobe hexdump lines have 8 hex groups (16 bytes) before the ASCII column; `[:8]` + even-length hex-token filter excludes ASCII-column false positives (L36-41).
— Cross-check with the record: the driver's outputs (my_frames/ref_frames/identical/divergent/max_pixel_delta/stream_md5_equal) are exactly the columns of the §D table in docs/dependencies/records/oxideav-mpeg4video-0.1.8.md L203-212 — the evidence chain is coherent (lane C's 37≠28 appears as my_frames vs ref_frames).

## 3. Findings

F-001 [S3]
Where: tools/vta/mpeg4/diff.py:135-137
Code:
```
        if st.get("my_frames") == st.get("ref_frames"):
            mism = [i for i, f in enumerate(frames) if not f.get("md5_match", False)]
            print(f"  md5_mismatch_frames: {mism[:10]}{'...' if len(mism) > 10 else ''} ({len(mism)} total)")
```
What is wrong: per-frame mismatch detail is printed ONLY on frame-count parity. On the count-mismatched lane (the packed-bitstream 0117 case — the record's headline defect) the driver prints no frame-level detail, forcing the operator to re-derive the duplicate-frame diagnosis from packet dumps. Why it matters: the most divergent lane gets the least diagnostics from the tool. Confidence: HIGH — Verification: read. (The record compensated manually — §D footnote 1.)

F-002 [S4]
Where: tools/vta/mpeg4/diff.py (whole __main__, L121-139)
What is wrong: no exit-code contract — divergence, short refs, or harness nonzero-rc all still exit 0 (the `harness_rc`/`ref_rc` keys are recorded but never gated). Fine for a human-read spike; a footgun if ever wired into a gate. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: tools/vta/mpeg4/diff.py:138
Code:
```
    slim = {k: {kk: vv for kk, vv in v.items() if kk != "per_frame"} for k, v in results.items()}
```
What is wrong: filters a "per_frame" key that `run_lane` never stores (st gets only scalar keys) — dead leftover from an earlier shape. Confidence: HIGH — Verification: read + py_compile.

F-004 [S4]
Where: tools/vta/mpeg4/diff.py:104 vs 105-106
What is wrong (minor undercount): `ndiff += 1` increments only inside `if not same and len(fr) == n` — frames with a SHORT ref slice count as md5-mismatched but not "divergent". Understates divergence count in truncation scenarios. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run_lane | 23-118 | REAL | S3 | ffprobe/harness/ffmpeg orchestration; per-frame MD5 + delta; early-returns record rc keys; F-001/F-004 |
| __main__ | 121-139 | REAL | S4 | lane loop, per-lane print, results.json write; F-002/F-003 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (no tests — driver) | n/a | mutation reasoning: a wrong frame-size formula would produce wrong `n`, mis-slicing every lane (the record's §D "harness bugs caught and fixed" section describes exactly this class being caught by the python byte-compare cross-check) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Method (every command re-usable from tools/vta/mpeg4/README.md)" — driver is the method | oxideav record L181 | REAL | this file + README committed; outputs match record's table columns |
| "ffprobe-extracted extradata hex (--extradata); VIDEO SAMPLES still came through our demuxer" | record L197-199 | REAL | driver code: ffprobe used ONLY for extradata (L27-43); samples via HARNESS (L46) |

## 7. Dead / duplicated / unreachable code
F-003 dead key filter; `import re` inside function (style). Driver is automation-unreachable by design.

## 8. Dependencies & flags
python3 stdlib only (hashlib/json/os/subprocess/sys/re). External tools at runtime: ffprobe + ffmpeg + the Rust harness binary (env-overridable paths, /tmp defaults).

## 9. Open questions for the next agent
1. Does tools/vta/mpeg4/README.md reproduce the exact lane commands (record's claim "every command re-usable")? README is not in my queue — check its command list matches this driver's env knobs.
