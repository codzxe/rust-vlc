# CLAIM AUDIT: docs/engineering-directives.md
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 8 / 0 / 0 / 0 / 0 / 1 / 1 |
| Internal contradictions found | 1 (archetype count) |

## 1. Blunt verdict
A binding standards doc whose every named project anchor exists in code:
BoundedQueue, audio-output SPSC ring, codec-api ConcealedFrame/CorruptInput
damage model, corpus classes vfr-av/dynres-video/meta-bomb-av/drift-av, the
splice-other corruption op, DRIFT_DUR_RANGE (45.0, 110.0), build_drift,
media-clock DriftCorrection, the D-10 q1..q4 citation in codecs benches,
gen_corpus.py/run_harness.py/speed_bench.py and the env-gated corpus.rs
suites in three crates. One numeric slip: "23 garbage archetypes" — the
list has 22. The directives themselves are policy prose (binding user
directives), not executable claims.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | D-1 anchors: "packet-queue `BoundedQueue`; audio-output SPSC ring" | :32-34 | REAL | crates/packet-queue/src/queue.rs:1 "The bounded queue core: [`BoundedQueue`]"; crates/audio-output/src/ring.rs:1 "a wait-free SPSC ring of interleaved f32 frames". |
| 2 | D-3 anchors: "codec-api damage model (ConcealedFrame/CorruptInput …)" | :59-60 | REAL | crates/codec-api/src/decode.rs:158 `CorruptInput,` :161 `ConcealedFrame,` :263 `RecoverReason::CorruptInput`. |
| 3 | D-5: "23 garbage archetypes (`GARBAGE_ARCHETYPES` in `tools/corpus/corpus_vocab.py`)" | :82-83 | FALSE (off-by-one) | corpus_vocab.py:254-266 `GARBAGE_ARCHETYPES = ["empty", …, "html-text",]` — 22 string entries, not 23. |
| 4 | D-5: "vfr-av / dynres-video / meta-bomb-av … `splice-other` corruption op … `speed_bench.py`" | :79-86 | REAL | corpus_vocab.py CLASS_WEIGHTS has `("vfr-av", 6)`, `("dynres-video", 3)`, `("meta-bomb-av", 1)`; :251 `"splice-other"` in CORRUPTION_OPS; tools/corpus/speed_bench.py:8 "best-of-N wall time … ratio" + :2 "Lumen vs ffmpeg (parity target)". |
| 5 | D-8: "the corpus `drift-av` class … `DRIFT_DUR_RANGE` 45–110 s … builder `build_drift`" | :204-208 | REAL | corpus_vocab.py:366 `DRIFT_DUR_RANGE = (45.0, 110.0)`; ("drift-av", 2) in CLASS_WEIGHTS; corpus_builders.py:894 `def build_drift(...)`. |
| 6 | D-8 anchor: "`crates/media-clock/src/drift.rs`, `DriftCorrection`" | :212-213 | REAL | crates/media-clock/src/drift.rs:12 "[`DriftCorrection`] is that bounded rate adjustment". |
| 7 | D-10 code-facing check "cited in-tree as D-10 q1..q4, e.g. crates/codecs/tests/benches.rs" | :281-283 | REAL | crates/codecs/tests/benches.rs:248 "The 4K60 budget question (D-10 q2): realtime is 60 frames/s." |
| 8 | D-11 anchors: "`tools/corpus/gen_corpus.py`, `run_harness.py`, `crates/{media-probe,demux,codecs}/tests/corpus.rs` (env-gated suites…)" | :302-305 | REAL | ls tools/corpus -> gen_corpus.py, run_harness.py present; corpus.rs present in all three crates' tests/ (codecs additionally carries differential corpus suites). |
| 9 | Numbering note: "D-8/D-9 handles … already cited throughout the tree (docs/audits/2026-10-22-directives-audit.md, crate doc-comments)" | :150-155 | REAL | docs/audits/2026-10-22-directives-audit.md exists (ls docs/audits/). |
| 10 | Status: "BINDING (user directive, 2026-10-22) … Audit compliance is mandatory" | :3-7 | UNVERIFIED (process claim) | Enforcement instrument is the per-card checklist + phase-gate re-audit protocol (§Compliance audit protocol); whether every card since 2026-10-22 carried the D-10 four answers is not mechanically checkable from this file. |

## 3. Contradictions / Fabrication
| Doc says | Code says | Truth |
|---|---|---|
| "23 garbage archetypes" :82 | corpus_vocab.py:254 list has 22 entries | Code is true; doc count stale/off-by-one (S4). |

No inflation: the doc even pins its own deviations (D-4 "Project note:
the Phase-2 engine is a deliberately synchronous virtual-time pump … Any
known deviation must be recorded, not silent" :69-73).

## 4. Matrix rows
None.
