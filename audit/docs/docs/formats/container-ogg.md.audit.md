# CLAIM AUDIT: docs/formats/container-ogg.md (104 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 REAL / 1 REAL-UNPROVEN (corpus MD5 lanes) / 0 / 0 / 0 / 0 / 1 UNVERIFIED (bit-exact digests) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Accurate record with the same self-diagnosed-stale-matrix pattern, and the amendment
landed (matrix container-ogg implemented x3, evidence citing this record and the
"5.REC-1: chain header-stash 28v31 divergence recorded" note — the divergence the record
itself documents in its honest-divergences section). Code exists as scoped
(crates/demux/src/ogg/{mod,page,error,fuzz,test_support,tests}.rs — 2,819 lines today vs
2,651 at authoring), fuzz_demux_ogg.rs is landed, corpus tests exist (corpus_ogg.rs,
ogg_fixtures.rs) and ran green in baseline.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Feature-matrix row container-ogg still reads not_started ... this record is that evidence; amendment recorded in-file" | :3-6 | REAL — amendment LANDED | matrix :154-166 implemented x3 + evidence "record docs/formats/container-ogg.md, 5.REC-1: chain header-stash 28v31 divergence recorded". |
| 2 | "self_built = [crates/demux/src/ogg/** (... 2 651 lines at 5.B1)]" | :12-13 | REAL (grew since) | wc -l -> 2,819 today across 6 files. |
| 3 | "EXTENSIONS = ["ogg","oga","ogv","opus","spx"] (ogg/mod.rs:115, verified this audit)" | scope § | REAL | ogg/mod.rs exists; the record's self-verification note is the honest pattern. |
| 4 | "fuzz_target = fuzz_demux_ogg (in-crate ogg/fuzz.rs + fuzz-lite sweeps)" | :10 | REAL | fuzz_demux_ogg.rs landed (35-target census); ogg/fuzz.rs exists in-crate. |
| 5 | Corpus 4/4 PASS + chained-cat + 9 derivatives with typed outcomes | evidence § | REAL-UNPROVEN (digests) | corpus_ogg.rs + ogg_fixtures.rs exist and ran green in baseline. |
| 6 | "Honest divergences (measured, recorded — 5.B1)" section | tail § | REAL disclosure | The matrix evidence string itself carries the divergence note — drift surfaced, not hidden. |

## 3. Contradictions
None.
## 4. Fabrication / inflation indicators
None.
