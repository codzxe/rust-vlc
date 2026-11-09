# Phase 8 card-to-issue breakdown — issue #23 close package

| Field | Value |
|---|---|
| Purpose | Convert doc 13 §11 (Phase 8, waves 8.A–8.H, 16 tasks) into issue-ready work items so the Lead can open trackable issues and close umbrella issue #23 |
| Source of truth | `docs/13-agent-wave-program.md` §11, lines 784–845 — read in full; **all card scope/AC text below is from that section, nothing invented** |
| Gate context | MP §16 gate: compatibility, security, soak, perf-vs-VLC, packaging, honest parity report. Detail docs: 11 §7/§9 (security, SBOM), 12 §9 (final perf), 10 §7–§8 (evidence, regression protocol), MP §17 (done definition). Parallelism 2–4 |
| Rule for the Lead | One issue per item below; keep card IDs in the issue body; close the item only on its stated AC |

## 0. Deltas since the wave program was written (fold into issue bodies, do NOT re-scope)

These are already-merged facts that make parts of the Phase 8 cards partially done. Verified in-tree today:

1. **Soak nightly EXISTS (issue #18)** — `.github/workflows/soak-nightly.yml` is live. 8.C1's "LSAN nightly clean" now has a running vehicle; the card's remaining work is the full 4 h × scenarios suite, reopen-cycle stress, memory-slope comparison, quarantine resolution, and regression-guard audit.
2. **VLC 3.0.23 oracle differentials now RUN (issue #19)** — `docs/corpus/vlc-diff-report-20261023.md`: 29/35 corpus files bit-exact (MD5 prefix match), 6/35 pcm_f32 on the documented f32 deviation path. Feeds 8.B1 matrix evidence labeling and 8.D1 honest comparisons.
3. **Netfuzz targets landed (issue #14)** — `fuzz/fuzz_targets/` now carries codec targets (aac, adpcm, alac, av1, flac, h264, hevc, mjpeg, mp2, mp3, …). 8.A1's fuzz-corpus expansion starts from a landed target set rather than zero.
4. **Security review done (issue #15)** — 6.H1 network/platform review produced **1 Medium** (same-host https→http scheme downgrade) + 3 Low, fix sketches in `docs/investigations/network-security-review.md`; phase6-exit recommends scheduling the Medium before exit sign-off. 8.A2 inherits this open finding as its first triage item.

## 1. External-dependency blockers (owner = user, not agents)

| Blocker | Blocks | Evidence / risk row | Owner |
|---|---|---|---|
| Apple Developer account + CI key custody (notarization/codesign) | 8.E2 macOS packaging; cascades to 8.E3 SBOM-all-platforms and 8.G2 RC | R-35 (risk register, open, User-decision; "P8 gate; trigger: first signed-build attempt"; doc 10 G12) | user |
| Windows code-signing certificate availability | 8.E1 signing + upgrade-path test | 8.E1 card itself says `[VERIFY] — escalate if absent` | user |
| 3-OS runner machines (live CI is currently one lean Linux job; 3-OS matrix parked) | 8.B1 matrix, 8.D1 3-OS perf runs, 8.G2 RC smoke on 3 OS + web | R-20 refresh note (issue #21): "3-OS matrix parked"; doc 10 §10.4 budget | user |
| CI billing for full matrix + RC builds | 8.B1, 8.D1, 8.E1–8.E3, 8.G2 | doc 10 §10.4 (~670 h/month budget line) | user |

Agents proceed on everything not blocked; when a blocker bites, mark the issue `blocked` per MP §17 (reason + next step), do not silently descope.

## 2. Work items (8 issues covering all 16 cards)

### Item P8-1 — Security sweeps: parsers, platform/network, supply chain + SBOM
* **Cards:** 8.A1, 8.A2, 8.A3 · **Deps:** none (consumes 7.L1-V, 1.E5 outputs, already in tree) · **Size:** L

**Proposed title:** `P8: Security sweeps — parsers fuzz/triage, plugins+control+network, supply chain + SBOM (8.A1–8.A3)`

**Body draft:** Three verification tracks per doc 11 §7. (a) Parsers (8.A1): expand the landed netfuzz codec corpus (issue #14 — targets already in `fuzz/fuzz_targets/`), triage **all** accumulated findings, probe/demux/decode hardening review; own `docs/security/sweep-parsers.md` + fix cards. (b) Platform/network (8.A2): sandbox-escape review, permission-model audit, loopback/token review, secret-redaction verification; own `docs/security/sweep-platform.md`; **first triage item is the open 6.H1 Medium (same-host https→http downgrade) + 3 Low** (issue #15, sketches in `docs/investigations/network-security-review.md`). (c) Supply chain (8.A3): cargo-deny/vet clean, SBOM regeneration per doc 11 §9, license-compliance final (incl. design-asset residual R-07 font check), provenance record; own `docs/security/sweep-supplychain.md`. **AC (per card):** every finding fixed or risk-registered with owner + next step; audit jobs green; SBOMs attached to artifacts.

### Item P8-2 — Full-corpus compatibility matrix × 3 OS + web, gap triage and status flips
* **Cards:** 8.B1, 8.B2 · **Deps:** P8-1 · **Size:** L

**Proposed title:** `P8: Full-corpus compatibility matrix (every format × 3 OS + web) + honest gap triage (8.B1–8.B2)`

**Body draft:** Run the full-corpus matrix (8.B1): every supported format × Windows/macOS/Linux + web, evidence per doc 10 §7 with mock-vs-real runs labeled; the VLC 3.0.23 differential oracle (issue #19: 29/35 bit-exact, 6/35 f32 deviation path, `docs/corpus/vlc-diff-report-20261023.md`) is the reference for claimed-supported cells. Then gap triage (8.B2): every gap either fixed (bounded fix) or status honestly `partial`/`blocked` with reason and next step per the MP §17 blocked definition; the parity tool applies all transitions. **AC:** complete matrix with no empty claimed-supported cells; zero unexplained gaps. **Blocked-by-external:** 3-OS runners (user).

### Item P8-3 — Soak/stress completion
* **Cards:** 8.C1 · **Deps:** P8-2 · **Size:** M

**Proposed title:** `P8: Soak & stress completion — full suite, reopen stress, memory slopes, quarantine (8.C1)`

**Body draft:** Complete the soak/stress program per doc 10 §8: full soak suite (4 h × scenarios), reopen-cycle stress, memory slopes vs target, LSAN nightly clean, flaky-test quarantine resolved or removed, regression-guard audit. **Already in flight:** the soak nightly workflow exists and runs (issue #18, `.github/workflows/soak-nightly.yml`) — extend it rather than building new CI. Own `tests/soak/**` reports. **AC:** soak reports committed; quarantine empty or scheduled.

### Item P8-4 — Final perf-vs-VLC benchmarks + bounded fixes
* **Cards:** 8.D1, 8.D2 · **Deps:** P8-3 · **Size:** L

**Proposed title:** `P8: Perf-vs-VLC final report + top-regression bounded fixes (8.D1–8.D2)`

**Body draft:** Final benchmarks per doc 12 §9 (8.D1): all metrics × all codecs × 3 OS on reference hardware, honest comparisons **including pure-Rust-slower cases**; the VLC 3.0.23 differential report (issue #19) anchors like-for-like comparisons. Then bounded fixes (8.D2): top-3 regressions vs baseline fixed with **no correctness trades**; report finalized on post-fix numbers. Own `docs/perf/final.md` + benches. **AC:** numbers + manifests committed; no silent re-runs; fixes regression-tested. **Blocked-by-external:** 3-OS reference hardware (user).

### Item P8-5 — Packaging per platform + SBOM
* **Cards:** 8.E1, 8.E2, 8.E3 · **Deps:** P8-4 · **Size:** L

**Proposed title:** `P8: Packaging — Windows MSI/signing, macOS dmg/notarization (R-35), Linux AppImage/deb/rpm + SBOM (8.E1–8.E3)`

**Body draft:** Three packaging tracks. Windows (8.E1): MSI/WiX installer (`[VERIFY]` toolchain purity), code signing (`[VERIFY]` certificate availability — escalate if absent), launch test, upgrade path. macOS (8.E2): .dmg, codesign + notarization — **externally blocked on the Apple Developer account R-35 (user)** — universal-binary decision, launch test. Linux + SBOM (8.E3): AppImage/deb/rpm (`[VERIFY]` toolchain), final SBOM + checksums per doc 11 §9 across all platforms; **AC:** packages launch-tested per platform; SBOMs + checksums attached to artifacts. 8.E3 formally depends on 8.E1+8.E2; if R-35 stays open, ship Linux+Windows SBOMs and track macOS as blocked per MP §17.

### Item P8-6 — Parity tool finalization + honest parity statement + known limits
* **Cards:** 8.F1, 8.F2 · **Deps:** P8-2 (8.F1 needs the matrix/gap outputs from 8.B2) · **Size:** M

**Proposed title:** `P8: Parity-report tool lockdown + honest parity statement + known-limits doc (8.F1–8.F2)`

**Body draft:** Finalize the parity-report tool per doc 10 §7 + MP §17 (8.F1): status transitions strictly from the evidence outbox; matrix-vs-claims diff; **no self-verification path exists in code** — AC: attempting a manual status flip fails. Then the user-facing statement (8.F2): verified/partial/unsupported/blocked with evidence links, plus the known-limits doc final (Lua extensions, DVD/BD, DVB, patent notes per MP §14); AC: every sentence traceable to evidence. Own `tools/parity-report/**`, `docs/parity-statement.md`, `docs/known-limits.md`.

### Item P8-7 — Docs completeness sweep + release checklist + RC smoke
* **Cards:** 8.G1, 8.G2 · **Deps:** P8-5, P8-6 · **Size:** L

**Proposed title:** `P8: Docs completeness sweep + release checklist + RC smoke on 3 OS + web (8.G1–8.G2)`

**Body draft:** Docs sweep (8.G1): format records complete, ADR register current, user docs + troubleshooting, README claims audited — **no "all formats" claims anywhere**; AC: audit checklist all green (own `docs/**` audit report). Release checklist + RC smoke (8.G2): version stamping from product.toml, RC builds on 3 OS + web, launch/play/install/uninstall tests, update-URL wiring; AC: checklist executable end-to-end on the RC (own `docs/release-checklist.md` + RC pipeline). **Blocked-by-external:** 3-OS machines + CI billing (user).

### Item P8-8 — [LEAD] Phase 8 gate + program exit
* **Cards:** 8.H1-V · **Deps:** P8-7 · **Size:** S

**Proposed title:** `P8: [LEAD] Program exit — MP §17 done-definition review + final evidence record (8.H1-V)`

**Body draft:** Lead-run verification (not agent-claimed): review vs the MP §17 done definition — language audit passed; product.toml + schemas drive the product with a passing rebrand test; UI within Figma tolerance; plugins load and sandbox correctly; web + desktop both play real media through the same command layer; CI merge-blocking on all platforms; parity report states exactly what is verified, partial, unsupported, blocked. **AC:** final evidence record in `docs/gates/phase8-exit.md` + worklog; program closed or follow-up program drafted.

## 3. Card coverage check (all 16 accounted for)

| Card | Item | Card | Item |
|---|---|---|---|
| 8.A1 | P8-1 | 8.D2 | P8-4 |
| 8.A2 | P8-1 | 8.E1 | P8-5 |
| 8.A3 | P8-1 | 8.E2 | P8-5 |
| 8.B1 | P8-2 | 8.E3 | P8-5 |
| 8.B2 | P8-2 | 8.F1 | P8-6 |
| 8.C1 | P8-3 | 8.F2 | P8-6 |
| 8.D1 | P8-4 | 8.G1 / 8.G2 / 8.H1-V | P8-7 / P8-7 / P8-8 |

## 4. Suggested sequencing (parallelism 2–4 per doc 13 §11)

P8-1 → P8-2 → (P8-3 → P8-4) and (P8-6 after P8-2) in parallel → P8-5 → P8-7 → P8-8. P8-5's macOS sub-track and P8-7's RC smoke can start early on unblocked platforms while R-35 / 3-OS blockers resolve. 8.H1-V is Lead-only and ends the program.
