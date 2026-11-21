# AUDIT: LICENSE
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-21 |
| Lines / functions | 21 / 1 license grant |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (payload wiring: scripts/release/package-deb.sh:71,86,89 copies LICENSE into `/usr/share/doc/<slug>/`; build-rpm.sh:61 `PACKAGING_RPM_LICENSE` mirrors it via packaging-env.py:310; baseline #17 packaging gate PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 byte-diff vs canonical MIT text (IDENTICAL); `rg -n "license"` root Cargo.toml + ui/package.json (absent); `rg -n "GPL\|LGPL\|AGPL\|MPL\|Apache"` over manifests; python3 scan of tools/*/Cargo.lock, tools/vta/*/Cargo.lock, fuzz/Cargo.lock (copyleft heuristics); sed -n quote verifications. |

## 1. One-paragraph verdict
The file is byte-identical to the canonical MIT License text (verified by diff against
the standard template with year 2026, holder "Lumen Media Player Contributors") — no
tampered clauses, no added restrictions. Dependency licensing in the main workspace is
permissive-only and machine-checked (cargo-deny licenses PASS, baseline #25; deny.toml
allowlist: MIT, Apache-2.0, BSD-2/3-Clause, Unicode-3.0, Unlicense, MPL-2.0, CC0 —
no GPL anywhere). Three hygiene defects: no crate manifest carries a license field
(pending ADR-006, admitted in deny.toml); the copyright-holder name drifts from
product.toml's publisher string; and three separate lockfile workspaces (fuzz/,
tools/vta/*, tools/bench-vs-vlc) sit outside cargo-deny's license coverage entirely.

## 2. What actually works (verified)
- Byte-identity: python3 comparison of LICENSE against the canonical MIT text
  (holder/year substituted) → `IDENTICAL`, single trailing newline. Real MIT, real
  grant.
- Shipped in payloads: package-deb.sh:71 `for f in "$cli_bin" "$desktop_bin" LICENSE
  README.md; do` / :86 `cp LICENSE README.md "$docdir/"`; RPM path mirrors it
  (build-rpm.sh:24 comment "License <- PACKAGING_RPM_LICENSE (mirrors the repo
  LICENSE)"). Baseline #17 `bash scripts/packaging/gate.sh` PASS.
- Main-workspace license gate: baseline #25 `cargo deny check` — "bans ok, licenses
  ok, sources ok" (advisories failed for an unrelated config-drift reason).
- The one Apache-2.0 runtime dep (claxon, codecs FLAC) is documented with a purity
  record: crates/codecs/Cargo.toml:12-14 "purity-verified in
  docs/dependencies/records/claxon-0.4.3.md (F1-F7 all NO, zero normal dependencies,
  Apache-2.0, sha256-pinned; the historical RUSTSEC-2018-0004 advisory is outside the
  pinned range)" — allowed by deny.toml, license-compatible with MIT distribution.

## 3. Findings

### F-001 [S4] No crate declares a license; posture deferred since before the audit
Where: LICENSE:1-21 (cross-evidence: root Cargo.toml — no `license` key, no
`[workspace.package]`; deny.toml:28-30)
Code (the admission):
```
#   * `licenses.private.ignore = true` skips the license check for our
#     publish=false workspace members (they carry no license field yet;
#     their license posture is decided with ADR-006, doc 11 §8.2).
```
What is wrong: 40 workspace crates ship with no `license` field; the root Cargo.toml
defines neither `license` nor a `[workspace.package]` template. The MIT grant exists
only at repo-root file level. `rg -n "license" Cargo.toml` → empty.
Why it matters: SPDX/tooling cannot discover per-crate licensing; if ADR-006 ever
picks a different posture for some crate (e.g. a codec crate with patent concerns),
today's files give no anchor. Honest (documented in deny.toml), hence S4 not higher.
What correct looks like: `[workspace.package] license = "MIT"` + `license.workspace =
true` in every member, or land ADR-006.
Confidence: HIGH — Verification: read + rg.

### F-002 [S4] Copyright-holder name drifts between LICENSE and product identity
Where: LICENSE:3 vs product.toml:24
Code:
```
Copyright (c) 2026 Lumen Media Player Contributors
```
vs product.toml:24:
```
publisher = "Lumen Contributors"          # [PROPOSAL] placeholder entity
```
What is wrong: the legal grant names "Lumen Media Player Contributors"; the product
SSOT's publisher is "Lumen Contributors" (itself a tagged placeholder). Two different
entity strings for the same project.
Why it matters: About/installer metadata (from product.toml) will not match the
copyright holder users see in the installed LICENSE; lawyers and downstream packagers
hate that. Both are placeholders, so S4.
What correct looks like: one canonical entity string in product.toml, referenced by
the LICENSE (or vice versa) once ADR-006/identity work lands.
Confidence: HIGH — Verification: read both lines.

### F-003 [S4] Separate lockfile workspaces are outside any license check
Where: fuzz/Cargo.lock (108 pkgs, incl. `ring 0.17.14`), tools/vta/{dnx,ac3,ruopus,
vp9,mpeg4}/Cargo.lock (9-30 pkgs each), tools/bench-vs-vlc/Cargo.lock (77 pkgs)
What is wrong: cargo-deny runs against the ROOT workspace lock; these seven locks
belong to standalone workspaces (root Cargo.toml `members` excludes fuzz and tools/vta
and tools/bench-vs-vlc) and are never license-scanned. My python3 scan found no GPL
family crates in them (ring 0.17.14 is MIT/ISC-style permissive; heuristic scan of
known copyleft names: none), but nothing enforces that.
Why it matters: a future `ffmpeg-sys-next` or GPL crate added to a tool workspace
would land silently; the "permissive-only" property of the repo is unchecked exactly
where the code is least reviewed.
What correct looks like: a `cargo deny --manifest-path <each>/Cargo.toml check
licenses` step in CI (cheap), or fold the tools into the main workspace.
Confidence: HIGH — Verification: python3 lock scans + root Cargo.toml members read.

## 4. Per-function table (file structure)
| Part | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Title + copyright line | 1-3 | REAL | S4 | canonical; holder name drift F-002 |
| Grant paragraph | 5-10 | REAL | — | byte-identical to canonical MIT |
| Inclusion condition | 12-13 | REAL | — | canonical |
| Warranty disclaimer + limitation | 15-21 | REAL | — | canonical |

## 5. Test honesty
No tests exist for or reference the LICENSE text (correct — it is legal text). The
RPM/deb packaging gate (baseline #17 PASS) does verify the file is present in
payloads, which is the only executable check touching it.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Repo is MIT-licensed | LICENSE itself | REAL | byte-identity check |
| "licenses ok" (cargo-deny) | baseline INDEX.md #25 | REAL | cited run; allowlist read |
| crates "carry no license field yet" (ADR-006 pending) | deny.toml:28-30 | REAL | rg over manifests: no license keys |
| clean-room VLC reimplementation (product description, locales/en-US.toml:70) | repo docs | UNVERIFIED | cannot prove absence of VLC-derived code from this file; flag for docs/code auditors — any copied GPL VLC code would void this MIT file (see §9) |

## 7. Dead / duplicated / unreachable code
None (single license file, no duplicates; ui/ and tools/ have no second LICENSE —
ls/rg verified via packaging references only finding the root one).

## 8. Dependencies & flags
Not applicable. Related config: deny.toml licenses.allow (permissive set, read);
`licenses.private.ignore = true` for the unlicensed members (F-001).

## 9. Open questions for the next agent
- The MP4/codec crates (crates/codecs) implement H.264/HEVC/AV1 parsers "clean-room".
  VLC is GPLv2 — if ANY demuxer/codec logic was ported from VLC or ffmpeg sources,
  this MIT LICENSE is misrepresenting the whole repo. A targeted wave should diff
  suspicious functions against ffmpeg/VLC sources (out of my file's scope).
- Does the MSI/WXS path (gen-wxs.sh) also embed LICENSE (it warns against a
  "hardcoded LICENSE/README pair" — check packaging.toml's docs list)?
