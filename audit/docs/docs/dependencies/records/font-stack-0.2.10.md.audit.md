# CLAIM AUDIT: docs/dependencies/records/font-stack-0.2.10.md
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Claims extracted | 9 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 / 0 / 1 / 0 / 0 / 1 / 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A proposal-only record, and its non-admission contract holds at HEAD: none of
swash/skrifa/zeno/yazi/read-fonts (nor the rejected ttf-parser/rustybuzz/
ab_glyph family) appear in Cargo.lock, the only in-tree mention is a
`[PROPOSAL]` doc comment (pipeline.rs:228), and the promised seam exists and
is real code (`FontStack` at font/mod.rs:408, `GlyphPipeline` trait at
font/pipeline.rs:230, `MonospaceCellPipeline` placeholder). One claim is FALSE
against the current tree: R10 blocker (a) names
`ui/src/assets/fonts/nunito-latin-var.woff2` as "the only font file" — the
tree carries `InterVariable.woff2` (the Nunito purge landed after the record's
2026-11-09 survey; onboarding.css:27 "Nunito is fully retired"). The blocker's
substance (a WOFF2 container needing Brotli, no raw TTF/OTF in-repo) still
holds at HEAD — verified by file scan — but the cited filename rotted. The
"already-allowlisted bytemuck" claim is true for deny.toml's machine allow set
(Zlib+MIT+Apache-2.0 all in licenses.allow) but bytemuck has NO row in the
human allowlist.md — loose wording, not a lie.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Nothing in this record is admitted: no Cargo.toml row ... no code path imports any crate below" | :7-9 | REAL | rg swash/skrifa/zeno/yazi/read-fonts in Cargo.lock -> 0; in crates/ -> only pipeline.rs:228 doc comment |
| 2 | "The in-tree deliverable is the SEAM ... FontStack/FontProvider + the GlyphPipeline trait with the MonospaceCellPipeline placeholder" | :9-11 | REAL | crates/subtitles/src/font/mod.rs:408 `pub struct FontStack`, :66 MonospaceCellPipeline; pipeline.rs:230 `pub trait GlyphPipeline` |
| 3 | "the only font file is `ui/src/assets/fonts/nunito-latin-var.woff2`, OFL 1.1" | :187-189 | FALSE (at HEAD) | tree has InterVariable.woff2 only (OFL 1.1, Inter Project Authors); Nunito retired (ui/src/screens/onboarding/onboarding.css:27 "Nunito is fully retired", MATRIX-20261113.md:32 purge) |
| 4 | "no default TTF/OTF face exists in-repo" | :187-188 | REAL | file scan: only woff2 present, zero .ttf/.otf |
| 5 | "the already-allowlisted `bytemuck`" | :76-77 | SHALLOW | bytemuck in Cargo.lock (1.25.2, :69) and its licenses (MIT/Apache/Zlib) all in deny.toml licenses.allow (:86,140); but allowlist.md has NO bytemuck row (rg -> 0 hits) — "allowlisted" is true only for the machine set |
| 6 | R2 graph: "swash 0.2.10 → skrifa ≤0.44 → 0.44.0 ... zeno ... yazi ... five new nodes" | :63-77 | UNVERIFIED | no network; graph never landed (nothing in lock) so no local corroboration possible |
| 7 | R8 maintenance: RUSTSEC-2026-0192 ttf-parser unmaintained, RUSTSEC-2026-0206 rustybuzz unmaintained, swash/skrifa 2026 releases | :129-146 | UNVERIFIED | no network; advisory IDs not re-queryable |
| 8 | R9 sha256 table (six crates) | :163-170 | UNVERIFIED | no network/tarballs; no local artifacts |
| 9 | "Every API fetch, OSV query, tarball download and scan above was executed in the build sandbox on 2026-11-09" | :195-197 | UNVERIFIED | scratch /tmp/font-audit not committed (by design); cannot re-run |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "the only font file is nunito-latin-var.woff2" (:187-188) | tree: InterVariable.woff2 bundled 2026-11-13 (fonts.css header); Nunito retired | The record was true on 2026-11-09; the k-wave font purge replaced it. Substance (WOFF2-only, no TTF/OTF) still true |
| "already-allowlisted bytemuck" (:76-77) | allowlist.md has no bytemuck row | deny.toml (machine) covers it; allowlist.md (human) does not — the allowlist doc is stale w.r.t. the lockfile (bytemuck, rav1d, symphonia rows missing — allowlist.md:55 admits codec admissions are pending rows) |

## 4. Fabrication / inflation indicators
- LOW. Non-admission posture verified; [VERIFY-P1] markers on every
  future-dependent claim; the record refuses capability claims beyond crate
  documentation fields (:199-200).
- The one rot: the nunito filename (S4-level drift, dated-record hazard — the
  Lead re-running this record per its own R10 instruction would immediately
  hit it and fix it).

## 5. Follow-ups for owners
- allowlist.md is missing rows for crates already in the lockfile (bytemuck,
  the phase-4–6 codec admissions it lists at :55 as pending) — that file's
  own audit should weigh it.
- The seam (crates/subtitles/src/font/) contains real, working code — the
  placeholder pipeline claim is TRUE, and `MonospaceCellPipeline` is the
  no-new-deps implementation (verified in pipeline.rs:15,226 doc comments).
