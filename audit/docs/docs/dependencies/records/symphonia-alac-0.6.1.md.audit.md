# CLAIM AUDIT: docs/dependencies/records/symphonia-alac-0.6.1.md
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 / 0 / 0 / 0 / 0 / 0 / 2 |
| Internal contradictions found | 1 (record's own field map vs doc 11 §2.1) |

## 1. Blunt verdict
Repo-side claims all verify: `symphonia-codec-alac = { version = "=0.6.1",
optional = true }` (codecs/Cargo.toml:115), in Cargo.lock (:1027), feature
`codec-alac` wired (:61), siblings (common/core/aac) present as the record's
graph says, and the "no bundle spelling for ALAC" name-check matches the tree.
But the record itself is malformed against the spec it cites: it claims
"doc 11 §2 format, fields R1–R10" while renumbering the canonical fields —
doc 11 §2.1 (docs/11-dependency-security-compliance.md:116-119) defines R7
License / R8 Maintenance / R9 Provenance / R10 Verdict; this record instead
uses R7 Withdrawn-versions / R8 License / R9 Provenance / R10 Size — and
therefore CONTAINS NO VERDICT AT ALL: the file ends at the F1–F7 summary
table with no pass/accept/reject statement, no pin sentence, and no
evidence-posture footer (all of which its sibling symphonia-aac-0.6.1.md
has). The crate is admitted in the tree with an exact pin, yet the audit
record that should carry the admission's R10 verdict does not have one. That
is a broken audit-chain link, not just cosmetics.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "the crate is `symphonia-codec-alac` ... and the 0.6.1 train matches the pinned core/common" | :9-13 | REAL | codecs/Cargo.toml:115 pin =0.6.1; Cargo.lock:1027 symphonia-codec-alac 0.6.1; no bundle-alac anywhere |
| 2 | "1 new registry package ... all siblings shared" (R2 graph) | :38-52 | REAL | Cargo.lock: symphonia-codec-alac + symphonia-common (1049) + symphonia-core + log all present — matches the recorded shape |
| 3 | "Final acceptance and the allowlist row remain Lead decisions ... untouched by 5.J3a" | :14-16 | SHALLOW->see F-001 | the crate IS admitted in the tree (pin at codecs/Cargo.toml:115) — the acceptance happened, but in a later wave; this record never records it (no verdict field) |
| 4 | tarball sha256 `920a78f96f3cf62932d0c497959c0cc2100ef36b03c7ca1417b260f432a482d8` matching the index checksum | :12-14, R9 | UNVERIFIED | no network/tarball in sandbox |
| 5 | "no native source ... single source file (src/lib.rs, 671 lines) ... #![forbid(unsafe_code)] at lib.rs:10" | R1 | UNVERIFIED | tarball-internal |
| 6 | "nothing yanked in the series; no RUSTSEC advisories against the symphonia family as of this session" | R7 | UNVERIFIED | network claim |
| 7 | "doc 11 §2 format, fields R1–R10" | :5 | FALSE (format claim) | doc 11 §2.1:116-119 defines R7 License / R8 Maintenance / R9 Provenance / R10 Verdict; this record's R7/R8/R10 are different fields and R10 Verdict is MISSING — see F-001 |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "doc 11 §2 format, fields R1–R10" (:5) | doc 11 §2.1 field table (116-119): R7 License, R8 Maintenance, R9 Provenance, R10 Verdict | The record deviates: its R7 is withdrawn-versions, R8 license, R10 size; the aac/mp3 sibling records DO follow the spec. The alac record is the outlier |
| header: "untouched by 5.J3a" + no verdict anywhere | tree: crate admitted (=0.6.1 pin, feature wired) | Both true at different dates, but the record was never updated with the admission verdict its format requires — the audit chain for ALAC ends without a decision sentence |

## 4. Fabrication / inflation indicators
LOW for facts; one structural omission: the missing R10 verdict means a
reader cannot learn from this record whether ALAC was accepted — only the
Cargo.toml proves it was. If the admission decision lives only in a worklog
line, the record-first audit chain the dependency program promises is broken
for this crate.

## 5. Follow-ups for owners
- The record needs an R10 verdict paragraph (pass/accepted, pin, Lead signoff
  reference) matching its siblings; the field renumbering should be reverted
  to the doc 11 §2.1 mapping (or doc 11 updated — one of the two, not both).
- Same family convention check owed for symphonia-vorbis-0.6.1.md and
  symphonia-mp3-0.6.1.md (not in my file list; their audits should verify
  they carry verdicts).
