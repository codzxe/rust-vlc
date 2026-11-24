# CLAIM AUDIT: docs/dependencies/records/ruopus-0.1.2.md
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL: 5 REAL-UNPROVEN: 2 SHALLOW: 0 STUB: 0 PAPER-ONLY: 0 FALSE: 0 UNREACHABLE: 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The ruopus record is the honest counter-example in the codec-admission
family: it explicitly does NOT claim bit-exactness ("No lane is f32-bit-
exact"), sets an s16-LSB tolerance bar, discloses the one lane that crosses
it (CBR stereo, 162/576,000 samples), and gates the admission on two
binding wrapper conditions. The in-tree admission matches the record's
shape exactly, and the in-tree differential test passes today. This record
is NOT a source of any "bit-exact" claim — by its own text.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | pin `ruopus = { version = "0.1.2", default-features = false, features = ["std"] }` | :48-50 | REAL | crates/codecs/Cargo.toml:137 `ruopus = { version = "=0.1.2", optional = true, default-features = false, features = ["std"] }` |
| 2 | "zero normal dependencies" (condition 1 shape) | :51-54 | REAL | Cargo.lock:763-766 `ruopus 0.1.2` entry with no `dependencies` list (leaf node) |
| 3 | "ADMITTED" verdict + two binding conditions | :151-166 | REAL | in-tree admission: crates/codecs/src/audio/opus/{decoder,error,head,mod,order}.rs exist |
| 4 | condition 2 implemented (OpusHead validation, pre-skip, end-trim) | :211-214 | REAL | opus wrapper has head.rs; opus_corpus_differential.rs:64 asserts `run.pre_skip == 312` ("the recorded 312") |
| 5 | "`opus_corpus_differential` — 2/2 pass" | :220-221 | REAL | baseline cargo-test-workspace.txt:1290-1295 `2 passed; 0 failed` (corpus_0047_mono, corpus_0077_surround_51) |
| 6 | "No lane is f32-bit-exact" (honest negative) | :135-138 | REAL-UNPROVEN | in-tree test asserts the tolerance class (s16 LSB bound) not bit-exactness — consistent; the float-divergence measurement itself not re-runnable here |
| 7 | "643 test cases ... 624 pass, 18 ignored, 0 fail" (2026-11-09) | :216-219 | REAL-UNPROVEN | past run; no toolchain; shape consistent with baseline |
| 8 | matrix `codec-audio-opus` flipped blocked→implemented | :222-225 | REAL | docs/feature-matrix.toml:746 `status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }` |
| 9 | "9,327 downloads", published 2026-07-01, MSRV 1.85, 69 unsafe lines | :14-16, :26-28 | REAL-UNPROVEN | registry/tarball facts not checkable from sandbox (no network, no vendored tarball) |
| 10 | hostile packets: "Zero panics", typed Errs, DTX concealment | :142-149 | REAL-UNPROVEN | vta harness run not repeatable here; consistent with the crate's Result-returning API documented at :74-77 |

## 3. Contradictions
None found. The record's tolerance-class verdict and the in-tree test's
tolerance-class assertions agree.

## 4. Fabrication / inflation indicators
None. The record advertises its own negative results (f32 not bit-exact;
162 LSB crossings on one lane) and marks s16-MD5 equality as an INVALID
criterion for this crate (:172-173) — the opposite of inflation.

## 5. Matrix rows
N/A (dependency record). Note: compat-matrix-20261110.md:245 lists opus in
"clean decode (0 concealed packets)" — no bit-exact claim attached to opus
anywhere; the 29/35 bit-exact rows are PCM-only (vlc-diff-report).

## 6. What actually works (verified)
— Cargo.lock:763-766 (ruopus leaf node, registry checksum); Cargo.toml:137
pin; ls of crates/codecs/src/audio/opus/; baseline opus_corpus_differential
2/2 pass (2026-11-24 run).

## 7. Open questions
The record's §D raw f32 MD5 pairs (e.g. ff e918a5f7... vs ruopus
a3b022c6...) are session measurements with no committed raw dumps — the
six-lane numbers are only as trustworthy as the record's self-report; the
in-tree 2-lane re-run covers only the two corpus lanes.
