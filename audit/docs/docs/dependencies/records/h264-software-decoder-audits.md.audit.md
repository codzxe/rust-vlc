# CLAIM AUDIT: docs/dependencies/records/h264-software-decoder-audits.md
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Claims extracted | 11 |
| REAL 3 · REAL-UNPROVEN 3 · SHALLOW 0 · STUB 0 · PAPER-ONLY 0 · FALSE 1 (stale-snapshot) · UNREACHABLE 0 · UNVERIFIED-offline 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A serious, honest candidate-evidence record: it separates upstream claims from in-house evidence everywhere ("self-validated by the upstream author ... until our own gate runs"), rejects openh264 on forbidden-bindings grounds, and ends in a PROPOSAL rather than an admission. Its one big defect: it is a 2026-11-09 snapshot whose headline ("no decoder crate is admitted and none is a workspace dependency") is FALSE on the current tree — rust_h264 0.6.1 was admitted (default-on feature `codec-h264-sw`, exact pin, in the root Cargo.lock), so the header now misleads anyone who reads only it. Every tarball-level claim (licenses, unsafe counts, conformance numbers) is unverifiable from this offline sandbox.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "no decoder crate is admitted and none is a workspace dependency" | :7-8 | FALSE-on-current-tree (REAL at 2026-11-09) | root Cargo.lock:769-772 `name = "rust_h264" version = "0.6.1"` (registry+checksum); crates/codecs/Cargo.toml:174 `rust_h264 = { version = "=0.6.1", optional = true }`; feature `codec-h264-sw` is in `default` (Cargo.toml:53) |
| 2 | "this record ends in a `[PROPOSAL]`, not an allowlist row" | :11-12 | REAL-at-capture / stale | Verdict section :204 "**NOT admitted** (Lead-gated; nothing changed in the workspace)" — but admission has since landed (rust_h264-0.6.1.md record exists in same dir; feature-matrix.toml note: "2026-11-15: the software track landed") |
| 3 | rust_h264 0.6.1 "**zero deps** (default `std`)" | :19 | REAL | root Cargo.lock rust_h264 entry has NO `dependencies` key (read :767-771) — zero-dependency graph confirmed by lockfile |
| 4 | "no `build.rs` at all — cleanest possible posture" (rust_h264) | :89 | UNVERIFIED (needs extracted 0.6.1 tarball; no registry cache in sandbox) | — |
| 5 | "3 `unsafe impl` + 26 `unsafe fn` + 31 blocks, all in `simd/`, `once.rs`, `picture_sync.rs`" | :19, :102-110 | UNVERIFIED (same reason) | codecs/Cargo.toml comment :168-170 repeats it ("re-verified against the 0.6.1 tarball") — but that is a claim, not proof |
| 6 | openh264 0.9.9 "**REJECT** (MP §9 forbidden-bindings)" | :21, :95-100 | REAL (outcome honored) | root Cargo.lock has NO openh264 entry (grep 'openh264' -> no package entry; only an allowlist comment mentions it) — rejection stuck |
| 7 | rusty_h264 0.17.2 "viable fallback" | :20 | REAL (not admitted, as stated) | root Cargo.lock has NO rusty_h264 entry (grep -> none) |
| 8 | "Admission is conformance-gated, not claim-gated" with 4-part gate (i-iv) | :214-223 | REAL-UNPROVEN | the admission record docs/dependencies/records/rust_h264-0.6.1.md exists (audited separately this wave); whether gate (i)-(iv) actually ran is checked there |
| 9 | "`va` 0.0.1 ... Vapory blockchain smart-contract language stub" | :23, :43-46 | UNVERIFIED (offline) | — |
| 10 | Toolchain "cargo 1.99.1"; sources in `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f` | :9-10 | UNVERIFIED | no cargo on auditor PATH (exit 127); no registry cache visible |
| 11 | "no decoder ran on real media in this window — all conformance numbers in R5 are quoted upstream claims" | :234-237 | REAL (honest self-labeling) | R5 header :120 "(crate-stated; NOT yet our evidence)" |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "no decoder crate is admitted and none is a workspace dependency" (:7-8) | crates/codecs/Cargo.toml:174 `rust_h264 = { version = "=0.6.1", optional = true }` + default feature list :53 | codecs/Cargo.toml + Cargo.lock:769 — admission landed after this record; the doc is a pre-admission snapshot without a superseded-banner |
| "Admission is Lead-gated ... ends in a [PROPOSAL]" (:11-12, :204) | codecs/Cargo.toml:160-162 "the Lead-admitted ADR-001 software track" | Admission happened (Lead sign-off recorded in the landed comment chain) |

## 4. Fabrication / inflation indicators
None. The doc systematically DOWNGRADES its own evidence ("crate-stated; NOT yet our evidence", "quoted upstream claims") — the opposite of inflation. The stale-header problem is an omission (no "SUPERSEDED: admitted 2026-11-15, see rust_h264-0.6.1.md" banner), not a fabrication.

## 5. Matrix rows
Not a matrix chunk. Related row: feature-matrix.toml `codec-video-h264` note cites this record for "[PROPOSAL, Lead-gated]" and separately records the 2026-11-15 software-track landing — the matrix is MORE current than this record; consistent chain of custody (auditor's cross-check, no contradiction added).
