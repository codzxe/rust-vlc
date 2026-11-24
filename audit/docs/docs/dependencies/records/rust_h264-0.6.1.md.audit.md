# CLAIM AUDIT: docs/dependencies/records/rust_h264-0.6.1.md
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Claims extracted | 10 |
| REAL 7 · REAL-UNPROVEN 0 · SHALLOW 0 · STUB 0 · PAPER-ONLY 0 · FALSE 0 · UNREACHABLE 0 · UNVERIFIED-offline 3 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The admission record's every tree-checkable hook is real: the exact pin is in the manifest, the graph is verifiably one node in the lock, the differential test file exists with the six pinned MD5s, the typed refusal strings exist in the wrapper, the boundary allowlist carries the crate, the CLI conformance lane test exists — and the orchestrator baseline EXECUTED the differential suite on the current tree: 17/17 green (both entry forms × six lanes + hostile battery). The only unverifiable material is tarball-level (purity counts, license files, unsafe inventory) — offline sandbox.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`rust_h264 = { version = \"=0.6.1\" }`, default features (`std` only)" behind `codec-h264-sw` | :16-20, :127-129 | REAL | crates/codecs/Cargo.toml:174 `rust_h264 = { version = "=0.6.1", optional = true }`; feature `codec-h264-sw = ["dep:rust_h264"]` (:73) in `default` (:53) |
| 2 | "the pinned graph resolves to exactly one node ... ZERO dependencies (default `std`)" | :20, :46 | REAL | root Cargo.lock rust_h264 entry (no `dependencies` key; registry+checksum 49a852e1...056a9) |
| 3 | "no `build.rs` ... 0 `extern \"C\"`" | :28 | UNVERIFIED-offline (tarball claim; no registry cache in sandbox) | — |
| 4 | unsafe inventory "3 `unsafe impl` + 26 `unsafe fn` + 31 blocks, all `pub(crate)`-internal ... None reachable from the public API" | :32-41 | UNVERIFIED-offline (tarball claim; mirrors the h264-software-decoder-audits.md R5 numbers) | — |
| 5 | §D differential: six libx264 lanes × two entry forms, MD5s pinned (`a0a9144b...`, `19520a80...`, `532c5717...`, `412bd350...`, `d0fc84c9...`, `810d6f44...`) | :62-81 | REAL (executed by orchestrator baseline) | crates/codecs/tests/h264_sw_differential.rs exists; all six MD5 constants present (grep counts); baseline cargo-test-workspace.txt:1161-1182: `Running tests/h264_sw_differential.rs ... running 17 tests ... test result: ok. 17 passed; 0 failed` — incl. lane_a..f annexb+mp4, hostile_garbage_then_real_recovers, hostile_truncation_sweep_is_panic_free |
| 6 | "4:4:4, 4:2:2, 10-bit ... refuse typed `h264.sw.surface_format`; SP/SI slices and FMO refuse typed (`h264.sw.unsupported`)" | :108-117 | REAL | crates/codecs/src/video/h264/sw.rs:373 `return Err(Self::unsupported("h264.sw.surface_format"));` and :305/:533 `"h264.sw.unsupported"` |
| 7 | "`lumen open tests/fixtures/generated/valid/mp4_h264_aac.mp4` plays ... exit 0 — the `mp4_h264_family_lane` conformance row pins it" | :92-95 | REAL (test exists; run claim is session-level) | fixture exists (ls -> present); apps/cli/tests/container_conformance.rs:426 `fn mp4_h264_family_lane()` |
| 8 | "`boundaries.toml` `[crates.codecs] external_allowlist`: `rust_h264` appended" | :130-131 | REAL | boundaries.toml:330 `external_allowlist = [... "rusty_vp9", "filmcraft-dnx", "rust_h264"]` |
| 9 | "`deny.toml`: no change (MIT OR Apache-2.0, already-allowed expressions)" | :132-133 | REAL | deny.toml:85-88 allow set includes MIT + Apache-2.0 |
| 10 | "Admission ... on the Lead's explicit 2026-11-15 instruction" | :4-5 | UNVERIFIED (no commit history access; consistent with feature-matrix.toml note "2026-11-15: the software track landed") | — |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| (none found) | — | The record is internally consistent AND consistent with h264-software-decoder-audits.md (candidate half) + codecs/Cargo.toml comment chain (:160-174) + feature-matrix.toml software-row note |

## 4. Fabrication / inflation indicators
None. The record keeps the honest split: candidate-audit record vs admission record, tarball claims marked as re-verified-this-session only where they were, and the profile coverage section (:108-117) explicitly lists what the wrapper REFUSES rather than claiming full crate coverage. The §D MD5 table is pinned and machine-checked (baseline 17/17).

## 5. Matrix rows
Not a matrix chunk. Related row: `codec-video-h264-software` (feature-matrix.toml) — its note cites this record; the software-track landing (2026-11-15) matches the admission date in :5.
