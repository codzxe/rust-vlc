# CLAIM AUDIT: docs/dependencies/records/oxideav-mpeg4video-0.1.8.md
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Claims extracted | 18 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 6 REAL / 10 REAL-UNPROVEN / 0 / 0 / 0 / 0 / 2 UNREACHABLE (unadmitted) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This is the rare dependency record that is BETTER than its subject: the crate was measured, failed the differential twice (tolerance-class outputs on 7/8 lanes + duplicate-frame emission on packed bitstreams), and was BLOCKED — and the repo state PROVES the block is real (zero occurrences of `oxideav` in the root Cargo.toml/Cargo.lock; the only presence is the standalone VTA harness). The record's externally-checkable claims (pin shape, dependency graph, not-admitted status) all verify. The measurement claims (lane tables, ffmpeg differentials, panic battery) are REAL-UNPROVEN: the harness exists and is wired exactly as described, but re-running the differential requires cargo build + ffmpeg, both outside this audit's budget (cargo-build-budget; ffmpeg presence untested).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "NOT admitted this round" | L21 | REAL | `grep -rn oxideav Cargo.toml Cargo.lock` -> zero matches (ran) |
| 2 | "no Cargo.toml edit, no crates/codecs wiring" | L293-294 | REAL | same grep — nothing admitted anywhere in the workspace graph |
| 3 | "oxideav-mpeg4video = { version = "=0.1.8" }" candidate shape | L19-20 | REAL | tools/vta/mpeg4/Cargo.toml:13 `oxideav-mpeg4video = "=0.1.8"` (exact pin as described) |
| 4 | "oxideav-core ... the ONE dependency" | L48 | REAL | tools/vta/mpeg4/Cargo.lock: `oxideav-mpeg4video` deps list = `["oxideav-core", ...]` single external crate (read) |
| 5 | "harness at tools/vta/mpeg4, committed" | L43-44 | REAL | `ls tools/vta/mpeg4` -> Cargo.toml, Cargo.lock, README.md, diff.py, src/ (ran) |
| 6 | "8-lane differential ... 7/8 tolerance, 1x1 bit-exact" | L143-146, L203-212 | REAL-UNPROVEN | table internally consistent (37≠28 lane C matches R10 duplicate finding); harness committed; re-run needs cargo+ffmpeg — not re-executed here |
| 7 | "packed bitstream emits duplicate frames ... 37 output vs 28" | L154-156 | REAL-UNPROVEN | same — consistent with lane table C and footnote 1; no re-measurement |
| 8 | "ffmpeg -idct int|simple|xvid ... byte-identical" negative control | L146-148, L233-235 | REAL-UNPROVEN | no artifacts in-repo to check; plausible, unverifiable offline |
| 9 | "0 panics anywhere (18 case-runs + 3 truncate sweeps)" | L262-263 | REAL-UNPROVEN | §H battery described; harness src/ present; not executed |
| 10 | "0 unsafe tokens in the whole crate" | L35-37 | REAL-UNPROVEN | vendored source not in-repo (crate not admitted — nothing to grep) |
| 11 | "No build.rs (build = false)" | L33 | REAL-UNPROVEN | same |
| 12 | "MIT" license | L39, L99 | REAL-UNPROVEN | not verifiable offline |
| 13 | "tarball sha256 ... == index cksum byte-for-byte" | L128-130 | REAL-UNPROVEN | hash of an external download; offline |
| 14 | "OSV ... {} — zero advisories" | L116-119 | REAL-UNPROVEN | external API, offline; date-stamped 2026-10-26 |
| 15 | "13 versions in 5 months ... 2,349 downloads" | L112-114 | REAL-UNPROVEN | external registry facts |
| 16 | "our AVI demuxer's v0 codec filter ... FILTERS OUT mpeg4 tracks" | L190-195 | REAL | crates/demux AVI v0 filter is a known repo fact (baseline W-lane audits); consistent with "no-supported-streams" behavior; corroborated by other records |
| 17 | "two harness bugs were caught and fixed ... recorded" | L241-247 | REAL-UNPROVEN | no git access to confirm the fix history; the record of the bugs is itself the honesty artifact |
| 18 | "the crate IS effectively forbid(unsafe)-clean" | L36-37 | UNREACHABLE | moot: crate not admitted; nothing shipped uses it |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| — | — | none found; the addendum (L290-328) re-affirms the block rather than quietly reversing it |

## 4. Fabrication / inflation indicators
NONE. The record documents its own failure modes (two harness bugs at L241-247, the max-delta accumulator bug at L246-247), states unmeasured tools explicitly (§G), and the admission status matches the repo graph exactly. The BLOCKED verdict on the strongest-purity candidate is the opposite of inflation.

## 5. Matrix rows
N/A (not a matrix chunk).

## 6. Cross-checks run
- `grep -rn oxideav Cargo.toml Cargo.lock` -> empty (claim 1/2 REAL).
- `ls tools/vta/mpeg4` -> harness committed (claim 5 REAL).
- `grep oxideav tools/vta/mpeg4/Cargo.toml` -> `=0.1.8` exact pin (claim 3 REAL).
- tools/vta/mpeg4/Cargo.lock contains oxideav-mpeg4video + oxideav-core (claim 4 REAL).
- Cannot execute: cargo build (budget), ffmpeg differential, network (OSV/crates.io) — all marked REAL-UNPROVEN above, reason recorded.
