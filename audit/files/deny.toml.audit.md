# AUDIT: deny.toml
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-23 |
| Lines / functions (config keys) | 407 / 5 sections, 12 allow licenses, 174 deny entries, 2 advisory ignores |
| File verdict | REAL (config coherent and cross-checked against both lockfiles; comments carry stale/paper claims — see findings) |
| Reachable from shipped entry point? | NO (dev/CI config; consumed by `.github/workflows/ci.yml` cargo-deny job + tauri admission lane; not part of any shipped binary) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | python3 static license/ban cross-check over Cargo.lock + apps/desktop-tauri/Cargo.lock vs registry cache (~/.cargo/registry/src) → 119 + 472 registry crates; `rg`/`od -c`/`sed` quote verification (incl. byte-level check of ci.yml `branches: [main]` and deny.toml:36); registry-cache reads of tauri-2.12.5/symphonia-*/unicode-ident/memchr manifests. cargo-deny NOT run (baseline: tool missing — every pass/fail behavior UNVERIFIED, audit is static). |

## 1. One-paragraph verdict
This is a genuinely maintained, reality-matched policy file, not decoration: every claim I could check statically against the two lockfiles holds — zero banned crates present (173 bare names + the libloading wrapper checked against 119 root + 472 tauri registry crates), zero git sources, zero GPL-family licenses in either tree, both advisory ignores name crates that are locked exactly as described (paste 1.0.15 via rav1d 1.1.0 in the root lock; proc-macro-error 1.0.4 via glib-macros 0.18.5 in the tauri lock only), the libloading wrapper allow matches the actual lock edge (libloading 0.7.4's sole dependent is libappindicator-sys), tray-icon is confirmed NOT a tauri 2.12.5 default feature, the "all 101 gst-plugin crates" count is exact (101 quoted entries counted), and spot-checked license attributions (symphonia ×7 = MPL-2.0, unicode-ident 1.0.27, memchr 2.8.3, ring/webpki-roots/subtle/ureq/to_method/foldhash/target-lexicon versions) are all verbatim correct. The CI wiring exists and matches the file's claims (ci.yml:490-520: cargo-deny 0.20.2, SHA-256-pinned, exact command `cargo deny --workspace check licenses sources bans advisories`). The defects are in the comments' honesty margin: the "only two expected warnings" wildcard claim is contradicted by the repo's own recorded sweep (29 warnings across 25 crates), two of the three claimed defense-in-depth layers (F1–F7 tarball scanner, tree-diff gate) do not exist in the repo, and the tauri admission tree — the largest dependency tree — is never `sources`-checked because its lane omits that check. GPL risk: none found; GPL/LGPL is deliberately absent from the allow list pending ADR-006, and no locked crate needs it.

## 2. What actually works (verified)
- Ban list vs both lockfiles (python3 static cross-check): banned crates present = NONE in root lock and NONE in tauri lock; `libappindicator-sys` flagged by my first pass was the ALLOWED wrapper (regex artifact), confirmed the deny entry is `{ name = "libloading", wrappers = ["libappindicator-sys"] }`.
- Sources policy vs both lockfiles: `source = "git+..."` → none in either lockfile; every source is the crates.io registry.
- GPL scan (registry-cache license fields, cache-resolvable crates): NONE in either lockfile.
- Advisory ignores vs locks: `paste 1.0.15` + `rav1d 1.1.0` in root lock ✓; `proc-macro-error 1.0.4` + `glib-macros 0.18.5` present ONLY in apps/desktop-tauri/Cargo.lock ✓ (root lock has neither — the "main workspace tree never sees it" comment is true).
- libloading edge: in apps/desktop-tauri/Cargo.lock the only package depending on `libloading` (0.7.4) is `libappindicator-sys` ✓; tauri-2.12.5 registry manifest: `default = ["wry","compression","common-controls-v6","dynamic-acl","x11","dbus"]` — tray-icon NOT default ✓; no `trayIcon` config in the repo ✓.
- License attributions: symphonia-{bundle-mp3,codec-aac,codec-alac,codec-vorbis,common,core,metadata} 0.6.1 all `license = "MPL-2.0"` ✓; `unicode-ident-1.0.27: "(MIT OR Apache-2.0) AND Unicode-3.0"` ✓; `memchr-2.8.3: "Unlicense OR MIT"` ✓; versions ring 0.17.14 / webpki-roots 1.0.11 / subtle 2.6.1 / ureq 3.5.0 / to_method 1.1.0 / foldhash 0.2.0 (tauri) + 0.1.5 (root) / target-lexicon 0.12.16 (tauri) all match the comment claims ✓.
- gst-plugin enumeration: exactly 101 quoted `gst-plugin*` entries (python3 count) — the comment's "all 101 crates below" is exact.
- Workspace members: all 39 member manifests carry `publish = false` (python3 scan) → `[licenses.private] ignore = true` is coherent.
- CI wiring: ci.yml:490-520 defines the cargo-deny job (0.20.2, sha256 `9f12ed4c…`, `cargo deny --workspace check licenses sources bans advisories` = deny.toml:6 verbatim); workflow triggers byte-verified as `branches: [main]` on push+pull_request (od -c; the `ain]` rendering is a viewer artifact — the in-file comment's explanation is correct).
- In-repo execution evidence (recorded, not my run): docs/security/sweep-supply-chain.md:26-63 records a cargo-deny 0.20.2 run at commit 784c6e6: `advisories ok, bans ok, licenses ok, sources ok`, zero errors, single advisory hit = the documented RUSTSEC-2024-0436 acceptance.

## 3. Findings
F-001 [S3]
Where: deny.toml:160-163
Code:
```
— the only two expected
warnings are the two workspace path edges (codec-api→media-types,
```
What is wrong: the claim "the only two expected warnings are the two workspace path edges" is false and is contradicted by the repo's own recorded sweep evidence: docs/security/sweep-supply-chain.md:38-46 counts "29 `wildcard` warnings across 25 crates" and explicitly states "deny.toml's bans comment still says 'the only two expected warnings are the two workspace path edges' — that count has rotted (25 members carry path deps now)". The protected invariant (every wildcard = intra-workspace path edge, zero registry `*` requirements) held in that run, but the tripwire as written ("any THIRD wildcard warning is a review stopper") would fire on every legitimate run today, training reviewers to ignore it. Why it matters: a rotten tripwire is worse than no tripwire. What correct looks like: the sweep doc's own proposal — reword to "every wildcard warning must name a workspace member; a registry name is a review stopper".
Confidence: HIGH — Verification: read (both files) + sweep doc's machine-verified count.

F-002 [S3]
Where: deny.toml:32-37 and 163-164
Code:
```
# — it checks manifests, licenses, advisories and sources. The native-code
# ban is therefore enforced in depth: this named-crates list (what
# cargo-deny CAN see) + the F1–F7 tarball scanner (doc 11 §4.2, lands with
# the G3 checker tooling) + the tree-diff gate (doc 11 §3.6). Banning a
```
What is wrong: of the three enforcement layers claimed, only the first exists in the repo. No F1–F7 tarball scanner exists anywhere under tools/ or scripts/ (rg across all .rs/.sh/.py — only unrelated rpm-packaging matches); it is at least disclosed in future tense ("lands with"). The tree-diff gate is cited twice as existing enforcement ("enforced in depth: … + the tree-diff gate"; "Registry pinning is enforced by the committed Cargo.lock + --locked CI builds + the tree-diff gate") with no tense qualifier, but docs/worklog/archive-2026-10.md states "Doc 11 §3.5's fuller audits-job chain (tree-diff, cargo vet, native scanner, SBOM) remains future cards". A PAPER-ONLY layer is cited as if it enforced anything. (SBOM, the one layer that did land, exists as `xtask sbom` — different section.) What correct looks like: mark both unimplemented layers "[PLANNED]" exactly as the file does elsewhere ("flips to 'deny' after the first dedup pass [PROPOSAL…]", L151-152).
Confidence: HIGH — Verification: read + repo-wide rg.

F-003 [S3]
Where: deny.toml:122-125 (accurately quoting the lane) + .github/workflows/ci.yml:738-743 + Cargo.toml:51
Code:
```
# required by the tauri-check lane's
# `cargo deny --manifest-path apps/desktop-tauri/Cargo.toml
# --exclude-dev check` run, which reuses this file as its config.
```
What is wrong: the tauri-check lane's command omits `sources` (`check bans licenses advisories`, ci.yml:743). Since apps/desktop-tauri is excluded from the workspace (root Cargo.toml:51 `exclude = ["apps/desktop-tauri"]`, own lockfile), the root job's `--workspace` + `sources` check never sees the tauri tree either. Net: the largest dependency tree in the repo (472 registry crates) is never checked against `[sources] unknown-git = "deny"` / `unknown-registry = "deny"`. Today that tree has zero git sources (verified statically), but the policy's stated intent — "crates.io only: no git dependencies" (L401) — is unenforced exactly where the dependency count is largest. What correct looks like: add `sources` to the tauri-check lane's check list.
Confidence: HIGH — Verification: read (both files, command strings compared) + lockfile scan.

F-004 [S4]
Where: deny.toml:14-15 and 286
Code:
```
#     verified against every locked crate's SPDX via the crates.io API,
#     2026-10-20, evidence in tmp/preaudit-e5r/). cargo-deny warns on
```
What is wrong: the cited evidence directory does not exist in a checkout — `.gitignore:40` ignores `tmp/` wholesale, so `tmp/preaudit-e5r/` (the crates.io-API verification evidence) and `tmp/preaudit-e5r/gst-plugin-search.json` (the 101-crate enumeration source) were never committed. Every "verified to EXIST on crates.io (API 200, 2026-10-20)" claim is unverifiable from the repo — an evidence citation that dangles for any reviewer, in a file whose sibling tooling (feature-audit R4) rejects exactly this pattern. (I verified the 101 count is internally exact, but the provenance claim stays UNVERIFIED.)
Confidence: HIGH — Verification: read + `ls tmp/preaudit-e5r/` → No such file or directory.

F-005 [S4]
Where: deny.toml:73
Code:
```
# Minimal allow set for the current lockfile (22 registry crates):
```
What is wrong: stale header. The current root Cargo.lock has 119 registry crates (python3 count; the recorded sweep saw 118 at commit 784c6e6). The later comment blocks (L89-141) honestly document each addition (6.A1b ureq tree, W18-KK symphonia/rav1d, wave-26 tauri classes), but the section header still asserts the set is "for the current lockfile (22 registry crates)". A reader trusting the header concludes 97 crates' licenses are unchecked. Related drift: the recorded sweep's "2 `duplicate` warnings (getrandom, syn)" vs the current lock's 3 duplicate-version crates (getrandom ×2, hashbrown ×2, syn ×2 — python3 count) — exactly the drift a live CI job would have caught (see also the file's own "billing-blocked" admission, L107).
Confidence: HIGH — Verification: read + python3 counts.

## 4. Per-key table (config inventory; per-function table N/A — config file)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[graph]` (empty, documented) | 39-43 | REAL | — | no target filter by design; comment accurate |
| `[advisories] yanked = "deny"` | 45-49 | REAL | — | coherent; sweep record: "no locked crate is yanked" |
| `[advisories] ignore[0]` RUSTSEC-2024-0436 | 53-59 | REAL | — | paste 1.0.15 + rav1d 1.1.0 locked exactly as claimed |
| `[advisories] ignore[1]` RUSTSEC-2024-0370 | 60-69 | REAL | — | proc-macro-error 1.0.4/glib-macros 0.18.5 in tauri lock only, as claimed |
| `[licenses] confidence-threshold = 0.93` | 83 | REAL | — | stricter than cargo-deny default (0.8) |
| `[licenses] allow (12 entries) | 84-142 | REAL | — | cross-checked: covers all 105 cache-resolvable root-lock crates; 8 slash-form `MIT/Apache-2.0` crates + wasmparser's `WITH` expression depend on cargo-deny's lax SPDX mode — UNVERIFIED (tool missing); 14 windows/wasi target-only crates not in local cache — UNVERIFIED |
| `[licenses.private] ignore = true` | 144-148 | REAL | — | all 39 workspace members publish=false (verified) |
| `[bans] multiple-versions = "warn"` | 150-153 | REAL | — | documented [PROPOSAL] to tighten; current lock has 3 duplicate-version crates (would warn) |
| `[bans] wildcards = "warn"` | 154-167 | SHALLOW | S3 | rationale sound (path deps version-less by design) but the "only two expected warnings" tripwire is rotted (F-001) |
| `[bans] highlight = "all"` | 168 | REAL | — | cosmetic |
| `[bans] deny (174 entries) | 170-398 | REAL | — | zero banned crates in either lock; 101 gst-plugin count exact; wrapper entry matches the actual libloading edge; crates.io-existence claims UNVERIFIABLE (F-004) |
| `[sources]` (4 keys) | 400-407 | REAL | S3 | policy holds in both locks today; never enforced on the tauri tree (F-003) |

## 5. Test honesty
No tests target deny.toml directly. The honest-evidence substitutes: (a) ci.yml cargo-deny job (runs the file against the lockfile — wiring verified, execution UNVERIFIED from sandbox); (b) docs/security/sweep-supply-chain.md records a local run at commit 784c6e6 with pasted output ("advisories ok, bans ok, licenses ok, sources ok") — a claim I cannot re-execute (tool missing); my static cross-check corroborates every part of it that is checkable without the tool (no bans hit, no git sources, advisory set, warning taxonomy direction).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Enforced by the cargo-deny CI job (.github/workflows/ci.yml …) with cargo-deny 0.20.2 (SHA-256-pinned there)" | deny.toml:3-4 | REAL-UNPROVEN | job exists (ci.yml:490-520, exact command+pin match); live runner execution UNVERIFIED (no network/git; file itself admits a "billing-blocked" era, L107) |
| "cargo deny --workspace check licenses sources bans advisories" | deny.toml:6 | REAL | ci.yml:520 verbatim |
| "verified against every locked crate's SPDX via the crates.io API … evidence in tmp/preaudit-e5r/" | deny.toml:14-15 | UNVERIFIED | tmp/ gitignored (.gitignore:40), dir absent from checkout (F-004) |
| "Every name below was verified to EXIST on crates.io (API 200, 2026-10-20)" | deny.toml:25-27 | UNVERIFIED | network claims, evidence gitignored; internal consistency exact (101 count) |
| "GPL/LGPL deliberately absent until ADR-006 decides" | deny.toml:82 | REAL (and holds) | zero GPL-family license expressions in either lockfile (registry-cache scan, cache-resolvable crates) |
| "the only two expected warnings are the two workspace path edges" | deny.toml:160-163 | FALSE | sweep record: 29 warnings / 25 crates, "that count has rotted" (F-001) |
| "The native-code ban is … enforced in depth: named-crates list + F1–F7 tarball scanner + tree-diff gate" | deny.toml:32-37 | SHALLOW | only layer 1 exists; scanner future-tense, tree-diff unqualified (F-002) |
| "Registry pinning is enforced by … --locked CI builds + the tree-diff gate" | deny.toml:163-164 | SHALLOW | --locked builds real (release lane, baseline); tree-diff gate does not exist (worklog: "remains future cards") |
| "libloading enters the LOCKFILE only through tray-icon → libappindicator … NOT enabled in this workspace" | deny.toml:179-189 | REAL | lock edge: libloading 0.7.4 ← libappindicator-sys only; tauri 2.12.5 default features exclude tray-icon (registry manifest); no trayIcon config in repo |
| "all 101 crates below came from the crates.io search API" | deny.toml:285-286 | REAL (count) / UNVERIFIED (provenance) | 101 quoted entries counted exactly; API evidence gitignored |
| "the tauri-check lane's … --exclude-dev check run, which reuses this file" | deny.toml:122-125 | REAL | ci.yml:738-743 verbatim (and omits `sources` — F-003) |

## 7. Dead / duplicated / unreachable code
None (config). Note: `multiple-versions = "warn"` currently covers 3 duplicate-version crates (getrandom, hashbrown, syn) — the file's dedup-pass proposal is still open.

## 8. Dependencies & flags
Consumers: .github/workflows/ci.yml (cargo-deny job + tauri-check lane), docs/security/sweep-supply-chain.md (evidence record). The file pins no versions itself; its authority comes from the CI job's SHA-256-pinned cargo-deny 0.20.2 binary. Baseline fact: cargo-deny not installed in this sandbox — all pass/fail behavior UNVERIFIED; this audit is the static cross-check the task prescribed.

## 9. Open questions for the next agent
- Has the ci.yml cargo-deny job ever gone green on a real runner? (Check GitHub Actions history / merge records — the file admits a billing-blocked era; the only recorded run is the local sweep at 784c6e6, which predates the wave-26 tauri-class additions at commit time.)
- Who owns ADR-006 (repo license posture)? `[licenses.private] ignore = true` means the 39 workspace crates ship with NO license declaration — fine for a private repo, a blocker the day it goes public.
- The 8 slash-form `MIT/Apache-2.0` crates (assert_matches, atomig, atomig-macro, string-interner, wasmi, wasmi_collections, wasmi_core, wasmi_ir) pass only under cargo-deny's lax SPDX parse; worth one real `cargo deny check licenses` run when the tool is available to confirm, plus the 14 uncached windows/wasi crates.
