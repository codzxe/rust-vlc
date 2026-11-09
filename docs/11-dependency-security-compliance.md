# 11 — Dependency, Security & Compliance Program

| | |
|---|---|
| **Implements** | MASTER-PROMPT §9 (Rust-only policy & dependencies — primary), §12 (security — primary), §14 (known limits: codec patents, licensing), §2 (clean-room rule, hardware-decode ADR exception), §13 (audit CI gates), §5 (telemetry opt-in only) |
| **Traces to** | doc 01 §5 (unsafe-code scans, ADR registry), §7 (ADR-001/002/004/005/006/007/010), doc 02 §6 (ADR briefs), doc 04 §1.2 (tarball purity method), §3.2/§4.2–4.4 (codec patent pointers), doc 05 §5.4 (control server), §2.13 (error redaction), doc 06 §5–6 (WASM host, forbidden-list enforcement), §9.4 (sandbox security tests), doc 10 §4.6 (gates G3/G4/G10), §5 (corpus licensing ledger) |
| **Consumed by** | doc 13 (wave program — every dependency-adding task), doc 14 (risk register), the ADR-006/007/010 authors, and every implementation agent that proposes a new crate |
| **Status** | Planning document (Wave 1, Task 1.C11). Facts verified by doc 04's 2026-10-19 crates.io/tarball inspection are cited as such; everything else is policy specification marked `[PROPOSAL]` or unverified fact marked `[VERIFY]`. Nothing here has been executed. |

## 0. Scope and how to use this document

This is the single specification for (a) how dependencies enter the repo, (b)
what security requirements bind every crate, and (c) how clean-room and
licensing obligations are discharged. MP §9 and §12 are normative; this
document turns them into procedures, records, gates and testable requirements.

**Who must read what:** any agent proposing a dependency reads §1–§4 and files
a record per §2. Security reviewers and the Lead read §5–§7. The Lead and
counsel read §8 before ADR-006/010. Release work reads §9. §10 lists decisions
requested.

---

## 1. The dependency policy as a process (MP §9)

### 1.1 The forbidden list as an actionable checklist

MP §9's forbidden list becomes checklist **F1–F7**, evaluated for every crate,
direct or transitive, on every target we build (including `wasm32`):

| # | Check (answer must be NO) | How it is checked |
|---|---|---|
| F1 | Does the crate contain C, C++ or Objective-C **sources**? | Tarball scan for `*.c *.cc *.cpp *.cxx *.h *.hpp *.m *.mm` (§4.1) |
| F2 | Does its **build script** invoke a C/C++/assembly toolchain? | `build.rs` inspection for `cc`, `clang`, `gcc`, `cl.exe`, `clang-cl`, `nasm`, `yasm`, `masm`, `cmake`, `make`, `xcrun` (§4.1 P5) |
| F3 | Is it a **wrapper around a third-party C/C++ media engine** (libav*, libopus, libvorbis, libass, openh264, dav1d-asm, x264…)? | Manifest `links=` key + `extern "C"` scan + name review (doc 04 §1.2 names the forbidden families) |
| F4 | Does it **vendor or download native code** at build time (bundled `.a`/`.lib`/`.o`, `git clone` of C sources, `curl` in build.rs)? | Tarball scan for binary blobs + build-script download tokens |
| F5 | Does it introduce **hidden transitive native dependencies** (a pure-looking facade whose `-sys` dep does the native work)? | Full `cargo tree -e normal,build` against the allowlist (§3.6) |
| F6 | Are we **bundling a native codec and calling it Rust**? | Same evidence as F1–F5; marketing rule MP §2 ("never all-Rust") |
| F7 | Hand-written **assembly** (`.S`, `.asm`, nasm/masm)? | Treated as native code: forbidden by default (doc 04 §4.4 rav1d precedent — assembly is not C, but it is non-Rust and must not sneak in without an ADR) |

A crate passes only when **all seven answers are NO**, or it is covered by an
exception below.

### 1.2 The two documented exception classes

MP §9 allows exactly two exception classes. Each instance requires an ADR with
(1) the **exact API surface list** (names, headers/frameworks/ABI), (2) a
**safety review** (invariants, unsafe scope, crash behavior), and (3) truthful
reporting of when it is active.

**E1 — OS webview (UI shell).** ADR-004 (doc 01 §7). The Tauri webview
(WebView2 / WKWebView / WebKitGTK) is outside the Rust-only rule because the UI
is React+TS by fixed decision (MP §2). The product is never described as
all-Rust. Security scope of the ADR: webview sandbox posture, IPC surface,
navigation/content isolation (see §5 TB3).

**E2 — Audited OS-service bindings (engine).** ADR-001 (hardware decoders:
VideoToolbox, Media Foundation, VA-API), ADR-005 (audio output), plus
windowing/GPU surface APIs under ADR-003/004. These call **OS-provided
services**, not vendored C code. The WebCodecs browser API is the web analog
and needs its own ADR scoping (doc 04 §4.4, doc 09 §5) `[PROPOSAL — pending
Phase 0]`.

**Boundary note on platform-ABI crates `[PROPOSAL]`:** every Rust binary on
some targets links the platform libc via the `libc`/`rustix` crates (Rust
declarations, no build.rs compiling C, nothing vendored). These are part of
the OS-ABI surface, not forbidden third-party engines. Proposal: a
**pre-approved registry entry** (E2a: `libc`, `rustix`, and equivalents)
recorded once with an audit, rather than per-use ADRs. Lead decision
requested (§10 Q4).

**Proposed-admission register — rustls TLS crypto provider `[PROPOSAL —
pending Lead admission, ADR-011]`:** the streaming transport's crypto core
(ureq 3.5.0 + rustls; record `deps/records/ureq-3.5.0.md` R10 — *admitted
conditionally on the Lead accepting the recorded ring deviation*). Fresh
audit 2026-11-09, real cargo data (ADR-011 §"The audit"): `rustls =
"0.23"` resolves **rustls 0.23.46, 35 packages locked, 19-crate build
graph**; the **default provider is aws-lc-rs 1.19.0** → aws-lc-sys 0.47.0
(a C/C++ build chain: cc 1.11.1, cmake 0.1.58, pkg-config 0.3.34,
fs_extra, dunce), while the alternate — **activated in our tree via ureq's
`rustls` feature — is ring 0.17.14**: 135 C/asm files, cc-only build, 230
unsafe tokens, all inside the crypto core (record R1/R8). No production
pure-Rust provider exists; native-tls is platform linkage (E-ineligible
and card-banned). Proposal: register ring as the **E2-class
audited-native-crypto-core exception** (the rav1d-asm posture, §2.4); the
Lead decides in ADR-011 — acceptance deletes the seam, rejection flips
`HTTPS_ADR011_ADMITTED` to the typed refusal.

### 1.3 The intake procedure (what an agent does)

1. Check the crate against F1–F7 using the §4.1 method *before* proposing it.
2. File an audit record (§2) into the dependency-audit PR; link the evidence.
3. Lead reviews, approves a version range, and the allowlist row (§3.1) is the
   merge gate. If no compliant implementation exists, document the limitation
   and continue — the rule is never quietly relaxed (MP §9).

---

## 2. The audit record format

### 2.1 Fields (R1–R10) — recorded before acceptance, updated on re-audit

| Field | Content | Evidence source |
|---|---|---|
| R1 Language & source | Rust/pure claim vs reality; repo URL, maintainers, crates.io publisher | Tarball inspection, repo |
| R2 Transitive graph | Full `cargo tree -e normal,build` output at the pinned version, every entry allowlisted | `cargo tree` |
| R3 Build scripts | Presence/absence of `build.rs`; token-level analysis of what it invokes (flag `cc`/`clang`/`nasm`/`cmake`/`bindgen`/`pkg-config`) | Tarball |
| R4 Native links | `links=` manifest keys; `extern "C"`/`extern "system"`/`#[link]` declarations; binary blobs (`.a .lib .o .obj .so .dll .dylib`) | Tarball grep |
| R5 Targets | x86_64/aarch64 Windows/Linux/macOS + wasm32 status; target-conditional native code (cfg gates in build.rs) | Manifest, CI matrix |
| R6 **Actual feature coverage** | What the crate verifiably does vs what our format record needs — a Rust API does not prove purity, and a version number does not prove coverage. Cite corpus/tests/limitations | Doc 04 findings, own tests |
| R7 License | SPDX expression, file in tarball, compatibility with the allowlist (§3.2) | Tarball, cargo-deny |
| R8 Maintenance & security record | Release cadence, open advisories, RustSec history, maintainer count/bus factor | crates.io, RustSec `[VERIFY]` |
| R9 Provenance | Tarball sha256 vs crates.io index; publisher identity; any VEX/vet notes | crates.io API, cargo-vet |
| R10 Verdict | `accepted` (version range) / `conditional` (feature pin, e.g. rav1d no-asm) / `rejected` (which F-checks failed) | — |

Records live in `deps/records/<crate>-<version>.md` `[PROPOSAL: layout]` and
are linked from the allowlist.

### 2.2 Filled example — accepted: `mp4parse` 0.17.0

> Evidence drawn from doc 04's verified inspection (2026-10-19). Fields not
> covered there are marked `[VERIFY-P0]` — even an accepted record may carry
> open verification items; they are closed during the Phase 0 audit, not waived.

```markdown
# deps/records/mp4parse-0.17.0.md
R1  Language/source: Rust, pure (tarball: no .c/.cpp/.h/.S/.asm found,
    no build.rs present). Repo github.com/mozilla/mp4parse-rust; Mozilla.
R2  Transitive graph: byteorder, bitreader, log, static_assertions
    [VERIFY-P0: regenerate exact pinned graph; confirm no build-deps].
R3  Build scripts: none — manifest has no `build` key (tarball-verified).
R4  Native links: none; no links=, no extern "C" declarations, no blobs.
R5  Targets: all desktop triples; wasm32 [VERIFY-P0].
R6  Actual coverage: production Firefox MP4 metadata/sample-table parser
    (moov/stsd/stts/stsc/stco, moof fragments). It is NOT a player-facing
    demuxer: seek/index API thin; QuickTime legacy atoms (cmov, old stsd
    entries) not guaranteed; fMP4 progressive-download patterns need our
    corpus tests. Use: cross-check oracle in tests + atoms Symphonia's
    isomp4 rejects (doc 04 §2.2). Engine demuxer remains symphonia-format-isomp4.
R7  License: MPL-2.0 (allowlisted; file-level MPL is fine — we link, not
    modify; modifications if any stay in vendored-file notice).
R8  Maintenance: shipped in Firefox release trains; active upstream
    [VERIFY-P0: cadence + RustSec query at audit date].
R9  Provenance: sha256 recorded from static.crates.io download; crates.io
    publisher Mozilla [VERIFY-P0: record hash].
R10 Verdict: ACCEPTED for test-oracle + metadata use, range ">=0.17, <0.18".
    Approved by: Lead (pending Phase 0 confirmation of R2/R5/R8/R9).
```

### 2.3 Filled example — rejected: the `ffmpeg-sys`-family wrapper

> Illustrative of the class MP §9 forbids ("third-party C/C++ media engines
> behind Rust wrappers"). Specifics of any one crate `[VERIFY]`; the rejection
> stands on the policy checks regardless.

```markdown
# deps/records/ffmpeg-sys-next-REJECTED.md
R1  Language/source: Rust *bindings* crate around the FFmpeg C libraries.
    A Rust API does not prove pure Rust — this is the canonical example.
R2  Transitive graph: pulls bindgen, cc, pkg-config (build graph is itself
    native tooling). → F5 FAIL.
R3  Build scripts: build.rs invokes bindgen over FFmpeg headers and
    pkg-config/lib discovery; may build/download FFmpeg when vendored
    features are on. → F2 FAIL, F4 FAIL.
R4  Native links: links="avcodec avformat avutil swresample swscale";
    all APIs reached through extern "C". → F1/F3 FAIL (the engine IS C/C++).
R5  Targets: desktop only; wasm32 impossible.
R6  Coverage: would cover nearly every codec/demuxer VLC has — which is
    exactly why it is tempting and exactly why it is forbidden: it is
    "bundling a native engine and calling the result Rust" (F6).
R7  License: LGPL-2.1+ by default, GPL with features — also off-allowlist
    pending ADR-006.
R10 Verdict: REJECTED. F1–F6 all fail. No ADR can cure this class (only
    classes E1/E2 exist); the compliant responses are the pure-Rust
    candidates of doc 04 or a documented gap.
```

### 2.4 Conditional example — `rav1d` 1.1.0 (shows the nuance)

Verdict template: **conditional**. Default build **fails F2/F7** (ships ARM
`.S` and x86 nasm built via `cc`/`nasm-rs` — doc 04 §4.4, tarball-verified).
Compliant only with `default-features = false, features = []` (asm off, Rust
fallbacks), which must be pinned via workspace dependency inheritance and
re-verified in CI (`[VERIFY-P0]`: zero non-Rust build activity with asm off,
plus benchmark of the speed cost). The record's version range is bound to the
feature pin; any feature change re-triggers audit (§3.7).

---

## 3. Allowlist and tooling

### 3.1 The allowlist file

`deps/allowlist.toml` `[PROPOSAL: layout]` is the machine-checked single
source of approval. One row per approved crate, consumed by G3:

```toml
version = 1  # allowlist schema version

[[crate]]
name = "mp4parse"
record = "deps/records/mp4parse-0.17.0.md"
version-range = ">=0.17.0, <0.18"     # semver range approved
features = []                          # if set: ONLY this feature set allowed
approver = "Lead"                      # AGENTS §1: dependency approval is Lead-only
date = "2026-10-19"
notes = "test-oracle + metadata; engine demuxer is symphonia-format-isomp4"

[[crate]]
name = "rav1d"
record = "deps/records/rav1d-1.1.0.md"
version-range = ">=1.1.0, <1.2"
features = "no-default-features"      # asm OFF — see record §2.4
approver = "Lead"
date = "…"
```

Rules: a crate in the lockfile with **no allowlist row fails G3**; a version
outside the range fails; a feature-set mismatch fails; `approver` must be the
Lead (delegation policy — §10 Q10). Dev-dependencies follow the same policy
(build-time code execution is in the trust boundary — see §4.3).

### 3.2 cargo-deny configuration sketch

`deny.toml` `[PROPOSAL — exact keys verified when the file lands]`:

```toml
[advisories]
version = 2
yanked = "deny"

[licenses]
version = 2
confidence-threshold = 0.93
# Aligned with ADR-006's pending decision; GPL/LGPL deliberately absent
# until the Lead decides (oxisynth LGPL-2.1 case, doc 04 §3.1).
allow = ["MPL-2.0", "MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception",
         "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "CC0-1.0",
         "Unicode-3.0", "Unicode-DFS-2016", "BSD-2-Clause-Patent"]

[bans]
multiple-versions = "warn"     # -> "deny" after initial dedup [PROPOSAL: date]
highlight = "all"
wildcards = "deny"             # crates.io wildcard/unspecified licenses
deny = [                       # named poison — belt & braces behind F3
  { name = "ffmpeg-sys-next" }, { name = "ffmpeg-sys" },
  { name = "libloading" },     # doc 06 §6: no native dynamic loading
  { name = "bindgen" }         # F3 signal: bindings-generation implies C headers
]

[sources]
unknown-registry = "deny"      # crates.io only; no git deps, no private registries
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

Note honestly: **cargo-deny cannot see inside `build.rs`** — it checks
manifests, licenses, advisories and sources. The native-code ban is enforced
by our own scanner (§4.2), which is why G3 runs both.

### 3.3 cargo-vet initialization

`cargo vet` proves provenance/audited review of each crate version
`[PROPOSAL — strategy]`:

1. **Bootstrap (Phase 1):** import every currently-locked crate as audited
   with `audit-as-crates-io = true` only for crates that have a §2 record
   (doc 04's verified set, completed in Phase 0 task 0.A9 — doc 02 §7).
   Unrecorded crates are marked `unaudited` with expiry dates, driving the
   Phase 0 audit queue.
2. **Steady state:** new dependency ⇒ new `audits.toml` entry (who, when,
   what was inspected: tarball sha256 + record link). The vet store is
   committed; CI (`cargo vet check --locked`) fails on unaudited versions.
3. **Scope:** both `normal` and `build` dependency classes are vetted
   (build-scripts execute on our machines).

### 3.4 SBOM generation and storage

Per release (and per `main` build, retained 30 days `[PROPOSAL]`):

- **Rust graph:** `cargo-cyclonedx` → CycloneDX 1.5 `sbom-rust.json`, built
  from the exact lockfile; `cargo auditable` embeds the dependency list into
  the binaries so the shipped artifact self-reports (survives source loss).
- **wasm build:** its own SBOM (the wasm graph differs, MP §10).
- **UI:** `@cyclonedx/cyclonedx-npm` over the pnpm lockfile → `sbom-ui.json`
  `[PROPOSAL]`.
- **Storage:** release tag artifact + evidence store naming per doc 10 §4.5
  (`evidence-<job>-<os>-<run_id>`), plus `docs/releases/<version>/`. SBOMs
  list license + version + hash so a later advisory can be retro-mapped to
  affected releases.

### 3.5 CI job order (doc 10 G-gates)

The `audits` job (linux, required check, runs on every PR — doc 10 §4.6 G3/G4)
executes in this order, cheap-first:

1. **Lockfile/tree diff** (§3.6) — fast-fail on unreviewed additions.
2. `cargo deny check licenses sources bans` (needs the allowlist import).
3. `cargo vet check --locked`.
4. `tools/dependency-audit --deny-native` — the tarball scanner (§4.2).
5. `cargo deny check advisories` (RustSec) — generates the auto-issue for
   dev-only advisories per doc 10 G4.
6. SBOM generation (on tags/main only).

The `audits` job is independent of the build/test matrix (parallel, both
merge-blocking) so a failing audit cannot be masked by green tests.

### 3.6 The `cargo tree` diff gate

`deps/tree-baseline.txt` is a committed artifact produced by
`cargo tree --workspace -e normal,build --prefix depth --charset ascii`
plus the same for `--target wasm32-unknown-unknown` `[PROPOSAL]`. CI
regenerates and diffs. **Any delta (addition, version change, removal) must
co-occur with a matching allowlist/record change in the same PR** — the
checker correlates tree-diff entries with changed `deps/` files and fails on
orphans. This is the "no hidden transitive native deps" tripwire (F5): a
facade crate pulling a `-sys` dep shows up in the tree even when Cargo.toml
looks clean.

### 3.7 Version pinning and update policy

- `Cargo.lock` is committed; builds are `--locked` in CI. The wasm and UI
  lockfiles are likewise committed.
- **Who bumps:** only the Lead merges dependency-update PRs (AGENTS §1).
  Automation (Dependabot/renovate `[PROPOSAL]`) may open PRs, never merge.
- **What re-triggers an audit:** any version change of an engine-graph crate;
  any **new transitive dependency** (caught by §3.6); a changed `build.rs` or
  `links=` between versions (the scanner diffs build scripts across the bump
  `[PROPOSAL]`); a license-file change; a RustSec advisory touching the
  current version; a feature-set change (§2.4).
- **Re-audit depth:** automated fields refresh (R2–R5, R7, R9) from the
  tarball; human fields (R6 coverage delta, R8 maintenance) are reviewed via
  the crate changelog; majors always get a fresh full record.

---

## 4. The purity verification method (doc 04 §1.2, codified)

### 4.1 Manual procedure (the authoritative check)

`P1–P8`, performed per crate version, producing the record of §2:

- **P1 Pin.** Take the exact version from `Cargo.lock`. Fetch the tarball
  `https://static.crates.io/crates/<name>/<ver>/<name>-<ver>.crate`.
- **P2 Verify.** Check the sha256 against the crates.io index record.
- **P3 Unpack** into a scratch directory. **Never build or execute the crate
  under audit** — inspection is static.
- **P4 Enumerate native sources:** `*.c *.cc *.cpp *.cxx *.h *.hpp *.m *.mm
  *.S *.s *.asm *.nas *.sx` anywhere in the tarball (including tests/benches
  only if wired into build).
- **P5 Inspect the build script** (`build.rs` or the manifest's `build` key):
  flag tokens `cc`, `gcc`, `clang`, `cl.exe`, `clang-cl`, `nasm`, `yasm`,
  `masm`, `cmake`, `make`, `ninja`, `xcrun`, `bindgen`, `pkg-config`, and
  network fetch (`curl`, `wget`, `git clone`, `download`).
- **P6 Inspect the manifest:** `links = "…"` (native library by definition);
  vendored directories; `[patch]`/`[replace]` remnants; `include`/`exclude`.
- **P7 Scan sources:** `extern "C"`, `extern "system"`, `#[link]`, and
  binary blobs by extension (`.a .lib .o .obj .so .dll .dylib .wasm`-payload
  is fine only for the plugin host's own samples).
- **P8 Record** findings per R1–R10; verdict per §1.1.

This is the method doc 04 used (verified 2026-10-19) and it remains the
**authoritative** check — CI (below) is an approximation whose job is to
catch drift, not to replace the human record.

### 4.2 Automated CI approximation `[PROPOSAL]`

`tools/dependency-audit` (the `--deny-native` mode wired into G3):

1. `cargo metadata --format-version 1 --locked` for each target; walk every
   resolved package (normal + build classes).
2. For each crate **with a build script**, fetch its pinned tarball (Cargo's
   own cache in CI; checksums already enforced by cargo) and run P4–P7
   token scans.
3. Fail when: native sources present; compiler-invocation tokens in
   `build.rs`; `links=` present without an E1/E2 ADR reference in the
   exception registry; binary blobs present — **unless** the crate+feature
   set is in the allowlist with a conditional verdict (§2.4) whose pinned
   features provably avoid the native path.
4. Emit `native-scan.txt` (doc 10 G3 evidence format).

### 4.3 Known blind spots and compensating controls

| Blind spot | Why it evades §4.2 | Compensating control |
|---|---|---|
| **Proc-macros executing arbitrary code at compile time** | A proc-macro is Rust that *runs* in our build; it could write native code to `OUT_DIR` or embed blobs from its own tarball | Proc-macro crates get **Tier-1 scrutiny**: full record + maintainer review; tarball blob scan still applies (P7); cargo-vet provenance; small, closed set — any addition is a Lead decision `[PROPOSAL]` |
| **Target-conditional build scripts** | CI scans the linux triple; a build.rs may compile C only on windows/macos | Run the native scan in **all three OS jobs** `[PROPOSAL]`; P5 records `cfg(target_os)` gates in the record; wasm graph scanned in G12 job |
| **Feature-switched native code** (rav1d asm) | Tokens exist in build.rs but are cfg'd off | Allowlist pins the feature set; xtask asserts `default-features=false` inheritance; scanner verifies the *resolved* feature set, not just presence of tokens |
| **Git/path/patch sources** | Bypass registry checksums and tarball publication | Denied structurally: `deny.toml [sources]` unknown-git/registry deny, `allow-git = []`; `[patch]` sections linted by xtask |
| **include_bytes! binary blobs** | Not a "build script" signal | P7 extension scan flags binary library files in the tarball regardless of reference mechanism |
| **Workspace-inherited deps** | Mask true origin of a version bump | Tree baseline (§3.6) is computed from the resolved graph, not from manifests |
| **Prebuilt artifacts shipped as "test data"** | Skipped as fixtures | Scanner applies to the whole tarball except paths the record explicitly justifies |

Residual truth: **static inspection cannot prove absence of malicious
compile-time behavior**; the allowlist + vet + Lead approval chain is the
compensating organizational control (defense in depth, not proof).

---

## 5. Threat model (MP §12)

MP §12: *"Treat all media, playlists, subtitles, metadata, network responses
and control messages as untrusted."*

### 5.1 Assets and trust boundaries

**Assets:** user media files and their paths; credentials embedded in URLs
(`user:pass@host` in playlists/streams); playback/queue history implied by
playlists; arbitrary local files (read-probe value); network responses
(playlists, HLS/DASH manifests, headers); the control surface (control
server, IPC); engine process integrity (memory safety, availability); the
UI/webview (XSS via crafted metadata/subtitles); the extension system; the
build pipeline (supply chain).

**Trust boundaries:**

| TB | Boundary | Crosser |
|---|---|---|
| TB1 | media bytes → engine parsers | demux/codec/probe code |
| TB2 | engine → UI | typed command layer (doc 05) |
| TB3 | UI → OS webview | React DOM, IPC, any rich text |
| TB4 | engine ↔ WASM extensions | versioned imports (doc 06 §5.2) |
| TB5 | engine ↔ network | streaming crate, TLS |
| TB6 | control server ↔ local clients/browser pages | control-protocol (doc 05 §5.4) |
| TB7 | engine ↔ OS services | platform crates (E2 ADRs) |
| TB8 | repo ↔ crates.io | the dependency pipeline (§1–§4) |

### 5.2 Adversary personas

- **A1 Malicious media file** — memory-safety/correctness attack via parser
  or decoder; decompression bombs; pathological timestamps driving unbounded
  buffering.
- **A2 Malicious playlist/subtitle with path tricks** — traversal to open
  arbitrary files (read oracle: does it play? duration leaks), UNC/network
  paths turning the player into an intranet prober; sidecar substitution.
- **A3 Malicious WASM extension** — sandbox escape, permission escalation,
  resource exhaustion, provenance forgery (doc 06 §5.3/§5.7).
- **A4 Network MITM** — TLS downgrade/strip, manifest injection mid-stream,
  credential theft on cross-origin redirect.
- **A5 Local control-port prober / drive-by webpage** — browser JS probing
  `127.0.0.1:<port>` (including DNS rebinding to reach the WS endpoint),
  token brute force, other local unprivileged processes.
- **A6 Compromised or typosquatted dependency** (supply chain) — malicious
  update, lookalike name, hijacked maintainer account.
- **A7 Slow/hostile server** — oversized claims (headers declaring GB of
  samples), slow-loris unbounded buffering, redirect loops.

### 5.3 Per-boundary threats and mitigations

| TB | Threats (persona) | Mitigations (→ requirement §6) |
|---|---|---|
| TB1 | parser memory bugs, bombs, pathological timestamps (A1, A7) | bounds SEC-01/02/03; int safety SEC-04; fuzzing SEC-11; no-unsafe lint SEC-14 |
| TB2 | command injection/shape abuse (A5 via TB6, A3 via panels) | schema validation at boundary, typed results (doc 05 §1.2), no reflection |
| TB3 | XSS via metadata/subtitle text rendered as HTML (A1/A2) | declarative rendering only — no innerHTML path for metadata/subtitles; React escaping; subtitle renderer consumes styled-token trees SEC-12 |
| TB4 | escape, escalation, exhaustion, forged provenance (A3) | doc 06 §5.2–5.7: separate linear memories, capability imports, limits, hash/signature, wasm-magic fence |
| TB5 | MITM, credential leak, bomb manifests (A4, A7) | SEC-07 TLS, SEC-08 redirect/credential policy, SEC-01/05 bounds+deadlines |
| TB6 | unauthenticated control, replay, origin abuse (A5) | §7 hardening: loopback, token, validation, rate limits, origin allowlist |
| TB7 | malformed OS-API misuse, hw-decoder attack surface (A1) | E2 ADR safety reviews; validated buffers across the ABI; error containment per MP §8 |
| TB8 | poisoned dependency (A6) | §1–§4 pipeline: allowlist, vet, tarball scan, tree diff, Lead-only merges |

### 5.4 STRIDE summary

| Class | Top exposure | Primary control |
|---|---|---|
| Spoofing | fake control client; forged plugin origin/publisher | token auth (§7); provenance checks (doc 06 §5.7) |
| Tampering | media/manifests in transit; extension files on disk; config | TLS SEC-07; hash+signature; layered config precedence (MP §5) |
| Repudiation | control-server actions | audit log (§7) |
| Info disclosure | credentials in logs/errors; path leakage in errors | SEC-09 redaction at the source trait; doc 05 §2.13 typed errors |
| DoS | every parser + queues + control port + extensions | SEC-01/02/03/05; §7 rate limits; doc 06 §5.4 limits |
| Elevation | plugin escalation; path traversal; unsafe code | capability imports (doc 06 §5.3); SEC-06 sandbox prefixes; SEC-14 forbid(unsafe) |

---

## 6. Security requirements catalog

Each requirement is testable; test IDs map into doc 10's suites (and doc 06
§9.4 for plugins). **SEC-01..SEC-16** `[PROPOSAL: numbering stable once
merged]`.

| ID | Requirement (traced) | Test |
|---|---|---|
| SEC-01 | **Bounded memory** everywhere: decode buffers, caches, preallocation are capped by `product.toml` limits; a header claiming 2⁶⁰ samples allocates nothing large (MP §5, §12) | `sec_oversized_claims`: bomb fixtures assert peak-RSS ceiling |
| SEC-02 | **Bounded probing depth** and probe byte/time budgets; recursion/nesting caps in demuxers and playlist parsers (MP §12) | `sec_probe_depth`: nested/composite probes error within budget |
| SEC-03 | **Bounded queues with backpressure**; stalled sink never grows a queue unboundedly (MP §8; doc 03) | `sec_queue_backpressure`: stall sink, assert cap + event |
| SEC-04 | **Safe integer conversions** — `[PROPOSAL: try_from discipline]`: `u32::try_from`, checked arithmetic in all parser/codec crates; no numeric `as` casts in `demux/ codecs/ media-probe/ subtitles/ playlists/` (clippy `cast_possible_truncation` denied there, audited by grep elsewhere) | lint G5 + review checklist; property tests on time-base math (doc 03 §7) |
| SEC-05 | **Deadlines + cancellation on all I/O**: every source/track/hw call carries a deadline; seek/stop/track-switch cancel stale work via generation IDs (MP §8, §12; doc 05 §4.2) | interrupted-network suite, stale-generation races (doc 10 G11) |
| SEC-06 | **Safe path handling**: no traversal from playlists/subtitles — canonicalize + sandbox-prefix check; relative refs resolve only within the playlist's directory tree (or a user-granted root); UNC/file-URL/network paths require explicit opt-in `[PROPOSAL]`; subtitle sidecar lookup restricted to the media file's directory | `sec_path_traversal` vector suite (§9.1) |
| SEC-07 | **TLS verification always on** for `https` sources: no disable option in product config. Dev-only override via `LUMEN_DEV_INSECURE_TLS=1`, refused in release builds (`debug_assertions`-gated compile-out) `[PROPOSAL]`; rustls + webpki roots — crate choice landed in-tree: ureq 3.5.0 + rustls, ring 0.17.14 provider, **conditionally admitted** (record `deps/records/ureq-3.5.0.md` R10; verification never off — `crates/streaming/src/client.rs::tls_config`); **ADR-011 refreshed 2026-11-09** with the fresh provider audit, RECOMMENDED pending Lead admission, and the typed seam `streaming.scheme.https-not-admitted` at scheme dispatch (`crates/streaming/src/schemes.rs`) | `sec_tls_*`: expired/self-signed/wrong-host rejected in release; env override no-ops in release |
| SEC-08 | **Explicit redirect + credential policy**: max 10 redirects (no loops); `Authorization`/`Cookie`/basic-auth **never re-sent cross-origin** (scheme+host+port equality); credentials in URLs never logged (→ SEC-09) | `sec_redirect_creds`: MITM-style fixture redirects assert header dropped |
| SEC-09 | **Secret redaction at the source trait**: credentials live in a `SecretString`/redacting URL type whose `Display`/`Debug` redact; log macro layer double-checks; tokens (control server) never logged (doc 05 §5.4) | `sec_redaction`: seeded URLs with userinfo → grep log sink, zero leaks |
| SEC-10 | **No default telemetry / no unexplained egress**: engine network activity = user-initiated media fetches only; update checks off by default (MP §5) | `sec_no_egress`: sandboxed run, assert zero connections beyond the test server |
| SEC-11 | **Fuzz every parser**: registry per doc 04 §8 / doc 10 G10 — no parser merges without a registered target | G10 fuzz-smoke + nightly long-run |
| SEC-12 | **Untrusted text never becomes markup**: subtitles/metadata render from declarative token trees; no `innerHTML`/HTML-injection path in UI; no auto font/asset fetching from document content | UI unit tests + review; `sec_no_html_injection` |
| SEC-13 | **Plugin sandbox invariants** (doc 06 §5–6): capability imports, limits, provenance, wasm-magic fence, no auto-exec | `plugin_*` suite doc 06 §9.4 |
| SEC-14 | **`forbid(unsafe_code)`** in application/core crates; unsafe only in ADR-listed platform adapters with documented invariants (MP §9; doc 01 §5) | G5 architecture scan |
| SEC-15 | **Dependency pipeline** (this doc §1–§4): allowlist, vet, native scan, tree diff, SBOM | G3/G4 |
| SEC-16 | **Control-server hardening** (§7) | `control_*` suite |

---

## 7. Control-server hardening (doc 05 §5.4, security view)

The control server is the one network-listening surface; it is specified by
doc 05 §5.4 and hardened as follows:

1. **Loopback-only bind, structurally.** Listens on `127.0.0.1`/`::1` only;
   any config requesting a non-loopback address is a **startup error**, not a
   warning. Test `control_bind_non_loopback_rejected`: config `0.0.0.0` →
   non-zero exit, no listener socket, actionable diagnostic.
2. **Versioned protocol.** WS + JSON, `hello`/`welcome` major negotiation;
   unknown/invalid frames answered with typed `protocol-error` then close.
   One major per connection (doc 05 §6.3).
3. **Request validation.** Max frame 1 MiB; every request validated against
   the same `schemas/commands.schema.json` as all hosts; unknown fields/IDs
   rejected; per-connection `request_id` monotonicity (anti-replay);
   concurrency cap 8 in-flight; rate limit 100 req/s burst 200 `[PROPOSAL]`.
4. **Authentication for sensitive ops.** 256-bit random token per start;
   file (0600) / inherited fd / `--print-token` distribution; constant-time
   compare; 5 failures → 60 s lockout + log; **no command executes without a
   valid token**; optional `read-only` mode admits only the fixed safe subset
   (`media.get-info`, `system.get-engine-info`, `tracks.list`, snapshot).
   `session.open` stays privileged (touches the filesystem). Tests:
   `control_no_token_no_command`, `control_lockout`, `control_readonly_subset`.
5. **Origin check.** Browser `Origin` must be on the configured allowlist
   (default empty ⇒ non-browser clients only) — the DNS-rebinding defense
   for persona A5 `[PROPOSAL]`. Test `control_origin_rejected`.
6. **Rate limiting & audit logging.** Per-connection limits above; an
   append-only JSONL audit log (`{ts, command, outcome, peer}` — no tokens,
   redacted URLs per SEC-09) written to the runtime dir and rotated, size-
   capped `[PROPOSAL]`. Test `control_audit_log_written`.
7. **Out of scope for v1 control:** reflection, arbitrary path access,
   plugin installation (doc 05 §5.4) — absent from the catalog, not merely
   unauthorized.

---

## 8. Clean-room and license compliance

### 8.1 GPL boundary rules (VLC as external oracle only)

MP §2: VLC is GPL; *running* it as an external reference oracle in tests is
allowed, *vendoring or copying* it is not. ADR-007 defines the boundary
(doc 01 §7). Rules that bind this repo:

1. **No VLC source, comments, assets, logos, shaders or branding** enter the
   tree (AGENTS §3.1). Agents consult public specifications, observed
   behavior and independent tests — never VLC source.
2. **Oracle hygiene:** VLC 3.0.23 is pinned and hashed (doc 02 §2.4) and
   fetched at test time by CI, never committed. Differential outputs
   (hashes, track listings, timestamps) recorded as expectation data are
   *observations of behavior*, which clean-room practice treats as facts,
   not copies `[VERIFY: legal framing — flag to counsel with ADR-007]`.
3. **No fixture extracted from VLC or any player distribution** (doc 10
   §5.1 — clean-room).
4. **CI check `[PROPOSAL]`:** `tools/license-audit` scans the tree for
   VLC-derived content: (a) banned tokens (`libvlc`, `videolan` outside the
   pinned-download reference in doc 02, VLC module names from the Phase 0
   inventory used as our file names, `VLC_` symbol prefixes); (b) binaries
   > 64 KiB outside `design/`, `tests/fixtures/` (which require ledger rows)
   and `deps/` caches; (c) conformance that all fixture ledger rows point to
   non-VLC sources. Fail = G3 evidence `license-audit.txt` non-empty.

### 8.2 ADR-006 — this repository's license (the brief, not the decision)

> **DECIDED 2026-10-19 — MIT** (user decision; record:
> `docs/adr/ADR-006-license-mit.md`; risk R-05 closed in doc 14; decision
> D-009 in docs/ROADMAP.md §3). The brief below is retained for context.

Per doc 01 §7, ADR-006 is a Phase 0 decision. **This section prepares the
brief; it does not decide.** Facts first `[VERIFY each with counsel]`:

- Clean-room reimplementation does not automatically inherit VLC's GPL —
  copyright covers expression, not functionality; independent
  implementation from specifications/behavior is the classic clean-room
  posture. Risk concentrates in *behavioral similarity of expression*
  (option strings, error messages copied verbatim) and in any accidental
  source ingestion — hence §8.1's mechanical checks.
- Our dependency base (Symphonia MPL-2.0; mp4parse MPL-2.0; ogg/lewton
  BSD/MIT; rav1d BSD-2; truehd Apache-2.0 — doc 04) is compatible with
  every candidate license; MPL-2.0 is file-level copyleft and travels with
  the crate, not our tree.
- Design assets (R-07, §8.4) and fixture licenses (doc 10 §5.1) are
  independent of the code license.

**Options and axes:**

| Axis | GPL-3.0(+)/GPL-2.0+ | MPL-2.0 | MIT / Apache-2.0 |
|---|---|---|---|
| Ecosystem signal | "the VLC successor stays free" — strongest alignment with the user base we reimplement for | middle: file-level copyleft, weak for whole-app | maximal adoption; proprietary forks allowed (rebrand goal makes this likely and legal) |
| Plugin ecosystem (WASM extensions) | boundary question: extensions talk across a sandboxed, versioned API — likely separate works, but GPL optics may chill third-party extension authors; VLC itself needed an explicit plugin exception `[VERIFY: VLC's module license clause]` | clean separation | clean separation; Apache-2.0 adds an explicit contributor patent grant |
| Patent posture (with §8.3) | implicit patent grant + §10/§11 retaliation | no patent grant | Apache-2.0 explicit grant + retaliation; MIT none |
| Contributor friction | highest (CLA-less GPL is fine; corporate contributors wary) | low | lowest |
| Later relicensing | hard (needs all contributors) | hard | easy with consensus |

Also for the Lead: dual-licensing (e.g. Apache-2.0 OR GPL-3.0) is used by
many infrastructure projects to serve both audiences `[PROPOSAL: include as
fourth option]`. The choice also interacts with whether we ever accept
GPL-only crates (none today; oxisynth LGPL-2.1 is the pending case, doc 04
§3.1). **Decision inputs required from Lead+counsel:** §8.1 fact review,
extension-boundary reading, patent-clause preference, fork/rebrand policy.

### 8.3 Codec patent risk register (ADR-010 input, MP §14)

> **ADR-010 status (2026-11-09, issue #24):** the decision record is now
> drafted — `docs/adr/ADR-010-codec-patent-shipping.md` — with the full
> implementation inventory, the per-codec posture table below re-based on
> current code, four shipping options, a recommended posture **[PROPOSAL]**
> and a 24-item counsel question ledger. **Decisions remain PENDING COUNSEL**
> (its Q1–Q24); the ADR stays *proposed* until the user+counsel sign-off gate
> passes, and R-03's rule stands: no public binary distribution of any build
> before that. This register remains the input layer; the ADR is the
> decision layer.

Source licenses ≠ patent licenses. A permissively licensed decoder can
still embody patented techniques; distributing a decoder can trigger pool
licensing. Register (best-effort facts — **every pool/status line is
`[VERIFY: current pool membership, rates, and jurisdiction reading with
counsel]`; this is a flag-for-legal-review, not a legal opinion**).
Exposure column refreshed 2026-11-09 to the implementation reality
(ADR-010 §1 is the file-evidence authority):

| Family | Patent posture (to verify) | Lumen exposure (2026-11-09) | Mitigation options |
|---|---|---|---|
| H.264/AVC | multiple pools (Via LA successor to MPEG LA's AVC pool; Access Advance; individual holders) cover *implementers/distributors* of decoders | software decoder still absent (parser + hw seam only — `crates/codecs/src/video/h264/**`, doc 04 §4.2); using OS decoders (Apple/Microsoft license their stacks) may shift exposure to the OS vendor, but distributor liability is jurisdiction-dependent (ADR-010 Q3) | ADR-010 shipping matrix: hw-only builds, regional builds, source-build path |
| HEVC | notoriously fragmented (3+ pools + individual licensors) | worse than AVC; same hw-only reality (doc 04 §4.3; `crates/codecs/src/video/hevc/**` — parser + hw seam, no software decode; ADR-010 Q2) | same |
| MPEG-2 / MPEG-4 ASP | MPEG-2 pool largely expired (2019+) `[VERIFY]`; ASP pool status varies | no decoders: MPEG-2 is parser/facts-only (5.H1), ASP candidate oxideav-mpeg4video BLOCKED at the differential gate (ADR-010 Q4/Q5) | re-verify at build time |
| AAC | active pools (Via Licensing AAC) covering decoders | **AAC-LC decoder implemented** (`crates/codecs/src/audio/aac/**`, card 5.C2a, symphonia-codec-aac =0.6.1) — the one implemented Tier-B codec; **flag before first public binary release stands** (ADR-010 Q6) | ADR-010 gate: audio-codec feature split |
| MP3 | principal patents expired (~2017) `[VERIFY: tail patents]` | MP3/MP2 decoders implemented (cards 5.A3/5.J3b) — proposed default-build included (ADR-010 Q7) | none expected |
| Opus | royalty-free: IETF RFC 6716; contributors granted royalty-free licenses; Cisco/MS patent commitments `[VERIFY]` | decoder implemented (card 6.M-opus, ruopus =0.1.2) — proposed default-build included (ADR-010 Q8) | none |
| VP8/VP9 | Google's irrevocable royalty-free patent pledge `[VERIFY: scope]` | VP9 decoder implemented (card 6.M-vp9, rusty_vp9 =0.1.3; matrix row drift noted in ADR-010 §1) — proposed default-build included (ADR-010 Q9) | none |
| AV1 | AOMedia member royalty-free grant with defensive-termination clause `[VERIFY]` | rav1d no-asm decoder implemented (card 5.E1) — proposed default-build included (ADR-010 Q10) | none |
| FLAC / Vorbis | no known patent claims (Xiph's stated position) `[VERIFY]` | decoders implemented (cards 5.A1/5.B2) — proposed default-build included (ADR-010 Q11) | none |
| AC-3/E-AC-3, DTS, TrueHD | Dolby/DTS licensing regimes `[VERIFY]` | decoders still unadmitted: oxideav-ac3 and truehd both BLOCKED at their admission gates, DTS has no candidate (ADR-010 Q12–Q14) | register before any self-build/admission lands (precondition already written into the records) |
| DNxHD/HR (VC-3) | **repo silent — ADR-010 Q15** `[VERIFY: counsel]` | decoder implemented (card 6.M-dnx, filmcraft-dnx =0.9.0 — record covers license/purity only) | tier decision pending counsel |
| MJPEG, ADPCM, ALAC, PCM | repo silent on patents — ADR-010 Q16–Q19 `[VERIFY: counsel]` | all implemented in-house (2.B4/5.J2/5.J3a/2.B5) | tier decision pending counsel |
| Fonts/subtitle rendering | not codec patents, but font licenses (OFL…) when bundling | doc 08 interim-font decision | ledger row required |

**Jurisdiction variance:** patent exhaustion, offering-for-sale doctrines
(e.g. Germany) and utility-model regimes differ; a "safe in the US" read is
not a global read. ADR-010 outputs a **shipping matrix**: which codecs are
compiled into which distributed binaries, with regional variance recorded
and the honest-parity rule applied (blocked where undistributable). The
drafted ADR (above) proposes option (a) — default build = patent-clean tier,
encumbered codecs behind optional/plugin builds per doc 06 §4.1 — **pending
user+counsel sign-off**; the jurisdiction question itself is its Q22.

### 8.4 Third-party asset licensing

- **Design package (R-07):** the committed `design/figma/` exports come from
  a Figma Community file ("VLC Redesign (Community)"). Figma Community
  publishing terms govern; "Community" does not automatically mean
  redistributable-in-a-repo or commercial-use-safe `[VERIFY: the file's
  license and Figma's Community terms]`. Tracked as risk R-07 (doc 08 §2).
  If terms are inadequate: remove the assets, rederive tokens from
  observation, and re-point doc 08's source of truth. Decision needed with
  ADR-006.
- **Fixture corpus:** governed by doc 10 §5.1's ledger
  (`tests/fixtures/CORPUS.md`). This document sets the final fixture
  license allowlist `[PROPOSAL]`: **public-domain dedications, CC0, CC-BY
  (attribution in the ledger), and project-licensed test files** — CC-BY-SA
  and more restrictive share-alike terms are excluded to keep fixtures
  freely reusable; generated fixtures are CC0.
- **Bundled runtime assets** (default fonts, icons if any): each needs a
  ledger row with license + redistribution terms before shipping in
  installers.

---

## 9. Security test plan

### 9.1 Abuse-case suites (added to doc 10's integration tier)

| Suite | Content | Pass criterion |
|---|---|---|
| `sec_malformed_media` | fuzz-corpus seeds: truncations, bit-flips, header lies, nested/composite probing, discontinuous timestamps | typed errors or bounded recovery; no panic, no hang, RSS within cap |
| `sec_path_traversal` | playlists/subtitles with `../`, `..\`, absolute paths, UNC `\\host\share`, `file://server/…`, URLs with userinfo, symlinked sidecar dirs, `movie.srt` referencing fonts via relative traversal | every case refused or confined to sandbox prefix; no file opened outside allowed roots |
| `sec_oversized_claims` | headers declaring huge dimensions/sample counts/durations; RF64 > 4 GB lie; MKV unknown-size segments | allocation bounded by policy; error surfaces, playback either proceeds bounded or fails honestly |
| `sec_plugin_escalation` | doc 06 §9.4 `plugin_sandbox_escape`, `plugin_permission_escalation`, `plugin_host_pointer_smuggling`, `plugin_malicious_manifest`, `plugin_no_auto_exec` | per doc 06 §9.4 |
| `sec_control_*` | §7 tests: non-loopback bind rejected; no-token-no-command; lockout; read-only subset; origin rejected; audit log written | per §7 |
| `sec_tls_*` / `sec_redirect_creds` / `sec_redaction` / `sec_no_egress` | per SEC-07/08/09/10 | per §6 |

### 9.2 Dependency-confusion controls

We publish nothing to internal registries, so classic confusion (internal
name squatting) is out; residual risks and controls: **typosquatting** (name
review at audit time: compare candidate names against the allowlist and
known-popular crates; suspiciously fresh/low-download names escalate to Lead
— part of R1/R8), **hijacked updates** (allowlist ranges + vet + Lead-only
merge of bump PRs), **lockfile substitution** (committed lockfiles +
`--locked` + tree-baseline diff), **non-registry sources** (denied, §3.2),
**UI-side** (pnpm lockfile committed; `pnpm audit --prod` in G4; no
postinstall-script packages without review `[PROPOSAL]`).

### 9.3 Release checklist

1. **SBOMs** regenerated for rust/wasm/UI from the exact release lockfiles
   (§3.4), published with the release.
2. **Advisory scan** clean: `cargo deny check advisories` + `pnpm audit
   --prod` at the tag, reports archived.
3. **`cargo auditable`** build for all shipped binaries.
4. **Native scan** evidence fresh (≤ 7 days at tag time) `[PROPOSAL]`.
5. **License report**: `cargo deny check licenses` + `tools/license-audit`
   clean; ADR-006/010 shipping matrix attached to release notes.
6. **Signing `[PROPOSAL — minimal viable]`:** macOS: notarized +
   Developer-ID signed (required by Gatekeeper anyway); Windows:
   Authenticode if a certificate is budgeted (else documented unsigned +
   checksums); Linux: `SHA256SUMS` + minisign detached signatures on
   artifacts + signed git tag. Long-term: reproducible builds so users can
   verify from source `[PROPOSAL]`.
7. Release notes list security-relevant fixes and any advisory IDs fixed.

---

## 10. Open questions for the Lead

- **Q1 (ADR-006):** repository license — decide from the §8.2 brief
  (GPL-3.0 / MPL-2.0 / permissive / dual) with counsel; also settles the
  oxisynth LGPL case and the cargo-deny license allowlist.
- **Q2 (ADR-010):** codec patent shipping matrix posture (§8.3) — hw-only
  distributed builds? regional binaries? source-only for encumbered codecs?
  **Status: decision record drafted 2026-11-09
  (`docs/adr/ADR-010-codec-patent-shipping.md`; issue #24) — evidence
  complete, options a–d, recommendation (a) `[PROPOSAL]`; awaiting user +
  counsel on its Q1–Q24 ledger. Decision still pending; no binary
  distribution until it passes (R-03).**
- **Q3:** fixture-license allowlist (§8.4) — confirm CC-BY-SA exclusion and
  the four allowed classes.
- **Q4:** platform-ABI crate class (§1.2 note): pre-approve `libc`/`rustix`
  etc. as a registry entry (E2a) instead of per-use ADRs?
- **Q5:** approve SEC-04's `try_from` discipline scope and SEC-07's
  `LUMEN_DEV_INSECURE_TLS` mechanism (debug-only, env var).
- **Q6:** signing budget (§9.3.6): Authenticode certificate? checksums+minisign
  acceptable for Linux/Windows v1?
- **Q7:** control-server unauthenticated `read-only` mode (doc 05 §5.4 /
  §7.4): keep, or always require the token?
- **Q8:** who may hold `approver` in the allowlist besides the Lead (§10 of
  AGENTS §1 delegation), and does §3.7's automation-open-PRs policy stand?
- **Q9:** date to flip `bans.multiple-versions` from warn to deny (§3.2).
- **Q10:** rav1d no-asm acceptance: proceed as conditional (§2.4) pending
  Phase 0 `[VERIFY-P0]`, or hold AV1 until the audit confirms zero non-Rust
  build activity and the speed cost is measured?

---

*Traceability: §1–§4 → MP §9 (+doc 01 §5/§7, doc 04 §1.2, doc 06 §6, doc 10
G3/G4); §5–§7 → MP §12 (+doc 05 §5.4, doc 06 §5/§9.4); §8 → MP §2/§14
(+doc 01 ADR-006/007/010, doc 08 §2, doc 10 §5.1); §9 → MP §13 (+doc 10
§4.6/§6). Marker discipline: [PROPOSAL] = design awaiting Lead decision;
[VERIFY]/[VERIFY-P0] = fact awaiting automated or Phase 0 evidence.*


