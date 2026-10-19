# Worklog — Lumen Media Player (multi-agent program)

Append-only shared log. Format and protocol: AGENTS.md §4.
Task statuses: not_started, in_progress, implemented, verified, blocked.

---
Task ID: 1.0
Agent: Lead
Task: Bootstrap repository and launch Wave 1 (planning).

Work Log:
- Inspected environment; found uploaded design package "VLC Redesign (Community) (1).zip" (105 SVG exports, ~77 MB: Player variants incl. Light/Music/hide-UI/customize, 14 Settings screens, Onboarding, Keyboard Shortcuts, network stream, playlists, context menus, EQ/Spatializer/Compressor/Stereo Widener, Video Colors/Crop/Essential/HDR, Components, Miniplayer, Icon).
- Unpacked design exports to design/figma/ (102 files on disk).
- git init on main; wrote .gitignore, README.md, AGENTS.md (agent protocol), this worklog.
- Writing docs/MASTER-PROMPT.md (verbatim authoritative brief) next.
- Wave 1 plan: Wave A (01-architecture, 02-phase0-audit-plan, 03-engine-plan, 04-codec-format-roadmap, 05-command-layer), Wave B (06-plugin-architecture, 07-config-driven-system, 08-ui-figma-fidelity, 09-web-wasm-target, 10-testing-ci-quality), Wave C (11-dependency-security-compliance, 12-performance-benchmarking, 13-agent-wave-program, 14-risk-register, 15-git-workflow), then docs/ROADMAP.md consolidation.

Stage Summary:
- Repo + governance scaffolding in place; design package pinned as visual source of truth.
- Decisions: working name "Lumen" (rebrandable via product.toml); design assets committed to repo (license check tracked as risk); Lead commits during planning waves.

---
Task ID: 1.A1
Agent: Sub-agent (general-purpose), Wave A
Task: Write docs/01-architecture.md — the definitive crate & architecture plan.

Work Log:
- Read worklog.md, AGENTS.md and docs/MASTER-PROMPT.md in full before writing.
- Designed layer model L0–L6 with strictly downward dependency edges; restated MP §3's nine rules as ARCH-1…ARCH-9 contracts, each mapped to a CI machine check.
- Wrote complete crate registry: all 30 crates from MP §4, plus apps/{desktop,web,cli,control-server} and ui/, each with responsibility, Rust interface sketch, allowed internal deps, prohibitions, test strategy, phase (MP §16), and builtin plugin kinds (MP §6). Summary table first.
- Specified media-types core types (SourceUri, Track/Stream/Generation ids, Packet, Frame, Timestamp/TimeBase with explicit Overflow/Discontinuity/Missing error variants, rational Rate, Duration Known/Unknown) and the stage-tagged 7-kind error taxonomy with the no-secrets rule.
- Specified threading model (thread roles, sync engine core / async only at edges, channel topology, backpressure, generation-based cancellation), the xtask check-architecture design (cargo metadata + boundaries manifest + cycle/global-state/unsafe scans), crate governance (seven gates, split/merge triggers, ownership map) and an ADR register seeded with ADR-001…006 plus ADR-007…010 [PROPOSAL].
- Included mermaid pipeline diagram and ASCII layer diagram; marked inventions [PROPOSAL] and unverified claims [VERIFY] throughout; ended with six open questions (Q1–Q6) for the Lead.
- Did not touch any other project file; no git commit (Lead commits).

Stage Summary:
- docs/01-architecture.md delivered (~6,200 prose words; 1,500 lines; sections §1–§8).
- Key [PROPOSAL]s needing Lead decision: shared-interfaces definition (plugin-api hosts cross-cutting traits), playback-engine implements the commands contract and does NOT depend on implementation crates, muxing lives in transcode, boundaries.toml manifest + xtask check-architecture, event coalescing policy, control-protocol/apps phases.
- Open questions Q1–Q6 logged at doc end (testkit dev-dep strictness, muxer ownership, trait placement, commands direction, control-protocol phase, event semantics).

---
Task ID: 1.A4
Agent: general-purpose sub-agent (Wave A)
Task: Write docs/04-codec-format-roadmap.md — format & codec coverage roadmap with per-format records.

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md (§8, §9, §14, §16).
- Verified ~70 crates via crates.io HTTP API (existence, version, date, license, repo) and downloaded 10 crate tarballs (rav1d, mp4parse, symphonia-format-mkv, mpeg2ts-reader, truehd, mpeg-ps, ogg, lewton, claxon, matroska) to inspect for C/asm sources and build.rs (purity proof).
- Wrote docs/04-codec-format-roadmap.md (~6,075 words): status vocabulary + P0–P3 priority scheme; container table (15 formats) + records; audio table (19 codecs) + notes; video table (18 codecs) + H.264/HEVC honest assessment; subtitle table (14 formats) + ASS styling decision; per-format record template (TOML front-matter + §8 fields) with 2 illustrative filled examples (FLAC strong, H.264 gap); ranked 10-entry gap register with parity consequences and truthful-reporting rules; phase sequencing (Phase 2 first vertical proposal: MP4+PCM+Motion-JPEG, zero ADRs) and fuzz-target registry; Phase 0 audit list.
- Key verified findings: Symphonia 0.6.1 upstream status table (WAV/FLAC/MP3/Vorbis/PCM "Excellent", AAC-LC "Great", HE-AAC/Opus/WavPack in-work, symphonia-codec-opus unpublished); rav1d 1.1.0 exists but ships .S/.asm + cc/nasm-rs build.rs (policy-compliant only with asm feature off); truehdd `truehd` crate is pure Rust (Apache-2.0); mp4parse/mpeg2ts-reader/ogg/lewton/claxon tarball-verified pure; NO pure-Rust H.264/HEVC/VP8/VP9/Theora/AC-3/DTS/WMA/VC-1 decoders exist (reserved or NOT_FOUND names confirmed); rustysynth/oxisynth for MIDI [PROPOSAL].
- All unverifiable claims marked [VERIFY]/[VERIFY-P0] (24 markers), inventions marked [PROPOSAL] (12); no C-wrapping crate recommended (opus/audiopus/vorbis/libdav1d/openh264/libass/wavpack explicitly listed as forbidden bindings).
- Touched only docs/04-codec-format-roadmap.md and this worklog entry; no git commit.

Stage Summary:
- docs/04-codec-format-roadmap.md delivered (6,075 words, 8 required sections, fuzz target per parser, all crate claims verified 2026-10-19 via API + tarball inspection with inline repo URLs).
- Biggest open decisions for Lead/Phase 0: ADR-001 hw-first for H.264/HEVC (+ WebCodecs-as-OS-decoder for wasm32), rav1d asm-off adoption, Opus strategy (contribute to Symphonia vs self-build), Phase 2 first-vertical pair confirmation.


---
Task ID: 1.A3
Agent: Sub-agent (general-purpose), planning wave A
Task: Write docs/03-engine-plan.md — subsystem-by-subsystem media engine specification.

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md (§3, §8, §13, §16 focus). docs/01-architecture.md absent (agent 1.A1 parallel) — proceeded from MASTER-PROMPT and noted the reconciliation dependency in doc 03 §10.
- Wrote docs/03-engine-plan.md (~7,200 prose words / 8,900 incl. code sketches): §2 engine overview (pipeline, ASCII dataflow diagram, ownership rules); §3 media-types foundations (Ts/PacketTs/MediaTime, no sentinels); §4 all 13 subsystems (input, media-probe, demux, packet-queue, codec-api, playback-engine, media-clock, audio-engine, audio-output, video-engine, renderer-wgpu, subtitles, playlists) each with responsibility / trait sketches / threading / errors / invariants / tests; §5 two-level epoch+pipeline generation semantics with check points and stale-drop guarantee; §6 seven-step seek flow with failed-seek contract (demuxer invariant D4); §7 Phase 2 vertical slice — evaluated WAV+PCM, AVI+MJPEG+PCM, Ogg/Theora+Vorbis, recommends AVI+MJPEG+PCM with generated fixtures; §8 testkit design + property tests + MP §13 scenario mapping table; §9 threading summary table; §10 open questions for Lead.
- Fixed all cross-references (§5–§9) and TOC numbering; verified no leftover <!-- CONT --> markers.
- Marked ~37 [PROPOSAL] items (probe defaults, queue limits, two-level generations, subtitle compositing, slice recommendation, etc.) and 3 [VERIFY] items; external crates deferred to doc 04 as instructed.

Stage Summary:
- docs/03-engine-plan.md delivered: full engine spec traced to MASTER-PROMPT §3/§8/§13/§16, ready for Lead review and doc 05 (command/event contract) consumption.
- Key proposals awaiting Lead decision: AVI+MJPEG+PCM first slice; seek/track-switch modeled as Buffering re-entry (no dedicated states); demux-stage Ts repair policy; queue internals (mutex+deque now, lock-free later); device-loss suspend default; Ended-state seek-back.
- No code, no other files touched, no git commit (per protocol).

---
Task ID: 1.A5
Agent: Sub-agent (general-purpose)
Task: Write docs/05-command-layer.md — complete command & event contract specification v1 (one typed command layer).

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md (§2, §3, §5, §8, §10, §12, §13). docs/03-engine-plan.md was absent at start (1.A3 parallel); drafted from MASTER-PROMPT, then reconciled against 03 after it landed: added §4.2 wire-mapping note (SessionId ↔ 03 session epoch, gen ↔ 03 pipeline generation; 03 internal event names → wire IDs; FirstFrame/SessionClosed absorbed).
- Wrote docs/05-command-layer.md (713 lines; ~4.9k prose words, ~9.5k raw tokens incl. mandated catalog tables/code sketches) in 3 parts (Write + two `<!-- CONT -->` marker Edits); consistency pass fixed envelope/payload field collisions (seekKind/trackKind), cross-refs (§7.4/§7.3), typos.
- Sections: §1 contract principles (truthful-UI hard rules C1–C8; S sync-complete vs A async-observed classes; boundary validation rules V1–V8; envelope sketches); §2 command catalog v1 (~65 IDs, kebab-case ↔ Rust enum mapping, typed params/results/completion/errors/notes per area: session, playback, tracks, audio, video, subtitle, playlist, media-info, config, system; common error taxonomy incl. redaction); §3 event catalog v1 (~24 events with payloads, ordering, throttling; playback state machine); §4 correlation & ordering (request_id, gen staleness rules, seq ordering invariants, subscriptions); §5 bindings (Tauri single lumen_dispatch + schema-first codegen [PROPOSAL]; wasm cooperative slicing + Worker/COOP-COEP plan + typed gaps; CLI top-30 table + schema-generated --help; control server loopback-only, versioned WS, token handshake, deny-by-default [PROPOSAL]); §6 versioning (v1 freeze, additive rules, deprecation, product.toml feature gating); §7 testkit conformance suite across 4 hosts incl. stale-gen scenarios + UI bypass prevention (5 enforcement layers); §8 keymaps/menus as data + default keymap seed table [PROPOSAL] + UI-action registry [PROPOSAL]; §9 open questions; Unfinished: none.
- Touched only docs/05-command-layer.md and this worklog append; no git commit (AGENTS §3.8).

Stage Summary:
- docs/05-command-layer.md delivered: the single typed command/event contract for all four hosts, fully traced to MASTER-PROMPT; all inventions marked [PROPOSAL].
- Lead decisions requested (doc §9): set-position vs seek; UI-action dual namespace; control transport + read-only mode; token distribution; event `origin` as contract; secondary-audio semantics; numeric defaults (volume ceiling, tick rate, budgets); web Blob param variants.


---
Task ID: 1.B8
Agent: Sub-agent (general-purpose), Wave B
Task: Write docs/08-ui-figma-fidelity.md — UI implementation & Figma fidelity plan.

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md (§2/§3.5/§5/§7/§10/§13/§16), design/README.md; skimmed docs/05-command-layer.md §8 (keymaps/menus as data) and docs/01-architecture.md `theme-tokens` crate. docs/07-config-driven-system.md absent (parallel agent) — noted in doc header, written to reconcile later.
- Mined all 102 SVGs with rg/python (no whole-file reads): fill/stroke/stop-color frequency, rx radii, stroke-widths, fill-opacity, backdrop-blur (25px/13.75px), feGaussianBlur shadows; layer-name state extraction (Property 1=hover/selected, state=open/click, Value=on/off, type=play/pause); text recovery via outlined-path ids; cap-height measurement via bbox script.
- Key findings: text fully outlined (no font attributes anywhere — family [VERIFY] needed); dark palette = ~10-step warm-gray ramp (#090909→#EAEAEA) + orange accent family (#D37918/#AB6314/#371F06/#E8851A/#C2711A/#D8BA9A) + 9 user-selectable accents; light theme re-maps 9 values, accent constant; radius ramp 2/4/6/10/12/14/19; 1.5 stroke icon system; component annotation chrome (#8A38F5 dashed) must be excluded; some artboards at 1.1× scale (normalization required); seek = 8×24 bar-knob; menu accelerators conflict with doc 05 seed keymap (Ctrl+O/F/N/D/C/R, Shift+F1); 13 settings areas on disk vs README's 14.
- Wrote docs/08-ui-figma-fidelity.md (~6,100 prose words, 490 lines) in 3 parts (Write + 2 CONT-marker Edits): §1 intake analysis (palette/typography/spacing/radii/components/screen→route inventory), §2 Figma source-of-truth strategy + license risk R-07, §3 token pipeline (W3C design tokens [PROPOSAL], primitive→alias→semantic grammar, 9-accent theming), §4 component architecture + §7 UX checklist + a11y, §5 truthful-UI patterns (PlayButton × engine-state table, pending-seek rules), §6 screenshot-diff harness (tolerances, per-OS baselines, xtask, flakiness controls), §7 video-in-webview candidates A/B + matrix + per-webview protocol + B-first recommendation [PROPOSAL], §8 ui/ workspace (Vite+vitest+RTL+Playwright [PROPOSAL]), §9 test plan mapped to CI gates, §10 open questions. 15 [VERIFY-DESIGN], 34 [PROPOSAL] markers; traceability summary to MP §§.
- Fixed a leftover self-correction note in the accent table; verified no CONT markers remain, headings 1–10 consistent.
- Touched only docs/08-ui-figma-fidelity.md and this worklog append; no git commit.

Stage Summary:
- docs/08-ui-figma-fidelity.md delivered: complete UI/fidelity plan traced to MASTER-PROMPT §7 with all values export-derived and marked. Screenshot-diff and token-pipeline designs feed doc 10 (testing/CI) and Phase 1-2 gates.
- Lead decisions requested (doc §10): Figma file/variables + R-07 license, interim font, tokens format, keymap reconciliation (design vs doc 05 seed), 13-vs-14 settings areas, missing-state designs, B-first video-in-webview order, per-OS baselines, JS toolchain approval.

---
Task ID: 1.A2
Agent: general-purpose (timed out before reporting; Lead verified output and appended this entry on its behalf)
Task: Write docs/02-phase0-audit-plan.md — Phase 0 audit plan (baseline pinning, inventory methodology, feature-matrix schema, feasibility report structure, ADR-001/002/003 briefs, Phase 0 agent breakdown).

Work Log:
- Wrote docs/02-phase0-audit-plan.md (739 lines): objectives/exit gate, VLC 3.0.23 pinning procedure, inventory methodology, feature-matrix.toml schema with example rows, feasibility report structure, ADR briefs, Phase 0 agent breakdown, risks, open questions, traceability + [VERIFY] ledger appendices.
- Agent hit context deadline before final report/worklog append; Lead inspected the file and confirmed all required sections present with no leftover markers.

Stage Summary:
- Doc 02 delivered. [VERIFY] ledger (V1–V13) feeds Phase 0. Open questions for Lead in doc §9.

---
Task ID: 1.B7
Agent: Sub-agent (general-purpose), Wave B
Task: Write docs/07-config-driven-system.md — complete config-driven product specification.

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md (§5 primary; §2, §4, §7, §13, §16), docs/05-command-layer.md (§2.11, §5.3, §6.4, §8), docs/01-architecture.md (§2.3 L1 config crates, §5 boundary enforcement), docs/02-phase0-audit-plan.md (§4 feature-matrix coupling), design/README.md; verified Player.svg exports at 1920×1080 for window-default reasoning. docs/06-plugin-architecture.md absent (parallel agent 1.B1) — noted in header and §11 Q9; proceeded.
- Wrote docs/07-config-driven-system.md (~7,270 words excl. code blocks; 891 lines) in 3 parts (Write + two `<!-- CONT -->` marker Edits), then consistency pass (§4.3/§7.4 cross-refs, naming-convention note).
- §1 philosophy: four litmus tests (rebrand/translation/policy-mechanism/safety-bound) + 13-row borderline-case verdict table + four anti-patterns. §2 complete product.toml draft (identity+per-OS ids, window+title template, brand, defaults, minimal Phase-2 feature exposure with F1/F2 rules, playback/track-selection rules incl. hw-decode pending ADR-001, safety-bounded limits, opt-in-off telemetry, URLs, migrations placeholder) — every key documented inline.
- §3 product.schema.json design (2020-12 sketch, rule inventory incl. unique-identity/reserved-names, path-existence + cross-field semantic passes, validator pipeline, actionable-diagnostic message contract). §4 six-layer precedence: per-OS file locations, admin override mechanisms [PROPOSAL] with lock lists, 7-rule merge algorithm, 16-row test matrix. §5 settings-schema: full entry format (10 mandated fields + D1–D6 no-duplicate rules), four generators (TS screens/CLI help/docs/runtime validation), seed of ~45 entries mapped to the 13 named design screens, honest v1 option-count budget (~250–350 [PROPOSAL] vs VLC ~2,000 [VERIFY]).
- §6 localization: key grammar + namespaces, ICU-subset format with en-US.toml example entries, fallback chain, 5 CI audit rules, Rust/TSX no-literal lint strategy with allowlists + positive/negative fixtures. §7 layouts/keymaps (binding-id indirection, doc-05 path mismatch flagged)/profiles/themes (token refs only). §8 rebrand: build-time vs runtime planes, 10-field×7-surface propagation matrix, Nova-fixture acceptance test with fast/nightly tiers. §9 CI: five MP §5 failing conditions (C1–C5, C5 → doc 01 §5) + 11-step job sequence. §10 per-component test plan (validator, merge, codegen idempotence, rebrand, lint effectiveness, l10n, settings runtime). §11 nine open questions; Unfinished: none.
- 35 [PROPOSAL] and 12 [VERIFY] markers counted. Touched only docs/07-config-driven-system.md and this worklog append; no git commit (AGENTS §3.8).

Stage Summary:
- docs/07-config-driven-system.md delivered: the config-driven product spec traced to MASTER-PROMPT §5, with the Phase-1-ready product.toml draft as its centerpiece.
- Key [PROPOSAL]s needing Lead decision: admin override per-OS mechanisms + lock semantics; session-scope key set; D1 default_ref heuristic; diagnostic message format; v1 option budget ~250–350; rebrand fast/nightly tiers; config/ directory role; en-US reference-catalog completeness rule.
- Open questions Q1–Q9 logged (incl. design-package 13-vs-14 settings-area count [VERIFY] for doc 08, keymap path mismatch vs doc 05 §8.1, doc 06 plugin-manifest reconciliation).


---
Task ID: 1.B9
Agent: Sub-agent (general-purpose), Wave B
Task: Write docs/09-web-wasm-target.md — complete web/WASM target specification.

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md (§2, §3.5, §10, §11, §12, §14, §16); consumed docs/01 (apps/web, ADR-009), docs/02 §4 (matrix schema/status vocabulary), docs/03 §4.9/§4.11/§9 (AudioSink, RenderSurface, threading), docs/04 (WebCodecs/web columns), docs/05 §5.2/§7 (wasm binding, conformance). docs/08-ui-figma-fidelity.md was ABSENT (parallel agent) — noted in doc header; UI-sharing statements derived from MP §2/§7 + doc 01, flagged for reconciliation.
- Verified live (2026-10-19): fetched @mdn/browser-compat-data (unpkg latest) and queried programmatically — WebGPU (Chrome 113/144-Linux-Intel, FF 141 Windows-only, Safari 26 incl. iOS; FF-Android none), SAB/wasm-threads+COOP-COEP gating, Atomics.waitAsync, AudioWorklet, WebCodecs VideoDecoder/VideoFrame (Chrome 94, FF 130 desktop, Safari 16.4), GPUExternalTexture, OffscreenCanvas, rVFC, FSA API (Chrome/Edge only), JSPI (Chrome 137, FF 153, Safari 27), WasmGC/EH-final; rustc book platform pages for wasm32-unknown-unknown (thread::spawn panics), wasip1 (spawn errors), wasip1-threads (nightly, untested by Rust), wasip2 (no threads documented).
- Wrote docs/09-web-wasm-target.md in 3 parts (Write + two <!-- CONT --> Edits), then consistency pass: fixed cross-refs (§3.4→§3.3, §9.4→§9.5), promoted §9 items to subsections, trimmed to ~5.2k prose words, verified 0 leftover CONT markers and no stray text.
- Sections: §1 target definition (4 pillars, <video> rejection [PROPOSAL]); §2 toolchain (wasm32-unknown-unknown recommendation w/ wasip1/wasip1-threads/wasip2 comparison table [VERIFY], wasm-bindgen vs raw imports [PROPOSAL], JSPI future, static bundling, size budget 3/6 MB [PROPOSAL] + per-codec tree-shake proof, 6-item web CI job); §3 threading (v1 cooperative engine-in-Worker with pump/slice budgets, v2 wasm-bindgen-rayon pool, COOP/COEP headers, capability negotiation + graceful degradation, honest `threading: single|pooled(n)`); §4 rendering (RenderSurface→WebGPU/WebGL2, 3 frame-upload paths incl. GPUExternalTexture [VERIFY], DPR, truthful backend negotiation); §5 audio (AudioWorklet mapping of AudioSink/RenderCallback, SAB ring vs message transport, sample-counter clock, drift/resample, autoplay gesture-required substate [PROPOSAL]); §6 I/O (fetch+Range seekability detection, File/Blob no-upload, HLS/DASH shared, blocked capabilities typed); §7 @lumen/wasm-client over Worker/MessagePort, lifecycle, WasmBrowserHost conformance; §8 browser matrix (BCD-cited table, iOS constraints, CI vs manual tiers, baseline promise); §9 security (CORS+Range preflight, no FS, CSP wasm-unsafe-eval, plugin-host-wasm re-audit on web, COOP/COEP embedding trade-off off-by-default, no telemetry); §10 known-limit table with matrix vocabulary (WebCodecs as web analog of ADR-001 [PROPOSAL pending ADR]); §11 test plan; §12 nine open questions; Unfinished: none (doc 08 reconciliation pending).
- 36 [PROPOSAL] + 14 [VERIFY] markers; all browser facts cite BCD/MDN/rustc-book fetched 2026-10-19.
- Touched only docs/09-web-wasm-target.md and this worklog append; no git commit (AGENTS §3.8).

Stage Summary:
- docs/09-web-wasm-target.md delivered (~5,215 prose words / 6,333 total, 335 lines, §1–§12), fully traced to MASTER-PROMPT §10/§2/§3.5/§12/§16 with live-verified browser/target facts.
- Lead decisions requested (doc §12): wasm-bindgen adoption; capability-report home in doc 05 catalog (EngineInfo extension vs system.get-host-capabilities); playback.gesture-required event; worker-crash event; size-budget numbers; WebCodecs ADR (web analog of ADR-001); OffscreenCanvas render worker; COOP/COEP off-by-default embedding policy; WebGL2-only tier semantics in the matrix.


---
Task ID: 1.B6
Agent: Sub-agent (general-purpose), Wave B
Task: Write docs/06-plugin-architecture.md — complete plugin system specification (taxonomy, manifests, registry/selection, builtin pattern, WASM extension host, forbidden enforcement, conformance kit, v1 fence, test plan).

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md in full (§6 primary; §5/§9/§12/§13/§14); skimmed docs/01-architecture.md (plugin-api/plugin-host-wasm sections, crate table, layering, ADR register) and docs/04-codec-format-roadmap.md §1–2 + §8.5 (fuzz registry), doc 05 plugin commands/events.
- Wrote docs/06-plugin-architecture.md (~5,430 prose words / 7,420 raw incl. TOML + tables; 1,045 lines) in 3 parts (Write + two `<!-- CONT -->` Edits); fixed a nested-backtick marker; verified no leftover markers, no duplication.
- Sections: §1 taxonomy (16 kinds — splits doc 01's SubtitleFormat/PlaylistFormat into parser+renderer/parser+exporter [PROPOSAL Q1], trait+owner crate+lifecycle+pipeline position per kind, WASM-allowed-kind allowlist); §2 manifest spec (field-by-field: id namespacing, kind, semver, api_version, capabilities per kind, priority, permissions [fs-sandboxed-read/http-fetch+host-allowlist/ui-surface/device-query], config_schema, dependencies, provenance; 4 complete examples: matroska demuxer, FLAC decoder, WASM metadata-provider, WASM ui-panel); §3 registry build (5 steps, slotmap+capability indexes), probe→candidates→selection flow, scoring+tie-breaks (probe score > priority > user order > builtin-first > id), never-invent mechanics, hot-reload=none [PROPOSAL]; §4 Cargo-feature registration pattern (typed factory enum, manifest/feature bijection CI check), no-cycles rules; §5 extension host (runtime candidates wasmtime/wasmi, wasm3/Fizzy rejected — all [VERIFY-P0] for ADR-002; linear-memory/no-host-pointers model, import-dispatch permission enforcement, concrete limit defaults table [PROPOSAL], lumen:v1.* versioning w/ deprecation window, trap isolation + backoff→Disabled, .lumenext package + user-initiated install flow + no-auto-exec, off-realtime threading + command-layer-only UI panels); §6 forbidden enforcement with CI audit commands; §7 conformance kit (6 suites incl. capability honesty + crash injection, fuzz/<kind-short>_<id> naming reconciled with doc 04, TOML report format); §8 v1 fence (native plugins permanent-forbidden, no marketplace/auto-updates, Lua extensions will not run — MP §14); §9 test plan (unit/property/integration/security incl. trap loops, memory exhaustion, permission escalation, malicious manifests, no-auto-exec); §10 open questions Q1–Q9 + marker ledger.
- Touched only docs/06-plugin-architecture.md and this worklog append; no git commit (AGENTS §3.8).

Stage Summary:
- Doc 06 delivered: full plugin spec traced to MP §6/§5/§9/§12/§13/§14, aligned with doc 01 traits/layers, doc 04 format tables, doc 05 command/event surface.
- Lead decisions requested (doc §10): 16-kind PluginKind split; SubtitleRenderer + PlaylistExporter trait placement; ADR-002 runtime pick; signature scheme; default limits + scoring constants approval; web extension loading; ui-panel protocol; fuzz id canonicalization.

---
Task ID: 1.B10
Agent: Sub-agent (general-purpose), Wave B
Task: Write docs/10-testing-ci-quality.md — complete testing, CI and quality-gate specification.

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md in full (§13 primary; §5, §8, §12, §15, §17); skimmed doc 01 §5 (boundary CI), doc 03 §8 (testkit/scenarios), doc 04 §6/§8.5 (records, fuzz registry), doc 05 §7 (contract suite), doc 02 §2.4/§4.3/§4.6 (oracle contract, status model, R1–R10).
- Docs 06–09 were absent at start; noted in §0 with reconciliation pointers. They began landing (partial, in-flux) during writing — references kept as "pending" deliberately.
- Wrote docs/10-testing-ci-quality.md (1,007 lines; ~5,500 prose words excl. code/tables; 76 [PROPOSAL], 2 [VERIFY]) in 3 parts (Write + two <!-- CONT --> Edits) + consistency fixes (mermaid labels, G10/G11 ref, R-H2 wording).
- Sections: §0 scope/traceability + standing honesty rules R-H1..H3; §1 principles; §2 local gate — verbatim MP §13 command table (10 commands incl. UI/Tauri/wasm), `cargo xtask gate` [PROPOSAL], deps-snapshot assertions over `cargo tree`, machine-printed gate report block (honesty rule, ties to AGENTS §3.9); §3 lint policy — exact [workspace.lints] block (rust: unsafe_code/unused_must_use/unused_imports/unused_variables; clippy: unwrap_used/expect_used/panic/todo/unimplemented/dbg_macro/await_holding_lock), [lints] workspace=true inheritance check, suppression registry (Lead-approved, expiring), UI equivalents (eslint i18n-no-literals, boundary imports, strict tsc); §4 CI pipeline — GHA [PROPOSAL], job graph, fast/full/nightly tiers, 21-check required list + merge queue, artifact/evidence store, gates G1–G13 each with runner/failure/evidence (schema, rebrand propagation, cargo-deny/vet/SBOM, advisories, architecture, feature-matrix R1–R10, visual, plugin conformance, packaging+launch, fuzz-smoke, contracts, wasm, regression-guard), YAML sketch; §5 corpus — CORPUS.md licensing ledger, acquired inventory plan, deterministic generators (malformed/truncated/discontinuous), expectation/1 TOML format, 150 MB budget, bidirectional mapping to doc 04 records; §6 taxonomy — 8 levels w/ owners, 6 property suites, complete 16-row MP §13 scenario table (pass criteria + evidence artifacts), soak/stress (4 h, <5 MB/h slope, LSAN nightly), differential vs VLC 3.0.23 (harness, MD5-PCM/lossless + PSNR/SSIM/lossy, non-comparable list, weekly + opt-in label [PROPOSAL]), visual pointer, fuzz registry (~45 initial targets incl. network framing + probe + commands_json) with seeding + tiered budgets; §7 evidence — mock/real labels, evidence/1 ledger schema, parity-report outbox → Lead-applied transitions (no self-verification); §8 regression protocol + G13 test-inventory heuristic + flaky quarantine (14-day deadline); §9 §17 cycle-report template wired to CI + metrics dashboard; §10 CI-for-CI (caching, nextest, tiering, ~670 h/month budget [PROPOSAL], actionlint, gate/CI parity check); §11 open questions + proposal index.
- No commands were executed against a real workspace (none exists yet — planning wave); all runtimes marked as estimates. Touched only docs/10-testing-ci-quality.md and this worklog append; no git commit.

Stage Summary:
- docs/10-testing-ci-quality.md delivered: full testing/CI/quality spec traced to MP §13/§8/§12/§15/§17 and docs 01–05; all inventions marked [PROPOSAL].
- Lead decisions requested (§11): CI platform + self-hosted runner, macOS arch breadth, oracle cadence, tool picks (pnpm/vitest/nextest/proptest/Playwright), mock-evidence verified-category split, corpus LFS, --retry policy, triage rota.
