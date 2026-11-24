# AUDIT: tools/vta/dnx/evidence/compare.py
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 31 / 1 (stats) + main loop |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — evidence-dir utility behind the 6.VTA-DNX-RATIFY spike; not in CI, not shipped |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | real run -> FileNotFoundError (inputs gone); ast.parse -> OK; synthetic 8-bit + 10-bit lanes via patched copy -> EXACT expected stats; short-mine probe -> IndexError; 45/45 percentages in compare-results.txt recomputed -> 0 mismatches |

## 1. One-paragraph verdict
The plane-diff math is real and verified exactly on synthetic inputs for both bit depths: Y/Cb/Cr slicing at 4:2:2 geometry, LE u16 reads for 10-bit, maxΔ/differing/percentage per plane, frame slicing, and the md5 line all reproduce hand-computed expectations, and all 45 stat lines in the committed compare-results.txt recompute exactly. But the script is unrunnable in-tree — the five .ref/.mine raw pairs it consumes were never committed, so the record's "five-lane differential" evidence chain is outputs-only, and the script itself crashes on size mismatch instead of diagnosing it.

## 2. What actually works (verified)
— verified by real run: `python3 compare.py` (from tools/vta/dnx/evidence) ->
```
FileNotFoundError: [Errno 2] No such file or directory: 'L1_1080p_175M.ref.raw'
```
— verified by synthetic execution (patched copy, lanes replaced with T8 8x4 8-bit 2-frame and T10 8x4 10-bit 1-frame, planted diffs):
```
T8: 2 frames
 frame 0:  md5 differs (dfea932bea68)
  Y: maxΔ=2 differing=2/32 (6.250%)   <- planted Y[0]+2, Y[5]+1
  Cb: maxΔ=1 differing=1/16 (6.250%)  <- planted byte 40 (Cb range [32..48))
  Cr: maxΔ=3 differing=1/16 (6.250%)  <- planted byte 55 (Cr range [48..64))
 frame 1:  md5 EQUAL (03bb230cc820) ... maxΔ=0 differing=0
T10: 1 frame: Y maxΔ=2 differing=2/32; Cb maxΔ=1 differing=1/16; Cr maxΔ=3 differing=1/16
```
(planted sample 0 +2, 3 +1, Cb#5 +1, Cr#2 +3 — all exact; LE u16 read confirmed via struct '<H' packing.)
— verified: all 45 "maxΔ/differing/n (pct)" lines of compare-results.txt recompute with 0 mismatches.
— verified: hardcoded lane list (5 names/dims/depths) == record filmcraft-dnx-0.9.0.md:192-196 lane table EXACTLY (1920×1080 8-bit x3, 1280×720 8-bit, 1920×1080 10-bit) and == lane-md5s.txt's 5 file pairs; record's L5 "2/1/2 (of 1023)" == max over compare-results L5 frames.

## 3. Findings
F-001 [S3]
Where: tools/vta/dnx/evidence/compare.py:25 (vs directory listing)
Code: `    a = open(f"{name}.ref.raw","rb").read(); b = open(f"{name}.mine.raw","rb").read()`
What is wrong: the script's five input pairs (L1..L5 .ref.raw/.mine.raw) are not in the tree — evidence/ holds only the outputs (compare-results.txt, lane-md5s.txt, ffprobe-facts.txt, hostile-results.txt). The committed per-frame stats backing the record's five-lane differential classification (TOLERANCE, maxΔ 2 LSB) cannot be re-derived or re-verified by anyone; only the output text exists. Same evidence-family gap W17-A06 flagged systemic ("outputs committed, zero raw inputs"). Why it matters: the differential evidence is assertion-only once its inputs are gone. What correct looks like: commit the raws (or a hash-tree + regeneration script) or note their absence in the README.
Confidence: HIGH — Verification: ran (FileNotFoundError pasted) + ls of the directory.

F-002 [S4]
Where: tools/vta/dnx/evidence/compare.py:27, 17-18
Code: `    nf = len(a)//fsz` / `        d = abs(get(a,i) - get(b,i))`
What is wrong: frame count is derived from the REF alone: a SHORTER mine crashes with a bare IndexError (probe: `IndexError: index out of range` from get), a LONGER mine has its extra frames silently ignored; no size-mismatch diagnostic anywhere.
Confidence: HIGH — Verification: ran (short-mine probe).

F-003 [S4]
Where: tools/vta/dnx/evidence/compare.py:1-2, 9, 26
Code: `import sys, hashlib` / `def stats(a, b, w, h, depth, name):` / `    fsz = (w*h + 2*(w//2)*h) * (1 if d==8 else 2)`
What is wrong: `sys` imported and never used; `name` parameter passed to stats() and never used; the depth≠8 branch's `fsz = 2*(w*h + 2*ch_w*h)` (:9) is computed and never used (Cr's end is given in samples); fsz is recomputed in the main loop (:26) duplicating the function's internal logic.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: tools/vta/dnx/evidence/compare.py:13-14
Code: `    am, bm = hashlib.md5(a).hexdigest(), hashlib.md5(b).hexdigest()
    out.append(f"  md5 {'EQUAL' if am==bm else 'differs'} ({am[:12]})")`
What is wrong: the log line prints only the REF hash, truncated to 12 hex chars; mine's hash never appears — the EQUAL/differs verdict is not re-derivable from the logged text, and lane-md5s.txt (full, whole-stream) has no per-frame counterpart to reconcile against. Also: 4:2:2 geometry is implicit in `ch_w*h` — reused on 4:2:0 content it would mis-slice without complaint (all five lanes are 4:2:2/422p10le, so latent only).
Confidence: HIGH — Verification: read.

## 4. Per-function table (31 loc — brief)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| stats() | 2-22 | REAL | F-003/F-004 | math verified EXACT (8/10-bit) |
| lanes + main loop | 23-31 | REAL-UNPROVEN in-tree | F-001/F-002 | inputs absent; IndexError on size mismatch |

## 5. Test honesty
No tests. My synthetic execution is the test: planted diffs of known magnitude/positions were reported with exact counts, magnitudes and percentages for both depths. The committed output (compare-results.txt) is arithmetically self-consistent (45/45) and matches the record's lane table.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "five-lane differential against ffmpeg 7.1.5" | record:9 | REAL-UNPROVEN | script + 5 hardcoded lanes + outputs committed; raws gone (F-001) |
| "byte-count parity exact on all 5 lanes" | record:198 | UNVERIFIABLE-here | requires the raws; output shows frame counts 3=3 consistently |
| "every frame within maxΔ 2 LSB" | record:198-199 | REAL | compare-results.txt 15 frames, all maxΔ ≤ 2 (read) |

## 7. Dead / duplicated / unreachable code
`sys` import; `name` param; else-branch `fsz`; duplicated fsz computation (F-003).

## 8. Dependencies & flags
stdlib (sys[dead], hashlib). No args, no shebang (invoked as `python3 compare.py`); reads 5 hardcoded lane file pairs from CWD.

## 9. Open questions for the next agent
- Were the .raw dumps ever committed anywhere (worktree, /tmp)? /tmp/my-project exists but has no dnx raws (not checked exhaustively — UNVERIFIED).
- L2's three identical per-frame md5 prefixes (a0f09fa72731 x3) are explained by the record ("bars are static") — consistent, no fabrication.
