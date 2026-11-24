# AUDIT: apps/desktop-tauri/Cargo.lock
| Field | Value |
|---|---|
| Auditor | W14-A01 |
| Date | 2026-11-24 |
| Lines / packages | 5138 / 493 `[[package]]` blocks (472 registry + 21 path) |
| File verdict | REAL (audited as DATA: a complete, self-consistent, cargo-generated resolution; every cross-checked doc claim about it verified true except the package count in INFRA-BLOCKED.md) |
| Reachable from shipped entry point? | YES — it is the `--locked` input of the shipped `lumen-tauri` binary (ci.yml tauri-check `cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked` ci.yml:731; release payloads ship the app per INFRA-BLOCKED.md §B.1) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 3 |
| Commands run | `wc -l` (5138 ✓); `rg -c '^\[\[package\]\]$'` → 493; python3 parsers/closure/dedup/license scripts (audit/scratch/W14-A01/{lockstats2,lockstats3,licenses}.py); `rg`/`sed` verifications (all quotes below sed-verified); registry-cache reads under ~/.cargo (read-only). No cargo, no git, no network. |

## 1. One-paragraph verdict
This lockfile is real data doing exactly what ADR-004 says it does: it carries the entire tauri/wry GUI closure that the workspace lockfile deliberately excludes. 493 packages; every one of the 472 registry crates is a crates.io source with a valid 64-hex checksum; every one of the 493 is reachable from the root `lumen-desktop-tauri` (zero orphans, zero dev-dep cruft); all 21 path packages are the real engine crates (audio-engine … video-engine, lumen-cli 0.1.3, desktop 0.1.3). The tauri half is enormous: closure(tauri 2.12.5) = 421 crates (85% of the file), closure(wry 0.57.2) = 246; the engine half reachable without tauri is only 129, and the GUI side adds 363 crates the engine never sees. Every deny.toml claim about this file that I could test locally is TRUE (verified below): the libloading wrapper chain, foldhash/Zlib, target-lexicon/LLVM-exception, proc-macro-error's entry path, tray-icon's feature-off status, and the workspace lock being tauri-free. No crate in the closure is GPL/LGPL-licensed; the only GPL-family text anywhere is LGPL-2.1-or-later as one branch of a 3-way OR on r-efi (permissive election). The defects are drift: the admission doc's package count is stale (468 claimed, 493 actual), 46 crate names carry multiple versions (52 redundant copies) under a multiple-versions policy that was promised to flip warn→deny and never did, and the same engine source code resolves against 32 different transitive versions in this universe than in the workspace universe (toml 1.2.1 here vs 1.1.8 there) — the shipped GUI and the shipped CLI are compiled in two dependency universes no gate reconciles.

## 2. What actually works (verified)
- Structure: `wc -l` → 5138 (matches manifest.tsv); 493 package blocks; `version = 4` format; 0 registry crates missing checksums, 0 malformed checksums, 0 non-crates.io sources (script output: "registry crates missing checksum: 0; malformed checksums: 0 / non-crates.io sources: NONE").
- Closure integrity: python3 graph walk from the root reaches all 493/493 ("closure from lumen-desktop-tauri: 493 packages … 0 outside"). Not one locked package is unreachable.
- Closure sizes: tauri 2.12.5 → 421; wry 0.57.2 → 246; tauri-runtime-wry 2.12.3 → 361; tauri-build 2.7.4 → 211; engine half (desktop+commands) → 129; tauri∩engine overlap → 58; GUI-only-over-engine → 363.
- Workspace isolation (the core ADR-004 claim): `rg -c '^name = "X"$' Cargo.lock` → tauri 0, wry 0, tao 0, gtk 0, webkit2gtk 0, muda 0, tray-icon 0, libappindicator 0, libloading 0, proc-macro-error 0, dom_query 0, target-lexicon 0. The main workspace graph is tauri-free — REAL, executed.
- deny.toml libloading wrapper (deny.toml:179-189): the only reverse edge to libloading in this lock is `libappindicator-sys 0.9.0 -> libloading` (script: "reverse deps of libloading … libappindicator-sys 0.9.0"), chain tray-icon 0.25.1 → libappindicator 0.9.0 → libappindicator-sys 0.9.0 → libloading 0.7.4; tauri 2.12.5's cached manifest shows `default = ["wry","compression","common-controls-v6","dynamic-acl","x11","dbus"]` (no tray-icon; `tray-icon = ["dep:tray-icon"]` is opt-in), and tauri.conf.json has no trayIcon key. Scoped-exception claim VERIFIED.
- deny.toml license-class admissions (deny.toml:126-141): foldhash 0.2.0 is in the lock (L908) with cached-manifest license `Zlib`, pulled by dom_query 0.28.0 (tauri-utils 2.10.2) AND dom_query 0.29.0 (wry); target-lexicon 0.12.16 is in the lock (L3682) with cached-manifest license `Apache-2.0 WITH LLVM-exception`. Both claims TRUE.
- Banned-name scan: of deny.toml's entire [bans].deny list (parsed with tomllib), the only name present in this lock is `libloading` (the scoped exception). No gstreamer/ffmpeg/vlc/bindgen/codec-wrapper name appears.
- License spread (471 of 472 registry crates have sources in the local cargo registry cache; only `windows 0.62.2` is uncached — read-only Cargo.toml license fields): NO crate is GPL- or LGPL-licensed. One nuance: `r-efi 5.3.0` and `r-efi 6.0.0` (pulled by getrandom 0.3.4/0.4.3, the duplicate pair) carry `MIT OR Apache-2.0 OR LGPL-2.1-or-later` — LGPL-2.1-or-later appears ONLY as one branch of a 3-way OR (permissive election; deny.toml's allow list passes the expression via its MIT/Apache-2.0 branches). Full distribution: MIT OR Apache-2.0 213; MIT 109; Apache-2.0 OR MIT 33 (+19 in `/`-separated spellings); Unicode-3.0 18 (+1 `(MIT OR Apache-2.0) AND Unicode-3.0` = unicode-ident); Zlib OR Apache-2.0 OR MIT 17 (objc2 family, bytemuck, dispatch2, tinyvec); MPL-2.0 14 (symphonia ×7, cssparser ×2, cssparser-macros, selectors ×2, dtoa-short, option-ext); Unlicense OR MIT 9 (+2 `/`-spelling); Apache-2.0 4 (+1 Apache-2.0 OR ISC OR MIT, +1 Apache-2.0 AND ISC = ring, +1 Apache-2.0 AND MIT = dpi); BSD-3-Clause 3 (+2 OR-combos, +1 `/`-spelling, +1 AND = brotli); ISC 3 (libloading 0.7.4 — matches allowlist §3a's "ISC" — rustls-webpki, untrusted); Apache-2.0 WITH LLVM-exception 1 (target-lexicon) + 3 more as OR branches (wasi, wasip2, wit-bindgen); Zlib 2 (foldhash, zlib-rs) + 4 Zlib-OR-combos; CC0-1.0 2; 0BSD 1 (adler2); CDLA-Permissive-2.0 1 (webpki-roots); MIT-0 1 as OR branch (dunce). Every class is covered by deny.toml's allow list through OR/AND semantics. Environment note: the registry cache GREW during this session (347 dirs at first probe → 491 at final probe; a concurrent process, likely another wave agent's cargo invocation, populated cross-platform sources) — all numbers above are from the final 491-dir state.

## 3. Findings
F-001 [S3]
Where: apps/desktop-tauri/INFRA-BLOCKED.md:21-22 (cross-ref against this file)
Code (verbatim):
```
  lane). Own lockfile (`Cargo.lock`, 468 packages incl. the tauri
  closure). Edges: `desktop` (path, the shell library), `commands`
```
What is wrong: The lockfile contains 493 packages (`rg -c '^\[\[package\]\]$'` → 493), not 468. The admission close-package document undercounts the audited surface by 25 packages. Why it matters: this is the one number the repo publishes about this file; any reviewer or auditor scoping "the tauri closure" starts from a false count, and nothing in CI re-derives it. What correct looks like: re-derive the count mechanically (or drop the number). Confidence: HIGH — Verification: ran (rg count vs doc quote).

F-002 [S3]
Where: apps/desktop-tauri/Cargo.lock (whole file); deny.toml:150-153 (policy cross-ref)
Code (verbatim, deny.toml:151-153):
```
# multiple-versions "warn" per the doc 11 §3.2 sketch; flips to "deny"
# after the first dedup pass [PROPOSAL: Lead sets the date, doc 11 §3.2].
multiple-versions = "warn"
```
What is wrong: 46 crate names carry multiple locked versions — 98 entries, 52 redundant copies — in this one lockfile: windows-targets/windows_* ×2 each (0.42.2 from `jni 0.21.1 -> windows-sys 0.45.0 -> windows-targets 0.42.2`, vs 0.52.6), dom_query 0.28.0 (tauri-utils) + 0.29.0 (wry), phf 0.13/0.14 families ×5 names, syn 1.0.109/2.0.119/3.0.10 (1.0.109 kept alive solely by proc-macro-error 1.0.4), toml_edit ×3 (0.19.15/0.20.2/0.25.19), toml 0.8.2/1.2.1, getrandom ×3, schemars 0.8.22/0.9.0/1.2.3, thiserror 1/2, bitflags 1/2, html5ever 0.39/0.40.1, winnow 0.5.40/1.0.5, and 30 more. The promised flip to `deny` never happened; the GUI closure more than triples the duplicate surface the policy was written against. Why it matters: duplicate copies bloat every desktop binary and broaden the advisory surface; the policy's own escalation path is unexecuted. What correct looks like: the dedup pass + policy flip, or a documented per-lock exception list. Confidence: HIGH — Verification: ran (dedup script; reverse-dep script pinned each root cause).

F-003 [S3]
Where: apps/desktop-tauri/Cargo.lock vs /Cargo.lock (drift); e.g. this file L4042-4043 vs workspace Cargo.lock:1150-1151
Code (verbatim, both sides):
```
name = "toml"
version = "1.2.1+spec-1.1.0"        <- apps/desktop-tauri/Cargo.lock L4042-4043 (and 0.8.2 at L4030-4031)
```
```
name = "toml"
version = "1.1.8+spec-1.1.0"        <- workspace Cargo.lock
```
What is wrong: The 21 path packages (the real engine: lumen-cli, settings-schema, plugin-api, playlists, streaming…) are shared source but not shared resolution: of 128 crate names common to both lockfiles, 32 resolve to different version sets here — engine-relevant ones are toml 1.1.8→1.2.1, toml_datetime 1.1.2→1.1.3, toml_writer 1.1.3→1.2.0, serde_spanned 1.1.2→1.1.3, syn 3.0.9→3.0.10, log 0.4.34→0.4.35, itoa 1.0.18→1.0.19, smallvec 1.16.4→1.16.5, cc 1.10.0→1.11.1 (the other 23 drifts are GUI-only windows_* families). The dt lock's own lumen-cli entry pins ` "toml 1.2.1+spec-1.1.0",` (L1917) while the workspace's lumen-cli rides 1.1.8. Why it matters: the shipped GUI and the shipped CLI compile the same TOML-parsing engine code (settings, playlists, plugin manifests) against different toml implementations; behavior can diverge on edge cases between the two shipped artifacts, and no CI step diffs the two universes. What correct looks like: a periodic lockfile-sync check (or a workspace-level `cargo update` coord step) for the shared path-crate closure. Confidence: HIGH — Verification: ran (diff script over both locks; sed verified both entries). Severity held at S3 because every drifted pair is semver-compatible.

F-004 [S4]
Where: apps/desktop-tauri/Cargo.lock:2653-2660; deny.toml:60-69 (acceptance ledger)
Code (verbatim, deny.toml:69):
```
    { id = "RUSTSEC-2024-0370", reason = "proc-macro-error 1.0.4 via glib-macros 0.18.5 (tauri admission tree only): unmaintained-but-functional, no fixed upstream release; drop when the gtk-rs stack past 0.18 regenerates apps/desktop-tauri/Cargo.lock" },
```
What is wrong: the unmaintained crate is in the shipped GUI closure, and deny.toml's narrative names one entry path ("glib-macros 0.18.5") while the lock shows two: `glib-macros 0.18.5 -> proc-macro-error` AND `gtk3-macros 0.18.2 -> proc-macro-error` (both verified by reverse-dep scan). The drop plan depends on a gtk-rs >0.18 migration that has no owner or date. Why it matters: an informational-but-real advisory rides every desktop build, and the ledger's description is incomplete (the second parent means a glib-macros-only fix would not remove it). What correct looks like: name both parents in the ledger; attach the gtk-rs bump to a wave/card. Confidence: HIGH — Verification: ran (reverse-dep script) + read.

F-005 [S4]
Where: apps/desktop-tauri/Cargo.lock (whole file) — license data limitation + LGPL-branch nuance
What is wrong: (a) Cargo.lock carries no license fields (format limitation), so license verification depends on registry sources; 1 of 472 registry crates (`windows 0.62.2`) has no cached source in this sandbox, so its license rests only on the claimed green CI cargo-deny runs (ci.yml:743 `cargo deny --manifest-path apps/desktop-tauri/Cargo.toml --exclude-dev check bans licenses advisories`). (b) deny.toml:82 "GPL/LGPL deliberately absent until ADR-006 decides" is true of the ALLOW LIST but can be misread as absence from the closure: `r-efi 5.3.0`/`6.0.0` carry `MIT OR Apache-2.0 OR LGPL-2.1-or-later` — LGPL text IS in the GUI closure's license expressions, though only as an OR branch a licensee may decline. Why it matters: (a) 0.2% of the closure is CI-trust; (b) a license audit that greps for "LGPL" in deny.toml and concludes "no LGPL anywhere" would be wrong about the text while right about the effective posture. What correct looks like: commit a cargo-deny license report as a baseline artifact; add one clarifying comment to deny.toml naming the r-efi OR-branch. Confidence: HIGH — Verification: ran (cache scan 471/472; r-efi manifests read; reverse-dep: getrandom → r-efi).

F-006 [S4]
Where: apps/desktop-tauri/Cargo.lock:1141 & 1153 (getrandom 0.3.4 / 0.4.3 blocks; 0.2.17 at L1130)
What is wrong: the getrandom duplicate pair drags BOTH r-efi 5.3.0 and r-efi 6.0.0 into the lock (verified edges `getrandom -> "r-efi 5.3.0"` and `getrandom -> "r-efi 6.0.0"`), i.e. the same UEFI support library is compiled twice at two majors — a concrete instance of the F-002 duplication cost. Why it matters: duplicated versions are not abstract bloat; each pair doubles a real subsystem's code in the artifact. What correct looks like: fold the getrandom pair (tauri 2.12.5 pins getrandom 0.3.4; the engine side rides 0.4.3) — tauri-upstream-dependent. Confidence: HIGH — Verification: ran (reverse-dep scan).

## 4. Per-part inventory (mandatory table; a lockfile has no functions — the 7 assigned parts)
| Part | Lines | Packages | Span (first..last) | Contents of note |
|---|---|---|---|---|
| 1 | 1-799 | 85 | adler2..dunce | header `version = 4`; engine path crates audio-engine/audio-output; atk/gtk FFI begins; assert_matches, atomig |
| 2 | 800-1598 | 74 | dyn-clone..indexmap | codecs/demux/desktop/input path crates; cssparser ×2 begins; gtk3-macros (proc-macro-error parent) |
| 3 | 1599-2387 | 78 | indexmap..parking_lot | lumen-cli/lumen-desktop-tauri (L1922) root block; libappindicator chain (L1818-1844); libloading (L1858); media-* path crates |
| 4 | 2388-3185 | 78 | parking_lot_core..serde_spanned | proc-macro-error (L2653); muda; playlists/plugin-api path crates; schemars ×3 (L2965-2992) |
| 5 | 3186-3979 | 61 | serde_with..time-macros | symphonia family; tao (L3631); target-lexicon (L3682); tauri 2.12.5 (L3688, deps incl. ` "tray-icon",` at L3730) |
| 6 | 3980-4771 | 77 | tinystr..windows-sys | tauri-build/codegen/macros/runtime/-utils/-winres; toml ×2 + toml_edit ×3 (L4030-4105); tray-icon (L4190); webkit2gtk; winnow ×2 |
| 7 | 4772-5138 | 40 | windows-sys..zmij | windows_* target crates ×2 sets; wit-bindgen; wry 0.57.2 (L4961); x11; zmij |

## 5. Test honesty
Not applicable — data file, no tests. The only executable checks over this file are the CI lane's (ci.yml:731-743: `cargo test --locked`, clippy `--locked`, `cargo deny --exclude-dev check`); they run on webview-equipped runners only (the sandbox build dies at gdk-sys — baseline audit/runs/baseline/cargo-build-desktop-tauri.txt:310 `error: failed to run custom build command for \`gdk-sys v0.18.2\``), so their greenness is REAL-UNPROVEN from here.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "own lockfile carries the tauri closure" | apps/desktop-tauri/Cargo.toml:9-11 | REAL | this file: tauri 2.12.5 L3688, wry L4961, closure 421 |
| "imported in the workspace-EXCLUDED subcrate `apps/desktop-tauri` (own lockfile) — the main workspace graph stays tauri-free" | records/tauri-wry-2.x.md:13-15 | REAL | rg over /Cargo.lock: tauri/wry/tao/gtk/webkit2gtk/muda/tray-icon/libappindicator/libloading/proc-macro-error all 0 hits |
| "Own lockfile (`Cargo.lock`, 468 packages incl. the tauri closure)" | INFRA-BLOCKED.md:21 | FALSE (stale count) | actual 493 (F-001) |
| "libloading enters the LOCKFILE only through tray-icon -> libappindicator -> libappindicator-sys … NOT enabled" | deny.toml:179-189 | REAL | only reverse edge to libloading is libappindicator-sys (dep edge at L1838); tauri default features exclude tray-icon (cached manifest); tauri.conf.json has no trayIcon |
| "Zlib — foldhash 0.2.0 (dom_query -> tauri-utils -> tauri 2.12.5)" | deny.toml:129-131 | REAL | foldhash L908; tauri-utils→dom_query 0.28.0→foldhash; cached license `Zlib` |
| "Apache-2.0 WITH LLVM-exception — target-lexicon 0.12.16 (the wry/tauri tree)" | deny.toml:133-134 | REAL | target-lexicon L3682; cached license matches; via cfg-expr 0.15.8 ← system-deps ← gtk-sys family |
| "proc-macro-error 1.0.4 … glib-macros 0.18.5 -> glib/gtk -> muda -> tauri 2.12.5" | deny.toml:60-62 | REAL but incomplete | two parents: glib-macros 0.18.5 AND gtk3-macros 0.18.2 (F-004) |
| "no deny.toml-banned name beyond the scoped libloading wrapper" | records/tauri-wry-2.x.md:172-174 | REAL | tomllib-parsed ban list × lock names → only `libloading` |
| tauri 2.12.5 / tauri-build 2.7.4 / tao 0.37.2 / wry <2.13.0 pins | allowlist.md §3a rows (L125-129) | REAL | lock: 2.12.5 / 2.7.4 / 0.37.2 / wry 0.57.2; licenses MIT OR Apache-2.0 (cached manifests) |
| pre-audit pins "tauri 2.12.4 and wry 0.57.0" | ADR-004:47 | REAL (historical, superseded) | record admission refresh re-pins 2.12.5 (records L169, L183); allowlist row range covers it |
| "468 packages" (variant) — deny.toml:73 "22 registry crates" | deny.toml:73 | N/A here | that count is about the WORKSPACE lock (audited in audit/files/Cargo.lock.audit.md), not this file |

## 7. Dead / duplicated / unreachable code
No unreachable packages (0 outside the root closure). Duplicated: 46 names / 52 extra copies (F-002 inventory). Notable singletons kept alive only by legacy parents: syn 1.0.109 (only parent: proc-macro-error 1.0.4), windows-targets 0.42.2 chain (only parent: windows-sys 0.45.0 ← jni 0.21.1), phf 0.13 family (cssparser 0.37 + selectors 0.38 + tauri-utils + web_atoms 0.2.6), toml 0.8.2 (schemars 0.8.22/tauri-build path), string_cache 0.9.0 (web_atoms 0.2.6 only).

## 8. Dependencies & flags
Root: `lumen-desktop-tauri 0.1.3` → commands (path), desktop (path), serde, tauri, tauri-build (L1922-1930, sed-verified). Engine half = 20 path crates + 109 registry crates (129; 71 of them NOT in the tauri half); GUI half adds 363 crates the engine never reaches (493 = 129 + 363 + 1 root). Feature posture verified: tauri default features on (no `default-features = false` in Cargo.toml:60 `tauri = "2"`), tray-icon/devtools/isolation OFF (not in tauri default set). The `tauri = "2"` requirement itself permits a future 2.13+ resolution on `cargo update` that would exceed allowlist §3a's "<2.13.0" row — only the committed lock + `--locked` (ci.yml:731,733) holds it; no machine check enforces the allowlist range (open question).

## 9. Open questions for the next agent
- W14-A06 (Cargo.toml) / W14-A02-A03 (app.rs, main.rs): confirm no feature or cfg path enables tauri's `tray-icon`/`devtools` (the lock cannot prove runtime feature state; I verified only the default set + absence of trayIcon config).
- Who owns the deny.toml multiple-versions warn→deny flip (F-002), and does the gtk-rs >0.18 migration that drops proc-macro-error (F-004) have a wave assignment?
- Is a committed cargo-deny license report for this lockfile feasible (F-005), so GPL-absence stops resting on CI-run claims?
- The tauri-check lane's `cargo deny` step installs cargo-deny 0.20.2 unpinned-to-audit-run (`"${DENY_BIN:-cargo-deny}" --version >/dev/null 2>&1 || cargo install cargo-deny --version 0.20.2 --locked`, ci.yml:740-742) — supply-chain-wise the version is pinned but the binary provenance is runner-cache-dependent; worth a W15+ CI-domain look.
