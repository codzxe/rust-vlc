# AUDIT: apps/desktop-tauri/Cargo.lock
| Field | Value |
|---|---|
| Auditor | W14-A06 |
| Date | 2026-11-21 |
| Lines / packages | 5138 / 493 packages (472 registry + 21 first-party path) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (consumed by `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml` — baseline run #2 passed the `--locked` lock-verification phase and failed later at gtk-sys/gdk-3.0.pc; CI lanes ci.yml:731/733/743, tauri-launch.yml:156/345/490; release.yml ships `lumen-tauri`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `awk`/`grep` full-graph extraction of all 493 package blocks (name+version+source+checksum); dup-version scan (`sort|uniq`); cross-lockfile `join` root Cargo.lock vs this file; `comm` name-set diff; `rg`/`grep` for callers/edges (assert_matches→rav1d, zmij→serde_json, foldhash→dom_query, proc-macro-error→glib-macros, muda→tauri); `sed -n` verbatim quote verification; read of Cargo.toml, root Cargo.toml:51, deny.toml:55-130, .github/workflows/{ci,tauri-launch,release}.yml, docs/security/sbom/README.md; baseline cites only (no builds per rules) |

Kind: `lockfile` (integrity audit: consistency with Cargo.toml, version duplication,
unusual entries). "Function table" is replaced by the dependency-consistency and
inventory tables in §4, as fits a generated artifact.

## 1. One-paragraph verdict

This is a genuine, internally consistent Cargo v4 lockfile for the ADR-004
workspace-EXCLUDED tauri shell: it is in sync with `apps/desktop-tauri/Cargo.toml`
(proven by baseline run #2, which used `--locked` and got past resolution into
parallel compilation before dying on the missing system lib `gdk-3.0.pc`), all 472
registry packages carry checksums, there are no git sources, no `[[patch]]`, no
`[metadata]`, and the tauri closure (tauri 2.12.5 / wry 0.57.2 / tao 0.37.2) stays
out of the root workspace lock exactly as the ADR claims. The problems are around
it, not in it: the repo now ships **two uncoordinated lockfiles over the same 21
first-party crates** that have already drifted (11 shared crates at different
versions, including `lumen-cli`'s direct settings dependency `toml` 1.1.8 vs
1.2.1), the committed SBOM documents only the root graph (zero coverage of this
lock's 472-crate closure), this lock carries two waived unmaintained-advisory
crates and has never had a deny/test run with committed evidence, and it contains
one dependency (`zmij`, pulled by `serde_json 1.0.154`) whose existence on
crates.io cannot be verified from this sandbox.

## 2. What actually works (verified)

- **Lock ↔ manifest consistency + resolvability** — verified by citing
  `audit/runs/baseline/INDEX.md` run #2 (`cargo build --release --locked
  --manifest-path apps/desktop-tauri/Cargo.toml`, EXIT=101) +
  `build-desktop-tauri.log.txt`: the failure is `gdk-3.0.pc needs to be installed`
  from gtk-sys's build script with "waiting for other jobs to finish" — i.e. cargo
  had already verified the lock against the manifests (else `--locked` aborts
  before compiling) and was mid-compile. The tauri binary itself remains
  standing UNVERIFIED-BUILD (webkit system libs absent in sandbox) — no claim
  here depends on it linking.
- **Direct-dependency satisfaction** (all 5 direct + build deps resolve at
  semver-compatible versions, path crate versions match their manifests):
  see table §4.1; `lumen-desktop-tauri 0.1.3` = Cargo.toml:25, `desktop 0.1.3` =
  apps/desktop/Cargo.toml, `commands 0.1.0` = crates/commands/Cargo.toml,
  `tauri "2"` → 2.12.5, `tauri-build "2"` → 2.7.4, `serde "1"` → 1.0.229.
- **Checksum/format integrity** — `grep -c '^checksum = '` = 472 =
  `grep -c '^source = "registry'` = 472; every registry entry checksummed;
  `sed -n '1,3p'` shows `version = 4`; no git/patch/metadata sections
  (`grep -n '^\['` outside `[[package]]` = none); no duplicate (name,version)
  pairs (`sort | uniq -c` max count 1).
- **Workspace-exclusion boundary holds** — root `Cargo.toml:51` `exclude =
  ["apps/desktop-tauri"]`; root `Cargo.lock` contains 0 `tauri`/`wry` entries
  (`grep -c` = 0/0) and 0 `proc-macro-error` entries, making deny.toml:62-70's
  claim "the main workspace tree never sees it" TRUE (verified).
- **No orphan packages** — every lock entry traces into the graph; the one
  suspicious-looking leaf (`assert_matches`) is a real dependency of `rav1d
  1.1.0` (lock line 2721, verified), and `foldhash 0.2.0` traces to
  `dom_query 0.28.0 → tauri-utils → tauri` exactly as deny.toml documents.

## 3. Findings

### F-001 [S2] Dual-lockfile drift: the two shipped binaries compile the same first-party crates against diverged third-party graphs, and only one graph has any executed evidence
Where: apps/desktop-tauri/Cargo.lock (whole file) vs /home/z/my-project/Cargo.lock
Code:
```
name = "lumen-cli"
version = "0.1.3"
dependencies = [
 ...
 "toml 1.2.1+spec-1.1.0",
```
(Cargo.lock:1898-1917; the root lock pins the same `lumen-cli 0.1.3` source against `toml 1.1.8+spec-1.1.0`.)
What is wrong: The repo resolves the identical 21 first-party path crates twice,
in two lockfiles that nothing reconciles. Verified true drift (root's version
absent from this lock, same crate name): `cc` 1.10.0→1.11.1, `foldhash`
0.1.5→0.2.0 (a 0.x breaking-range bump, feeding `hashbrown`/`indexmap`), `itoa`
1.0.18→1.0.19, `log` 0.4.34→0.4.35, `serde_spanned` 1.1.2→1.1.3, `smallvec`
1.16.4→1.16.5, `syn` 3.0.9→3.0.10, `toml` 1.1.8→1.2.1, `toml_datetime`
1.1.2→1.1.3, `toml_writer` 1.1.3→1.2.0, `winnow` 1.0.4→1.0.5 (all verified by
`join` of both locks' name/version extractions). The workspace's executed
evidence (baseline runs #1,#3,#4,#5: 1681/1751 tests, clippy, deny) covers ONLY
the root graph; the lanes that would test this graph (ci.yml:731 `cargo test
--manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets`, ci.yml:743
deny, tauri-launch.yml:156) have zero committed run output in
`audit/runs/baseline/` — the tauri lane's only cited "CI run 43451838662" lives
in a deny.toml comment, which is not evidence.
Why it matters: `lumen-tauri` (shipped by release.yml) runs the same settings
parsing (`lumen-cli` → `toml`), logging (`log`), and serialization (`serde_json`
→ `itoa`) code against dependency versions no test suite has ever executed; a
regression in toml 1.2.x parsing would hit the GUI binary only, invisible to all
1681+ workspace tests. This is precisely the "GUI behaves differently from the
CLI twin" class the Wave-0 evidence already exhibits.
What correct looks like: either (a) a CI lane that runs the tauri graph's
`cargo test` on webview hosts with committed evidence plus a drift gate (e.g.
`cargo update --dry-run` diff between the two locks for shared crates), or (b)
pin shared-crate versions via a shared `[patch]`/workspace-global version
alignment so both locks resolve the same versions for the 21 shared first-party
crates' trees.
Confidence: HIGH — Verification: read (both lockfiles cross-joined; baseline
INDEX cited).

### F-002 [S3] The committed SBOM does not cover this lockfile: a shipped binary's 472-crate supply chain is undocumented
Where: apps/desktop-tauri/Cargo.lock (scope) vs docs/security/sbom/cyclonedx.json, docs/security/sbom/README.md:3-5
Code:
```
`cyclonedx.json` is the software bill of materials for the **resolved
Cargo.lock graph** (registry dependencies + workspace members), generated
by the std-only xtask subcommand:
```
(README.md:3-5; `grep -c 'tauri' docs/security/sbom/cyclonedx.json` = 0 over 159 bom-refs.)
What is wrong: The SBOM is scoped to the single root `Cargo.lock` graph (158
packages). This lockfile's closure — 472 registry crates including the entire
webview stack (webkit2gtk, gtk, glib, soup3), the HTTP stack (reqwest, hyper,
cookie, tower-http) and the crypto-adjacent edges (sha2, digest) — appears
nowhere in it, while release.yml ships `lumen-tauri` as a product binary.
Why it matters: an SBOM that silently omits one of the two shipped binaries'
dependency closures fails its entire purpose (supply-chain audit, CVE response);
a webkit/gtk CVE would never be answerable "are we affected?" from the committed
artifact.
What correct looks like: `cargo xtask sbom` gains a second subject (or a second
document) covering `apps/desktop-tauri/Cargo.lock`, and the drift gate checks
both.
Confidence: HIGH — Verification: read (grep of cyclonedx.json = 0 tauri mentions;
README scope statement; release.yml header citing lumen-tauri).

### F-003 [S3] Two waived unmaintained-advisory crates ride this lock, and this lock has never had an executed deny/test run
Where: apps/desktop-tauri/Cargo.lock:2653-2658 (proc-macro-error), 1219-1231 (glib-macros); deny.toml:69, deny.toml:61
Code:
```
name = "proc-macro-error"
version = "1.0.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "da25490ff9892aab3fcf7c36f08cfb902dd3e71ca0f9f9517bea02a73a5ce38c"
```
(`glib-macros 0.18.5` at Cargo.lock:1219-1231 lists `"proc-macro-error"` — the
RUSTSEC-2024-0370 edge; `paste 1.0.15` via `rav1d 1.1.0` carries
RUSTSEC-2024-0436. Both are ignored in deny.toml:61/69 with documented reasons.)
What is wrong: The advisories are informational ("unmaintained"), the waivers are
documented and honest — but the ONLY lane that would re-check this graph
(ci.yml:743 `cargo deny --manifest-path apps/desktop-tauri/Cargo.toml
--exclude-dev check bans licenses advisories`) has no committed run output
anywhere in `audit/runs/baseline/`; baseline #25 ran deny on the root workspace
graph only. The waiver conditions ("drop when the gtk-rs stack past 0.18
regenerates apps/desktop-tauri/Cargo.lock", deny.toml:69) are therefore
unenforced by any evidence.
Why it matters: waived advisories in a graph nobody re-scans rot: gtk 0.18.2 /
glib-macros 0.18.5 are frozen here indefinitely with no gate forcing the
promised regeneration.
What correct looks like: the tauri-check CI lane's deny output committed as a
baseline artifact (or REQUEST-ORCH-RUN executed at the wave gate), making the
waiver ledger auditable.
Confidence: HIGH — Verification: read (lock edges verified; deny.toml and
baseline INDEX cited; no deny run for this manifest exists in baseline).

### F-004 [S4] 52 of 441 distinct crates are locked at multiple versions
Where: apps/desktop-tauri/Cargo.lock (whole file); e.g. syn at Cargo.lock:3566 (1.0.109), 3576 (2.0.119), 3587 (3.0.10); getrandom at 1130/1141/1153
Code:
```
name = "syn"
version = "1.0.109"
```
(Cargo.lock:3566-3567; siblings 2.0.119 and 3.0.10 at 3576/3587.)
What is wrong: Not a defect — cargo permits it — but the duplication is heavy:
`syn` ×3, `getrandom` ×3 (0.2.17/0.3.4/0.4.3), `schemars` ×3 (0.8.22/0.9.0/1.2.3),
`proc-macro-crate` ×3, `toml_edit` ×3, `windows-sys` ×3 (0.45.0 from the gtk-0.18
legacy line, 0.52.0, 0.61.2), `windows-targets`/`windows_*` ×2 each, `thiserror`
×2, `bitflags` ×2, `base64` ×2, `indexmap` ×2, `hashbrown` ×2, `heck` ×2, plus 37
more (full list in §4.3).
Why it matters: compile-time, binary-size and audit-surface cost; the 0.45/0.42
windows line is dead weight on every non-Windows host of the GUI build.
What correct looks like: a periodic `cargo update -p <crate>` consolidation pass
when the gtk-rs 0.19+ line lands (same trigger deny.toml already names).
Confidence: HIGH — Verification: read (mechanical dup scan over all 493 entries).

### F-005 [S4] `zmij 1.0.23` — an (offline-)unverifiable serde_json dependency; `ryu` is absent from both lockfiles
Where: apps/desktop-tauri/Cargo.lock:5134-5138 (zmij), 3145-3155 (serde_json deps)
Code:
```
[[package]]
name = "zmij"
version = "1.0.23"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "29666d0abbfad1e3dc4dcf6144730dd3a3ab225bbbdac83319345b1b44ccfc1b"
```
(`serde_json 1.0.154` at Cargo.lock:3145-3155 lists deps `itoa, memchr, serde,
serde_core, zmij` — and `grep -c '^name = "ryu"'` = 0 in both lockfiles.)
What is wrong: Nothing provable — but this auditor cannot confirm a crate named
`zmij` exists on crates.io (no network in sandbox; SAFETY rule). It is consistent
across BOTH lockfiles (root Cargo.lock carries the identical
serde_json-1.0.154→zmij edge and checksum), and deny.toml:75-76 matter-of-factly
lists "zmij" among the MIT/Apache serde family, so the repo treats it as the
float-formatting successor to `ryu`. External provenance: UNVERIFIED.
Why it matters: an unrecognized leaf dependency of `serde_json` in a shipped
binary is exactly the typo-squat profile a supply-chain audit must be able to
clear; today it cannot be cleared from inside this sandbox.
What correct looks like: a deny/sources run against this manifest (proves the
registry origin + advisory status), and ideally a note in the dependency record
documenting what `zmij` is and when serde_json adopted it.
Confidence: MEDIUM — Verification: UNVERIFIED (offline); cross-lockfile
consistency and deny.toml:75 mention verified by read.
REQUEST-ORCH-RUN: cargo deny --manifest-path apps/desktop-tauri/Cargo.toml --exclude-dev check bans licenses advisories sources

## 4. Inventory tables (lockfile equivalent of the per-function table)

### 4.1 Direct dependencies (Cargo.toml → lock satisfaction)
| Cargo.toml requirement | Locked at | Line | Satisfies req? | Path version matches manifest? |
|---|---|---|---|---|
| `desktop = { path = "../desktop" }` | desktop 0.1.3 | 653 | YES (path) | YES (apps/desktop/Cargo.toml 0.1.3) |
| `commands = { path = "../../crates/commands" }` | commands 0.1.0 | 426 | YES (path) | YES (crates/commands/Cargo.toml 0.1.0) |
| `tauri = "2"` | tauri 2.12.5 | 3688 | YES (^2) | n/a |
| `serde = { version = "1", features=["derive"] }` | serde 1.0.229 | 3081 | YES | n/a |
| build: `tauri-build = "2"` | tauri-build 2.7.4 | 3739 | YES (^2) | n/a |
| (root of graph) | lumen-desktop-tauri 0.1.3 | 1922 | — | YES (Cargo.toml:25) |

### 4.2 First-party path packages in this graph (21)
audio-engine, audio-output, codec-api, codecs, commands, demux, desktop,
input, lumen-cli, lumen-desktop-tauri, media-clock, media-probe, media-types,
packet-queue, playback-engine, playlists, plugin-api, renderer-wgpu,
settings-schema, streaming, video-engine — all 0.1.0 except desktop /
lumen-cli / lumen-desktop-tauri at 0.1.3; every version matches its
manifest (`crates/*/Cargo.toml` loop verified). Workspace members correctly
ABSENT (not deps of `desktop`): xtask, lumen-mem-audit, lumen-sweep,
first-vertical, control-protocol, control-server, device-discovery,
localization, lumen-web, platform-{linux,macos,windows}, plugin-host-wasm,
product-config, subtitles, testkit, theme-tokens, transcode, template —
`apps/cli/Cargo.toml` references none of them (grep = 0), so the lock shape is
correct, and this confirms the GUI shell compiles the CLI twin's exact closure.
Notable shape fact: `renderer-wgpu 0.1.0` (Cargo.lock:2826-2830) depends only on
`media-types, video-engine` — **no wgpu anywhere in this lock** (`grep -c` = 0);
the GUI's engine-side presentation is the software-compositor path (matches
baseline `engine-info` feature `video.software-compositor`), and `wry`'s own
`softbuffer` is the only framebuffer layer present. No `tauri-plugin-*` crates
exist in the lock — the webview's IPC surface is exactly the app-registered
commands plus tauri/wry core (supports the shell-audit agents' scope).

### 4.3 Multi-version crates (52 of 441 distinct names)
base64 0.21.7/0.23.1 · bit-set 0.8.0/0.11.1 · bit-vec 0.8.0/0.10.2 · bitflags
1.3.2/2.13.2 · cssparser 0.37.0/0.38.0 · dom_query 0.28.0/0.29.0 · getrandom
0.2.17/0.3.4/0.4.3 · hashbrown 0.12.3/0.17.1 · heck 0.4.1/0.5.0 · html5ever
0.39.0/0.40.1 · indexmap 1.9.3/2.14.2 · jni-sys 0.3.1/0.4.1 · markup5ever
0.39.0/0.40.0 · miniz_oxide 0.8.9/0.9.1 · phf(+codegen/generator/macros/shared)
0.13.1/0.14.0 · proc-macro-crate 1.3.1/2.0.2/3.5.0 · r-efi 5.3.0/6.0.0 ·
schemars 0.8.22/0.9.0/1.2.3 · schemars_derive 0.8.22/1.2.3 · selectors
0.38.0/0.41.0 · serde_derive_internals 0.29.1/0.30.0 · serde_spanned
0.6.9/1.1.3 · servo_arc 0.4.3/0.5.0 · string_cache 0.9.0/0.11.0 ·
string_cache_codegen 0.6.1/0.11.2 · syn 1.0.109/2.0.119/3.0.10 · thiserror
1.0.69/2.0.21 · thiserror-impl 1.0.69/2.0.21 · toml 0.8.2/1.2.1 · toml_datetime
0.6.3/1.1.3 · toml_edit 0.19.15/0.20.2/0.25.19 · web_atoms 0.2.6/0.3.0 ·
windows-sys 0.45.0/0.52.0/0.61.2 · windows-targets 0.42.2/0.52.6 ·
windows_{aarch64_gnullvm,aarch64_msvc,i686_gnu,i686_msvc,x86_64_gnu,
x86_64_gnullvm,x86_64_msvc} 0.42.2/0.52.6 each · winnow 0.5.40/1.0.5.
(No identical (name,version) duplicates — lock is structurally valid.)

### 4.4 Cross-lockfile drift (root lock version → this lock version; verified absent-in-this-lock)
cc 1.10.0→1.11.1 · foldhash 0.1.5→0.2.0 · itoa 1.0.18→1.0.19 · log
0.4.34→0.4.35 · serde_spanned 1.1.2→1.1.3 · smallvec 1.16.4→1.16.5 · syn
3.0.9→3.0.10 · toml 1.1.8+spec-1.1.0→1.2.1+spec-1.1.0 · toml_datetime
1.1.2→1.1.3 · toml_writer 1.1.3→1.2.0 · winnow 1.0.4→1.0.5. (Additional
older-version baggage only in this lock — e.g. bitflags 1.3.2, base64 0.21.7,
thiserror 1.0.69 — is additive, not drift.)

## 5. Test honesty
No tests live in a lockfile. The relevant lanes that would exercise this graph —
ci.yml:731 (`cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked
--all-targets`), ci.yml:733 clippy, ci.yml:743 deny, tauri-launch.yml:156/345/490
build+smoke — have NO committed execution evidence in `audit/runs/baseline/`
(INDEX lists exactly one tauri-manifest run: the failed build #2). Status: the
tauri graph is CI-declared but evidence-orphaned (see F-001/F-003).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the main workspace graph stays exactly the pre-admission posture … 'NOT imported in the workspace lockfile' (records/tauri-wry-2.x.md)" | apps/desktop-tauri/Cargo.toml:5-11 | REAL | `grep -c '^name = "tauri"' Cargo.lock` = 0, `wry` = 0 (root lock); root Cargo.toml:51 `exclude = ["apps/desktop-tauri"]` |
| "wry + tao ride its own graph in THIS crate's lockfile" | apps/desktop-tauri/Cargo.toml:58-59 | REAL | wry 0.57.2 (Cargo.lock:4961), tao 0.37.2 (3631) present here, absent from root lock |
| "Version '2' per the record's admission range" (tauri) | apps/desktop-tauri/Cargo.toml:57-58 | REAL | tauri 2.12.5 (Cargo.lock:3688) |
| deny.toml:61-69: RUSTSEC-2024-0370 enters "ONLY through the ADR-004 admission subcrate's lock (glib-macros 0.18.5 → glib/gtk → muda → tauri 2.12.5)" | deny.toml:62-70 | REAL | glib-macros 0.18.5 → proc-macro-error edge verified (Cargo.lock:1219-1231); tauri 2.12.5 → muda verified; proc-macro-error absent from root lock (grep = 0) |
| deny.toml:122-130: "Zlib — foldhash 0.2.0 (dom_query → tauri-utils → tauri 2.12.5)" | deny.toml | REAL | dom_query 0.28.0 deps include "foldhash" (Cargo.lock:741-755); foldhash 0.2.0 at 908 |
| "cargo-deny-check covers this graph" (implied by ci.yml:743 lane) | .github/workflows/ci.yml:743 | REAL-UNPROVEN | lane exists in config; zero executed output in baseline (INDEX #25 = root graph only) |
| SBOM = "the resolved Cargo.lock graph" | docs/security/sbom/README.md:3-5 | SHALLOW (covers one of two shipped graphs) | cyclonedx.json: 159 bom-refs, 0 "tauri" mentions (F-002) |

## 7. Dead / duplicated / unreachable code
- No orphan lock entries (all 493 trace into the graph; assert_matches→rav1d
  Cargo.lock:2721, zmij→serde_json Cargo.lock:3154, foldhash→dom_query
  Cargo.lock:748 — all verified edges).
- Duplication: §4.3 (52 multi-version crates) — build-cost duplication, not
  unreachable code.
- Notable absence (context for the shell/frame-audit agents): zero
  `tauri-plugin-*` crates, zero `wgpu`, zero `cpal`/`symphonia`-in-this-lock —
  the GUI engine closure is the CLI twin's exact set (§4.2).

## 8. Dependencies & flags
- Lockfile format `version = 4` (Cargo.lock:3) — requires Cargo ≥1.78; repo pins
  Rust 1.99.1 (baseline INDEX header). Consistent.
- 472 registry entries, 472 checksums, 0 git sources, 0 `[[patch]]`, 0
  `[metadata]`. Registry host is exclusively
  `registry+https://github.com/rust-lang/crates.io-index` (472/472).
- Committed-lock policy honored: `.gitignore:91-92` "Cargo.lock is COMMITTED —
  application workspace; doc 11 §3.7 'Cargo.lock is committed; builds are
  --locked'"; only `Cargo.lock.bak` is ignored (line 7). File present in tree at
  123,356 bytes. (Git-tracking itself UNVERIFIABLE this session — no-git
  deviation; presence + policy cited.)
- Feature-flag interplay: nothing in a lockfile reveals features, but the lock's
  shape (no wgpu, software stack only) matches the shipped features
  `first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor`
  cited in baseline INDEX.

## 9. Open questions for the next agent
1. Shell/IPC auditors (app.rs, tauri.conf.json, capabilities): the lock confirms
   NO tauri plugins and NO wgpu — so any video display must be app-side
   (canvas/custom protocol) or absent; reconcile with Wave-0 "GUI mounts no
   <canvas>".
2. Whoever audits `records/tauri-wry-2.x.md` / `INFRA-BLOCKED.md`: confirm the
   documented CI run ids (43451838662, 43422427483 in deny.toml comments) point
   at real, passing runs — this audit could not (comments are not evidence).
3. Wave-16/gate owner: execute the REQUEST-ORCH-RUN deny command so F-003/F-005
   (waiver ledger + zmij provenance) get evidence.
4. tauri-launch.yml's smoke lane asserts a DISPLAY is not needed for
   compile/test — that claim's runtime half belongs to the launch-lane audit.
