# DATA AUDIT: docs/corpus/compat-matrix-20261110.json
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 2,986 lines JSON; 16 formats + 22 codecs records; 67 regen_outcomes; 5 transitions |
| File verdict | REAL (internally consistent measurement report; hand-edit disclosed in-file) |
| Findings | S0: 0 S1: 0 S2: 1 (ecosystem, code-cited) S3: 1 S4: 2 |
| Commands run | python3 json parse + census (exit 0); raw-rows.jsonl census; sed -n quote checks |

## 1. What it is / provenance
Machine record of the compat-matrix run on 2026-11-10T19:27:42Z, host linux/x86_64,
git_commit 3dfe1c0b19c593011a5d238074fc4f67eb390882 (UNVERIFIED — no git allowed). Corpus:
committed_files 46, malformed_files 54, regen_files 66 (plus one FAILED regen disclosed in
regen_outcomes: 31 present_verified + 35 present_random + 1 failed = 67 entries).
Lanes: linux-native (ran), web-wasm (recorded build steps; ships DECODER-LESS, disclosed),
windows/macos (blocked-lane citations — no result inferred). Oracle: pinned VLC 3.0.23
Vetinari (3.0.23-2-0-g79128878dd); lossless family compared only — honest scope.
Header verbatim (sed -n '1,9p'):
```
{
  "schema": "compat-matrix/report/1",
  "generated": "2026-11-10T19:27:42Z",
  "host": {
    "os": "linux",
    "arch": "x86_64"
  },
```

## 2. Data verification (executed, python3)
- counts.files_total 166 == wc -l raw-rows.jsonl == 166 EXACT.
- Bucket reconciliation (my arithmetic): raw-rows statuses decoded 97 / decoded_partial 33 /
  refused_probe 4 / refused_demux 24 / demux_fatal 8 (sum 166). counts block: decoded 84 /
  decoded_partial 22 / refused_probe 2 / refused_demux 4 / malformed_typed 54 (sum 166).
  Difference per bucket = the 54 malformed files (13+11+2+20+8=54 EXACT) rebucketed under
  malformed_typed. Both totals reconcile with zero residue.
- Oracle: bit_exact 29 + tolerance_fail 6 = 35 == the lossless-comparable rows 1-35 claimed
  in corpus.regen_provenance. Tolerance passes 0 — honest (6 FAILs kept, not hidden).
- Transitions: 5 entries, 4 distinct (row,platform) cells — codec-audio-aac-lc linux appears
  TWICE with different reasons (concealed-damaged-packets vs refused-no-decoder); the
  duplicates target the same from->to (implemented->partial) so application is idempotent.
- Lanes census (formats+codecs cells): linux-native decoded 23 / refused 2 / no-decoder 8 /
  not-covered 4 / decoded-damaged 1; windows+macos all blocked; web-wasm all
  engine-core-only. No cross-platform inference — the honest lane discipline holds.

## 3. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "committed_files": 46 | corpus | REAL-UNPROVEN | committed fixtures countable; corpus media partly gitignored — spot count not re-derived |
| 2 | "malformed_files": 54 | corpus | REAL | my raw-rows rebucketing arithmetic = 54 EXACT |
| 3 | "regen_files": 66 | corpus | REAL | regen_outcomes 67 incl. 1 failed = 66 + 1 disclosed |
| 4 | counts block totals | counts | REAL | 166 == raw-rows line count; buckets reconcile exactly (above) |
| 5 | "Windows (blocked-by-external — no result inferred)" | lanes | REAL | lanes[3] kind=blocked, steps=[] |
| 6 | wasm module "ships DECODER-LESS on wasm32" | notes[1] | REAL | apps/web/Cargo.toml codecs edge target-gated (W18-A04 modules-linux.json corroborates) |
| 7 | "Transitions are PROPOSALS in the outbox (5)" | notes[3] | REAL | 5 entries counted; no `verified` proposed |
| 8 | "the raw rows … are the immutable evidence and were not touched" | :2984 | REAL-UNPROVEN | no git; hand-edit itself is disclosed verbatim below |
| 9 | oracle run on pinned VLC 3.0.23 | oracle.source | REAL | version hash matches dumps 04/13/14/16/17 baseline set |

## 4. Findings
F-001 [S2 — ecosystem] Where: apps/cli/src/host.rs:1643-1654 (consumer of this report's
decoder-split claims). Code (verbatim, sed-verified):
```
        decoders: vec![
            DecoderInfo {
                name: "mjpeg".to_string(),
                kind: TrackKind::Video,
                hardware: false,
                backend: None,
                active: false,
            },
            DecoderInfo {
                name: "pcm_s16le".to_string(),
                kind: TrackKind::Audio,
                hardware: false,
                backend: None,
                active: false,
            },
        ],
```
engine_info() hardcodes exactly two decoders regardless of the ~15 wired in
pipeline_host.rs — the same registry disagreement W13 flagged; it also explains the
baseline's "no decoder in this build" text while other registries list 12 landed codecs.
The compat-matrix json is the honest measurer; the CLI registry is the liar.
Confidence: HIGH — Verification: read + sed.
F-002 [S3] Machine record hand-edited post-run. Where: :2984 verbatim:
"POST-RUN EDITORIAL NOTE (appended by hand by the W24-P review-fix lane, branch w24/p,
2026-11-10 — finding P2-1 of the W24-L audit, docs/reviews/w24-d-compat-findings.md): the
linux-native cell details of the avi and wav format rows were surgically updated …"
Disclosed in-band and claims raw rows untouched — but "machine record" purity is broken;
consumers must know cells for avi/wav are editorial, not measured. Confidence: HIGH.
F-003 [S4] counts-block bucketing (malformed_typed absorbs statuses the raw rows name
differently) requires the arithmetic above; no in-file note explains it.
Confidence: HIGH — Verification: ran.
F-004 [S4] Duplicate transition target (codec-audio-aac-lc linux implemented->partial
twice). Idempotent to apply, but "5 proposals" overstates 4 distinct corrections.
Confidence: HIGH — Verification: ran.

## 5. Consumers
docs/feature-matrix.toml (transition targets), docs/corpus/compat-matrix-20261110.md (the
human report), compat-matrix-transitions-applied.jsonl, docs/reviews/w24-d-compat-findings.md.
Prior W19-A09 audit (old tree audit/docs/corpus/...) is corroborated on every number I
re-derived.
