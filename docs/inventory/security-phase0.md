# Security threat model — Phase 0/1 expansion (Task 0.D1-I)

| | |
|---|---|
| **Implements** | doc 02 card 0.A11 (threat-model draft: parsers, plugins, control server, oracle runs; matrix R4 evidence-integrity rules); doc 02 §2.4 (oracle sandboxing spec); MP §12 |
| **Expands** | doc 11 §5 (program-level skeleton: 8 trust boundaries, 7 personas, STRIDE). This document does NOT duplicate §5 — it instantiates it with Phase-0/1 realities and adds the boundaries, personas and rules that only exist once the Phase-0 evidence program (oracle runs, probe harness, CI, corpus) is real. |
| **Evidence base** | docs/baselines/vlc-3.0.23.md (pin + DR-01..DR-16); docs/baselines/raw/linux/probe-work/ (run_probes.sh, gen_py.py, 61 logs + status.txt); docs/inventory/modules-linux.json + modules-summary.md (502 registrations: 57 demuxers, 32 subtitle, 21 playlist); docs/inventory/formats.json (82 families, 59 probed); docs/feasibility/sections/{subsystems,video-codecs,audio-codecs,subtitles}.md (2026-10-19 crate re-verifications); docs/inventory/platform-synthesis.md (SV ledger). |
| **Consumed by** | doc 11 §5–§7 revision (Lead folds TB9–TB14, A8–A11, OH rules into doc 11 at gate review); 0.D3 decision wave; doc 14 risk register; corpus-plan.md (this task's sibling deliverable); later `tools/` security gates (doc 11 §9). |
| **Status** | Draft for Lead review. Everything traced to the docs/data above; inventions marked `[PROPOSAL]`; unverified facts marked `[VERIFY]`. No code exists yet — "where enforced" columns name the future crate/CI gate that will own each mitigation. |

## 0. Method

Doc 11 §5 was written in Wave 1 against the brief, before any oracle session,
census or probe existed. Phase 0 has since produced: (a) a pinned, hash- and
GPG-verified GPL oracle binary living in gitignored `tmp/pinning/extract/`
(pin record §1–§2); (b) 61 preserved oracle log sessions from 60+1 runs over
synthetic fixtures (probe-work); (c) a complete parser-surface census
(502 module registrations; fuzz-target population = 57 demuxers + 32 subtitle
modules + 21 playlist-bucket registrations, modules-summary §2); (d) an 82-family
format inventory with per-family probe evidence (formats.json); (e) live
dependency re-verifications including a **negative TLS finding** (0.C3-I: no
production pure-Rust HTTPS stack today — rustls defaults to aws-lc-rs C code,
ring carries an inert-but-declared `cc` build-dep, native-tls is OpenSSL).
Each of these creates or sharpens a security surface that the §5 skeleton
abstracted away. Sections below: §1 assets, §2 boundaries (TB1–8 expanded,
TB9–14 new), §3 personas (A1–7 expanded, A8–11 new), §4 per-boundary STRIDE
table, §5 oracle hygiene, §6 top-10 risks, §7 SEC-01..16 inputs, §8 ledger.

## 1. Asset inventory (expands doc 11 §5.1)

| Asset | Phase-0/1 instantiation (evidence) | Custodian |
|---|---|---|
| User media + paths | Not yet held; will be TB1 crosser. Probe fixtures today are our own synthetic files (probe-work/gen_py.py, ffmpeg lavfi) | engine (crates/demux, media-probe) |
| Credentials in URLs (`user:pass@host`) | None held today. Will arrive via playlists/streams (SEC-08/09). VLC's own option set (network-caching, network-synchronisation, metadata-network-access — options-linux.json) shows the config surface our settings schema will mirror | streaming crate; control-protocol |
| Playback/queue history | None today; future library/playlist state | app + UI |
| Local file read-probe value | Real today in oracle terms: probe-work logs show the oracle's `directory` demux enumerating fixture dirs; our engine will inherit this read-oracle exposure (persona A2) | access layer |
| Network responses | Probe-work exercised http via a loopback python server (http_server.log: 3 GETs, 127.0.0.1:8123). Real-network streaming is Phase 5+ | streaming crate |
| Control surface | Not built (doc 05 §5.4; doc 11 §7) | apps/control-server |
| Engine process integrity | No engine yet; the *oracle's* integrity is a live Phase-0 asset (§5) | CI |
| UI/webview | Not built (doc 08; TB3) | ui/ |
| Extension system | Phase 7 (doc 06) — included, marked later-phase | plugin-host-wasm |
| **Pinned oracle binaries + hashes** (new) | `tmp/pinning/` (gitignored): 5 installers/dmgs + 8 debs, all hash-matched vs official sidecars; re-verified 7/7 at consolidation (pin §5) | CI/reference machines |
| **Committed evidence** (new) | raw dumps, probe-work logs, formats.json — the truth anchoring the whole matrix; tampering = poisoned decisions (R4 rules, doc 02 card 0.A11) | repo (Lead-only merges) |
| **Fixture generators + corpus** (new) | gen_py.py today; tests/fixtures/ per doc 10 §5 + corpus-plan.md | testkit |
| **CI runners + workflow definitions** (new) | GitHub Actions [PROPOSAL topology per doc 10 §4.1]; run untrusted-ish: oracle + malformed corpus + PR-supplied code | CI |
| **Config/keymaps/locales/themes** (new) | Shipped-with-app but user-overridable (MP §5 layered config) — semi-trusted inputs, distinct from both code and media | product-config, settings-schema |

## 2. Trust boundaries

### 2.1 Doc 11 §5's TB1–TB8, instantiated with Phase-0 realities

| TB | Boundary | Phase-0 instantiation / new evidence |
|---|---|---|
| TB1 | media bytes → engine parsers | Target population now enumerated: 57 demuxer registrations / 51 unique container parsers, 32 subtitle modules, 21 playlist registrations (modules-summary §2/§7). Probe evidence shows the surface is *flexible by design* — e.g. the `es` demuxer claims 11 audio scheme shortcuts; probe races exist (formats.md: standalone text subs lose to `ps` in probe race) — flexibility is attack surface |
| TB2 | engine → UI | Typed commands (doc 05); not built |
| TB3 | UI → OS webview | Not built (doc 08) |
| TB4 | engine ↔ WASM extensions | Phase 7 — later-phase; boundary design frozen by doc 06 §5.2 |
| TB5 | engine ↔ network | **Sharpened**: 0.C3-I verified there is NO production pure-Rust HTTPS path today (rustls default TLS = aws-lc-rs C; ring 0.17.14 has non-optional cc build-dep; rustls-rustcrypto alpha). SEC-07's "rustls + webpki roots [VERIFY crate choice]" is now ADR-011's problem: every candidate drags C or immaturity across this boundary |
| TB6 | control server ↔ local clients | Not built; doc 11 §7 hardening stands; note the probe-work loopback http server (127.0.0.1:8123) as the repo's first loopback-listener precedent |
| TB7 | engine ↔ OS services | Platform adapters; 0.C-synthesis SV ledger tracks E2-classification calls (VA-API SV-28 etc.) |
| TB8 | repo ↔ crates.io | **Sharpened**: Phase-0 agents re-verified dozens of crates via crates.io API + tarball scans across waves 0.C1–0.C3 (evidence appendices in each feasibility section). Known live landmines already found by inspection: `symphonia-codec-wavpack` 0.1.1 = third-party GPL-3.0; `cue` = libcue C bindings; `openh264` = C source build. The pipeline (doc 11 §1–4) works as designed — but it currently depends on agent diligence, not tooling |

### 2.2 New boundaries TB9–TB14 (Phase-0/1 specific) `[PROPOSAL: fold into doc 11 §5.1]`

| TB | Boundary | Crosser | Why it exists now |
|---|---|---|---|
| TB9 | repo working tree ↔ `tmp/` scratch | agent scripts, oracle extraction, probe media, downloads | All Phase-0 binaries/media live in gitignored `tmp/` (pin §8.2). Scratch is where untrusted bytes (oracle output, malformed media, fetched tarballs) meet the tree that gets committed |
| TB10 | oracle invocation environment | run_probes.sh-style harness: `timeout 25 vlc -I dummy --play-and-exit --aout dummy --vout dummy -vv` + `LD_LIBRARY_PATH` pinned to the extract dir | A GPL C binary with a huge native parser surface runs on files we authored — and later (Phase 2+) on malformed corpus. The oracle itself is both asset (evidence source) and liability (crash/hang/escape risk) — §5 |
| TB11 | CI runners ↔ repo + network | workflow YAML, cached toolchains, fetched oracle artifacts, published evidence | Runners execute PR-supplied code, fetch the oracle over HTTPS (mirror drift recorded: DR-01 3.0.24 front-door, DR-13 HTML redirect page, DR-12 snapshot 509), and hold any future tokens |
| TB12 | shipped-but-user-editable config | `config/ themes/ keymaps/ layouts/ profiles/ locales/` (MP §4) | Neither code (reviewed) nor remote media (sandboxed): user-supplied settings files parse at startup and steer engine behavior (limits, paths, network options). Options inventory (options-linux.json, 1476 records) previews the key names this surface will carry |
| TB13 | plugin package ↔ engine (later phase) | WASM module bytes + manifests | Phase 7 per doc 06; listed so TB13's *manifest* format (untrusted metadata parsed before sandboxing) is designed with SEC-11-style fuzz coverage from day one |
| TB14 | docs/ evidence ↔ reality | expectation files, recorded hashes, feature-matrix rows | The matrix's R4 evidence rules (doc 02 card 0.A11): an expectation silently re-blessed or a hash edited without a run is a *forged test*. Integrity of committed evidence is a security property of the program, not just QA |

## 3. Adversary personas (expand doc 11 §5.2's A1–A7)

- **A1 Malicious media file** (doc 11 A1) — concretized: the arrival vector is
  "user opens a file" or "CI ingests a curated fixture"; the parsing population
  is exactly the 57+32+21 module census. Phase-0 corroboration that this surface
  is real: the oracle itself prints module-selection for every family
  (formats.json probe_evidence); known VLC-era parser classes (AVI, ASF, MKV)
  are the ones our own demux plan self-builds (doc 04 §2). Decompression bombs
  and pathological timestamps remain the top vectors (SEC-01/02).
- **A2 Malicious playlist with path tricks / URL credentials** (doc 11 A2) —
  concretized by probe-work reality: our own generated XSPF/ASX/B4S fixtures
  embed absolute `file://` locations and the M3U one uses a relative
  `../media/` path — both resolve today in the oracle. The attack variant
  (traversal `../../etc/passwd`, UNC `\\host\share`, `file://server/`,
  `http://user:pass@host/`) is a *corpus* requirement (SEC-06 vectors,
  corpus-plan class B/D). Read-oracle leak channel: playlist probing
  differences (play vs error vs duration) — doc 11 §5.2 already notes this.
- **A2b Malicious subtitle sidecar** (split out of A2) — the sidecar lookup
  path (`movie.srt` next to `movie.mkv`) is auto-triggered without user action
  on open; probe-work `subsl_*` sessions prove the trigger points (second-pass
  `--sub-file` runs; formats.md "text subs are sidecar-only in this oracle").
  Sidecar + font references + ASS embedded styling make subtitles a parser AND
  a resource-fetcher (SEC-12: no auto font/asset fetching).
- **A3 Malicious WASM extension** (doc 11 A3) — **later phase (Phase 7)**,
  included per task: sandbox escape, capability escalation, provenance forgery.
  Phase-0 relevance only insofar as doc 06 §5's boundary design and the
  manifest format (TB13) are frozen early.
- **A4 Network MITM** (doc 11 A4) — unchanged in shape; sharpened by the TLS
  purity finding (TB5): the classic control (TLS verification) is exactly the
  control whose implementation is now uncertain in pure Rust (ADR-011).
- **A5 Local control-port prober / drive-by webpage** (doc 11 A5) — unchanged;
  note probe-work's loopback-only python server as precedent for the default
  posture (bind 127.0.0.1, no exposure).
- **A6 Compromised / typosquatted dependency** (doc 11 A6) — Phase-0 evidence:
  tarball inspections caught real traps (GPL WavPack third-party publish,
  reserved-name crates like `ffv1 0.0.0`, placeholder `symphonia-codec-opus`).
  Residual: today the "scanner" is an agent reading tarballs; `tools/`
  automation (doc 11 §3) does not exist yet.
- **A7 Slow/hostile server** (doc 11 A7) — unchanged; UDP/RTP probe sessions
  (rtp_ts exit 124 = timeout on live stream) are the local precedent for
  deadline-bounded consumption (SEC-05).
- **A8 CI-poisoned fixture / PR** (new) `[PROPOSAL: fold into doc 11 §5.2]` —
  a malicious or careless PR that (a) adds a "curated" fixture with embedded
  exploit content or false license attribution, (b) weakens a generator so
  malformed cases vanish, (c) edits workflow YAML to exfiltrate secrets, or
  (d) re-blesses expectations (overlaps TB14). Crosses TB8/TB9/TB11/TB14.
- **A9 Hostile oracle-artifact channel** (new) — the pin depends on
  download.videolan.org / get.videolan.org redirecting to third-party mirrors
  (observed: `mirror-hk.koddos.net`, DR-13), an expired DSA-1024/SHA-1 signing
  key accepted with caveats (DR-02), a front-door version drift to 3.0.24
  (DR-01), and snapshot.debian.org being unreachable (DR-12). A mirror
  compromise or a repo-side artifact swap is defeated only by the pin's
  hash-sidecar cross-check + version-string guard.
- **A10 Malicious local config/keymap/locale/profile/theme** (new, TB12) — a
  shared machine or a downloaded "settings pack" that steers the engine:
  disabled limits, redirected subtitle/font dirs, changed network options,
  huge cache caps. Config is user-space trust; the threat is *tampering +
  privilege funneling*, mitigated by schema validation (product.schema.json),
  layered precedence (MP §5) and bounds that config can lower but never raise
  above product caps `[PROPOSAL]`.
- **A11 In-repo / local-process adversary** (new, TB9) — race or symlink
  attacks on `tmp/` scratch (oracle writes logs next to fixtures another
  process reads), worklog/evidence tampering by a compromised agent or
  process, TOCTOU between hash-verify and use of fetched artifacts. Low
  likelihood, but Phase 0 *is* the phase where everything lives in one
  scratch tree.

## 4. Per-boundary threat table

STRIDE letters: Sp spoofing, T tampering, R repudiation, I info disclosure,
D DoS, E elevation. "Where enforced" = owning crate or CI gate (future unless
marked). Phase = first phase the mitigation must exist.

| TB | Threats (STRIDE, persona) | Mitigations | Where enforced | Phase |
|---|---|---|---|---|
| TB1 | parser memory bugs, bombs, absurd size fields, pathological timestamps, probe recursion (Sp/T/D/E, A1) | SEC-01/02/03 bounded memory+probe+queues; SEC-04 try_from discipline; SEC-11 fuzz targets for all 57+32+21-analog parsers; malformed corpus classes B (corpus-plan §1) | crates/demux, subtitles, playlists, media-probe; fuzz/ registry (doc 04 §8.5); CI gate G10 | 2–6 |
| TB1 | probe-race misselection (wrong demux wins) (T, A1) | deterministic probe ordering + recorded per-family probe evidence as expectations (formats.json probe_evidence is the reference set) | media-probe; tests/fixtures/expectations | 3 |
| TB2 | command shape abuse, oversized payloads (T/D, A5/A3) | schema-validated typed commands, doc 05 §1.2; no reflection | crates/commands; schemas/commands.schema.json | 2 |
| TB3 | XSS via metadata/subtitle text (T/E→I, A1/A2b) | SEC-12 declarative token trees; React escaping; no innerHTML path | ui/; sec_no_html_injection | 3–4 |
| TB4 | WASM escape/escalation/exhaustion/forgery (E/D/Sp, A3) | doc 06 §5.2–5.7 (separate memories, capability imports, limits, hash/signature, wasm-magic fence) | plugin-host-wasm; plugin_* suite | **7** |
| TB5 | MITM, TLS strip, credential re-send, bomb manifests (Sp/T/I/D, A4/A7) | SEC-07 TLS always-on (implementation risk → ADR-011); SEC-08 redirect/credential policy; SEC-01/05 bounds+deadlines | crates/streaming; sec_tls_*/sec_redirect_creds | 5 |
| TB6 | unauthenticated control, replay, DNS rebinding, origin abuse (Sp/E, A5) | doc 11 §7 in full: loopback-only, versioned WS+JSON, 1 MiB frames, token auth + lockout, origin allowlist, rate limits, audit log | apps/control-server, control-protocol; control_* suite | 2 |
| TB7 | malformed OS-API use, hw-decoder attack surface (E/D, A1) | E2 ADR safety reviews; validated buffers across ABI; error containment | platform-*; G5 architecture scan | 2–5 |
| TB8 | poisoned/typosquatted/hijacked dependency (T/E, A6) | doc 11 §1–4 pipeline (allowlist, vet, tarball scan, tree diff, lockfile `--locked`, Lead-only merges); SBOM | tools/dependency-audit; G3/G4 | 1 (process now, tooling by 2) |
| TB9 | scratch-tree tampering, symlink/TOCTOU, untrusted bytes leaking into commits (T/E, A11) | OH rules §5.4–5.6 below; `git status`-clean checks before commit (pin §8.2 precedent); mktemp-per-run dirs; no commits of tmp/ (gitignore + `*.exe`) | harness scripts; Lead review; .gitignore | **0** (active) |
| TB10 | oracle crash/hang/escape; poisoned oracle evidence (D/T/E, A1/A9) | §5 oracle hygiene OH-1..12; version guard (doc 02 §2.4); hash-verify before use; timeout-bounded runs; disposable env | tools/parity-report, bench-vs-vlc; CI oracle job | **0** (active), formalized 2 |
| TB11 | workflow poisoning, secret exfil, cache poisoning, artifact-channel attack (T/I/E, A8/A9) | workflow YAML Lead-reviewed; no secrets in PR-context jobs; pin-hash assert on every oracle fetch; `--locked` builds; egress allowlist on runners `[PROPOSAL]` | .github/workflows; CI policy (doc 10 §4) | 1 |
| TB12 | config tampering, bounds inflation, path redirection (T/E, A10) | settings-schema validation; product caps as hard ceilings `[PROPOSAL]`; SEC-06 path rules apply to config-sourced paths too | crates/product-config, settings-schema; G2 | 2 |
| TB13 | malicious manifest/module bytes pre-sandbox (E/T, A3) | manifest parsed with same fuzzed-parser discipline as media (SEC-11 target `fuzz_plugin_manifest` `[PROPOSAL]`); provenance before execution (doc 06 §5.7) | plugin-api/host; plugin_* suite | 7 |
| TB14 | forged evidence: re-blessed expectations, edited hashes, matrix rows without runs (T/R, A8/A11) | R4 evidence rules (doc 02 card 0.A11): expectations frozen, change = regression paperwork (doc 10 §5.4/§8.2); hashes recomputed in CI from artifacts, not trusted from files `[PROPOSAL]`; feature-audit G6 bidirectional mapping | tools/feature-audit; CI gate G6 | 2 |

## 4.1 STRIDE summary, expanded for TB9–TB14 (extends doc 11 §5.4)

Doc 11 §5.4's six-class table stands for TB1–TB8. The new boundaries add:

| Class | New-boundary exposure | Primary control |
|---|---|---|
| Spoofing | forged oracle artifact (A9 — mirror serves 3.0.24 or a tampered deb); forged "curated" fixture provenance (A8) | OH-10 hash-vs-pin + OH-7 version guard; ledger rows with source URLs + hashes; `tools/license-audit` |
| Tampering | scratch-tree writes (A11); workflow edits (A8); evidence/log edits after the fact (A8/A11, TB14) | OH-1 per-run dirs; Lead-reviewed workflows; CI-recomputed hashes; append-only worklog |
| Repudiation | "the oracle said X" claims without a preserved log; generator output swapped without seed change | verbatim-log discipline (OH-11); MANIFEST.toml seed binding (corpus-plan §2) |
| Info disclosure | absolute paths / usernames in committed logs (observed — see §5.1); CI secrets via PR context | OH-4 scrub rule; no PR-context secrets `[PROPOSAL]` |
| DoS | oracle hang on live/malformed input (rtp_ts precedent); runner exhaustion via corpus bombs | OH-8 timeouts; corpus-B budget assertions (rss/wall caps in expectations) |
| Elevation | oracle escape from scratch dir (worst case, A1 via fixture); agent-scope violation (scratch → commit) | OH-1/OH-6 isolation + read-only fixtures; Lead commit gate; gitignore mechanics |

## 5. Oracle hygiene (Phase-0 unique)

The one place Phase 0 deliberately runs a large, aging, native C attack
surface (GPL VLC 3.0.23) on files we control. Rules OH-1..OH-12; status column
reflects what probe-work/pin already prove vs what CI must institutionalize.

| # | Rule | Status / evidence |
|---|---|---|
| OH-1 | **Separate scratch dir per run**: every oracle session gets a fresh `mktemp -d` working area; no two runs share mutable state; harness never writes into the repo tree | `[PROPOSAL]` — probe-work used one shared `tmp/probe` tree (acceptable for 60 read-only sessions, single agent, single day; recorded as deviation, not practice to copy) |
| OH-2 | **No network for the oracle except explicit protocol tests**: default = no egress; loopback-only test servers (probe-work precedent: python http.server on 127.0.0.1:8123, 3 GETs logged) are the only sanctioned exception, and only for http/udp/rtp protocol probes | proven pattern (http_server.log, udp_ts, rtp_ts); formalize as runner network policy `[VERIFY: CI isolation mechanism — doc 02 §2.4 open item]` |
| OH-3 | **The oracle never sees credentials**: no userinfo URLs, no auth headers, no cookies, no keystores in any oracle invocation, fixture or playlist; test credentials meant for OUR engine never transit oracle inputs | respected so far (no cred fixtures exist); corpus-plan adds cred-shaped playlists for *our* SEC-08/09 tests only, expectations redacted |
| OH-4 | **Logs scrubbed before committing as evidence**: absolute sandbox paths (`/home/z/my-project/tmp/...`) in committed logs are either relativized or noted as harmless-local-path leakage; no usernames, machine names beyond what policy allows, or token-like strings are committed | partially respected: committed probe-work logs contain absolute `tmp/probe` paths (benign, but the rule going forward is: scrub or annotate; `[PROPOSAL]` a tiny `tools/scrub-log` xtask + check in the evidence CI job) |
| OH-5 | **Oracle binaries never committed**: `tmp/` gitignored, `*.exe` doubly ignored; repo contains text evidence only (verified `git status` clean of binaries at pin consolidation §8.2; re-assert at each gate) | proven (pin §8); make it a mechanical CI check (doc 11 §8.1.4b binaries >64 KiB scan) |
| OH-6 | **Fixture files passed read-only**: oracle opens fixtures read-only (it does by default); harness additionally mounts/copies fixtures into the per-run dir with no write-back path; oracle output (logs) goes to a separate directory from inputs | mostly proven (logs dir separate from media dir in probe-work); formalize per-run copy `[PROPOSAL]` |
| OH-7 | **Version-string guard before every invocation**: assert the verbatim `--version` line (`VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)`, pin §2) matches the pin record; drift aborts the run (doc 02 §2.4 Guard) | data proven (raw/linux/04); enforcement is future harness code (Phase 2) — the guard spec is now binding |
| OH-8 | **Timeout-bounded invocations**: every oracle run under a hard wall-clock timeout (probe-work: `timeout 25`; live-stream case rtp_ts hit it at 124 — the mechanism works and must stay) | proven pattern; keep `timeout` in every documented mode M1–M4 |
| OH-9 | **Disposable environment**: fresh `HOME` (0.B1 precedent: fresh HOME + `--reset-config` for the prefs walk), pinned `LD_LIBRARY_PATH` to the extract dir only, no inherited VLC config/plugins from the host; env vars explicitly enumerated in the run manifest | proven in pieces (run_probes.sh exports exactly one LD_LIBRARY_PATH; 0.B1 fresh-HOME); bundle into a documented invocation profile |
| OH-10 | **Artifacts hash-verified before use**: every fetched oracle artifact (deb/exe/dmg/7z) checked against the pin record's sidecar-matched SHA-256 before first execution; re-verification on cache reuse (7/7 precedent, pin §5) | proven at pin time; make recompute-automatic in CI (TB14 note: recompute, don't trust stored values) |
| OH-11 | **Evidence recorded verbatim and regenerable**: logs committed as text with command manifests; a re-run must yield equivalent output (doc 02 §3.1 reproducibility rule); parsing of oracle output restricted to the recorded log grammar (M1 [VERIFY: exact grammar]) | proven (17-format-probes.txt + manifests); grammar pinning still open |
| OH-12 | **Oracle runs only on repo-controlled fixtures** (CI): no arbitrary/user media ever reaches the oracle in automation; malformed-corpus runs are exactly the class-B fixtures whose bytes we generated and hashed | `[PROPOSAL]` — becomes binding when Phase-2 differential testing starts (M3 mode) |

Contingency (from doc 02 §2.4, retained): disposable VM/container per corpus
run for the CI fleet — approach `[VERIFY: isolation tech — Lead + CI budget]`.
Probe-work's stopgap is: synthetic fixtures only, timeout, loopback-only
network, single-user sandbox.

### 5.1 Observed hygiene facts from the 61 preserved sessions

What the committed evidence already demonstrates (and what it quietly
violates — recorded honestly, because the rules above were written *from*
these observations):

- **Timeout mechanism works**: `timeout 25` bounded all 57 status-tracked
  runs; exactly one live-stream case (rtp_ts) hit the wall clock (exit 124) —
  proving the bound is load-bearing, not decorative (status.txt).
- **Loopback-only network precedent**: the sole network exposure was python
  `http.server` on 127.0.0.1:8123 serving our own WAV + M3U (3 GETs logged,
  http_server.log). No credentials, no external hosts, no egress.
- **Environment pinning precedent**: exactly one `LD_LIBRARY_PATH` export
  pointing into the extract dir; `-I dummy --play-and-exit --aout dummy
  --vout dummy` — no host config inheritance beyond the ambient env
  (run_probes.sh; a full fresh-HOME discipline was proven separately by 0.B1's
  prefs walk).
- **Read-only fixtures, separate output**: oracle wrote only logs (own dir);
  media dir was never written back (logs vs media split in tmp/probe).
- **Observed deviation D-OH-1 (absolute paths in committed logs)**: the
  committed logs contain `/home/z/my-project/tmp/probe/...` absolute paths
  throughout (and run_probes.sh hardcodes `EXTRACT=`/`P=`). Harmless today
  (sandbox username `z`, no secrets), but it is exactly the leakage class
  SEC-09 exists for — disposition `[VERIFY SV-P7]`: one-time scrub vs
  annotation.
- **Observed deviation D-OH-2 (shared scratch tree)**: all 60 sessions ran
  against one `tmp/probe` tree across one day (single agent). Acceptable for
  read-only playback probes; NOT acceptable once malformed corpus (class B)
  and parallel CI shards exist — OH-1 exists because of this deviation.
- **Observed deviation D-OH-3 (unpreserved generator commands)**: the ffmpeg
  command lines behind the media fixtures were lost (0.B4-R V1), i.e. the
  fixtures are not currently reproducible from committed state — the
  corpus-plan §2 determinism contract closes this class of gap.
- **Positive**: no VLC source consulted anywhere; binaries stayed in
  gitignored tmp/ (verified `git status` clean at pin consolidation, §8.2);
  dumps are text with manifests (OH-11 shape).

## 6. Top-10 security risks for the program (with early indicators)

Ranked by (likelihood × blast radius) for the *whole program*, not just Phase 0.

| # | Risk | Early indicator (already observable) | Detect/trigger |
|---|---|---|---|
| 1 | **Parser memory-safety defect shipped** (A1) — 57+32+21-analog parser surface, self-built demuxers for AVI/ASF/MKV/PS are the biggest new-code parsers | any panic/hang in fuzz smoke; corpus class-B failures | G10 fuzz gates; nightly long runs; SEC-01/02 ceilings |
| 2 | **TLS stack impurity or weakness** (A4) — no production pure-Rust HTTPS today; SEC-07's assumed stack is contested | 0.C3-I crate findings (aws-lc-rs default, ring cc, alpha rustcrypto) | ADR-011 decision; sec_tls_* suite; cargo-deny native scan |
| 3 | **Path traversal via playlist/subtitle** (A2/A2b) — relative-ref resolution is convenience feature and attack at once | probe-work fixtures already exercise `../media/` (legal) — the illegal variants are one edit away | sec_path_traversal suite; corpus class-B/D vectors |
| 4 | **Credential leakage in logs/errors** (A4/A7→I) | no cred paths exist yet — the risk is *building* them wrong in Phase 5 streaming + playlists | SEC-09 SecretString trait from day one; sec_redaction grep test |
| 5 | **Supply-chain poisoning** (A6) — fast-moving young crates are the plan (2026 H.264/HEVC wave all <1yr, self-validated) | tarball catches already made (GPL wavpack crate, C-bound cue/openh264, reserved names) | G3/G4 pipeline; tree-diff on every bump; Lead-only merges |
| 6 | **Evidence forgery / silent re-blessing** (A8/A11, TB14) — matrix decides scope; forged evidence corrupts decisions | expectation-change discipline not yet mechanized (doc 10 §8.2 is policy) | CI recomputes hashes; feature-audit G6; regression paperwork gate |
| 7 | **Control-server exposure** (A5) — the only listening surface; misbind = remote control of a media engine | not built; §7 spec exists; loopback precedent good | control_bind_non_loopback_rejected + suite §7 |
| 8 | **Oracle channel compromise** (A9) — mirrors, expired key, front-door drift | DR-01/DR-02/DR-12/DR-13 all *already observed* | OH-7/10; pin re-check at phase gates |
| 9 | **Decompression/oversize bombs in stream filters** (A1/A7) — zip-in-playlist (`tone_zipped.zip` probed!), gzip stream filters, RF64/MKV unknown-size lies | probe-work ran the oracle on a zipped tone successfully — archive access is in scope | SEC-01 bomb fixtures; RSS ceilings |
| 10 | **CI runner compromise via PR** (A8) — workflows + fixtures + oracle fetch execute in one context | no CI exists yet — designing it wrong now bakes the risk in | workflow review policy; egress allowlists; no PR-context secrets `[PROPOSAL]` |

## 7. Inputs to SEC-01..16 (doc 11 §6) from Phase-0 findings

| SEC | Phase-0 finding → refinement |
|---|---|
| SEC-01 | Corpus must include oversize-claim bombs per family (trun `sample_count` absurd, RF64 >4 GB lie, MKV unknown-size) — subsystems.md §1.2 class 11 enumerates the fMP4 instances; generalize to all P0 containers (corpus-plan class B op "oversize-field") |
| SEC-02 | Probe-depth bound has a measured reference: the oracle's own probe behavior (probe races recorded in formats.md) — our probe budget should be tested against the same fixtures to ensure *equivalent selection with bounded cost* |
| SEC-03 | No new input; queue backpressure tests should reuse discontinuous-timestamp fixtures (corpus class C) as load |
| SEC-04 | No engine code yet; the census shows where the lint must bite: the crate list in doc 11 §6 SEC-04 matches the self-build demuxer/subtitle/playlist crates exactly |
| SEC-05 | rtp_ts (exit 124) is the recorded shape of an unbounded live source under a wall-clock kill — the engine's deadline behavior must be *better*: typed timeout, not process kill. Add to interrupted-network suite as reference behavior |
| SEC-06 | Probe-work fixtures give the *legal* path-resolution cases (`../media/` in m3u, absolute file:// in xspf/asx/b4s); corpus-plan §1B adds the illegal twins; subtitle sidecar lookup rule (same-dir only) must be tested with symlinked sidecar dirs (doc 11 §9.1 sec_path_traversal row) |
| SEC-07 | **Material change**: "[VERIFY: crate choice Phase 0]" is resolved-as-negative — no pure-Rust production option; ADR-011 must choose between C-carrying rustls/ring, alpha rustls-rustcrypto, or an E2-classified adapter. SEC-07's *requirement* (always-on verification, dev-only env override compiled out in release) is unaffected |
| SEC-08 | Playlist credential policy now has a concrete test corpus need: `http://user:pass@host/` entries in m3u/xspf fixtures with redacted expectations; cross-origin redirect header-drop test fixtures (MITM-style local server, loopback precedent OH-2) |
| SEC-09 | Committed probe-work logs contain absolute local paths — evidence-scrubbing (OH-4) is the Phase-0 face of SEC-09: redaction discipline starts with *our own* committed logs, before any user credential can exist |
| SEC-10 | Probe-work's python http.server (loopback, explicit, logged) is the sanctioned shape of test-local network; sec_no_egress should codify "engine connects only to loopback test servers in CI" as the assertable form |
| SEC-11 | Fuzz-target population enumerated: 57 demuxer + 32 subtitle + 21 playlist registrations (oracle census) → our parser crates 1:1; seed corpus = class-B generated malformed + curated D files (doc 04 §8.5 rules) |
| SEC-12 | ASS override tags (`{\b1}` in probe-work ass fixture) prove styled text arrives with markup-ish syntax — the token-tree renderer contract (doc 03 §4.11) must treat ASS override blocks as *data*, never markup; add ASS-tag corpus rows |
| SEC-13 | Later phase (7); manifest parsing should be an SEC-11 target from its first commit (TB13) |
| SEC-14 | Nothing to add — no code; G5 scan spec stands. Note rav1d asm-off plan (0.C1-I §3) as the model for future E2 exceptions |
| SEC-15 | Pipeline validated manually across dozens of crates this phase (0.C1–0.C3 evidence appendices); land `tools/dependency-audit` before Phase 2 so the 2026-wave young crates (all <1yr, self-validated) get automated re-vetting on every bump |
| SEC-16 | No change; §7 spec complete; first implementation Phase 2 |

## 7.1 New checks this expansion adds (beyond SEC-01..16) `[PROPOSAL]`

| Check | Content | Gate |
|---|---|---|
| `oh_scrub_log` | committed evidence logs contain no absolute paths / usernames beyond an allowlist | evidence CI job (SV-P3/SV-P7) |
| `oh_no_binaries` | no binaries > 64 KiB outside sanctioned dirs (doc 11 §8.1.4b) — covers OH-5 | G3 (`tools/license-audit`) |
| `oh_version_guard` | every oracle invocation in CI is preceded by the pin version-string assert | differential harness (Phase 2) |
| `oh_hash_recompute` | corpus + artifact hashes recomputed from bytes, never read-from-file and trusted | G6 / evidence job (TB14) |
| `manifest_seed_bind` | every generated fixture's hash matches (generator, seed) re-run | card 2.A3 AC / corpus MANIFEST check |

## 8. [VERIFY] ledger (this document)

| # | Item | Unblock path |
|---|---|---|
| SV-P1 | CI fleet isolation mechanism for oracle + malformed corpus (disposable VM/container choice) | Lead + CI budget decision; doc 02 §2.4 open item |
| SV-P2 | M1 log grammar pinning (exact -vv line formats we parse as evidence) | Phase 2 harness work; doc 02 §2.4 [VERIFY] |
| SV-P3 | tools/scrub-log feasibility (what counts as sensitive in committed logs — paths? usernames in tmp paths?) | Lead policy call + small xtask |
| SV-P4 | Product-cap ceilings config cannot raise (TB12 `[PROPOSAL]`) | settings-schema design (doc 07 §5) |
| SV-P5 | CI egress allowlist enforceability on GitHub-hosted runners | CI design (doc 10 §4) |
| SV-P6 | Manifest fuzz target naming + Phase-7 enforcement point | doc 06 revision at Phase 6 |
| SV-P7 | Whether committed probe-work logs' absolute paths warrant a one-time scrub now vs annotation-only | Lead call (OH-4 disposition) |
| SV-P8 | fuzz targets on wasm32 feasibility (doc 04 §8.5) | Phase 2 spike |

## 9. Evidence map

| Claim (section) | Evidence |
|---|---|
| §0/§2.1 parser population 57+32+21 | modules-summary.md §2 rollup table; modules-linux.json |
| §2.1 probe races, es-demuxer shortcuts | formats.md subtitle two-pass table + §7; modules-summary §7 |
| §2.2 TB5 TLS findings | subsystems.md §2 + evidence appendix (crates.io 2026-10-19 re-verifications) |
| §2.2 TB8 landmines | audio-codecs.md (WavPack GPL), subsystems.md §5.1 (cue = libcue), video-codecs.md (openh264) |
| §3 A2 path/URL shapes | probe-work gen_py.py fixtures (m3u `../media/`, xspf/asx/b4s absolute file://) |
| §3 A2b sidecar trigger points | subsl_* sessions, formats.md two-pass table |
| §3 A9 mirror/key/drift | pin record DR-01, DR-02, DR-12, DR-13 |
| §4 TB9–TB11 mechanics | pin §8.2 (gitignore, clean status); run_probes.sh (LD_LIBRARY_PATH, timeout); doc 10 §4.1 CI topology |
| §5 all OH statuses | run_probes.sh + status.txt + http_server.log + 17-format-probes.txt preamble; pin §2/§5/§8 |
| §6 risk-9 zip access | tone_zipped session (17-format-probes.txt) |
| §7 SEC-05 shape | rtp_ts exit 124 (status.txt) |
| §7 SEC-12 ASS tags | ass_sample.ass `\\b1` override (gen_py.py §10) |

*Traceability: §1–§4 expand doc 11 §5 with formats.json, modules-summary §2/§7,
pin record DR-01..16, probe-work evidence; §5 implements doc 02 §2.4
sandboxing; §6 feeds doc 14; §7 feeds doc 11 §6. Marker discipline per AGENTS
§3.4: [PROPOSAL] = design awaiting Lead decision; [VERIFY] = fact awaiting
automated or later-phase evidence.*
