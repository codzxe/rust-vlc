# AUDIT: LICENSE
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-23 |
| Lines / functions | 21 / n-a (license text) |
| File verdict | REAL |
| Reachable from shipped entry point? | N-A — legal artifact shipped with every distribution; referenced by docs/adr/ADR-006-license-mit.md ("The repository is licensed under the **MIT License** (see `LICENSE`)"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 canonical-MIT phrase check -> all 4 key phrases present, 21 lines; `rg '^license' crates/*/Cargo.toml apps/*/Cargo.toml tools/*/Cargo.toml` -> zero members declare a license; python3 SPDX cross-check of 119 lockfile registry crates' license expressions vs deny.toml's allow set (105 verified in offline registry cache, all covered; 14 UNVERIFIED — not in cache) |

## 1. One-paragraph verdict
The file is the canonical MIT License text, verbatim (all standard phrases verified programmatically), copyright "2026 Lumen Media Player Contributors" — exactly the copyright line ADR-006 (accepted 2026-10-19, user decision) prescribes. Dependency-license compatibility with an MIT project holds: every one of the 12 SPDX classes in deny.toml's allow list is permissive or weak-copyleft-compatible with MIT distribution (MIT, Apache-2.0, Unicode-3.0, Unlicense, ISC, BSD-3-Clause, CDLA-Permissive-2.0, MPL-2.0, BSD-2-Clause, CC0-1.0, Zlib, Apache-2.0 WITH LLVM-exception); GPL/LGPL are deliberately absent from the set, so no copyleft conflict with the root license is possible through the workspace tree. I verified the allow set against the actual lockfile: 105 of 119 registry crates' license expressions are covered (offline registry-cache manifests); the 14 unverifiable ones are windows/wasi target crates never downloaded in this Linux sandbox. The defects are hygiene: ADR-006 decided MIT on 2026-10-19, yet deny.toml still says the license posture is undecided "until ADR-006", and not a single crate, app, standalone tool, or ui/package.json carries a license field — the MIT grant exists only at the repo root.

## 2. What actually works (verified)
- python3 canonical-MIT check on LICENSE: "Permission is hereby granted, free of charge", "THE SOFTWARE IS PROVIDED", "WITHOUT WARRANTY OF ANY KIND", "AUTHORS OR COPYRIGHT HOLDERS" all present — standard MIT, no alterations.
- LICENSE <-> ADR-006 agreement: ADR-006-license-mit.md:20-22 quotes this file and the exact copyright line.
- deny.toml allow set vs lockfile: python3 parsed license expressions of 105/119 registry crates from ~/.cargo/registry/src manifests — every expression (including "Unlicense OR MIT", "(MIT OR Apache-2.0) AND Unicode-3.0", "Apache-2.0 WITH LLVM-exception") decomposes to classes inside the allow set. Result: "all-license-covered" for the 105.
- sources policy consistent with the lockfile: deny.toml:404-407 allows only the crates.io registry, and Cargo.lock's 119 sources are all `registry+https://github.com/rust-lang/crates.io-index` (verified in the sbom audit).

## 3. Findings
F-001 [S4]
Where: LICENSE:1-3 (vs all member manifests)
Code:
```
MIT License

Copyright (c) 2026 Lumen Media Player Contributors
```
What is wrong: The MIT grant exists only at the repo root. Zero of 39 workspace members, the standalone tools (bench-vs-vlc, tools/vta/*), apps/desktop-tauri, or ui/package.json declare `license` (rg verified). deny.toml's `[licenses.private] ignore = true` skips members precisely because they "carry no license field". Why it matters: anyone consuming an individual crate, the npm UI package, or a vendored copy inherits ambiguity the root file resolves only by convention; SPDX tooling finds no per-component license. What correct looks like: `license = "MIT"` in every member manifest now that ADR-006 exists (deny.toml's skip was justified only while the decision was open).
Confidence: HIGH — Verification: ran (rg + python3).
F-002 [S4]
Where: deny.toml:28-30 and 144-148 (stale justification)
Code:
```
# `licenses.private.ignore = true` skips the license check for our
# publish=false workspace members (they carry no license field yet;
# their license posture is decided with ADR-006, doc 11 §8.2).
```
What is wrong: ADR-006 was accepted 2026-10-19 — before these comments' framing — and decided MIT; doc 11 §8.2's open question is explicitly superseded (ADR-006 header: "Supersedes: open question in docs/11-... §8.2; risk R-05"). The comment presents as pending a decision that has been made for a month. Why it matters: config comments that contradict accepted ADRs mislead the next dependency reviewer. What correct looks like: "skipped because members are publish=false; repo license is MIT per ADR-006".
Confidence: HIGH — Verification: read both files.
F-003 [S4]
Where: deny.toml:73 (vs Cargo.lock) and docs/adr/ADR-006-license-mit.md:33-35
Code:
```
[licenses]
# Minimal allow set for the current lockfile (22 registry crates):
```
What is wrong: Two wording drifts: (a) "22 registry crates" — the lockfile now has 119; the allow set grew through three later admission blocks (ureq, symphonia/rav1d, tauri) recorded below the stale header; (b) ADR-006 calls "only permissive-licensed dependencies merge" permanent baseline policy while the allow set includes MPL-2.0 (symphonia family) — weak copyleft, compatible with MIT distribution but not "permissive" in the strict sense; MPL-2.0-covered files carry file-level source-availability obligations into distributions. Neither breaks compatibility; both are comment-vs-reality drift in the license chain. What correct looks like: drop the hard-coded count; note MPL-2.0's obligation class in ADR-006's consequences.
Confidence: HIGH — Verification: read + lockfile count (119, verified in sbom audit) + allow-list read.

## 4. Per-function table
N-A (license text; no code). Sections verified: title, copyright line, permission grant, notice condition, warranty disclaimer, liability clause — all present and canonical.

## 5. Test honesty
N-A — no tests. The enforceable analog, cargo-deny, is wired in CI (ci.yml:527 `cargo deny --workspace check licenses sources bans advisories`, cargo-deny 0.20.2 SHA-pinned at ci.yml:513-521) but no cargo-deny run exists in the orchestrator baseline — its current pass/fail state against the 2026-11 lockfile is UNVERIFIED in this sandbox (cargo-deny not installed, no network). My offline manifest check (105/119) is the strongest local evidence that the license gate would pass.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The repository is licensed under the MIT License (see LICENSE)" | ADR-006:20 | REAL | LICENSE is canonical MIT (python3 check) |
| Copyright line "Lumen Media Player Contributors" | ADR-006:21-22 | REAL | LICENSE:3 verbatim |
| "only permissive-licensed dependencies merge" | ADR-006:33-35 | MOSTLY REAL | 105/119 verified permissive-or-compatible; MPL-2.0 is weak copyleft (F-003); 14 windows/wasi crates UNVERIFIED |
| "GPL/LGPL deliberately absent until ADR-006 decides" | deny.toml:82 | STALE | ADR-006 decided (MIT) 2026-10-19; GPL remains absent — the posture is decided, not pending |
| workspace members "carry no license field yet" | deny.toml:29, 146 | REAL | rg: zero license fields in member manifests |

## 7. Dead / duplicated / unreachable code
N-A.

## 8. Dependencies & flags
N-A for the file itself. Context: deny.toml bans 100+ GStreamer/gst-plugin/FFmpeg/libVLC crates (the clean-room rule — VLC is external oracle only), with a scoped libloading wrapper exception (libappindicator-sys, ADR-004); none of the banned names appear in Cargo.lock (the lock's 119 sources are all crates.io-index per the sbom-audit source scan).

## 9. Open questions for the next agent
- The 14 UNVERIFIED windows/wasi/redox/r-efi crates (target-gated, never downloaded here): confirm via CI's cargo-deny job log that their license classes are inside the allow set. UNVERIFIED (reason: no cargo-deny binary, no network, crates absent from the offline registry cache).
- tools/vta/* and bench-vs-vlc have their own lockfiles pulling oxideav-ac3/oxideav-mpeg4video — do those standalone trees' licenses pass any check at all? (Same coverage hole as the boundaries gate; nothing scans standalone-workspace lockfiles.)
- ui/ (npm) has no license field in package.json while shipping under the root MIT — confirm the release packaging (release.yml) attaches the LICENSE to npm-built artifacts.
