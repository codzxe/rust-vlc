# CLAIM AUDIT: docs/dependencies/records/truehd-0.7.2.md
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Claims extracted | 9 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 / 0 / 0 / 0 / 0 / 0 / 3 |
| Internal contradictions found | 0 (one cross-doc inconsistency with feature-matrix.toml noted) |

## 1. Blunt verdict
This is an honest BLOCKED record, and its central claim — admission blocked,
nothing shipped — is verified against the repo: `truehd`, `bitstream-io`,
`no_std_io`, `anyhow` are all absent from Cargo.lock and from codecs/Cargo.toml
(rg: zero hits), exactly as "no Cargo.toml edit, no module, no tests shipped"
demands. The naming-reality check is real (doc 04:547 says "truehdd crate
hardening" while doc 04:195 row spells `truehd` 0.7.2 — the discrepancy the
record documents exists verbatim). The unblock companion record exists and is
consistent. The measurement claims (bit-exact decode table, MD5s, OOM bisection,
crates.io facts, OSV queries) cannot be re-run from this sandbox (no network,
no tarball) — they are internally consistent, honestly posture-marked, and the
decode-quality prose never leaks into shipped code. Nothing inflated found.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "admission BLOCKED ... no `Cargo.toml` edit, no module, no tests shipped with this card" | :14-21 | REAL | `rg 'truehd|bitstream-io|no_std_io' Cargo.lock` -> 0 hits; `rg truehd crates/codecs/Cargo.toml Cargo.toml` -> 0 hits |
| 2 | "the brief, the wave plan and doc 04 §8.6 item 5 all say 'the `truehdd` crate' ... the library ... is `truehd` 0.7.2" | :8-11 | REAL | docs/04-codec-format-roadmap.md:547 "TrueHD: truehdd crate hardening (M)" vs :195 "`truehd` 0.7.2 [github.com/truehdd/truehdd]" — the mismatch is exactly as described |
| 3 | "doc 04 §3.1 row 'TrueHD (MLP)' spells this correctly" | :11-12 | REAL | doc 04:195 spells `truehd` 0.7.2 |
| 4 | "4 new registry packages would enter the lock (`truehd`, `anyhow`, `bitstream-io`, `no_std_io2`)" | :63-64 | REAL | none of the four in Cargo.lock today (rg 'name = "anyhow"' -> no match) — the graph never landed |
| 5 | "Apache-2.0 ... Already in deny.toml's allow set (claxon precedent)" | :143-147 | REAL | deny.toml:86 `"Apache-2.0",` in licenses.allow |
| 6 | "5.Q2-V backfill: blocked (pending-unblock) per record (5.K1)" (matrix row consistent with blocked verdict) | matrix:789 | REAL | docs/feature-matrix.toml:779-789 row codec-audio-truehd-mlp cites this record as blocked-with-unblock-path |
| 7 | crates.io/OSV facts: created 2025-07-23, 12 versions, 0.7.2 published 2026-09-15, ~3.4k downloads, no advisories; publisher saindriches | :149-168 | UNVERIFIED | no network in this sandbox; internal cross-refs (matrix, unblock record) repeat consistent dates |
| 8 | "four ffmpeg 7.1.5 authored streams bit-exactly ... 0 of 441 000 ... MD5 72aa2a86..." (R6 table) | :112-135 | UNVERIFIED | no fixtures/tarball in sandbox; MD5s not re-computable; claims marked as scratch measurements, consistent with blocked verdict |
| 9 | "rustc 1.99.1 is OOM-killed at ~3.4–3.6 GB anon-rss ... minimal repro ~20 lines ... root cause ToBits expansion" (R11) | :189-218 | UNVERIFIED | cannot compile (load discipline + no cargo on sandbox PATH); the bisection narrative is detailed and falsifiable but was not re-executed here |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "latest 0.7.2 published 2026-09-15" (library, R8) | feature-matrix.toml:789 "truehdd 0.6.2 (2026-09-15)" — CLI name+version with the library's publish date | The record's own naming check (:7-11) proves truehdd 0.6.2 ≠ the library; the matrix note mixes the two. Matrix row's STATUS (blocked) is still consistent; the name/date pairing in the note is wrong. Not this record's defect — flagged for the matrix owner. |

## 4. Fabrication / inflation indicators
- LOW. The blocked posture is corroborated by the repo state (nothing in the
  lockfile). The record explicitly refuses to claim more than measured
  ("what remains blocked is admission"), marks every future-check with
  [VERIFY-P1]/[VERIFY], and the honest gap (20 extra trailing zero samples)
  is recorded against itself. One number cross-check I COULD do (12 779 lines
  / 28 .rs) is tarball-relative — UNVERIFIED (no network).
- The evidence-posture footer (:234-242) claims "every statement above was
  executed locally" — unfalsifiable from here; consistent with everything
  checkable.

## 5. Reachability / blast radius
Record is docs-only (PAPER record of a NOT-shipped dependency); the blocked
verdict means zero shipped surface. The companion decision record
truehd-bitstream-io-unblock.md exists (:file present, head read) and cites this
record as parent — chain intact. worklog.md:354 confirms both records were
committed as part of the admissions wave.
