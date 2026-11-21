# AUDIT: deny.toml
| Field | Value |
|---|---|
| Auditor | W01-A06 |
| Date | 2026-11-21 |
| Lines / functions | 407 / 14 config keys (189 list entries: 13 SPDX allows, 174 bans.deny, 2 advisory ignores) |
| File verdict | REAL (artifact genuine, consumed, 3/4 sections pass baseline) — but its header enforcement claim is FALSE (F-001) and it cannot pass its own gate as committed (F-002) |
| Reachable from shipped entry point? | NO — by design not linked into `lumen`/`lumen-tauri`; consumed only by external cargo-deny invocations: .github/workflows/ci.yml:520 and :743 (never executed — CI billing-blocked), scripts/quality/07_supply_chain.sh:12 (non-failing reporter), baseline run #25 (executed, FAILED). xtask's only reference (tools/xtask/src/cmd/feature_audit.rs:449) is a filename token in a path-existence list, not consumption. |
| Findings | S0: 1 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | `sed -n` (all quotes below verified verbatim); `rg -n 'name = "X"' Cargo.lock apps/desktop-tauri/Cargo.lock` (claimed crates/versions, banned-crate absence); `rg -c 'source = "registry' Cargo.lock` (119); `rg -n 'path *= *"'` over 27 member manifests (144 version-less path deps); `rg -n 'deny' .github/workflows/ci.yml scripts/full_gate.sh scripts/quality/*.sh` (consumers); `git ls-files audit/runs/baseline/` (committed evidence); `rg -n '^license'` member manifests (none); `ls tmp/preaudit-e5r/` (absent) |

## 1. One-paragraph verdict
deny.toml is a genuinely constructed cargo-deny config: every one of its 16 crate-version citations matches the lockfiles, none of its 174 banned crates appears in either lockfile, and the single baseline execution parsed it and passed bans/licenses/sources. That is where the good news ends. Its header asserts "Enforced by the `cargo-deny` CI job" — false at the audited commit: CI has never executed a single job (billing), the file itself admits the job is "billing-blocked" 100 lines below the header, the local quality lane that runs cargo-deny is classified as a non-failing reporter, and scripts/full_gate.sh (the implemented local gate) contains no cargo-deny step at all. The one executed run of this config FAILED (exit 1, advisories). Additionally, its documented wildcards tripwire ("only two expected warnings") is off by two orders of magnitude (144 version-less path edges today), its Zlib license-ledger provenance is factually wrong for the workspace tree, its "verify-before-ban" evidence lives in a gitignored tmp/ path that does not exist, and three comments still defer to an ADR-006 decision that was accepted the day before the file's own date stamps. The supply-chain story this file exists to tell is currently enforced by nothing that runs.

## 2. What actually works (verified)
- All 16 version citations accurate — verified by `rg -A1 '^name = "X"'` on both lockfiles: ws lock has paste 1.0.15, rav1d 1.1.0, ureq 3.5.0, ring 0.17.14, rustls-webpki 0.103.16, untrusted 0.9.0, subtle 2.6.1, webpki-roots 1.0.11, to_method 1.1.0, unicode-ident 1.0.27, memchr 2.8.3; tauri lock has target-lexicon 0.12.16, glib-macros 0.18.5, proc-macro-error 1.0.4, tauri 2.12.5, wry 0.57.2 — all exactly as cited in comments.
- Symphonia claim accurate — all 7 family crates (`symphonia-bundle-mp3`, `-codec-aac`, `-codec-alac`, `-codec-vorbis`, `-common`, `-core`, `-metadata`) present at 0.6.1 in the workspace lock.
- gst-plugin enumeration count accurate — `sed -n '292,392p' | rg -c '"gst-plugin'` → 101, exactly as the comment claims.
- Ban list efficacious on paper — loop over all 174 banned names against both lockfiles: zero hits in either. Consistent with baseline INDEX #25 "bans ok".
- Baseline execution: `cargo deny check` (INDEX #25, audit/runs/baseline/INDEX.md:25) — config parsed; **bans ok, licenses ok, sources ok**; advisories FAILED (see F-002).
- CI job YAML real and pinned — .github/workflows/ci.yml:490-520 defines the `cargo-deny` job, installs 0.20.2 with SHA-256 verification (ci.yml:515), and the command at ci.yml:520 matches the header quote at deny.toml:6.
- `[sources].allow-registry` URL matches the lockfiles' `source = "registry+https://github.com/rust-lang/crates.io-index"` strings; sources ok at baseline.

## 3. Findings

### F-001 [S0] Header's enforcement claim is false: no lane that runs enforces this file
Where: deny.toml:3-6
Code:
```
# Enforced by the `cargo-deny` CI job (.github/workflows/ci.yml, gates
# G3/G4 per doc 10 §4.6) with cargo-deny 0.20.2 (SHA-256-pinned there):
#
#     cargo deny --workspace check licenses sources bans advisories
```
What is wrong: "Enforced by" is asserted as present fact. Four independently verified facts contradict it:
1. The CI job has never executed. docs/reviews/wave24-followup-ledger.md:336: "**CI Actions billing** (every run dies at 0 steps) | all CI gates; … none has ever executed". deny.toml's own comment (lines 106-107) concedes "the billing-blocked cargo-deny CI job / could not surface the drift".
2. The one local lane that invokes cargo-deny cannot fail: scripts/quality/07_supply_chain.sh:12-13 pipes the check into `tee|tail` and reduces the outcome to a print — `say "deny exit=$?"` — with no `-e` and no exit propagation; and scripts/quality/99_run_all.sh:35 classifies it `J 07_supply_chain.sh`, where 99_run_all.sh:16 defines `J` as "a REPORTER: its exit code is recorded but does not fail the tier".
3. scripts/full_gate.sh — the implemented local gate with 9 cargo steps (fmt/clippy/test/check-architecture/fixtures/lint-literals/check-identity/sbom/web-budget, full_gate.sh:11-71) — contains zero cargo-deny invocations (`rg -c 'deny' scripts/full_gate.sh` → 0).
4. The single executed run of this config FAILED: baseline INDEX #25, `cargo deny check`, exit 1.
Why it matters: this file is the project's entire cargo-deny policy plane — 174 ban entries, 13 license classes, 4 source rules, 2 advisory acceptances. Every one of those rules is presently enforced by nothing: not CI (never ran), not the local gate (omits it), not the quality suite (cannot fail). The clean-room/no-FFmpeg/no-dlopen supply-chain guarantee documented across doc 11 rests on a gate that has never successfully executed anywhere. A reader of line 3 would believe otherwise.
What correct looks like: either the header states the truth ("DESIGNED to be enforced by the cargo-deny CI job; as of 2026-11 CI is billing-blocked and no local gate fails on this file — see R-16"), or a lane that actually runs (full_gate.sh step, or JG classification in 99_run_all.sh with exit propagation in 07_supply_chain.sh) enforces it. Note the mechanism is real and correctly written (ci.yml:490-520); the falsity is the unqualified "Enforced by" claim plus zero operative enforcement — on the S0/S2 boundary, the scale's "claim is false" and the pick-higher rule put this at S0.
Confidence: HIGH — Verification: read (all four legs: ci.yml, 07_supply_chain.sh, 99_run_all.sh, full_gate.sh, wave24-followup-ledger.md) + baseline INDEX #25.

### F-002 [S2] Committed config fails its own gate; one file serving two graphs is structurally broken
Where: deny.toml:189 (with 179-188 rationale), baseline INDEX.md:25
Code:
```
    { name = "libloading", wrappers = ["libappindicator-sys"] },  # nagisa/rust_libloading
```
What is wrong: baseline INDEX #25 records `cargo deny check` exit 1: "advisories FAILED (config drift: unused wrapper `libappindicator-sys` for libloading); bans ok, licenses ok, sources ok". Verified structural cause: the workspace Cargo.lock contains NEITHER `libloading` NOR `libappindicator-sys` (rg → 0 hits both), while apps/desktop-tauri/Cargo.lock contains both. The wrapper exception exists solely for the tauri admission graph, so in the workspace check (the graph the baseline ran, tauri being workspace-excluded per root Cargo.toml `exclude = ["apps/desktop-tauri"]`) the spec is necessarily dead — cargo-deny 0.20.2 flags it as drift and the committed config cannot go green on the workspace graph. A second dead-in-workspace entry compounds it: the RUSTSEC-2024-0370 ignore (deny.toml:69) is for proc-macro-error, which exists only in the tauri lock (verified); the file predicts this surfaces as merely "an unused-ignore warning, which is the honest ledger" (lines 63-64), yet advisories FAILED at baseline — so either that prediction is wrong or another advisory hit the tree; the exact diagnostic cannot be attributed because INDEX-referenced `cargo-deny-check.log` is not committed (git ls-files audit/runs/baseline/ shows no .log files at all).
Why it matters: a gate config that fails its own check as committed means the moment CI billing is enabled, every PR is red on G3/G4 — or worse, the team learns to ignore the red. The two-graph design (workspace + excluded tauri subcrate sharing one config) is unresolvable under 0.20.2 without splitting the config or versioning the wrapper exception.
What correct looks like: a second deny.toml (or `[graph]`-scoped variant) for the tauri lane carrying the wrapper exception, and a workspace config with no wrapper spec; plus the failing diagnostic re-verified and either fixed or the unused ignore removed.
Confidence: HIGH on the failure (baseline-cited) and on lockfile structure (rg-verified); MEDIUM on the precise failing diagnostic (log absent) — Verification: read + baseline INDEX #25; REQUEST-ORCH-RUN below.

### F-003 [S3] Wildcards tripwire arithmetic is dead: "only two expected warnings" vs 144 version-less path edges
Where: deny.toml:160-163
Code:
```
# later cargo-deny). So: "warn" — the only two expected warnings are the
# two workspace path edges (codec-api→media-types, media-types→testkit);
# any THIRD wildcard warning is a real `*` version requirement in a
# manifest and is a review stopper.
```
What is wrong: counted today: member manifests contain **144** braced `path = "../x"` dependency declarations and **0** that carry a version alongside the path (rg over all 27 member Cargo.tomls). The two named edges do exist (crates/codec-api/Cargo.toml:23 `media-types = { path = "../media-types" }`; crates/media-types/Cargo.toml:29 `testkit = { path = "../testkit" }`) — they are simply 142 short of the total. Under the comment's own asserted 0.20.2 semantics ("the wildcards lint also fires on intra-workspace PATH dependencies"), a run today yields 144 wildcard warnings, not two.
Why it matters: the documented review rule — "any THIRD wildcard warning is a review stopper" — is unusable; the tripwire drowned the moment the workspace grew past three crates, and nobody can distinguish an injected real `*` requirement from the noise. The stated mitigation for loosening doc 11 §3.2's `wildcards = "deny"` to "warn" therefore has no compensating control.
What correct looks like: the comment states the real expected-warning count (or a maintained allowlist of the 144 intra-workspace edges), and the tripwire is redefined (e.g., "any wildcard warning naming a non-workspace-member crate is a stopper").
Confidence: HIGH — Verification: read + rg count (144/0).

### F-004 [S3] Zlib license-ledger provenance is false for the workspace tree
Where: deny.toml:129-139
Code:
```
#   Zlib — foldhash 0.2.0 (dom_query -> tauri-utils -> tauri
#          2.12.5); OSI-approved permissive, same posture as
#          MIT/BSD (the doc 11 §3.2 sketch's wider list already
#          included Zlib).
...
# The workspace-tree check does not encounter these classes (they
# enter only through the admission subcrate), so it now carries
# unused-allowed warnings for the pair — warnings, not errors,
```
What is wrong: the workspace Cargo.lock DOES contain a Zlib-licensed foldhash — version **0.1.5** (Cargo.lock:241-244), pulled by `hashbrown 0.15.5` whose dependency list is `["foldhash"]` (Cargo.lock:269-276; consumers include indexmap and string-interner). The comment's claims are wrong on three counts: (a) the workspace-tree check DOES encounter the Zlib class; (b) the cited provenance (dom_query → tauri-utils → tauri) describes only the tauri lock's foldhash 0.2.0 (apps/desktop-tauri/Cargo.lock:908-910); (c) the workspace tree carries a different foldhash version via a different path.
Why it matters: the file's "minimal-set posture" ledger is its audit trail for every license class admitted. This entry's rationale is factually wrong for the main workspace, which means the Zlib allow is load-bearing for the MAIN tree while documented as admission-subcrate-only — the ledger cannot be trusted to reason about actual license exposure, and removing "tauri-only" classes during a future cleanup would silently break the workspace check.
What correct looks like: the ledger records both provenance chains (tauri: foldhash 0.2.0 via dom_query; workspace: foldhash 0.1.5 via hashbrown 0.15.5).
Confidence: HIGH — Verification: read + rg on both lockfiles.

### F-005 [S3] "Verify-before-ban" evidence lives in a gitignored tmp/ path that does not exist
Where: deny.toml:25-27, 285-287
Code:
```
#     Every name below was verified to EXIST on crates.io (API 200,
#     2026-10-20) before banning; the single candidate that 404'd
#     (gstreamer-utils-sys) is skipped and noted in place.
```
and
```
# family is enumerated: all 101 crates below came from the crates.io
# search API (evidence tmp/preaudit-e5r/gst-plugin-search.json) and
# thus all exist.
```
What is wrong: `tmp/` is gitignored (.gitignore:40) and `tmp/preaudit-e5r/` does not exist in the repo (`ls` → No such file or directory). The existence-verification of all 174 banned names — the file's stated safety net against dead entries — is unverifiable from the repository, permanently. Same for the CI run IDs cited as first-surfacing evidence (43451838662 at line 68; 43422427483 at line 127) — unverifiable offline.
Why it matters: a typo'd or nonexistent crate name in a ban list is a dead entry that no tool flags (cargo-deny does not error on banning absent crates); the only claimed control was this evidence, and it is not in the repo. 101 enumerated gst-plugin names (including odd ones like `gst-plugin-debugseimetainserter`) are now trust-me assertions.
What correct looks like: the verification evidence committed under a tracked path (e.g., docs/dependencies/records/ or audit/), or the claim downgraded to "verified at authoring time, evidence not retained".
Confidence: HIGH on absence (ls + .gitignore); the actual crates.io existence is UNVERIFIED (no network).

### F-006 [S4] "Current lockfile (22 registry crates)" is 5x stale
Where: deny.toml:73
Code:
```
# Minimal allow set for the current lockfile (22 registry crates):
```
What is wrong: the workspace Cargo.lock today contains 119 registry crates (`rg -c 'source = "registry' Cargo.lock` → 119). "Current" is false; the addenda blocks below document the growth but the header still frames 22 as the present state.
Why it matters: minor, but this header is the file's own summary of its posture; a reader computing "how many crates does the license allowlist actually cover" gets 22 vs 119.
What correct looks like: "the lockfile at 1.E5-R freeze (22 registry crates; 119 as of 2026-11)".
Confidence: HIGH — Verification: read + rg count.

### F-007 [S4] Three comments defer to an ADR-006 decision that had already been made
Where: deny.toml:29-30, 82, 145-147
Code:
```
#   * `licenses.private.ignore = true` skips the license check for our
#     publish=false workspace members (they carry no license field yet;
#     their license posture is decided with ADR-006, doc 11 §8.2).
```
and deny.toml:82 `# GPL/LGPL deliberately absent until ADR-006 decides (doc 11 §3.2, §8.2).` and deny.toml:146 `# carry no license field — skipped until ADR-006 sets the repo license`.
What is wrong: docs/adr/ADR-006-license-mit.md line 3: "**Status:** accepted (user decision, 2026-10-19)" — decided MIT, one day BEFORE this file's own "verified 2026-10-20" stamps. ADR-006 also settles the GPL question permanently ("GPL-licensed source remains strictly forbidden — no vendoring, no copying", ADR-006:48). All three future-tense deferrals were stale the day they were written. (The factual basis of private.ignore does hold: no member manifest carries a `license` key — verified.)
Why it matters: the escape hatch `[licenses.private].ignore = true` (all workspace members skip license checking) is justified by a decision that had already landed; with ADR-006 accepted, members could carry `license = "MIT"` and the skip could be dropped rather than maintained on a void rationale.
What correct looks like: comments updated to cite ADR-006 as accepted (MIT; GPL permanently absent), and a tracked task to either add member license fields or re-justify the skip.
Confidence: HIGH — Verification: read (ADR-006:3, :48; rg '^license' over member manifests → none).

### F-008 [S4] Self-description drift: PackageSpec spelling and the quoted tauri-lane command
Where: deny.toml:22-24, 123-125
Code:
```
#   * `bans.deny` entries are bare crate names (the 0.20.2 PackageSpec
#     accepts `"<name>"` or `{ crate = "<name>", reason = "..." }`; the
#     doc 11 sketch's `{ name = ... }` spelling is the older alias).
```
What is wrong: (a) the file's own table-form entry at line 189 uses exactly the spelling the comment calls "the older alias" — `{ name = "libloading", wrappers = [...] }`, not `{ crate = ... }`; (b) deny.toml:123-125 quotes the tauri-check lane command as `cargo deny --manifest-path apps/desktop-tauri/Cargo.toml --exclude-dev check`, while the real command (ci.yml:743) is `cargo deny --manifest-path apps/desktop-tauri/Cargo.toml --exclude-dev check bans licenses advisories` — the quoted form implies all checks including `sources`, which that lane does not run.
Why it matters: cosmetic, but this file is unusually heavily commented precisely so its deviations can be audited; two of its self-descriptions are wrong, which degrades the reliability of the rest.
What correct looks like: comment says the table form is `{ name = ..., wrappers = ... }`; the lane quote includes `bans licenses advisories`.
Confidence: HIGH — Verification: read (deny.toml:189; ci.yml:743).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[graph]` (empty, comment-only) | 39-43 | REAL | — | Consumed as defaults; comment's virtual-manifest claim TRUE (root Cargo.toml is `[workspace]`-only with members list); "no target filter" means all targets graphed. |
| `[advisories].yanked = "deny"` | 49 | REAL | — | Consumed; baseline reached advisories (failed on drift, not on a yanked crate — INDEX #25). |
| `[advisories].ignore` RUSTSEC-2024-0436 (paste/rav1d) | 59 | REAL | — | Used in workspace graph: paste 1.0.15 + rav1d 1.1.0 both in ws lock (verified). Informational unmaintained advisory; documented removal plan. |
| `[advisories].ignore` RUSTSEC-2024-0370 (proc-macro-error) | 69 | REAL | — | Dead in workspace graph (proc-macro-error 1.0.4 tauri-lock-only, verified); file predicts warning-only (63-64) but baseline advisories FAILED — see F-002. |
| `[licenses].confidence-threshold = 0.93` | 83 | REAL | — | Consumed; matches doc 11 §3.2 sketch (docs/11-dependency-security-compliance.md:239). |
| `[licenses].allow` (13 SPDX classes) | 84-142 | REAL | — | Consumed; licenses ok at baseline. 11 of 13 entries' provenance comments verified accurate against lockfiles; Zlib entry's provenance FALSE (F-004); "22 registry crates" header stale (F-006). |
| `[licenses.private].ignore = true` | 144-148 | REAL | — | Skips license check for all workspace members; factual basis holds (no member carries a license key) but justification voided by accepted ADR-006 (F-007). |
| `[bans].multiple-versions = "warn"` | 153 | REAL | — | Consumed; still "warn" — doc 11 §12 Q9 (docs/11...:744) date never set; ws lock carries duplicates (hashbrown 0.15.5 + 0.17.1). |
| `[bans].wildcards = "warn"` | 167 | REAL | — | Documented deviation from doc 11 sketch's "deny" (sketch at docs/11...:249); tripwire arithmetic dead — F-003. |
| `[bans].highlight = "all"` | 168 | REAL | — | Consumed; matches sketch (docs/11...:248). |
| `[bans].deny` (174 entries: 173 bare + 1 wrapper form) | 170-398 | REAL | — | bans ok at baseline; zero banned crates in either lockfile (loop-verified); existence-verification evidence absent (F-005); libloading wrapper spec structurally dead in the workspace graph (F-002); gstreamer-utils-sys skip honestly noted in place (257-259). |
| `[sources].unknown-registry = "deny"` | 404 | REAL | — | Consumed; sources ok at baseline. |
| `[sources].unknown-git = "deny"` | 405 | REAL | — | Consumed; matches sketch (docs/11...:258). |
| `[sources].allow-registry = ["https://github.com/rust-lang/crates.io-index"]` | 406 | REAL | — | Matches lockfile registry source strings (verified). |
| `[sources].allow-git = []` | 407 | REAL | — | No git deps in either lockfile (all `source` lines are registry). |

## 5. Test honesty
No tests exist for deny.toml anywhere in the repo (only reference is tools/xtask/src/cmd/feature_audit.rs:449, which lists the filename among root files whose referenced paths must resolve — path validation, not policy testing).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (none) | N/A | Nothing in the repo asserts deny.toml's content, drift status, or pass/fail — which is exactly how F-002's drift survived to commit. The only detector would be a cargo-deny run in a failing lane, which does not exist (F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Enforced by the cargo-deny CI job … gates G3/G4" | deny.toml:3-4 (also doc 10 §4.6, doc 11 §3.5) | FALSE | F-001: CI never executed (wave24-followup-ledger.md:336; deny.toml:106); local reporter non-failing (99_run_all.sh:16,35; 07_supply_chain.sh:12-13); full_gate.sh has no deny step; baseline run failed. |
| Deviations from doc 11 §3.2 sketch are all recorded (version keys omitted; minimal license set; wildcards warn; bare-name bans) | deny.toml:8-37 vs docs/11-dependency-security-compliance.md:228-261 | REAL | Sketch read and compared: all four deviations present and documented; sketch's extra classes (Unicode-DFS-2016, BSD-2-Clause-Patent) justifiably absent (unicode-ident 1.0.27 uses Unicode-3.0, verified). |
| "bans ok, licenses ok, sources ok" for this config | baseline INDEX #25 | REAL | Executed run, result recorded in INDEX.md:25. |
| "advisories FAILED (config drift: unused wrapper libappindicator-sys for libloading)" | baseline INDEX #25 | REAL | Executed; structural cause verified (F-002); detailed log not committed — diagnostic attribution MEDIUM confidence. |
| "the only two expected warnings are the two workspace path edges" | deny.toml:160-161 | FALSE | 144 version-less path dep lines in member manifests (rg count); F-003. |
| "The workspace-tree check does not encounter these classes [Zlib]" | deny.toml:136-137 | FALSE | foldhash 0.1.5 via hashbrown 0.15.5 in ws Cargo.lock (241-244, 269-276); F-004. |
| "Every name below was verified to EXIST on crates.io … evidence in tmp/preaudit-e5r/" | deny.toml:25-27, 285-286 | PAPER-ONLY | Evidence path gitignored (.gitignore:40) and absent (ls); unverifiable from repo; F-005. |
| "GPL/LGPL deliberately absent until ADR-006 decides" | deny.toml:82 | FALSE (stale) | ADR-006 accepted 2026-10-19 (ADR-006:3) — before the file's own 2026-10-20 stamps; GPL permanently forbidden (ADR-006:48); F-007. |
| cargo-deny 0.20.2 SHA-256-pinned in CI | deny.toml:4 vs ci.yml:510-518 | REAL | Pin + sha256sum verification present at ci.yml:515. |

## 7. Dead / duplicated / unreachable code
- The `{ name = "libloading", wrappers = ["libappindicator-sys"] }` spec (deny.toml:189) is structurally dead in every workspace-graph check — neither crate is in the workspace lock (verified); it lives only for the tauri lane, which has never executed (F-001/F-002).
- The RUSTSEC-2024-0370 ignore (deny.toml:69) is dead in workspace-graph checks (proc-macro-error is tauri-lock-only).
- The Zlib and Apache-2.0 WITH LLVM-exception allow entries are live in the tauri graph; in the workspace graph Zlib is live (contrary to its own comment — F-004) and Apache-2.0 WITH LLVM-exception is expected-unused (target-lexicon tauri-only, verified).
- Not reachable from any shipped binary — by design; it gates the dependency graph, not runtime.

## 8. Dependencies & flags
- Tool dependency: cargo-deny 0.20.2, external to the workspace (not a dev-dependency of any crate; xtask implements its own SBOM generator instead — ci.yml:521-528). Pin + SHA-256 verified in ci.yml:510-518; install fallback in the tauri lane (ci.yml:742) uses `cargo install cargo-deny --version 0.20.2 --locked` — not tarball-SHA-pinned, a (minor) inconsistency with the "no floating tool installs" posture stated at ci.yml:479-481 (ci.yml's audit, noted for that agent).
- `--workspace` flag (deny.toml:6, ci.yml:520, 07_supply_chain.sh:12): UNVERIFIED that cargo-deny 0.20.2 accepts it — the baseline ran plain `cargo deny check` (INDEX #25); cargo-deny is not installed in this sandbox (checked: `which cargo-deny` → not found) and no network is available. If the flag were invalid, the CI command would fail before checking anything.
- No feature flags gate this file; `[graph]` deliberately sets no targets/features (deny.toml:39-43).

## 9. Open questions for the next agent
- REQUEST-ORCH-RUN (W01-A06): `cargo deny --workspace check licenses sources bans advisories 2>&1 | tail -80` from the repo root (cargo-deny 0.20.2, advisory DB fetched) — needed to (a) attribute the baseline advisories FAILURE precisely (unused-wrapper error vs unused advisory-ignore vs a real unignored advisory hitting the tree), and (b) count actual wildcards warnings to close F-003's arithmetic (predicted: 144, or 0 if the comment's premise about 0.20.2 path-dep behavior is itself wrong — either outcome falsifies the "two").
- Baseline evidence gap (wave-00 owner): INDEX.md references 20+ run logs (cargo-deny-check.log, build-release-workspace.log, …) that are NOT committed under audit/runs/baseline/ (git ls-files confirms only smoke txt + hygiene + INDEX.md + samples). INDEX prose is currently the only citable record for most runs.
- For the ci.yml auditor: the tauri lane's cargo install fallback (ci.yml:742) and whether `sources` being absent from that lane's check list (ci.yml:743) is intentional.
- For the 07_supply_chain.sh / 99_run_all.sh auditor: the `say "deny exit=$?"` swallow (07_supply_chain.sh:13) means even a JG reclassification would not propagate the deny failure without an explicit exit aggregation in that script.
- If a future cleanup removes "tauri-only" license classes per the (wrong) ledger comment, the workspace license check will break — F-004 is the trap.
