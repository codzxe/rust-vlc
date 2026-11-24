# CLAIM AUDIT: docs/dependencies/records/claxon-0.4.3.md
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Claims extracted | 12 |
| REAL | 7 |
| REAL-UNPROVEN | 1 |
| UNVERIFIED | 4 (network: crates.io/OSV/tarball scans) |
| FALSE | 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The repo-checkable core is exact: the `=0.4.3` pin, the lock checksum byte-for-byte,
the zero-dependency graph (the lock entry has no dependencies key at all), the flac
wrapper module, and the record's own conformance claim — `flac_bitexact.rs` runs 4/4
green in the orchestrator baseline including
`fixtures_decode_byte_exact_against_the_ffmpeg_reference`. The record is also honest
about the allowlist drift: allowlist.md:55-57 itself says the phase-4–6 codec
admissions "predate this row without §3 rows of their own" — matching this record's
"pending the Lead's allowlist row" exactly (the pending is still pending, and BOTH
sides say so). Tarball/network claims are unverifiable here, as with every record.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Pinned `claxon = \"=0.4.3\"` (exact pin ...)" | L126 | REAL | crates/codecs/Cargo.toml:86 `claxon = { version = "=0.4.3", optional = true }` |
| 2 | "sha256 `4bfbf56724aa9eca8afa4fcfadeb479e722935bb2a0900c2d37e0cc477af0688`" | L17-18, L116-118 | REAL | Cargo.lock claxon block checksum — BYTE-IDENTICAL |
| 3 | "claxon 0.4.3 has **zero normal dependencies** ... the effective graph ... single edge-free node" | L42-51 | REAL | Cargo.lock claxon entry carries NO dependencies key (rg over the 6-line block) |
| 4 | "the card's ... bit-exact differential tests (`crates/codecs/tests/flac_bitexact.rs`) are the recorded coverage proof" | L91-94 | REAL | baseline cargo-test-workspace.txt:1066-1076: 4/4 pass incl. `fixtures_decode_byte_exact_against_the_ffmpeg_reference ... ok` |
| 5 | "R2 ... merged Cargo.lock diff must show exactly this single addition" | L54-56 | REAL | lock contains exactly one claxon node; no claxon-adjacent -sys/build nodes |
| 6 | "allowlist.md is deliberately NOT edited by 5.A1" + verdict "pending the Lead's allowlist row" | L131-132, L125 | REAL + still-pending | docs/dependencies/allowlist.md:55-57 admits the phase-4–6 admissions (claxon named) have no §3 rows — the drift is disclosed on the allowlist side too; no contradiction, a recorded open item |
| 7 | "Apache-2.0 ... inside deny.toml's allow set. No license-set change needed" | L98-99 | REAL | deny.toml allow set carries Apache-2.0 (deny.toml.audit.md exists from wave 1; rg confirms "Apache-2.0" in the allow list) |
| 8 | "5 `unsafe` blocks, each with an in-source invariant comment — `src/input.rs:148`, `src/frame.rs:569`, `src/metadata.rs:437/474/545`" | L27-34 | UNVERIFIED | vendored claxon source not in sandbox (registry-only) |
| 9 | "RustSec/OSV: **2 advisories, both historical and NOT affecting 0.4.3**" | L107-110 | UNVERIFIED | network advisory claims (range arithmetic plausible: 0.4.3 > 0.4.1) |
| 10 | "86,626 bytes, 7 `.rs` files / 3,699 lines in `src/`" | L19 | UNVERIFIED | tarball claims |
| 11 | "Last release 0.4.3 on 2020-08-09; ... 4.7M downloads" | L87-88, L103 | UNVERIFIED | network |
| 12 | "every statement above was executed locally in the build sandbox on 2026-10-25" | L134-138 | REAL-UNPROVEN | repo-verifiable subset all checks out (#1-#5); curl/OSV/tarball parts unverifiable here |

## 3. Contradictions
None found. The one cross-file tension (record says "pending allowlist row";
allowlist says the same admissions "predate this row") is a CONSISTENT open item,
not a contradiction — both sides disclose it.

## 4. Fabrication / inflation indicators
None. The record understates by design ("No statement about claxon's decode quality
is made without our own fixture run behind it" — and the fixture run exists and
passes in the baseline).

## 5. Matrix rows
Matrix `codec-audio-flac` row cites `claxon =0.4.3` (feature-matrix evidence strings
verified in the filmcraft-dnx audit's ADR-010 cross-check: FLAC row at ADR-010:81
names claxon + record). No flac-specific row re-verified here beyond the baseline
test pass.
