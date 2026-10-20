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

---
Task ID: 1.C11
Agent: Sub-agent (general-purpose), Wave C
Task: Write docs/11-dependency-security-compliance.md — dependency, security and legal/compliance program.

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md in full (§9, §12, §14, §2 primary); skimmed doc 04 §1–2 (verified crates + tarball-purity method), doc 06 §5–6 + §9.4 (WASM host, forbidden enforcement, sandbox tests), doc 10 §4.6 (G3/G4/G10 gates) + §5.1 (corpus ledger), doc 05 §5.4 (control server), doc 01 §5/§7 (ADR register incl. ADR-006/007/010).
- Wrote docs/11-dependency-security-compliance.md (709 lines, ~6,470 words incl. tables; 31 [PROPOSAL], 24 [VERIFY]/[VERIFY-P0]) in 3 parts (Write + two <!-- CONT --> Edits) + one cleanup fix (deny.toml bans list).
- Sections: §1 MP §9 forbidden list as checklist F1–F7 + exception classes E1 (webview, ADR-004) / E2 (OS-service bindings, ADR-001/005; WebCodecs ADR flagged; libc/rustix registry-entry proposal) + intake procedure; §2 audit-record fields R1–R10 with filled ACCEPTED example (mp4parse 0.17.0 from doc 04's verified findings, open items honestly [VERIFY-P0]), REJECTED example (ffmpeg-sys family: F1–F6 all fail), and CONDITIONAL example (rav1d 1.1.0 no-asm feature pin); §3 allowlist.toml format (crate→record→range→approver), cargo-deny sketch (licenses allowlist GPL-absent pending ADR-006, sources deny git, bans), cargo-vet bootstrap strategy, SBOM per release (cargo-cyclonedx + cargo auditable + npm), audits-job CI order, cargo tree baseline diff gate vs deps/ changes, version-pinning/re-audit triggers; §4 purity method codified P1–P8 (static tarball inspection, never execute) + CI approximation via tools/dependency-audit [PROPOSAL] + blind-spot table (proc-macros, target-conditional build.rs, feature-gated asm, git/patch sources, include_bytes blobs) with compensating controls; §5 threat model — assets, 8 trust boundaries, 7 adversary personas, per-boundary mitigation table, STRIDE summary; §6 SEC-01..SEC-16 testable requirements (bounds, try_from discipline [PROPOSAL], deadlines/generation IDs, path sandbox, TLS no-disable + debug-only env override [PROPOSAL], cross-origin credential drop, SecretString redaction trait, no default telemetry, fuzz registry, no-HTML-injection, forbid unsafe); §7 control-server hardening with named tests (non-loopback bind rejection, token/lockout, read-only subset, origin, audit log); §8 clean-room/GPL boundary (oracle hygiene, tools/license-audit CI scan [PROPOSAL]), ADR-006 license brief (GPL/MPL/permissive/dual axes — brief only, NOT decided), codec patent register (H.264/HEVC/AAC/AC-3 pools vs Opus/AV1/VP8-9 royalty-free, all [VERIFY], jurisdiction variance, ADR-010 shipping matrix), R-07 Figma Community asset risk + final fixture-license allowlist [PROPOSAL]; §9 abuse-case suites, dependency-confusion controls, release checklist (SBOM/advisories/signing minimal-viable [PROPOSAL]); §10 ten open questions; traceability footer.
- No WebSearch tool was available in this environment; patent/license facts are stated as widely-known positions with explicit [VERIFY: counsel] flags rather than citations.
- Touched only docs/11-dependency-security-compliance.md and this worklog append; no git commit.

Stage Summary:
- docs/11-dependency-security-compliance.md delivered: complete dependency/security/compliance program traced to MP §9/§12/§14 and docs 01/02/04/05/06/08/10; record format demonstrated with accepted/rejected/conditional examples.
- Lead decisions requested (§10): ADR-006 license pick; ADR-010 patent shipping matrix; fixture-license allowlist; libc/rustix E2a pre-approval; SEC-04/SEC-07 mechanism approval; signing budget; read-only control mode; approver delegation; multiple-versions deny date; rav1d conditional acceptance.

---
Task ID: 1.C12
Agent: Sub-agent (general-purpose), Wave C
Task: Write docs/12-performance-benchmarking.md — the performance engineering & benchmarking program (principles, metrics catalogue, bench-vs-vlc harness, baseline protocol, provisional targets, architecture-for-performance, profiling, soak perf, reporting).

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md in full (§11 primary; §1/§2/§8/§13/§16/§17); read doc 03 §2/§4.4/§4.9/§4.10/§4.11/§9, doc 05 §1/§2/§3 (event table), doc 04 §7 (gap register), doc 02 §2 (pin record, M1–M4 oracle contract, hardware template), doc 09 §2.4/§3/§4/§5/§8, doc 10 §4.5/§6.5/§6.6/§9.
- Re-verified tooling facts over network (2026-10-19): crates.io API — flamegraph/cargo-flamegraph (upd. 2026-08-12), criterion (2026-02-04), wide (2026-09-14), pulp (2026-06-20), dhat (2024-02-04, slow cadence noted), cargo-instruments (2026-06-04), iai-callgrind (2025-07-30), iai stale (2021, rejected); doc.rust-lang.org unstable book confirms portable_simd/std::simd still nightly-only.
- Wrote docs/12-performance-benchmarking.md (720 lines; ~4,900 prose words excl. tables/code; 29 [PROPOSAL], 20 [VERIFY]) in 2 parts (Write + one <!-- CONT --> Edit); verified no leftover marker.
- Sections: §1 principles (baseline-first, same-files/hardware/pins, honest-reporting, no-correctness-trade, engine-reported-not-UI-estimated); §2 metrics catalogue M-01..M-12 (endpoints, units, owning stage, doc 05 event pairs per metric, VLC M-mode points, cold/warm + steady-window defs, measurement asymmetry notes, doc 05 instrumentation deltas: envelope ts_ns, session.probe-completed, video.first-frame-presented, system.get-stats [PROPOSAL]); §3 bench-vs-vlc harness (clean-room-safe VLC surfaces rc + --play-and-exit/verbose-log [VERIFY], Lumen driven through command layer only, bench corpus tiers under tests/fixtures/bench with doc 10 §5 ledger, env control incl. governor/warmup/N/median+p95/noise budget, results/ JSON + committed docs/baselines/perf/, noise-aware regression thresholds [PROPOSAL], web suite, no-CI-by-default + perf-check label + nightly/release gates, 6 anti-gaming rules); §4 baseline protocol (Phase 0/2/3+/8 capture, storage + doc 02 singular/plural reconciliation flag, re-baselining rules); §5 provisional target table + escalation ladder + platform caveats (macOS ARM E/P cores, Linux compositors, browser families); §6 architecture-for-performance (zero-copy sketch + copy ledger, GPU color conversion per doc 03 §4.11, SIMD policy with verified crate facts + nightly-only std::simd, hw-decode truthful attribution via system.get-engine-info + sw-vs-sw/hw-vs-hw suites, bounded memory as asserted invariant, allocation discipline incl. steady-state alloc-rate≈0); §7 profiling (per-platform table, xtask profile subcommand, review-enforced profile-before-optimize, optimization PR template); §8 soak extensions (memory slope, queue stability, 1000 reopen cycles, CPU drift, layered leak detection, soak ledger); §9 reporting (cycle-report Performance block with run-ids, Phase 8 report, 5 public honesty rules); §10 ten open questions.
- Touched only docs/12-performance-benchmarking.md and this worklog append; no git commit (AGENTS §3.8).

Stage Summary:
- Doc 12 delivered: full perf program traced to MP §11/§8/§16/§17 and docs 01–05/09/10; every metric tied to doc 05 events (engine-reported); VLC comparisons tied to doc 02 M4 pin contract; all inventions marked [PROPOSAL].
- Lead decisions requested (doc §10): doc 05 ts_ns envelope field + new events/stats command; baseline dir naming (baseline vs baselines); reference hardware + self-hosted runners; VLC automation surface confirmation; SIMD crate pick; regression thresholds/enforcement; energy proxy scope; bench-corpus generator approval; browser perf tier.

---
Task ID: 1.C15
Agent: Sub-agent (general-purpose), Wave C
Task: Write docs/15-git-workflow.md — version control & branch governance specification.

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md (§13/§15/§16/§17 focus); verified repo state: branch `main`, 4 commits (f6d2ae5 initial, 33f86fa bootstrap, 3139b58 wave-a, 9850301 wave-b — all Lead-authored), no `.github/` yet; `.gitignore` (61 lines) already ignores `worktrees/`; design/figma = 102 SVGs / 75 MB.
- Cross-read docs/10-testing-ci-quality.md §4 (required checks, merge queue, evidence store) and the 1.A2 worklog precedent (Lead commits on agent's behalf). Docs 11–14 are parallel Wave C work — noted in doc header.
- Wrote docs/15-git-workflow.md (~4,150 words excl. code/tables; §0–§11): trunk-based model + anti-gitflow rationale; branch taxonomy (agent/, phase/<n>-integration, release/vX.Y, hotfix/, archive/); Lead-only merges; one-merge-per-card; squash with Task-ID trailer preservation [PROPOSAL]; Conventional Commits + worklog parity; signing deferral [PROPOSAL]; planning-phase exception (documented current state) + 7-step Phase 1 cutover; worktree layout, dependency-order merges, cleanup protocol; 10-item Lead review checklist; rebase-pre-merge, never-merge-red; phase gate tags + 0.x→1.0 versioning; design SVGs keep-in-repo vs LFS [PROPOSAL: keep, 5 MB/250 MB thresholds]; generated artifacts committed + CI freshness check [PROPOSAL]; traceability chain card→branch→commit→worklog→evidence→matrix; git-cliff changelog [PROPOSAL]; concrete branch-protection rules + force-push policy; four recovery procedures (revert, 1.A2-style salvage, corrupted main, secret purge); 10 open questions for the Lead.
- No other file touched; no git commit (Lead commits).

Stage Summary:
- Doc 15 delivered. Core policies: main always green under merge-blocking CI; agents never merge to main; squash-per-card with surviving Task-ID trailers; worktree parallelism keyed to doc 13 Depends-on order; immutable evidence-anchored phase-gate tags. Open questions Q1–Q10 await Lead (squash confirmation, archive retention, forge, `tool-results/` gitignore entry, etc.).

---
Task ID: 1.C14
Agent: Sub-agent (general-purpose), Wave C
Task: Write docs/14-risk-register.md — the program-wide risk register.

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md in full (§14 seed, §1/§2/§9/§11/§12/§15/§16/§17); skimmed doc 04 §7 (gap register) + §4.2–§4.5/§8, doc 02 §4.3/§4.6/§8/§9, doc 09 §8/§10/§12; pulled ADR register (doc 01 §7), CI budget (doc 10 §10.4), first-vertical divergence (doc 03 §7 vs doc 04 §8.1), R-07 references (doc 08 §2, design/README.md), 1.A2 timeout precedent.
- Wrote docs/14-risk-register.md (273 lines; 44 risks) in 3 parts (Write + two CONT-marker Edits) + one repair Edit (restored the missing R-07 detail row); verified: 0 standalone CONT markers, all 44 IDs present in both master and detail views, no stray text.
- Sections: §0 purpose/scope; §1 register format (12 columns over two synchronized views, L/I→exposure 1–9 scale, sort rule, re-scoring rule [PROPOSAL]); §2 register — exposure-sorted master table (5×critical 9, 21×severe 6, 17×moderate 4, 1×minor 3) + per-tier detail tables (indicators/mitigations/contingency, doc-cited) + cross-cutting observations; §3 top-10 exposure narratives; §4 ten risk-acceptance statements (A1–A10: Lua, DVD/BD ADR-008, DVB, web limits, legacy codecs, perf gap, <video> rejection, native plugins, ASS subset, FF-Android tier) tied to MP §14 "record, do not hide"; §5 monitoring hooks (matrix R5/R6, ADR statuses, worklog flags, CI G-gates, perf baselines, corpus ledger) + 6-step phase-gate re-score procedure [PROPOSAL]; §6 escalation matrix (10 user-decision triggers per MP §15: ADR-001/006/010, Figma R-07, platform drop, WebCodecs, COOP/COEP, CI spend, naming, VLC-4 scope); §7 ten open questions; §8 MP §1–§17 traceability table.
- Top exposures: R-01 H.264 sw-decode gap, R-02 HEVC, R-03 codec patents, R-04 WebKitGTK video-in-webview, R-05 repo license (ADR-006). R-07 ID preserved for the Figma design-package license risk per doc 08 cross-refs.
- [VERIFY] on R-06 Opus, R-08 rav1d asm-off, R-29 cpal, R-30 runtime candidates, R-38 Lumen collision; ~30 [PROPOSAL] procedures/thresholds awaiting Lead confirmation. Docs 11/12/13 cited as parallel/pending with reconciliation note (§7 Q10).
- Touched only docs/14-risk-register.md and this worklog append; no git commit (AGENTS §3.8).

Stage Summary:
- docs/14-risk-register.md delivered: 44-risk register sorted by exposure, fully traced to MASTER-PROMPT §§ and docs 01–10, with acceptance statements, monitoring hooks, phase-gate review procedure, escalation matrix and open questions.
- Lead decisions requested (§7): register governance rules, ADR-006 timing, ask-user-now on R-07 Figma license, AVI-vs-MP4 first-vertical pick (R-28), Lumen collision-search owner, CI/Apple account custody, escalation thresholds, A10 tier semantics, R-37 co-ownership, wave-close reconciliation vs docs 11/12/13.

---
Task ID: 1.C13
Agent: Sub-agent (general-purpose), Wave C
Task: Write docs/13-agent-wave-program.md — the complete multi-agent execution program (Phases 0–8 as waves of small agent task cards, 150+ total).

Work Log:
- Read worklog.md, AGENTS.md, docs/MASTER-PROMPT.md in full (§15/§16/§17 primary; §2 fixed decisions); skimmed doc 01 §2.1 crate/phase table, doc 02 §7 Phase 0 agent breakdown, doc 04 §8 sequencing/fuzz registry.
- Wrote docs/13-agent-wave-program.md (960 lines) in 4 parts (Write + three <!-- CONT --> marker Edits, marker removed at end; verified none remain).
- Content: §1 program principles (why small tasks; concurrency/conflict rules R1–R7 incl. Lead-owned surfaces and per-format registry merge points; 14-field task-card format; standard clauses C1–C8 incl. +FB format bundle and +SB screen bundle; 8-step inter-wave review gate; timeout/failure protocol with R1/R2 relaunches and post-mortem lines; agent hygiene); §2 Task ID grammar <phase>.<wave><seq> extending AGENTS.md §5 with -I/-V suffixes, Rn relaunches, not-selected status [PROPOSAL supersession]; §3–§11 all nine phases as wave tables — every card carries ID/type/scope+AC/owns/consumes/deps/size; §12 wave execution ledger (200 cards, 77 waves, cumulative counts, parallelism 2–6, wall-clock in wave-units 77 nominal / ~96–115 with 1.25–1.5× rework, delta log); §13 rules of engagement (launch/verify/commit/ledger, MP §17 DoD wiring, escalation policy, re-planning) + traceability + 5 open questions.
- Verified per-phase card counts by script: 0=17, 1=20, 2=17, 3=19, 4=25, 5=45, 6=17, 7=24, 8=16; total 200 (matches ledger). Phase 2 includes both [DECISION-PENDING] demuxer variants (AVI per doc 03 vs MP4 per doc 04) — exactly one executes, other closes not-selected.
- Key structures: H.264/HEVC split as parser (5.F1/5.G1) + hw-path integration [CONDITIONAL on ADR-001] (5.F2/5.G2) + per-OS adapters in 7.B1–B3; Phase 5 decision/-I cards for Opus, ASF/WMA, AC-3+DTS, VP8/9, Theora, MPEG-4 ASP, blocked-registry (honest records, not hidden); settings split framework + 3 area groups (4 groups of cards covering the 13–14 areas); wasm extension host as 4 cards per doc 06 §5.
- Touched only docs/13-agent-wave-program.md and this worklog append; no git commit (AGENTS §3.8).

Stage Summary:
- docs/13-agent-wave-program.md delivered: 200 launch-ready task cards in 77 waves across Phases 0–8, all S/M (L forbidden pre-launch), fully traced to MASTER-PROMPT §15/§16/§17 and docs 01–12.
- Lead decisions requested (§13 open questions + §5): ratify Task-ID grammar supersession and not-selected status; AVI-vs-MP4 pick before wave 2.B; rework-factor confirmation; ad-hoc fix cards in Phase 8.

---
Task ID: 1.LEAD-FINAL
Agent: Lead
Task: Consolidate Wave 1 planning into docs/ROADMAP.md; resolve cross-doc conflicts; close planning phase.

Work Log:
- Reviewed all 15 domain docs (spot-checks + agent reports; no leftover CONT markers).
- Wrote docs/ROADMAP.md: phase gates 0-8 with exit-gate evidence, wave program summary (200 cards / 77 waves), decision register D-001..D-008, cross-doc reconciliation log, user-escalation list, ADR index, doc index.
- Lead decisions: D-001 first vertical = AVI+MJPEG+PCM; D-002 settings areas = 13; D-003 keymaps = VLC-3.0.23-observed baseline overridden by design package; D-004 PluginKind = 16; D-005 Task-ID grammar + not-selected status ratified; D-006 "Lumen" placeholder pending trademark check; D-007 docs/baselines/ naming; D-008 ADR timing.
- Corrected design/README.md settings-area count (14 -> 13).
- Committed Wave 1 and tagged.

Stage Summary:
- Planning phase COMPLETE: 16 governance/plan docs + pinned design package + 17 worklog agent records; all committed on main.
- Wave 1 execution stats: 16 planning agents launched, 15 completed on first attempt, 1 (1.A2) timed out post-write and was salvaged by Lead verification.
- Next: Phase 0 wave 0.A launch (baseline pinning + inventory), pending user decisions on ADR-006 license, R-07 Figma terms, patent posture, ADR-001 appetite.

---
Task ID: 0.PRE
Agent: Lead
Task: Record user decisions (MIT license; R-07 design clearance); launch Phase 0.

Work Log:
- User decisions received 2026-10-19: (1) repository license = MIT; (2) Figma Community design package attested open-source, no terms restriction.
- Wrote LICENSE (MIT, "Lumen Media Player Contributors").
- Wrote docs/decisions/ADR-006-license-mit.md (status: accepted).
- Updated: docs/14-risk-register.md (R-05, R-07 -> closed + closure log), docs/11 §8.2 (DECIDED banner), docs/ROADMAP.md (D-009, D-010; §5 resolved/open split; ADR index; status line; §8 next action), design/README.md (clearance note).
- Created docs/baselines/ (D-007 plural naming) + raw/ subdir; tmp/ for non-committed downloads.
- Committing, then launching wave 0.A agents (0.A1 Windows, 0.A2 Linux, 0.A3 macOS platform pinning) per doc 13 §3.

Stage Summary:
- ADR-006 accepted (MIT); R-05/R-07 closed. Phase 0 begins: wave 0.A baseline pinning.

---
Task ID: 0.A3
Agent: Sub-agent (general-purpose), Wave 0.A
Task: Pin VLC 3.0.23 on macOS — pin record + raw evidence (feeds gate G1 macOS row).

Work Log:
- Read worklog.md, AGENTS.md, doc 02 §1.2/§2, MASTER-PROMPT §1–2; confirmed D-007 path docs/baselines/.
- Created gitignored tmp/pinning/; probed official trees. Card URL ".../3.0.23/macos/" → nginx 404; correct segment is "macosx/" (discrepancy recorded). Full directory listings captured from get.videolan.org and download.videolan.org (identical): arm64/intel64/universal dmgs + .asc/.md5/.sha1/.sha256 sidecars.
- Downloaded all three dmgs (first attempt each, no retries; 31.0s/34.7s/2.3s; sizes match listing: 51,273,389 / 57,544,217 / 89,124,160). `file` + koly trailer confirm UDIF disk images.
- Downloaded all 12 sidecars; computed sha256/sha512/sha1/md5 of each dmg ourselves; 9/9 cross-checks vs official sidecars PASS. Universal sha256 matches doc 02 §2.2 pre-verified value.
- GPG verification (gpg 2.4.7, key from keys.openpgp.org): Good signature ×3 from "VideoLAN Release Signing Key (2018)" 65F7C6B4206BD057A7EB73787180713BE58D1ADC; key EXPIRED + DSA/SHA-1 caveats recorded honestly.
- Web citations captured: release page (3.0.23 Vetinari), news index (release date 2026-01-08; 3.0.24 drift 2026-09-22), macOS download page (now serves 3.0.24; "requires Mac OS X 10.7.5 or later… 64-bit Intel or Apple Silicon").
- Wrote docs/baselines/vlc-3.0.23-macos.md (pin record: version/date/URLs; arch coverage; acquisition; hashes computed+cross-checked; GPG result + codesign/spctl runbook [VERIFY-blocked-on-hardware]; doc 02 §2.3 JSON row; NOT INSTALLED + install runbook; smoke-dump runbook [VERIFY-blocked-on-hardware]; clean-room statement; 7 discrepancies/notes) + 6 TEXT raw evidence files in docs/baselines/raw/macos/.
- Binaries never executed, never committed (tmp/ gitignored, verified via git check-ignore); no VLC source in repo. No git commit (AGENTS §3.8).

Stage Summary:
- macOS 3.0.23 pin: hashes verified in-sandbox (9/9 sidecar matches + GPG Good ×3 with expired-key caveat); install/codesign/smoke dumps remain hardware-blocked runbooks. Card/doc-13 "macos/" path corrected to "macosx/". Front-door now serves 3.0.24 — baseline stays 3.0.23 per MP §1.

---
Task ID: 0.A1
Agent: Sub-agent (general-purpose), Wave 0.A
Task: Pin VLC 3.0.23 on Windows (doc 02 §2): official-source acquisition, hashes + signature result, install record, dump-channel smoke runbook.

Work Log:
- Read worklog, AGENTS.md, doc 02 §1.2/§2, doc 13 §3 (card), MP §1–2. Created docs/baselines/raw/windows/ (did not pre-exist despite brief; D-007 paths used) and tmp/pinning/ scratch.
- Discovered official URLs by direct listing: 3.0.23/ exists on download.videolan.org and get.videolan.org with win32/win64/winarm64 (+ .exe/.7z/.msi/.zip/debugsym, each with .asc/.md5/.sha1/.sha256 sidecars). Source tarball deliberately NOT downloaded (clean-room, doc 02 §2.1).
- Downloaded vlc-3.0.23-win64.exe (45,948,080 B; redirect served by mirror-hk.koddos.net) and vlc-3.0.23-win64.7z (39,583,707 B) into gitignored tmp/pinning/ (git check-ignore verified; *.exe doubly ignored).
- Computed + cross-checked: SHA-256 20ad1913…daaf MATCHES official sidecar on BOTH hosts AND doc 02 §2.2 pre-verified value; sha256sum -c/sha1sum -c/md5sum -c all OK. SHA-512 computed (no official .sha512 sidecars exist anywhere in tree). MSI/ZIP sidecar values fetched as text (not downloaded).
- GPG: .asc sidecar verified — Good signature from "VideoLAN Release Signing Key (2018)" DSA 65F7C6B4…E58D1ADC, sig created 2025-12-31; key expired 2026-08-01 (after signing); key fetched from keyserver.ubuntu.com (no VideoLAN-hosted key found; /keys/ and /pub/videolan/KEYS 404). Authenticode NOT run (no Windows host, no osslsigncode) → [VERIFY-blocked-on-hardware] with PowerShell runbook.
- Release date 2026-01-08 verified via news.html anchor news-2026-01-08; download-server upload timestamps are 31-Dec-2025 (staging). 3.0.24 exists and the Windows download page now serves it — drift risk recorded, baseline stays 3.0.23.
- Wrote docs/baselines/vlc-3.0.23-windows.md (pin record: facts, hashes, signature, JSON for 0.A4, install runbook, smoke runbook --version/--list/--list-verbose (+ --list-plugins flagged as probably not a 3.0.x flag [VERIFY]), clean-room statement, 8-item discrepancy ledger) + raw/windows/acquisition-log.txt (489 lines, every command + actual output incl. failures) + raw/windows/official-sources.txt (URLs + verbatim sidecar/signature text + negative results).
- Sandbox honesty: Linux x86_64, no WebSearch/WebFetch tools (curl used, recorded); Windows binaries never executed; no VLC source/assets acquired; nothing binary committed. Flagged stray untracked macosx-listing-download.html at repo root (concurrent agent's, untouched).

Stage Summary:
- Windows pin record complete for all sandbox-executable steps: artifact located, downloaded, hash-verified against official sidecars on two official hosts, GPG-verified (with key-expiry + keyserver-anchor caveats), size/type/dates recorded. Blocked-on-hardware items (Authenticode, install, --version/--list dumps, x64 payload check) documented as exact runbooks with expected shapes, all [VERIFY]-marked. Feeds G1 via 0.A4. Discrepancies: raw/windows/ path missing (created), --list-plugins flag doubt, PE32 installer stub vs win64 payload, expired release key, no official SHA-512 sidecars, 3.0.24 drift.
---
Task ID: 0.A2
Agent: Sub-agent (general-purpose), Wave 0.A
Task: Pin VLC 3.0.23 on Linux (doc 02 §2 pin record; gate G1 input).

Work Log:
- Read worklog (0.PRE context), AGENTS.md, doc 02 §1–§2, MASTER-PROMPT §1–§2. Used docs/baselines/ (D-007).
- Sandbox discovery: Debian 13 (trixie) x86_64 headless container, uid 1001 (no root), no vlc/Qt installed; apt carries BOTH 3.0.23-0+deb13u1 (trixie/main) and 3.0.24-0+deb13u1 (trixie-security).
- Distro decision (evidence): Ubuntu 24.04 noble ships 3.0.20, plucky 3.0.21-9, questing 3.0.21-11 (Launchpad API) — doc 02 §2.2 default unusable; pinned Debian trixie/main 3.0.23-0+deb13u1. Flathub org.videolan.VLC = 3.0.23 stable (API) recorded secondary.
- Acquired root-free via apt-get download at exact =3.0.23-0+deb13u1: 8 packages (vlc, vlc-bin, vlc-data, libvlc-bin, libvlc5, libvlccore9, vlc-plugin-base, vlc-plugin-video-output); all 8 computed SHA-256 MATCH the signed Packages index; trixie InRelease GPG-verified (2 Good signatures) — apt's Linux signature-check requirement of doc 02 §2.2 step 4 satisfied with recorded commands/output.
- Extracted with dpkg-deb -x (no install); resolved 43 plugin .so missing-deps via Contents-amd64.gz soname lookup + 32 support-lib debs (round-1 name-guess failure honestly recorded); final: 372/372 plugins ldd-clean.
- VLC RAN: "VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)" — dumps: --version, --list (502 modules), --list-verbose, -I dummy smoke (exit 0). --list-plugins does NOT exist on 3.0.x (VLC 4.x flag) — failure recorded; --list/--list-verbose are the 3.0.x equivalents.
- Upstream cross-evidence: get.videolan.org/vlc/3.0.23/ listing (no Linux binaries — confirms doc 02; no .sha512 sidecar — card assumption wrong, .sha256 recorded: e891cae6…3e7e0 for vlc-3.0.23.tar.xz; tarball NOT downloaded per clean-room doc 02 §2.1).
- Wrote docs/baselines/vlc-3.0.23-linux.md (pin record + JSON block + runbook + [VERIFY] ledger) and 11 text dumps in docs/baselines/raw/linux/. No binaries/debs committed (tmp/ git-ignored; *.deb ignored). No git commit (AGENTS §3.8).

Stage Summary:
- Linux pin COMPLETE and executable in-sandbox: Debian trixie 3.0.23-0+deb13u1, 8/8 hashes MATCH signed index, InRelease GPG Good, 502-module baseline dumps captured (base+video-output only; qt/skins2/… excluded — headless). Open: Lead ratify Debian-over-Ubuntu distro choice at G1; [VERIFY] snapshot.debian.org (blocked 509), Flathub commit, full-plugin census on desktop stack, sandbox CPU model.

---
Task ID: 0.A4
Agent: Sub-agent (general-purpose), Wave 0.A
Task: Consolidate the three platform pins into the master pin record; validate the doc 02 §2.4 oracle invocation contract; write hardware-template.md (completes G1 evidence).

Work Log:
- Read worklog (0.PRE, 0.A1–0.A3), AGENTS.md, doc 02 §1.2/§2, doc 13 §3 card, doc 10 §6.6 + doc 12 §3.3 (harness consumption of the pin JSON), all three pin records + raw/ listings.
- Consolidation-time re-verification: sha256sum over retained tmp/pinning/ artifacts (win64.exe, win64.7z, 3 dmgs, libvlccore9 + vlc-plugin-base debs) — 7/7 MATCH the pin records; no re-download needed.
- Wrote docs/baselines/vlc-3.0.23.md (master pin): exec summary table; canonical oracle identity (3.0.23 Vetinari, git describe 3.0.23-2-0-g79128878dd [Linux-observed; Win/mac pending], release 2026-01-08, URLs); per-platform pointers; §2.4 contract validation table (M1 PROVEN-subset on Linux: -I dummy --play-and-exit exit 0); G1 evidence checklist per-row (Linux complete; Windows/macOS partial-pending-hardware on native-signature/--version/run-env items — Lead ticks doc 02 §1.2 at 0.D4-V); consolidated discrepancy register DR-01…DR-16; clean-room statement; Lead-ratification checklist R-1…R-5.
- Wrote docs/baselines/vlc-3.0.23.json (validated with python3 -m json.tool): {baseline, codename, git_describe, release_date, signing_key, version_guard, platforms{windows,linux,macos}} with per-artifact hash objects (algorithm+value+computed_by_us+sidecar match), source URLs, run_status (linux sandbox-verified; windows/macos blocked-on-hardware), dump availability (existing + pending-hardware filenames), oracle_invocation_contract, consolidation_recheck, clean_room, DR index.
- Wrote docs/baselines/hardware-template.md: generic rules (drift guard, context-in-every-dump, text-only), environment-field table (OS/CPU/GPU/locale + doc 12 §3.5 machine_id), sign-off block; §2 contract PROVEN/PENDING table; §3 Windows runbook (env, hash, Authenticode, /S install, --version/--list/--list-verbose/--list-plugins probe, M1 smoke, x64 payload check; 10 named artifacts); §4 macOS runbook (codesign/spctl/notarization, install+arch-slice, dumps; 11 named artifacts); §5 Linux desktop-stack runbook (full 15-package pinned install, full-census dumps incl. Qt, snapshot.debian.org re-fetch, CPU model, Flathub).
- Discrepancies found while consolidating (recorded, not silently resolved): release-date confidence divergence across records (DR-10, closed by cross-record corroboration of the news-index anchor); macos/ vs macosx/; expired-key DSA with predating signature + two different keyserver anchors (same fingerprint); no .sha512 sidecars anywhere; --list-plugins 4.x-only; PE32-i386 stub; Debian-vs-Ubuntu pending; trixie-security build string from main pool; one .html raw file (macos 00-). Stray root file macosx-listing-download.html (0.A1 D8) no longer present (git status clean of untracked binaries).
- No downloads, no VLC source, no binaries committed; doc 02 NOT edited (per card); no git commit (AGENTS §3.8).

Stage Summary:
- Master pin record complete: docs/baselines/vlc-3.0.23.md + .json + hardware-template.md. G1 evidence: all sandbox-executable items COMPLETE (URLs, hashes 8/8+2+9/9 cross-checked, GPG results, Linux verbatim version line, dates, 7/7 re-verification); Windows/macOS remain partial-pending-hardware exactly on Authenticode/codesign, verbatim --version lines and run environments — runbooks + expected artifacts + sign-off fields ready. Escalated to Lead for ratification: R-1 Debian-over-Ubuntu, R-2 expired-key acceptance, R-3 hardware-run owners, R-4 doc 02/13 path spelling (baselines/), R-5 G1 tick posture at 0.D4-V.

---
Task ID: 0.A4
Agent: general-purpose (completed; reported normally)
Task: Consolidate VLC 3.0.23 master pin record + hardware template + machine-readable pin JSON.

Work Log:
- Wrote docs/baselines/vlc-3.0.23.md (master pin, G1 evidence status, DR-01..DR-16 discrepancy register, Lead-ratification checklist), vlc-3.0.23.json, hardware-template.md.
- 7/7 consolidation-time SHA-256 re-checks passed. Contract M1 proven on Linux (headless -I dummy, exit 0); M2-M4 pending hardware.

Stage Summary:
- G1 evidence: sandbox-complete; Windows/macOS partial-pending-hardware. Escalations R-1..R-5 to Lead (Debian-over-Ubuntu ratify, expired-key acceptance, hardware-run owners, path spelling, G1 tick posture).

---
Task ID: 0.C4-I (partial)
Agent: general-purpose (timed out; Lead verified partial output, logging on its behalf)
Task: Dependency pre-audit + WASM runtime candidates (ADR-002 evidence pack).

Work Log:
- COMPLETED before timeout: docs/dependencies/audit-method.md (398 lines, reproducible crates.io/tarball purity method incl. worked example), allowlist-format.md (164 lines, TOML schema + R1-R10 record template), records/wasmi.md (132 lines, worked audit record).
- MISSING (retry scoped): docs/adr/ADR-002-evidence.md (candidate table incl. wasmtime) + foundation pre-audit (serde/toml/schemars/cpal/wgpu/winit/wasm-bindgen/windows-rs).
- No worklog append happened before timeout (this entry covers it).

Stage Summary:
- 3/5 deliverables exist and are usable; retry 0.C4-R launched with narrowed scope.

---
Task ID: 0.C5-a
Agent: Sub-agent (general-purpose), split retry of 0.C5-I (Linux record only)
Task: Linux platform capabilities inventory (WebKitGTK/wry, video-in-webview candidates A/B, VA-API hw decode, audio crates, wgpu/GPU) → docs/inventory/platform-linux.md.

Work Log:
- Read worklog (0.A1–0.A4, 0.C4-I), AGENTS.md §3–§4; created docs/inventory/platform-linux.md (298 lines) + gitignored tmp/linuxaudit/ evidence (50+ files). No WebSearch/WebFetch tools in sandbox; curl used for all web evidence (recorded). GitHub API core quota (60/h, shared IP) exhausted mid-run — issue details via HTML pages; GNOME/freedesktop GitLab blocked by anti-bot (Anubis) → upstream WebKit issue search left as [VERIFY] follow-up.
- WebKitGTK: current stable 2.54.1 (2.54.0 2026-09-16); 4.0/libsoup2 API dead since 2.52; Debian trixie ships 2.52.6 (main)/2.54.0 (security) for both 4.1 and 6.0 series; Ubuntu 24.04–26.04 = 2.50.4–2.52.6 (Launchpad API). DMABUF-only renderer since 2.44; Skia compositor since 2.54; rendering timeline cited from webkitgtk.org release posts.
- wry 0.57.0: webkit2gtk =2.0.2 → GTK3/4.1 only; with_transparent = gdk RGBA(0,0,0,0); child webviews X11-ONLY (source-verified) — Candidate A has no Wayland embedding path; gtk4/webkit6 migration still open PR (wry#1767). No WEBKIT_DISABLE_* env handling in wry/tauri-runtime-wry 2.12.4.
- Artifacts evidence table: tauri#14924 (NVIDIA transparent crash/GBM Error 71/black corners), wry#1731 (wl_shm buffer accumulation on transparent Wayland surfaces, unbounded), #9304/#10702/#15936/#16226/wry#1366/#1727, 44 issues mention WEBKIT_DISABLE_DMABUF_RENDERER.
- WebCodecs: PROVEN compiled into Debian libwebkit2gtk-4.1 2.52.6 via apt-get download + dpkg-deb -x + strings (VideoDecoder/AudioDecoder/Encoders/Chunks/VideoFrame; feature flags WebCodecsVideo/Audio/AV1/HEVCEnabled; 79 "WebCodecs" strings; also MediaSource/ManagedMediaSource, WEBKIT_GST_ENABLE_LEGACY_VAAPI). 2.48 notes: prefer-hardware → GStreamer hw components. Default-enabled state left [VERIFY]. MSE+ImageBitmap fallbacks documented; wry transport surface (custom protocol/ipc/init script/evaluate) verified in source.
- VA-API: libva 2.22.0 installed in sandbox; codec matrices from intel/media-driver README (H.264/HEVC/VP9/AV1/VVC decode + encodes), Mesa radeonsi [VERIFY per-ASIC], nvidia-vaapi-driver (NVDEC shim, decode-only, Firefox-targeted). Rust bindings census: cros-libva 0.0.13 (ChromeOS, BSD-3, 1.28M dl, bindgen+pkg-config, links C libva), forks (libva 0.1.5 mlm-games, cros-libva-extended, vacc-libva), libva-sys (stale 2021), fev, higher-level (moq-vaapi flagged "(AI GENERATED)", cros-codecs-*, nuxodecs, oxideav-vaapi, vacc, rawshift-hwdec). dlopen-vs-link policy question recorded both framings, no verdict. VDPAU: vdpau-va-driver GONE from trixie; legacy note. VLC 3.0.23 module dumps cross-checked: vaapi/vaapi_drm/glconv_vaapi_{x11,wl,drm} + vdpau family present (oracle corroboration).
- Audio: alsa 0.12.1 (alsa-sys→libasound C), pipewire 0.10.1 (upstream pipewire-rs, C-ABI, trixie pipewire 1.4.2), libpulse-binding 2.30.1 (C), pulseaudio 0.3.2 (PURE RUST protocol, colinmarc), cpal 0.18.2 (Linux: ALSA default + optional JACK/PipeWire/PulseAudio backends, ALSA always required). Decision-input matrix written, no choice made.
- wgpu 30.0.1: pure-Rust over Vulkan (first-class)/GL; CompositeAlphaMode modes; lavapipe = mesa-vulkan-drivers 25.0.7 trixie [VERIFY doc URL]; X11 vs Wayland candidate-A substrate comparison table; winit with_transparent cited.
- Verdict block: ADR-003 B-first on Linux (A blocked-ish on Wayland, NVIDIA-risky; contingencies), ADR-001 VA-API readiness + policy question, ADR-005 inputs, 12 [VERIFY] follow-ups with concrete probes, 8 risk observations (L1–L8) for doc 14. No ADR decided; no git commit (AGENTS §3.8).

Stage Summary:
- docs/inventory/platform-linux.md complete (298 lines, every external fact URL+date cited, 23 [VERIFY] marks). Linux headline: B-first for video-in-webview; WebCodecs compiled-in proven in-binary; VA-API technically ready with an unavoidable C-ABI and an open policy classification; audio = PipeWire reality with one pure-Rust option. Ready for Lead synthesis (0.C5 cross-platform) and ADR-001/003/005 inputs.

---
Task ID: 0.C5-b
Agent: Sub-agent (general-purpose), Phase 0 (split retry of 0.C5-I; Windows+macOS half)
Task: Platform audit records for Windows (WebView2/MF/WASAPI/wgpu-DX12) and macOS (WKWebView/VideoToolbox/CoreAudio/wgpu-Metal) feeding ADR-001/003/005.

Work Log:
- Read worklog.md (last 100 lines) + AGENTS.md §3/§4 per first steps. No WebSearch/WebFetch tools in sandbox; used curl (+ agent-browser headless Chromium only for the JS-rendered Microsoft Store) — recorded in both files' method notes. All citations carry access date 2026-10-19.
- crates.io API versions pinned: wry 0.57.0, tauri 2.12.4 (3.0.0-alpha.5 newest), tao 0.37.1, winit 0.30.13, raw-window-handle 0.6.2, webview2-com 0.39.1, windows 0.62.2 / windows-sys 0.61.2 / windows-core 0.100.0, wgpu 30.0.1, cpal 0.18.2, objc2 0.6.5 + framework crates 0.3.2, core-foundation 0.10.1, core-video 0.6.1, coreaudio-rs 0.14.2 (vs different "coreaudio" 0.5.0 crate — flagged), coreaudio-rs used by cpal verified.
- Source-verified from published crate tarballs: wry 0.57 (with_transparent → ICoreWebView2Controller2::SetDefaultBackgroundColor(0,0,0,0); macOS drawsBackground KVC with in-source "Private API" note; WebViewExtWindows::hwnd()/controller()/environment()/webview(); WebViewExtMacOS::webview()/ns_window(); build/build_as_child on HasWindowHandle; addSubview/setContentView hierarchy), tauri 2.12.4 (WebviewWindow: HasWindowHandle+HasDisplayHandle, hwnd()/ns_window()/ns_view(); macos-private-api no-op since 2.12.1), tauri-runtime-wry 2.12.2 (pins wry 0.57.0 + tao 0.37.0), tao 0.37.1 (no WS_CLIPCHILDREN control; NO_BACK_BUFFER platform attr), cpal 0.18.2 (WASAPI: AUDCLNT_STREAMFLAGS_EVENTCALLBACK+SetEventHandle, shared-mode only, windows 0.62), wgpu 30.0.1 (dx12+metal in default features), windows 0.62.2 (Win32_Media_MediaFoundation/Direct3D11/Dxgi features present).
- Microsoft Learn fetched: ICoreWebView2Controller2 DefaultBackgroundColor (alpha 0/255 only, E_INVALIDARG otherwise, Win7 excluded, documented white-flicker + WEBVIEW2_DEFAULT_BACKGROUND_COLOR workaround); Windowed vs Visual hosting (3 modes incl. new "Window to Visual" env-var mode); Differences Edge-vs-WebView2 (WebCodecs not restricted); MF Supported Media Formats (in-box table: H.264/AAC etc.; no HEVC/VP9/AV1); distribution/evergreen page.
- Store situation (apps.microsoft.com search API + displaycatalog, 2026-10-19): HEVC Video Extensions $0.99 at NEW id 9nmzlz57r3t7; legacy id 9N4WGH0Z6VHM now 410/ProductNotFound (deep-link breakage recorded); AV1 Video Extension free (9mvzqvxjbq9v); VP9 (9n4d0msmp0pt), Web Media Extensions (9n5tdp8vcmhs), MPEG-2 (9n95q1zzpmh4), AVC Encoder (9pb0trcnrhfx) free; "Device Manufacturer" HEVC variant unlisted.
- MF API names verified live (HTTP 200): MFStartup, MFTEnumEx, IMFTransform, MFCreateDXGIDeviceManager, h-264-video-decoder; MFCreateVideoDecoder does NOT exist (404) — brief's assumed name corrected.
- Field evidence: wry #1540 (Windows transparent webview showed desktop not host content; fixed via bevy#16544 with_clip_children(false) on winit) + wry #1658/#408 artifacts; wry #1867 (macOS 15.7 black-base transparency regression, fixed in 15.8.1) + #1780 (transparency flags removed/always-on).
- WebKit/WebCodecs: WebKit blog 16.4 ("adds support for the video portion of Web Codecs API") and 26.0 ("adds AudioEncoder and AudioDecoder"); caniuse data (Safari 16.4–18.2 partial video-only, 26.0 full; Chrome/Edge 94); MDN BCD VideoDecoder Safari 16.4+; Safari 17 blog AV1-on-hardware-decode quote. Apple docs JSON fetched for VTDecompressionSession, VideoToolbox root ("direct access to hardware encoders and decoders", VTIsHardwareDecodeSupported), notarization requirements. docs.rs confirms objc2-video-toolbox 0.3.2 exports VTDecompressionSessionCreate + VTIsHardwareDecodeSupported.
- Notable discoveries recorded: windows-rs generation shift (0.62.x vs new 0.100.0 line incl. NEW windows-webview/window/composition/canvas crates, tag 74, 2026-09-03); WebView2 "Window to Visual" hosting mode; HEVC store id rotation.
- Wrote docs/inventory/platform-windows.md (322 lines) and docs/inventory/platform-macos.md (305 lines): sections (a)-(g) per brief, verdict blocks + [VERIFY] ledgers; no ADR decisions made; E2-class classifications recorded not verdicted.
- No git commit (AGENTS §3.8). tmp/ scratch only (/tmp/c5b, outside repo).

Stage Summary:
- Both platform records complete: ADR-003 evidence — Candidate A feasible on both, structurally cleaner on macOS (sibling NSView + private drawsBackground KVC, always enabled in Tauri ≥2.12.1) vs Windows conditions (alpha 0/255-only DefaultBackgroundColor + no-WS_CLIPCHILDREN parent; wry lacks composition hosting); Candidate B available now in evergreen WebView2 (WebCodecs since Chromium 94, not on WebView2's disabled list), on macOS gated by support floor (video-only 16.4+/macOS 13.3+, full 26/macOS 26). ADR-001 — MF path pure-Rust-ready (windows-rs, MFTEnumEx pattern; no MFCreateVideoDecoder), HEVC/AV1/VP9 store-extension-dependent with runtime detection required; VideoToolbox path pure-Rust-ready (objc2 0.6.5 family), no install friction, D3D11-vs-DX12 interop and CVPixelBuffer→wgpu zero-copy are the top [VERIFY] spikes. 15 open [VERIFY] items across the two files.

---
Task ID: 0.C4-R (completed by Lead after agent timeout #2)
Agent: general-purpose (wrote ADR-002-evidence.md) + Lead (salvage: preaudit-foundation.md)
Task: Complete 0.C4: ADR-002 evidence pack + foundation dependency pre-audit.

Work Log:
- Agent delivered docs/adr/ADR-002-evidence.md (196 lines: hard requirements, 4-candidate table — wasmi 2.0.0 / wasmtime 49.0.3 / wasmer 7.5.0 / fizzy 0.8.0, wasmtime C-inventory, wasmer+fizzy rejection evidence, decision framing, sources) before timing out (2nd timeout on this task line).
- Lead completed preaudit-foundation.md via direct crates.io API fetches (18 crates; cpal 0.18.2 dep tree verified: all-native deps are E2 OS-audio bindings, no third-party media C libs).
- Verdicts: serde family/schemars/thiserror/tracing/raw-window-handle/wasm-bindgen = pass; cpal/wgpu/winit/windows(-sys)/objc2/core-foundation = conditional (E2 envelope); tauri/wry = exception-ADR-004. toml license [VERIFY-P1].

Stage Summary:
- 0.C4 COMPLETE (5/5 deliverables across 0.C4-I + 0.C4-R + Lead salvage). ADR-002 decision input ready for 0.D3.

---
Task ID: 0.C5 (wave, complete)
Agent: 0.C5-a + 0.C5-b (split retry; original 0.C5-I timed out with no output)
Task: Platform & hardware capability records for ADR-001/003/005.

Work Log:
- 0.C5-a: docs/inventory/platform-linux.md (298 lines, 23 [VERIFY]): Candidate B safer on Linux (wry 0.57 = WebKitGTK 4.1/GTK3; child webviews X11-only -> no Wayland candidate-A path; NVIDIA transparent-window crashes; WebCodecs proven compiled-in to Debian 2.52.6). VA-API technically ready; OS-service-vs-C-library classification = Lead call.
- 0.C5-b: docs/inventory/platform-windows.md + platform-macos.md: Windows candidate-A feasible-with-conditions (alpha-only bg, WS_CLIPCHILDREN), B ready (WebCodecs since C94); macOS candidate-A structurally cleaner (sibling NSView; drawsBackground fixed in Tauri >=2.12.1), B gated by macOS 13.3+/26 floors. HW decode: windows-rs MFTEnumEx + objc2-video-toolbox 0.3.2 pure-Rust-ready.
- platform-synthesis.md NOT yet written (deferred to next batch with 0.C1-C3 inputs).

Stage Summary:
- Platform evidence complete for ADR-001/003/005. Synthesis pending.

---
Task ID: 0.B2
Agent: Sub-agent (general-purpose), Wave 0.B
Task: Module/plugin census on all 3 platforms (doc 02 §3.2, doc 13 §3 card); seven-category G3 counts; platform diffs annotated.

Work Log:
- Read worklog (0.A2–0.C5), AGENTS.md §3–§4, doc 02 §3.2 only, doc 13 §3 card, doc 06 §1 (16 kinds), doc 04 phase context.
- Parsed the existing real dumps 05/06 (0.A2) with a new assertion-checked parser (docs/inventory/raw/modules/parse-vlc-list.py): 502 registrations = 502 --list lines cross-checked 1:1; 356 unique names; 56 capabilities + 4 no-capability base entries (export/bridge/playlist/core -> "(none)"); every module carries dump_line for traceability.
- Wrote docs/inventory/modules-linux.json (validated): by_capability (all 56 groups, count + name/shortcuts/description/score/dump_line per module), counting rule, seven-category rollup with sum_check=502, stats (multi-registration + shortcut leaders + hw modules).
- Wrote docs/inventory/modules-windows.json + modules-macos.json: status blocked-on-hardware, ZERO fabricated counts; expected platform-specific modules each cited to VideoLAN wiki module pages (dshow, mmdevice, waveout, directx_aout, direct3d, glwin32, wingdi, auhal, avcapture, qtsound, screen) and official 3.0.0 release notes (Direct3D11 vout, WASAPI pass-through, VideoToolbox hw decode, SMB, MIDI decoder), all fetched live 2026-10-19; wiki smb page confirmed dead (page id 0).
- Wrote docs/inventory/modules-summary.md: G3 statement (Linux satisfied; Win/mac open exactly per pin record), counting rule, full capability->our-kind table, 16-kind mapping with no-v1-analog flags, platform-diff annotations, 0.A2 scope limitation (10 headless-excluded Debian plugin packages), 9-item [VERIFY] ledger, highlights.
- Sanity: all quoted numbers re-verified programmatically against the JSON (rollup sums, avcodec=5/lua=11/playlist=15 registrations, packetizer membership, demux scores, top scores 10001 ffmpeghw / 10000 dav1d).
- No VLC source consulted; no git commit (AGENTS §3.8); scratch in tmp/census (gitignored tmp/).

Stage Summary:
- G3 Linux COMPLETE: 502 modules -> demuxers 57, decoders 35, access 57, filters 110, subtitle 32, playlist 21, services-discovery 26, other 164 (sum 502; rules documented). Win/mac skeletons expected-only, cited, blocked-on-hardware. Key flags for Lead: packetizer (25) has no v1 kind (engine-design item for doc 03/06); sout family (46) out of scope by design; avcodec ffmpeghw score 10001 + dav1d 10000 = oracle prefers hw decode (ADR-001 input). Open: V1–V9 [VERIFY] ledger, all unblocked by hardware runs per hardware-template.md §3–§5.

---
Task ID: 0.B3
Agent: Sub-agent (general-purpose), Wave 0.B
Task: Menus, dialogs & hotkeys inventory (doc 02 §3.4–3.5) by text transcription only; hotkeys.toml seed (keymaps/default.toml per D-003); crosswalk to design package; design-vs-VLC accelerator conflict flags. AC: G6.

Work Log:
- Read worklog (0.A1–0.C5), AGENTS.md §3–§4, doc 02 §3.4–3.5 only, doc 13 §3 card, doc 05 §2 + §8, doc 08 §1, ROADMAP D-003, design/README.md.
- Oracle observation (Linux pin, tmp/b3 scratch): `--longhelp --advanced` → 222 key options = 111 actions (names + help descriptions); NEW clean-room method beyond the card's minimum: `--reset-config --config <fresh>` dump → ALL 111 `key-*` DEFAULT VALUES binary-observed (e.g. Space, f, Esc, m, s, n/p, Shift+arrows 3 s, Alt 10 s, Ctrl 60 s, Ctrl+Alt 300 s, k/j audio delay, h/g sub delay, v/b track cycles, Shift+S snapshot, Shift+R record, a/c/d/z video, Ctrl+Q quit, F1..F10/Ctrl+F1..F10 bookmarks, PageUp/Down viewpoint); all 111 `global-key-*` default EMPTY; 5 actions unbound by default (subpos ±, viewpoint-roll ±, boss). Jump sizes 3/10/60/300 s + wheel modes recorded; filter defaults for EQ/Compressor/Spatializer/StereoWidener/adjust/sharpen/gradfun cross-checked.
- Public docs (curl, URLs + oldids + access date cited in-file): QtHotkeys wiki table (Qt accelerators Ctrl+B..Y, Alt+M/L/A/T/S/I/H, hotkey semantics); User Guide pages Open_Media/Playback/Audio/Video/Subtitles/Playlist/Interface/Hotkeys (menu paths + dialog controls; 1.x/2.x-era caveat flagged); Mac OS X page (menu structure + macOS key variants, stale-flagged); How-to global hotkeys.
- Design package text mining (doc 08 §1.1 method): Keyboard Shortcuts.svg (9 rows: F/esc/T/I + Space/S/N/P/X); Onboarding-2.svg (12 shortcuts incl. DESIGN keys: ←/→ seek, </> next/prev, ,/. frame step, ↑/↓ volume, Q queue, L file browser, SHIFT+S/R); right-click menu.svg (14 items incl. Hide UI Ctrl+\, Record R); menu.svg (12 items, accelerators); Settings - controls.svg (skip amounts 3/10/60/300 = VLC defaults; Edit Keyboard Shortcuts); customize panels; A/V tool panels (values match VLC defaults except StereoWidener feedback 0.2 vs 0.3 → D-3).
- Wrote docs/inventory/ui-inventory.md (457 lines): §1 Qt menu bar 8 menus / 58 leaf items with per-row provenance (15 partial-[VERIFY]); §2 23 dialogs with transcribed controls incl. binary-corroborated filter defaults; §3 context menus (playlist doc-sourced; playback-surface [VERIFY] + design replacement; tray partial) + modes (Skins2 unsupported-by-design); §4 macOS; §5 full 111-action hotkey table + method + unbound list + per-platform status + doc-vs-binary discrepancies (B-1 extrashort 5 s wiki vs 3 s binary; B-2 scale help-string swap); §6 crosswalk VLC→design→command→disposition (~45 rows); §7 conflicts C-01..C-09 + design-internal D-1..D-3 with D-003 resolutions; §8 G6 statement + V-01..V-10 ledger; §9 open questions Q1–Q8.
- Wrote keymaps/default.toml (141 bindings + 4 jump-size scalars + 9 macOS overrides; TOML-validated via tomllib; every entry has key/label-msgid/source/command-or-ui_action/mode + when/args/recipe/note). FORMAT FLAG: card+D-003 format (keymaps/*.toml, section-grouped binding-id tables) vs doc 05 §8.1 ([[binding]], config/keymaps/ path) — Q1 for Lead.
- Conflicts resolved per D-003 (design wins): arrows=seek (disc-nav gating Q4), ↑/↓ volume vs Ctrl+arrows, L file-browser vs VLC loop (repeat unbound), Ctrl+\ hide-UI vs Ctrl+H minimal-view (fold Q), ,/. frame keys vs VLC e (frame-prev NEW), Shift+F1 About (VLC none), both N/P and </> kept (design-internal D-2), Record Shift+R vs R (D-1, Onboarding+VLC agree), Ctrl+C capture kept (both sources agree, copy-convention note).
- No screenshots; no VLC source; no git commit (AGENTS §3.8). Scratch tmp/b3 (gitignored tmp/).

Stage Summary:
- G6 COMPLETE for text-transcription scope: 8 menus/58 items, 23 dialogs, context menus+modes, 111/111 hotkey actions with Linux binary-observed defaults, 150-entry keymap seed with per-entry provenance, 12 conflicts/inconsistencies flagged+resolved. Windows/macOS exactness and Qt-menu labels remain V-01..V-06 runbook [VERIFY]s (card is text-only by design). Escalations: Q1 keymap format/path reconciliation (doc 05 §8.1 vs D-003), Q2 shuffle-toggle arg shape, Q4 in-disc-menu condition token, Q6 title/program/disc-menu commands missing from v1 catalog, Q8 macOS key-collision gating.

---
Task ID: 0.B1
Agent: Sub-agent (general-purpose), Wave 0.B
Task: Options & preferences inventory (doc 02 §3.3): help dumps -> normalized options-{os}.json; three-way reconciliation; mapping to settings-schema seed categories. AC: G4.

Work Log:
- Read worklog (0.A–0.C, 0.B2/0.B3), AGENTS.md §3–§4, doc 02 §3.3 only, doc 13 §3 card, doc 07 §5.4 (13 seed areas) for the mapping table.
- Re-verified the extracted binary (VLC 3.0.23 Vetinari, 3.0.23-2-0-g79128878dd) via --version; recorded in dump 12's preamble. Card's suggested filename 08-options-longhelp.txt collided with 0.A2's 08-vlc-dummy-smoke.txt (numbers 01–11 taken) -> used 12–16; adaptation recorded in options-summary.md header.
- Flag probe matrix (dump 15): --help works (407 lines: usage + core non-advanced only); --longhelp works (1516); --advanced --longhelp works (3352); --full-help/-H works (5495, exhaustive set + expanded long descriptions; byte-identical to --advanced --full-help, diff = 0 lines); --help-verbose is NOT a dump flag on 3.0.23 (no stdout; without a stream it runs until killed, timeout exit 124; with vlc://quit exits 0 silently). Footers verbatim: --longhelp hides 67 advanced-only modules; 13/14 still hide 4. Wiki (Documentation:Command_line/, live 2026-10-19) says "--longhelp --advanced = all options" + "--help-verbose for more detailed help" -> option-set claim consistent with observation; --help-verbose claim is not (drift recorded).
- Captured dumps 12/13/14 (--longhelp / --advanced --longhelp / --full-help, each with -I dummy, stdout+stderr merged, stderr empty, timeout-bounded, sha256 in manifest) and dump 16 = preferences-file walk (doc 02 §3.3 extraction 2): fresh HOME + --reset-config --play-and-exit vlc://quit -> vlcrc written (191 sections, 1439 name=value lines, all commented = defaults); plain run writes NO file (recorded); byte-identical across two runs (cmp); on-disk UTF-8 BOM noted+stripped in the committed copy.
- Wrote regenerable parser docs/inventory/raw/options/parse-options.py (grammar + 5 enforced checks in raw/options/manifest.json) -> docs/inventory/options-linux.json (schema lumen-phase0/options-inventory/1.0): 1476 option records from dump 14 (canonical), each with option_name/short_form/negation_form/type/choices/min/max/help_text/long_description/default/module/module_name/category/subsection/visibility/is_advanced/in_vlcrc/vlcrc_default; totals: 615 longhelp-visible (41.7%) vs 861 advanced-only records (58.3%); 442 core-program options (9 categories: Audio 18, Video 44, Subpictures 67, Decoders 2, Input 4, Stream output 16, Miscellaneous 6, Playlist 35, Hot keys 250) + 1034 in 187 module sections; types string 519 / integer 420 / bool 267 / enum 162 / float 108; 15 short forms; 235 ranged; 162 enums; scope_notes record the 10 headless-excluded Debian plugin packages (qt family therefore absent).
- Parse coverage: 615/1476/1476 records vs 615/1476/1476 raw option lines (100%); leftovers 3/2/1 = dump footer notes only (quoted verbatim in JSON); grammar handled hotkey "+" pairs (key-jump+short), camelCase names (audiobargraph_v-barWidth), empty col-33 help slots (chromecast), 3-space subsections incl. one ending in ")" (v4l2), flush-left description continuations, and the "[cdda:][device][@[track]]" syntax line.
- Three-way reconciliation (extraction 3, Qt prefs tree, blocked headless -> hardware runbook): vlcrc-not-in-fullhelp = 0; fullhelp-not-in-vlcrc = 37 annotated (16 CLI/meta-only flags, 13 pointer/callback/handle options, 8 [VERIFY] remainder); 6 flat-namespace name collisions (scaletempo-* x2 modules, rc-host, glconv, wl-display) — the SAME 6 names duplicated in the vlcrc walk (cross-extraction agreement); default agreement 252/252 (0 disagreements after enabled/disabled->1/0); 1187 vlcrc-only defaults (help marks mostly bools). No unexplained diffs between the two runnable extractions.
- Wrote options-windows.json + options-macos.json: same schema, status blocked-on-hardware, options: [] (NOT fabricated); expected_differences cited to live public docs (Preferences wiki: vlcrc paths per OS; Command_line wiki: Windows --option="value" quoting, dvd:// device syntax; module pages dshow/mmdevice/direct3d/auhal/qtcapture verified HTTP 200 2026-10-19) + [VERIFY] platform-invariant-core hypothesis.
- Wrote docs/inventory/options-summary.md: headline numbers, flag matrix, per-category table (with Subpictures-scope warning: the dump's Subpictures header runs until Input, so Track settings/Playback control/Network settings/etc. subsections hold Qt-"Input/Codecs" options — subsection field is the finer key), reconciliation D1–D7, full mapping table VLC->our 13 seed areas (global/interface/privacy/playback/video/audio/subtitles/controls/library/network/performance/recording/extensions) with engine-internal and not-adopted rows, G4 statement, [VERIFY] V1–V7 ledger.
- Key findings for design: 58% of options are advanced-folded (validates doc 07 §5.2 `advanced` field); defaults are authoritative in the config-file walk, not help text (V7: schema generator should source defaults vlcrc-style); VLC's flat CLI namespace has 6 real collisions (our D3 validator rule exists precisely to prevent this); options are module-scoped -> our engine-internal vs user-setting split maps cleanly (only ~442 core options are candidates, of which a minority are user settings).
- No VLC source consulted; no binaries committed (dumps are text; tmp/ scratch gitignored); no git commit (AGENTS §3.8).

Stage Summary:
- G4 COMPLETE for Linux: 1476 normalized records, 100% parse coverage, zero unexplained cross-extraction diffs, annotated D1–D7; Windows/macOS documented skeletons (blocked-on-hardware, citable expected differences only). Open: V1 (9 non-persisted options), V2 (4 hidden advanced-only modules), V3 (qt/skins2 families need desktop-stack run), V4–V6 hardware dumps incl. Qt prefs tree + --help-verbose retest, V7 default-source guidance for doc 07 §5.3 G2. Escalation: none blocking; V7 recommends Lead note that schema default seeds come from config-walk dumps, not help text.

---
Task ID: 0.C5-s
Agent: Sub-agent (general-purpose), Phase 0
Task: Platform synthesis — consolidate the three platform records (0.C5-a/b) into decision inputs for ADR-001/003/005; consolidated [VERIFY] ledger.

Work Log:
- Read worklog tail (0.B1-B3, 0.C5 stage notes), AGENTS.md §3-§4 (no commit; append worklog; [VERIFY] discipline), doc 13 §5 (2.D3/2.E2-V prototype cards), ROADMAP D-008/§6, MP §2/§9.
- Read all three platform records fully (platform-linux.md 297 ln, platform-windows.md 321 ln, platform-macos.md 305 ln). Consolidation only — zero new research; every claim cites [L/W/M §x].
- Wrote docs/inventory/platform-synthesis.md (212 ln): (1) five comparison tables — Candidate A per OS (conditions+confidence+citations), Candidate B per OS incl. WebCodecs floors, hw-decode readiness (MF/VT/VA-API × coverage × Rust access × classification × zero-copy), audio options per OS with policy classification, wgpu/GPU per OS; (2) eight honest bottom lines (A: Win feasible-with-conditions / macOS structurally cleaner / Linux no Wayland path → B safer; VA-API classification = Lead/user call; cpal conditional-E2 acceptable; HEVC-store edge case; macOS point-release fragility; oracle hw-preference corroboration); (3) ADR open-question matrix — 8+8+7 rows for ADR-001/003/005, every row answered-with-evidence (linked) or concrete [VERIFY] with owner, none unanswered-in-principle; (4) decision inputs only — ADR-003 references doc 13 §5 cards 2.D3/2.E2-V (no re-planning; evidence-weighted per-OS emphasis + sequencing note), ADR-001 hw-first-with-truthful-reporting framing (probes, fallbacks, per-OS caveats, spike-gated zero-copy, API-name corrections), ADR-005 ranked options per OS; (5) deduped [VERIFY] ledger SV-1..42 with vehicles/owners; (6) gaps listed not filled (Fedora/Arch, GNOME GitLab anti-bot, Apple per-SoC matrix, AMD citation, WKWebView exception list, doc-only Win/mac records).
- Fixed all SV-xx cross-references after renumbering; verified line budget and no fabricated facts (all numbers traced to records).
- No VLC source; no git commit (AGENTS §3.8).

Stage Summary:
- Platform synthesis COMPLETE: docs/inventory/platform-synthesis.md. ADR-001/003/005 now have per-OS comparison tables, bottom lines, and a fully-resolved open-question matrix; 42-item deduped [VERIFY] ledger with owners (Lead/user policy calls: SV-28 VA-API classification, SV-22 exclusive-mode scope, SV-26 pro-audio scope; top spikes: SV-7/8/9 zero-copy interops; cheapest early win: SV-1 Linux WebCodecs probe).

---
Task ID: 0.C2-I
Agent: Sub-agent (general-purpose), Wave 0.C
Task: Audio + subtitle codec feasibility sections (doc 04 §3/§5; §8.6 items 1, 5, 6).

Work Log:
- Read worklog (through 0.C4/0.C5/0.B1-0.B3), AGENTS.md §3-§4, doc 04 §1/§3/§5/§7/§8.6 only, doc 02 §5 outline, doc 03 §4.11-4.12, doc 06 §7, doc 14 R-06/R-09/R-10, census JSON (audio decoder 18 reg / subtitle 32).
- Re-verified live (2026-10-19, crates.io API + raw.githubusercontent): symphonia family 0.6.1 (2026-08-13, MPL-2.0); symphonia-codec-opus NOT on crates.io AND a 1-byte-lib.rs placeholder on main (README: "placeholder crate"); symphonia-codec-wavpack upstream = same empty placeholder, but published symphonia-codec-wavpack 0.1.1 is third-party (pierreaubert/symphonia-add-ons) GPL-3.0-or-later; AAC crate = LC+AdtsReader only, no sbr/ps, no features; C adapters symphonia-adapter-fdk-aac/libopus (0.2.0/0.3.0) exist; truehd 0.7.2/truehdd 0.6.2 (2026-09-15, Apache-2.0, verify cmd + concealment counters, MSRV 1.95, "experimental" banner); oxisynth 0.1.0 LGPL-2.1 (sf3→lewton); rustysynth 1.3.7 MIT, std-only, SMF INCLUDED (closes doc 04's SMF S estimate); NOT_FOUND re-checks ac3/wma/speex/musepack/atrac/cc708/eia608; cea608 0.0.1 = builder; subtitle crates (ass-parser 0.2.3, ssa-parser, subparse, webvtt-parser 1.2.x, vobsub, srt stale).
- Wrote docs/feasibility/sections/audio-codecs.md (446 lines): 20-row verdict table + census reconciliation (g711/aes3/uleadvdaudio flagged for doc 04); Symphonia deep-dive; truehdd Phase-6 hardening list; Opus options (i)-(iv) with [PROPOSAL] framing for 0.D3 (v1 blocked + typed error; Phase 5/6 L-XL self-build gate, upstream-first option; WebCodecs=web-only via ADR; "wait" collapsed into self-build); R-10 honesty paragraphs (WavPack GPL wrinkle new); evidence appendix 37 items; 11 [VERIFY].
- Wrote docs/feasibility/sections/subtitles.md (435 lines): 14-format verdict table + full 32-module census reconciliation (ARIB/Teletext/scte/textst/SVCD dispositions); ASS gap quantification with tag-coverage matrix (render/degrade/drop) + unsupported-tag counters per doc 06 §7.8 + libass-output-as-oracle [PROPOSAL]; PGS/DVB/VobSub unified OverlayLayer refinement of doc 03 §4.11 (YCbCr palette [VERIFY], epoch-flush-on-seek tests, 8.3MB/layer budget); CC 608=M / 708=M-L complexity notes + extraction points; oxisynth LGPL classification (flag to doc 11) with [PROPOSAL] v1 MIDI=later via rustysynth (user-supplied .sf2), oxisynth stays out-of-tree; evidence appendix 20 items; 12 [VERIFY].
- No VLC/libass/FluidSynth source consulted; scratch in tmp/c2i (gitignored tmp/); no git commit (AGENTS §3.8).

Stage Summary:
- 0.C2-I COMPLETE. Headline: Symphonia covers 8 codec families v1 (PCM/MP3/MP2/FLAC/ALAC/Vorbis/AAC-LC/ADPCM); HE-AAC has ZERO upstream code (features rows aspirational); Opus = empty placeholder (R-06 "wait" option non-viable as stated; re-scoped for 0.D3); truehdd best-shape gap (testing project M, not codec-writing); WavPack new GPL landmine; ASS subset matrix proposed with measurable counters; rustysynth closes MIDI's SMF need. 23 [VERIFY] total across both files, all owned and unblocked. Escalations: 0.D3 Opus framing; doc 04 table updates (3 audio + 5 subtitle census-only formats); WavPack GPL + oxisynth LGPL -> doc 11.

---
Task ID: 0.C3-I
Agent: general-purpose (sub-agent, Phase 0 wave 0.C)
Task: Subsystem feasibility section — demuxers, network/streaming, outputs, filters, playlists/SD, sout, devices; closes doc 04 §8.6 items 3 (frag-MP4 corpus), 4 (mpeg-ps scope), 9 (Kate/FFV1/DVD ADRs).

Work Log:
- Read worklog tail, AGENTS.md §3–§4; doc 04 §2 + §8.6 items 3/4/9 only; doc 03 §2 + §7 only; doc 01 §2.7/§4.2/§7; doc 13 phase cards 2–7; doc 14 register; modules-linux.json census; platform-{linux,windows,macos}.md; preaudit-foundation.md; ui-inventory §2.11; MP §10/§13/§14; design/figma panel SVGs (element-id extraction, text-as-paths).
- Re-verified via crates.io API (UA lumen-preaudit-0.C3-I, 2026-10-19, raw in tmp/c3i/): reqwest 0.13.5 (default-tls now = rustls), hyper 1.12.0, ureq 3.4.2, rustls 0.23.46 (default aws_lc_rs), ring 0.17.14 (non-optional cc build-dep → C/asm, honest), native-tls 0.2.18 (openssl on Linux), rustls-rustcrypto (alpha 2024); m3u8-rs/quick-m3u8/hls/hls-runtime/dash-mpd/dash-mpd-core; retina 0.4.21 + rtsp 0.1.0 + sdp; m3u/pls/xspf/cue(libcue C); rupnp/mdns-sd/simple-mdns; biquad/fundsp; mp4 0.14.0, mpeg2ts, flacenc, vobsub; kate/dvdnav/dvdread/bluray NOT_FOUND, ffv1 0.0.0 RESERVED, dvdcss = C bindings, decklink = NDI bindings.
- Tarball inspections (audit-method M4–M6): mpeg-ps 0.6.0 (published 2026-10-17! no_std, pure; scope = pack/system header/PSM/PES; PSD + MPEG-1 rejected; no VOB substream split), mp4parse 0.17.0 (read_mp4 SKIPS moof — oracle role for fMP4 limited to moov), mpeg-pes 0.6.0, symphonia-format-isomp4 0.6.1 (MoofSegment/trafs/mfhd/SidxAtom present — engine fMP4 path confirmed).
- Wrote docs/feasibility/sections/subsystems.md (767 lines): §1 demuxers (family rollup; 11-class frag-MP4 corpus plan with testkit fMP4 writer + pass criteria → 5.C1/6.C; mpeg-ps scope table incl. private-stream substreams S self-build + nav-pack skip + MPEG-1 open; D-001 confirmation: doc 03 §7 needs no structural change, 3 recorded notes); §2 network (TLS honest finding: NO production pure-Rust HTTPS — new [PROPOSAL] ADR-011 input; ureq-first recommendation; HLS/DASH estimates; RTSP blocked-v1 verdict citing retina/rtsp crates + 3 blockers; brief's "R-31-class" was a mis-ID, register row proposed); §3 outputs (ADR-005 matrix, cpal-first + escape hatches [PROPOSAL]; wgpu path + D3D11↔12 interop); §4 filters (110-census decomposition; 4 audio + 3 video panel mappings w/ VLC binary defaults + D-3; shader-uniform proposal; doc 13 FILTER CARD GAP escalated [PROPOSAL]); §5 playlists (self-build S×4 [PROPOSAL], cue crate = C) + SD minimal-v1 [PROPOSAL]; §6 sout (v1 = 3/46 honesty, ~7%); §7 devices (capture blocked hardware-gated; 3 μADR briefs: Kate drop-v1, FFV1 later+ADR-010, DVD nav blocked no-E2-path + ADR-008 numbering mismatch flagged vs task brief's "ADR-009"); §8 synthesis (availability×risk×phase table, 3 ranked risks, ADR inputs, 0.D3 pull-list); §9 evidence appendix [E1–E22] + 17-item [VERIFY] ledger.
- No VLC source; no git commit; scratch tmp/c3i (gitignored).

Stage Summary:
- 0.C3-I COMPLETE. Headline: all subsystems have viable v1 paths; two policy-grade holes found (TLS purity — new ADR-011 input; Linux audio purity — only pulseaudio-protocol is pure); honest ceilings: sout 3/46, network video = hw-only. Deliverable: docs/feasibility/sections/subsystems.md. Escalations for Lead: ADR-011 (TLS), ADR-005 recommendation, ADR-008 inputs (3 μADRs), filter-implementation card gap in doc 13, brief's ADR-009/R-31 numbering mismatches. Open: 17 [VERIFY] items in §9.1.

---
Task ID: 0.C1-I
Agent: general-purpose (sub-agent, Wave 0.C)
Task: Video codec feasibility section — pure-Rust availability verdicts, H.264/HEVC deep-dive, rav1d asm-off closure (doc 04 §8.6 item 2), WebCodecs ADR scope (item 8). ADR-001 input for 0.D3.

Work Log:
- Read worklog (0.A–0.C5), AGENTS.md §3–§4, doc 04 §1/§4/§7/§8 only, doc 09 §8/§10/§12, platform-{windows,macos,linux}.md hw-decode sections, modules-linux.json (video/hw decoder groups: avcodec ffmpeghw 10001, dav1d 10000), ROADMAP §6, doc 13 0.C1-I card.
- Re-verified ~50 crates via crates.io API (2026-10-19): rav1d 1.1.0 features/deps; jpeg-decoder 0.3.2; zune-jpeg 0.5.16-rc2; openh264 0.9.9 (= C source build, forbidden); reserved-name checks (h264/theora/h265/ffv1 0.0.0; vp8/vp9/hevc/de265/libhevc-rs NOT_FOUND).
- MAJOR FINDING: doc 04's "no production pure-Rust H.264 decoder" is stale — a 2026 wave exists: rusty_h264-decoder 0.16.1 (BSD-2, forbid(unsafe), Baseline+B+most-High+CABAC-Main, bit-exact vs openh264 35/35 self-claimed), rust_h264 0.4.0 (Baseline/Main/High+MBAFF, 174 ffmpeg tests), rusty_h265 0.6.2 (Main/Main-10, JCT-VC 147-stream gate), rust_h265 0.1.0 (~5x slower than ffmpeg), rusty_vp9 0.1.3 (315/315 libvpx vectors claim), oxideav mpeg12video/mpeg4video, FFV1/ProRes/DNxHD candidates. Tarball-scanned 9 crates: all 0 C/asm, 0 build.rs. All young/self-validated — framed as conformance-gated candidates, not adoptions.
- rav1d asm-off closed by inspection: build.rs main() entirely #[cfg(feature="asm")]; all asm_arm64_* imply asm; default-features=false alone fails (bitdepth compile_error) → correct config default-features=false + bitdepth_8+bitdepth_16; 91 asm files but 0 C files in tarball; cc/nasm-rs declared-but-inert build-deps (policy nuance recorded); nightly pin caveat (arm-only unstable features); rav1d-safe = AGPL, rejected. Phase 5 proof plan P5.1–P5.7 written (no-C-toolchain sandbox build, cargo-deny, dav1d-oracle corpus, asm-on/off benchmark, fuzz, anti-regression pin).
- WebCodecs scope: W3C registry (av01/avc1/avc3/hev1/hvc1/vp8/vp09; "not required to support any particular codec"), MDN codec list + subset warning, BCD VideoDecoder re-verified (Chrome/Edge 94+, Firefox 130 desktop-only, Safari 16.4+), ADR-008 decision surface (6 points) + ADR-001 relationship; flagged doc 01 vs ROADMAP ADR-008/009 numbering conflict (F-12).
- Wrote docs/feasibility/sections/video-codecs.md (595 lines): 18-codec verdict table (pure-Rust × MF/VT/VA-API/WebCodecs × v1 path × parity consequence), H.264/HEVC paths (a)–(d) with [PROPOSAL] framing (hw-first primary + conformance-gated software track; parser self-build always), rav1d §3, WebCodecs §4, per-codec §5, evidence appendix §6 (URLs+dates; F-1..F-12 follow-ups for 0.D3). 58 [RV] marks, 26 [VERIFY] carried/flagged, 9 [PROPOSAL].
- No VLC source; no git commit (AGENTS §3.8); scratch in tmp/c1/video (gitignored tmp/).

Stage Summary:
- 0.C1-I COMPLETE. Video-codec input ready for 0.D3/ADR-001. Headline: H.264/HEVC risk reframed from "missing" to "candidates exist, unproven"; rav1d asm-off statically proven, Phase 5 plan concrete; FFV1 [VERIFY-P0] closed (candidates named); 12 follow-ups (F-1..F-12) incl. ADR-008 scoping, VA-API E2 call, doc-04 fold-back, ADR numbering conflict.
---
Task ID: 0.B4-R
Agent: Sub-agent (general-purpose), Phase 0 (retry of 0.B4 — parsing-only)
Task: Formats inventory from preserved probe evidence (~60 pinned-VLC 3.0.23 sessions run by the timed-out prior 0.B4 agent). AC: G5.

Work Log:
- Read worklog tail, AGENTS.md §3–§4 (no commit; append worklog; [VERIFY] discipline), docs/inventory/modules-linux.json (census + shortcuts via 06-vlc-list-verbose.txt), probe-work/logs/ (61 .log files + status.txt: 57 exit rows, all 0 except rtp_ts=124), run_probes.sh, gen_py.py. Zero new oracle sessions; parsing only.
- Counted honestly: 61 logs = 60 VLC sessions (58 with module-selection evidence) + 1 python http.server log; 4 logs lack status rows (smoke_wav, http_server, rtp_ffmpeg, udp_ffmpeg — the last two are 0-byte failed sender-side captures).
- Extracted decisive lines per log via rg (`using (demux|access|access_demux|packetizer|audio|video|spu decoder|stream_filter|stream_directory) module`, playlist-reader/found-valid/detected-format/forced-subtitle/no-suitable/Codec-not-supported lines): built the full per-probe table. Key selections: mkv/mp4/ogg/ts/ps/avi/asf/wav/flacsys/aiff/au/es/h26x/mpgv/smf demuxes; avcodec(h264/hevc/vp9/mpeg2/asp/mjpeg/wmv2/wmav2/alac/adpcm), dav1d(av1), faad(aac), flac, mpg123(mp3), opus, vorbis, speex, theora, araw(pcm), spdif(ac3/eac3/dts = passthrough-only, decode NOT exercised [V3]); sidecar subs via demux "subtitle" + spu decoders subsdec/libass/webvtt/ttml.
- Wrote docs/baselines/raw/linux/17-format-probes.txt (1,670 lines, 62 sections): per-probe command (reconstructed where not in run_probes.sh, marked), fixture desc (ffmpeg command lines were NOT preserved — content re-derived 2026-10-19 via ffprobe on still-present tmp/probe/media + x265 .err remnants [V1]), exit code, verbatim decisive lines; cross-check section with negative-probe reconciliation (ES-event counts: sub_* 0 events, cue 0 events, midi 1).
- Wrote docs/inventory/formats.json (schema lumen-phase0/formats-inventory/1.0): 82 families — container 21 / audio 18 / video 12 / subtitle 12 / playlist 7 / protocol 12; 59 probed with probe_evidence (log files verified to exist), 23 census-only; per-family vlc_modules + shortcuts (from census dump) + usage_share P0×27/P1×23/P2×19/P3×13 with one-line rationales; usage-share sources cited: Wikimedia Commons Special:MediaStatistics (live fetch 2026-10-19, numbers quoted, free-format skew flagged), doc 04 §1.3 anchor; Bitmovin 403 bot-blocked + archive.org unreachable recorded as [V5].
- Wrote docs/inventory/formats.md: headline counts + P0 list, probe-method description incl. blind spots (spdif passthrough, decode-vs-selection honesty, excluded fluidsynth deb), per-kind result tables (incl. subtitle two-pass table with quirks: ps wins standalone-sub probe race; avcodec "refusing to decode non validated subtitle codec" before subsdec; MicroDVD fps inference), doc-04 §2–5 comparison table (family × doc04 pure-rust status × our phase target × oracle-chosen module), G5 statement, [VERIFY] ledger V1–V10, subsl_*/empty-probe/http-server notes.
- subsl_* = subtitle-SLAVE second pass (mp4_h264_aac.mp4 + --sub-file; "forced subtitle: … (forced: 1)", "loading spu-es slave") — NOT "subtitle language". Standalone sub opens are negatives (demux claimed, 0 ES, "nothing to play"): text subs are sidecar-only in this oracle.
- Deliverable checks: JSON valid + all evidence paths exist + every family has usage_share+rationale; md counts cross-checked against JSON; no existing files modified; scratch in tmp/b4r (gitignored); no VLC source consulted; no git commit (AGENTS §3.8).

Stage Summary:
- 0.B4-R COMPLETE. G5: all 82 families carry explicit probe_status (59 probed with log-file evidence, 23 census-only). Artifacts: docs/inventory/formats.json, docs/inventory/formats.md, docs/baselines/raw/linux/17-format-probes.txt. Notable evidence: dav1d chosen for AV1 (rav1d analog); ALAC/ADPCM/VP9 decode via avcodec; playlists are stream_filters; rtp is an access_demux auto-detecting MP2T; cue = negative probe (no module); midi = demux-yes/decode-no (excluded fluidsynth deb). Open: V1–V10 ledger (top: V3 AC3/DTS decode re-probe with real aout, V1 committed fixture generators for 5.A1, V5 usage-share surveys unreachable from sandbox, V10 cue scope decision to 0.D3).

---
Task ID: 0.D1-I
Agent: Sub-agent (general-purpose), Phase 0
Task: Security threat-model draft (doc 02 card 0.A11) + test-corpus plan (card 0.A12 / gate G12).

Work Log:
- Read worklog tail (0.A–0.C5), AGENTS.md §3–§4 (no commit; append worklog; [VERIFY]/[PROPOSAL] discipline), doc 11 §5+§7 only (as briefed) + §8.4 fixture allowlist for cross-reference, formats.json (82 families, structure + full family list), modules-summary.md, doc 10 §5 + doc 12 §3.4, doc 02 §2.4 + cards 0.A11/0.A12, pin record (vlc-3.0.23.md incl. DR-01..16), 17-format-probes.txt preamble, probe-work gen_py.py/run_probes.sh/status.txt/http_server.log, subsystems.md §1.2 (fMP4 11-class) + §5.1, doc 13 cards 2.A3/4.D1/5.D1/5.I1/6.A1/6.B1.
- Computed sha256 of the 8 committed probe-work fixtures + 2 generators (2026-10-19) for the corpus seed ledger; verified git-tracked status; confirmed au/aiff/adpcm/midi probe fixtures were ffmpeg-generated (cmds unpreserved) — corrected "proven" marks accordingly.
- Wrote docs/inventory/security-phase0.md (324 ln): assets table; TB1–TB8 instantiated with Phase-0 evidence (TLS negative finding → ADR-011; TB8 landmines found by tarball inspection); NEW TB9–TB14 (tmp/ scratch, oracle invocation env, CI runners, user-editable config, plugin manifests [Phase 7], evidence-integrity); personas A1–A7 concretized + NEW A8 (CI-poisoned fixture/PR), A9 (hostile oracle-artifact channel: DR-01/02/12/13), A10 (malicious config), A11 (scratch-tree races/evidence tampering); 14-row per-boundary STRIDE×mitigation×where-enforced×phase table + expanded STRIDE summary; ORACLE HYGIENE OH-1..12 (per-run tmp dirs, no-egress-except-loopback-tests, never-see-credentials, log scrubbing before commit, binaries never committed, read-only fixtures, version guard, timeouts, disposable env, hash-verify-before-use, verbatim regenerable evidence, repo-controlled-fixtures-only in CI) each with proven/[PROPOSAL] status + §5.1 observed facts incl. 3 recorded deviations (absolute paths in committed logs, shared scratch tree, unpreserved ffmpeg cmds); top-10 risks w/ early indicators; SEC-01..16 input table (SEC-07 material change: no pure-Rust HTTPS — ADR-011; SEC-11 fuzz population = 57+32+21; SEC-06/08 corpus vectors from probe-work path/URL shapes); 5 new oh_* checks; SV-P1..8 ledger + evidence map.
- Wrote docs/inventory/corpus-plan.md (360 ln): taxonomy A generated-valid / B generated-malformed (5 seeded ops: boundary-truncate via structure maps, seeded bit-flip, oversize-field, nested-depth, invalid-magic) / C multi-track+discontinuous+chapters+cover-art / D curated (allowlist per doc 11 §8.4; no VLC-derived, no unknown provenance) / E perf (pointer to doc 12 §3.4); determinism+seed spec (python bootstrap → card 2.A3 Rust port; ffmpeg only as committed-command encoder or cross-check [PROPOSAL]); FULL 82-family matrix (verified programmatically: all formats.json families present, 21+18+12+12+7+12) with classes/generator/first-phase (card-traced: MKV 5.D1, TS 5.I1, M3U 4.D1, HLS/DASH 6.B/6.C, http 6.A1, srt 3.A3–5.M1)/expectation fields; expectation TOML spec (extends doc 10 §5.4 with SC/CU/ST/EP/PL/EV/error-budget fields) + storage layout (curated ≡ doc 10 acquired/, naming flagged Q2); ledger row format + allowlist; <50 MB v1 budget [PROPOSAL] + per-PR fast subset / nightly full CI strategy; seed ledger seeded with 8 probe-work fixture hashes (all CC0, generated); Q1–Q5 + C-V1..8 ledger.
- No VLC source consulted; scratch = none needed beyond hash computation; no existing files modified; no git commit (AGENTS §3.8).

Stage Summary:
- 0.D1-I COMPLETE. Deliverables: docs/inventory/security-phase0.md + docs/inventory/corpus-plan.md. G12-shape satisfied: per-family fixture mapping (82/82), provenance/license columns, malformed-generator spec with seeds, expectation format. Headline threats: parser memory-safety across 57+32+21-analog surface, TLS implementation gap (ADR-011), path traversal via playlists/sidecars, evidence forgery (TB14). Open: SV-P1..8 (CI isolation, log-scrub policy, config ceilings), C-V1..8 (flacenc/rav1e purity, DASH-IF/Apple/MPEG vector licenses, PGS writer, CC carrier), Q1–Q5 Lead decisions (50 MB budget, curated-vs-acquired naming, ffmpeg policy, class-B committed-vs-generated, G12 sign-off posture).

---
Task ID: 0.D2
Agent: Sub-agent (general-purpose), Wave 0.D (Phase 0 synthesis)
Task: Freeze docs/feature-matrix.toml schema (doc 02 §4) + initial population from the completed inventories; population log docs/inventory/matrix-seed-notes.md.

Work Log:
- Read worklog tail (0.A–0.C waves + 0.B4-R), AGENTS.md §3–§4 (no commit; append worklog; [VERIFY] discipline), doc 02 §4 only (schema being frozen), doc 13 phases 2–8 + MP §16/§17 for phase targets, formats.json fully (82 families, usage_share P0×27/P1×23/P2×19/P3×13), modules-summary/options-summary/ui-inventory (headers + §5.2 hotkey table + §6 crosswalk), feasibility verdict tables (video/audio/subtitles/subsystems §1/§8.1 + §2.3 RTSP + §4 filters + §5 SD + §6 sout + §7 devices), platform-synthesis.
- Wrote docs/feature-matrix.toml (3,404 lines): [meta] block (schema_version = 1, FROZEN 2026-10-19, baseline "VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)", change-control note) + header comment blocks: status vocabulary (doc 02 §4.3's 7 values; doc 13's `not-selected` documented as a CARD status, not a matrix status), platform columns (windows/linux/macos/web; dedicated web column per MP §10 with rationale), evidence-link rules (verified REQUIRES evidence artifact; blocked/unsupported REQUIRES reason; partial REQUIRES notes; not_started→verified jumps rejected), validation rules R1–R10 (what tools/feature-audit will enforce), phase-target semantics (2–8; 8 parks later rows; blocked rows keep revisit phase).
- Populated 252 rows across 21 frozen categories: container 21, video-codec 16, audio-codec 19, subtitle-format 17, text-render 3, playlist-format 7, protocol-access 11, network-feature 7, playback-op 13, audio-feature 11, video-feature 10, playlist-feature 9, config-feature 20, plugin-feature 4, platform-feature 18, services-discovery 6, ui-screen 11, ui-dialog 15, menu-item 16, hotkey 17, extension 1. Priorities: P0 61 (24.2%) / P1 92 / P2 60 / P3 39. Category taxonomy = 0.D2 card's, reconciled against doc 02 §4.2's enum in seed-notes §2 (subtitle-codec→subtitle-format; ui→screen/dialog/menu; device+outputs→platform-feature; protocol→protocol-access+network-feature; retired prefixes documented).
- Statuses honest: NOTHING verified/implemented/in_progress (no CI); 211 rows not_started ×4; 33 rows blocked on ≥1 platform with cited reasons (headline: opus [empty placeholder crate, R-06], he-aac [zero upstream SBR/PS — detect+report never silent], h264/hevc SOFTWARE rows blocked pending ADR-001 conformance gate [hw-first rows separate, per feasibility's "matrix row says" column], rtsp/udp/rtp [6.G1-I registry: hw-only payload + async + camera-quirk tail], dts [spec availability], dvd/bluray/capture [C libs + legal/hardware], midi, wavpack [GPL], deinterlace, teletext, kate, smb/nfs/ftp/mms, SD trio); 8 unsupported-only rows (per-OS aouts, web-only rows, skins2, VLM, Lua→WASM).
- Traceability: every row carries inventory_ref; all options-linux.json#* anchors verified programmatically against the inventory (0 misses, incl. fixes: subsdec-encoding, sub-autodetect-fuzzy, key-chapter-next, volume-step, audio-replay-gain-mode); all formats.json#* anchors verified against family ids (0 misses). Hotkey groups partition the 111 binary-observed actions exactly (110 in 16 groups + audio-device-cycle on its feature row); global-key-* = 1 scope row.
- Phase targets from doc 13 wave program (D-001 first vertical: avi/mp4 DECISION-PENDING pair both ph2 + mjpeg + pcm ph2 + file/aout/vout/probe ph2; playback-ops ph3; UI/config/menus/hotkeys ph4; format breadth ph5; network/blocked-registry ph6; hw adapters/discs/recording/web ph7; later/post-v1 ph8). P0 ⇒ ph ≤ 6 holds with 0 violations (hw adapters split to P1 ph7 rows — open question Q1).
- Wrote docs/inventory/matrix-seed-notes.md: counts cross-tab, schema-freeze declaration + category reconciliation table, mapping decisions (1476 options → 20 config rows via 13 areas; 502 registrations → vlc_modules + 9 rollup rows; 58 menu items → 16 rows; 23 dialogs → 15 rows; 111 hotkeys → 17 groups), not-adopted list (sout family 3/46 honesty, transcode beyond WAV/remux, VLM, skins2, Lua, SD, capture, fingerprinting, addons store, wallpaper, novelty filters), blocked list with reasons, 7 open questions for the Lead, validation record.
- Validation: tomllib round-trip + generator assertions (id regex/uniqueness/prefix↔category, status keys/values, priority/phase enums, blocked⇒reason, P0≤6, anchors). Scratch generator in tmp/d2/ (gitignored). No VLC source; no git commit (AGENTS §3.8).

Stage Summary:
- 0.D2 COMPLETE. docs/feature-matrix.toml frozen (schema_version 1) and populated: 252 rows, 21 categories, P0 61/252 (24.2%), 33 blocked rows with cited reasons, 0 verified (honest — no CI). THE parity instrument now exists; tools/feature-audit (1.E) enforces R1–R10 from Phase 1. Open for Lead: 7 questions in matrix-seed-notes §6 (top: Q1 P0-vs-ph7 adapter split under R2, Q2 doc 02 category pointer edit, Q3 not-selected ratification, Q5 P0 rubric confirmation).

---
Task ID: 0.D3 + 0.D4-V
Agent: Lead
Task: Feasibility report assembly; ADR-001/002/003 decisions; ADR-011 brief; Phase 0 gate review.

Work Log:
- Wrote docs/adr/ADR-001-hardware-decode.md (ACCEPTED: hw-first H.264/HEVC via E2 OS decoders with truthful active-decoder reporting; conformance-gated software-adoption track for the 2026 pure-Rust wave; parsers self-built; VA-API classified E2-conditional; WebCodecs = browser-API analog).
- Wrote docs/adr/ADR-002-wasm-runtime-wasmi.md (ACCEPTED: wasmi; no-WASI; lumen:v1.* capability imports; 32-64 MiB/fuel/IO limits; crash isolation; nested-wasm spike for web).
- Wrote docs/adr/ADR-003-video-in-webview.md (ACCEPTED: per-OS landing — A on macOS, A-or-B on Windows, B on Linux; engine strategy-agnostic; prototype gates 2.D3/2.E2-V; truthful path reporting).
- Wrote docs/adr/ADR-011-tls-https.md (PROPOSED: no pure-Rust HTTPS today; 4 options; decision due <= Phase 5 gate).
- Wrote docs/feasibility-report.md (assembly: baseline measured, codec/subsystem/platform verdicts, honest gap register top-7, parity ceiling statement, Phase 1 readiness).
- Wrote docs/gates/phase0-exit-report.md (0.D4-V): 13/13 exit criteria PASS with evidence links; Lead ratifications R-1..R-5; new decisions D-011..D-014; top-5 carried risks; Phase 1 launch list.
- Updated docs/ROADMAP.md: status = Phase 0 COMPLETE; D-011..D-014; ADR index (001/002/003 accepted, 011 proposed); reconciliation log +4 rows; open-decisions trimmed to patents only.

Stage Summary:
- PHASE 0 GATE: PASS. Tag phase-0-gate. 252-row feature matrix frozen; feasibility report + 4 ADRs decided; all inventories evidence-linked.
- Phase 0 execution stats: 20 subagent launches, 15 clean completions, 5 context-deadline timeouts all salvaged (3 complete-files, 1 Lead-completed, 1 split-retry). 9 commits.
- Next: Phase 1 — Foundation (doc 13 §4, waves 1.A-1.F, 20 cards).

---
Task ID: 1.A1
Agent: Sub-agent (general-purpose), Wave 1.A (Phase 1: Foundation)
Task: Workspace skeleton — root `Cargo.toml`, `rust-toolchain.toml`, `[workspace.lints]` per doc 10 §3, crate template with `lints.workspace = true`, `.editorconfig`. AC: check/test green on the template; lints proven inherited by a canary violation test.

Work Log:
- Read AGENTS.md §3–§4 (no commit; append-only worklog; [VERIFY]/[PROPOSAL]), worklog tail (0.D3+0.D4-V: Phase-0 gate PASS), doc 13 §4 Wave-1.A card + §1.4 C1–C8, doc 10 §2 (gate table) + §3.1–§3.3 (exact lint block / inheritance / no-suppression), doc 01 §1–§2 (crate registry overview only — no real crates created), MP §4 layout, doc 11 §3.7 (lockfile posture).
- Environment: verified rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1; installed missing rustfmt (1.10.0-stable) + clippy (0.1.99) components. Scratch experiments in gitignored tmp/a1 (removed afterwards).
- Scratch findings — forced deviations from lead notes, each proven empirically on cargo 1.99.1:
  F1: `members=["crates/*"]` + `exclude=["crates/_template"]` leaves ZERO members → `cargo check --workspace` hard-errors: "the manifest is virtual, and the workspace has no members".
  F2: an excluded crate cannot build at all with `lints.workspace = true` — "error inheriting `lints` from workspace root manifest's `workspace.lints` / failed to find a workspace root" (both `cd crates/_template && cargo check` and `--manifest-path`).
  ⇒ `crates/_template` IS a workspace member (package `template`, `publish = false`); no `exclude` key. Matches the lead's own canary command `cargo clippy -p <template>` (requires membership). Conflict-avoidance intent fully preserved by the glob.
  F3: `name = "TEMPLATE"` fires `non_snake_case` warn → escalates under clippy `-D warnings` ⇒ snake_case name `template`.
  F4: unregistered `#[cfg(lint_canary)]` fires `unexpected_cfgs` warn (fails `-D warnings`); cargo REJECTS mixing `[lints] workspace = true` with a local `[lints.rust] unexpected_cfgs` check-cfg entry ("cannot override `workspace.lints` in `lints` …"); root lints must stay doc-10-§3.1-exact ⇒ minimal template-only `build.rs` emitting `cargo::rustc-check-cfg=cfg(lint_canary)` (delete-on-copy; build scripts remain audit suspects per doc 10 §2.3).
- Wrote root `Cargo.toml`: resolver 2, `members = ["crates/*"]`, `[workspace.lints.rust]` + `[workspace.lints.clippy]` VERBATIM doc 10 §3.1; comment block records membership policy + pre-announced 1.A3 append of `"tools/xtask"` (single planned post-1.A1 edit).
- Wrote `rust-toolchain.toml`: channel "1.99.1", profile minimal, components ["rustfmt","clippy"] (gate tools guaranteed on fresh install; verified rustup auto-installs the pinned toolchain in-repo, same commit hash).
- Wrote `.editorconfig`: root=true, utf-8/lf/final-newline/trim; rs+toml 4-space; yml/yaml/json 2-space; md no-trim + max_line_length off.
- Wrote `crates/_template/`: `Cargo.toml` (name template, edition 2024, publish=false, `[lints] workspace = true` only), `build.rs` (canary cfg registration), `src/lib.rs` (usage doc: copy checklist, lint policy, canary pointer), `src/lint_canary.rs` (sanctioned violations — unwrap_used + unused_variables — compiled ONLY under `--cfg lint_canary`, proof commands documented in-file), `tests/smoke.rs` (real smoke test).
- `.gitignore`: appended Cargo.lock-posture comment only (COMMITTED per doc 11 §3.7; `target/` + `**/*.rs.bk` already covered). All prior content preserved.
- `Cargo.lock` generated (v4, `template` package only, zero external deps) — left in place for Lead commit.
- VERIFY (in-repo, toolchain 1.99.1, final clean run — all green, chain exit 0):
  `cargo check --workspace` → "Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s"
  `cargo test --workspace` → "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
  `cargo fmt --check` → no output (clean); also `cargo fmt --all -- --check` clean
  `cargo clippy --workspace --all-targets -- -D warnings` → "Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s"
  Also green: C3 strict forms (`check --workspace --all-targets --all-features`, `test --workspace --all-features`) and `cargo check --workspace --locked`.
- CANARY PROOF (commands pass NO deny flags — denies can only originate from `[workspace.lints]`):
  `cargo clippy -p template --all-targets -- --cfg lint_canary` → exit 101: "error: unused variable: `unused_var` … = note: requested on the command line with `-D unused-variables`" and "error: used `unwrap()` on an `Option` value … = note: requested on the command line with `-D clippy::unwrap-used`" and "error: could not compile `template` (lib) due to 2 previous errors" (both tables inherited).
  `RUSTFLAGS="--cfg lint_canary" cargo check -p template --all-targets` → exit 101: "error: unused variable: `unused_var` … requested on the command line with `-D unused-variables`" (rust table via plain rustc).
  Negative control: default clippy gate re-run after the canary runs → green.
- Hygiene: no git state mutated (working tree: `.gitignore` M + 5 untracked paths, all within ownership); scratch removed; no VLC source; zero external dependencies (C5 trivially satisfied); no unsafe/unwrap/expect/panic/todo/unimplemented/dbg outside the sanctioned cfg-gated canary.

Stage Summary:
- 1.A1 COMPLETE. Workspace skeleton landed: root manifest (glob membership + verbatim doc 10 §3.1 lints), toolchain pin 1.99.1 (+fmt/clippy components), `.editorconfig`, member template crate with lint inheritance + canary, `Cargo.lock` (committed posture). Mandated gate chain green; canary proves rust+clippy tables inherited with no CLI deny flags.
- [PROPOSAL, Lead ratification requested] Deviations forced by cargo (evidence F1–F4 above): (1) template is a workspace member, not excluded; (2) package name `template`, not `TEMPLATE`; (3) template-only `build.rs` registering the canary cfg. Membership-glob plan intact: later agents add `crates/<name>/` without touching root `Cargo.toml`; 1.A3 pre-announced to append `tools/xtask`.
- Nothing blocked. Follow-up for 1.E1: supersede the canary with the metadata-based `check-architecture` step 0 (doc 10 §3.2) once it lands.


---
Task ID: 1.A2
Agent: Sub-agent (general-purpose), Wave 1.A (Phase 1: Foundation)
Task: CI skeleton — GHA fast tier (fmt/check/clippy/test x windows/linux/macos), concurrency groups, actionlint, later-gate job stubs marked non-blocking. AC adapted per Lead: no remote/runner in sandbox, so workflow + actionlint + local command evidence instead of a green-run link.

Work Log:
- Read AGENTS.md §3–§4 (no commit, append-only log, [VERIFY]/[PROPOSAL]), worklog tail (1.A1 COMPLETE), doc 13 §4 card 1.A2 + §1.4 C1–C8, doc 10 §2 (gate table) + §4 (CI pipeline, G1–G13) + §10 (CI-for-CI), doc 11 §3.6–3.7 (audits chain for the cargo-deny stub; "builds are --locked in CI").
- Pinned actions by full SHA, resolved via `git ls-remote` (GitHub REST API rate-limited): actions/checkout v7.0.1 3d3c42e5; dtolnay/rust-toolchain `1.99.1` branch d124f86f (per-version branch hard-codes the toolchain — no input drift); Swatinem/rust-cache v2.9.2 6323deb1; actions/upload-artifact v7.0.2 cf430e03. Version+SHA tables recorded in ci.yml header + workflows/README.md.
- Runner research (raw.githubusercontent, 2026-10-19): ubuntu-24.04 images ship NO preinstalled Rust/rustup (Readme has no Rust section) -> bare rustup steps would break on ubuntu; dtolnay action bootstraps rustup when missing. macOS-15/windows-2025 images ship rustup 1.29.0/1.29.1.
- rustup 1.29.1 behaviors proven locally: (T1) rust-toolchain.toml components auto-install on first proxy use (removed rustfmt, `cargo fmt --version` re-downloaded it); (T2) RUSTUP_TOOLCHAIN env var overrides toolchain files; (T3) a missing override toolchain is auto-installed (masks naive drift checks). => every Rust job carries a 3-way assert step: rust-toolchain.toml channel == action-installed name == effective `rustc -V`; all mismatch directions + env-var hijack fail loudly.
- Wrote `.github/workflows/ci.yml`: name CI; triggers push(main)/pull_request/merge_group/workflow_dispatch; permissions contents:read; defaults.run.shell bash; concurrency `${{ github.workflow }}-${{ github.ref }}` with cancel-in-progress only on pull_request (doc 10 §10.1). Jobs: fmt (ubuntu, doc 10 §2.1 #1 verbatim); actionlint (ubuntu, actionlint 1.7.12 via pinned-tag download script — exact steps rehearsed locally); deps (ubuntu: `cargo metadata --format-version 1 --locked` lockfile assertion per doc 11 §3.7 + `cargo tree --workspace` + `--edges normal,build,dev` per doc 10 §2.1 #5/§2.3 + evidence-deps-ubuntu-latest-<run_id> artifact, 90-day retention per §4.5); check/clippy/test on 3-OS matrix (ubuntu/windows/macos-latest) with fail-fast:false (doc 10 §4.2), commands #2/#3/#4 verbatim; test needs [fmt, check] (doc 10 §4.2 cheap-first ordering). Four STUB jobs: boundary-check (1.E1, G5), schema-validation (1.E2, G1/G6), wasm-build (1.E3, G12 — 1.E3 owns wasm.yml, may supersede the stub), cargo-deny (1.E5, G3/G4) — each continue-on-error:false, exit-0 placeholder step, `[STUB until <card>]` name suffix, activation instructions in banner comments.
- Wrote `.github/workflows/README.md`: honest-status header (no remote — green-run link PENDING first push), job map, stub activation protocol (with "do not add to required checks until activated"), action-pin table, design decisions (toolchain drift guard, --locked posture, runner labels, evidence uploads minimal until 1.E2, branch protection Lead-only). `.github/actionlint.yaml` not needed (no config required).
- VERIFICATION (in-repo, toolchain 1.99.1): actionlint 1.7.12 on ci.yml exit 0 — with shellcheck 0.11.0 (actionlint -shellcheck shellcheck) AND plain auto-detect with shellcheck in PATH (mirrors ubuntu runner where shellcheck is preinstalled). Local exit codes of every cargo command CI invokes (workspace = HEAD b3141a6 state, members crates/*): #1 fmt 0; #2 check 0; #3 clippy -D warnings 0; #4 test 0 (smoke 1 passed); metadata --locked 0; tree 0; tree --edges 0. --locked variants (data for Lead): check 0, clippy 0, test 0. Assert-step simulation: 3-way match PASS (1.99.1/1.99.1/1.99.1); stub placeholder exit 0.
- NEGATIVE CONTROLS: assert with inst=1.98.0 -> exit 1 (drift caught); `cargo metadata --locked` on a full /tmp workspace copy with an unregistered dep appended -> exit 101 "cannot update the lock file ... because --locked was passed" (deps job genuinely gates the lockfile); deliberately-injected workflow (untrusted-expression echo) -> actionlint exit 1 (actionlint job genuinely gates).
- PARALLEL-AGENT NOTE: 1.A3 landed tools/xtask + the root-Cargo.toml members append DURING my verification. All green runs above are on the pre-append workspace (b3141a6 state). Re-runs after the append show 1.A3's in-flight xtask temporarily failing fmt (--check diffs) and check/clippy/test (compile errors, since being fixed by 1.A3 as it iterates — later re-run: check 0, fmt still 1). Not my surface; did not touch tools/ or root manifests; 1.A3's own AC + wave review step 4 (doc 13 §1.5) own the final full-workspace green.

Stage Summary:
- 1.A2 COMPLETE. CI skeleton landed: `.github/workflows/ci.yml` (fast tier mirroring doc 10 §2.1 verbatim + concurrency + actionlint self-check + 4 non-blocking 1.E* stubs) and `.github/workflows/README.md` (job map / activation protocol / SHA pin table). actionlint 1.7.12 clean (with shellcheck); every CI cargo command green locally with recorded exit codes; 3 negative controls prove the new gates actually fail when violated.
- [PROPOSAL, Lead ratification requested] Deviations: (1) stubs are running exit-0 placeholders (actionlint's if-cond rule rejects `if: false`; doc 10 §10.5 mandates actionlint) — names carry `[STUB until <card>]`, continue-on-error:false pre-set, README forbids required-checks listing pre-activation; (2) doc 10 §2.1 has no `--locked` flags while doc 11 §3.7 says CI builds run `--locked` — resolved as verbatim gate strings + deps-job `cargo metadata --locked` assertion; strong form `cargo xtask ci <job>` (doc 10 §10.5) proposed to own this once 1.A3 lands; (3) runner labels `-latest` per Lead instruction vs doc 10 §4.1's proposed ubuntu-24.04/windows-2025/macos-15 — one-line switch, README notes it.
- green-run artifact link: PENDING first push to remote (blocked: no remote in sandbox) [PROPOSAL: gate evidence on first push]. Branch protection (doc 10 §4.4) is Lead-only repo settings, to be configured only after a first green run exists.
- Nothing else blocked. Follow-ups: 1.E2 wires full §4.5 evidence uploads + job graph; 1.E3 decides wasm.yml vs stub; first real run will confirm runner-side assumptions (assert step is the tripwire).

---
Task ID: 1.A3
Agent: Sub-agent (general-purpose), Wave 1.A (Phase 1: Foundation)
Task: xtask v0: `cargo xtask gate` runs the doc 10 §2 command table in order, captures per-command pass/fail, prints the machine-readable honest gate report. AC: gate fails when any command fails; report shape matches doc 10 §2.

Work Log:
- Read AGENTS.md §3–§4, worklog tail (1.A1 skeleton + 1.A2 CI entry), doc 13 §4 Wave-1.A card + §1.4 C1–C8, doc 10 §2 (gate table + xtask behavior + §2.4 report block) + §7.2 (evidence/1 envelope) + §10.5 (gate/CI parity).
- Wrote `tools/xtask/` (package `xtask`, edition 2024, publish=false, `[lints] workspace = true`, ZERO external dependencies — std-only): `src/main.rs` (XtaskError enum, dispatch, usage, exit-code contract 0/1/2), `src/cmd/mod.rs` (frozen registry), `src/cmd/gate.rs` (implementation + 29 unit tests), five stubs `src/cmd/{schema_gen,tokens,codegen,check_architecture,rebrand}.rs` (typed `NotYetImplemented { subcommand, card }`, exit 2), `tests/cli.rs` (7 integration tests via CARGO_BIN_EXE_xtask; never runs the real gate table — cheap invocations only).
- FROZEN-INTERFACE declaration recorded in main.rs + cmd/mod.rs headers: subcommand table registered extension-first (gate=1.A3 implemented; schema-gen=1.B2, tokens=1.B4, codegen=1.D4, check-architecture=1.E1, rebrand=1.E4); later agents edit ONLY their own `src/cmd/<name>.rs` stub (headers say "Owned by card X"); new subcommands need Lead-granted ownership of the frozen files.
- Gate semantics: doc 10 §2.1 table verbatim (argv joined = reported cmd string, unit-tested); fail-fast default + `--keep-going`; `--fast` (1–4+6–8) and `--no-ui` (drops 6–9) [PROPOSAL: interpretation] mark rows `SKIP (excluded: …)`; steps whose target dir is absent (ui/, apps/desktop/, crates/engine-wasm-binding) report `SKIP (<path> not present)` — honest NOT-RUN lines that do not fail the gate; spawn failures and nonzero exits are FAILs. Per step captured: exit code, duration (ms in JSON), stdout/stderr tails ≤40 lines; first `error` line shown inline after FAIL (doc §2.4 hint); `--- failure details ---` section on failure. Report block matches doc 10 §2.4 shape (exact-string unit test); JSON log always written to `target/gate-report.json` [PROPOSAL: doc names gate-report.json; target/ avoids dirtying the tree without .gitignore ownership] with schema "evidence/1" envelope (hardware_class "mock" [PROPOSAL], junit null until nextest). Root discovery: walk up for Cargo.toml with [workspace]; works from subdirs; typed error outside workspace.
- `.cargo/config.toml`: `[alias] xtask = "run -p xtask --"` (only content; build-affecting config stays Lead-owned).
- Root `Cargo.toml`: the single pre-announced edit — members = ["crates/*", "tools/xtask"] + comment updated. `Cargo.lock` regenerated mechanically (xtask package, zero deps). No .github/** touched (1.A2 parallel); no git mutations.
- Scratch: epoch vectors for the std-only ISO-8601 conversion verified with `date -u`; cargo unknown-flag exit code checked (=1) before using it as the injected failing command in tests.
- VERIFY (all green, toolchain 1.99.1): `cargo check --workspace` ✓; `cargo test --workspace` → template 1 + xtask 29 unit + 7 integration passed, 0 failed ✓; `cargo clippy --workspace --all-targets -- -D warnings` ✓ (zero unwrap/expect/panic/todo/unimplemented/dbg — typed errors only); `cargo fmt --check` ✓; also `cargo check --workspace --locked` ✓ and the stricter gate forms (`--all-targets --all-features`).
- `cargo xtask gate` (full real run): exit 0 — `result: SUCCESS (0 of 10 failed, 5 skipped)`; steps 1–5 PASS (fmt/check/clippy/test/tree), 6–10 SKIP (ui/, apps/desktop/, crates/engine-wasm-binding not present — honest Phase-1 start state). gate-report.json validated with python3 json.tool; per-step exit codes + ms durations + test-count tails captured.
- `cargo xtask schema-gen`: exit 2 — `xtask error: subcommand 'schema-gen' is not yet implemented (card 1.B2)` (same for the other four stubs, integration-tested).

Stage Summary:
- 1.A3 COMPLETE. `cargo xtask gate` is the single entry point for the doc 10 §2 local gate: verbatim 10-step table, fail-fast or --keep-going, honest machine-readable block + evidence/1 JSON log, exit 1 on any failure, 5 later cards pre-registered on the frozen dispatch table with typed not-yet-implemented errors.
- [PROPOSAL, Lead ratification requested]: (1) report file at target/gate-report.json; (2) hardware_class "mock" in the gate envelope; (3) --no-ui = drop steps 6–9; (4) wasm binding crate path const `crates/engine-wasm-binding` pending the doc 05 §5.2/doc 09 card; (5) junit counts deferred to nextest wiring (counts honestly present in stdout tails).
- [VERIFY] when ui/ lands: `pnpm` on Windows is a .cmd shim — Command::new("pnpm") spawn behavior needs a check then (documented in gate.rs limitations).
- Nothing blocked. Follow-ups: 1.B2/1.B4/1.D4/1.E1/1.E4 replace their stubs; CI (1.A2) can route jobs through the gate's exported command strings for §10.5 parity (`gate --dry-run --json` dumps the table).

---
Task ID: 1.B3-R
Agent: Sub-agent (general-purpose), Wave 1.B salvage retry (Phase 1)
Task: Complete card 1.B3's remains after its context-deadline timeout (salvage per doc 13 §1.6): repair the localization crate's 9 compile errors plus latent mid-refactor logic bugs, repair corrupted refs fixtures, add the missing seed-integrity test, and land the never-created ui/ lint stubs. NO git commits.

Work Log:
- Reproduced all 9 errors with `cargo check -p localization` (CARGO_TARGET_DIR=tmp/1b3r/target); a 10th (key.rs:214 `Ok(())` in a `()`-returning test) surfaced under --all-targets.
- Syntax/type repairs: message.rs:333 `&'a self.parts` → `&self.parts`; message.rs:304/343 or-pattern `Select(arms)|Plural(arms)` split into separate match arms (bindings had different types); audit.rs:22 removed unused `MessageKey` import; locale.rs:55 `const REFERENCE` (String, non-const) → `Locale::reference()` constructor reading `catalog::REFERENCE_LOCALE` (single source of truth), fallback.rs call site updated; refs.rs `RefError::Io.source: String` (thiserror `as_dyn_error` misuse) renamed to `detail`; refs.rs E0282 fixed by extracting `refs_parse_error(src, &toml::de::Error)` (same pattern as catalog.rs); catalog.rs E0382 `span` move fixed with `.as_ref()` on first use; message.rs unused `count` param removed from `render_placeholder` (+ its `too_many_arguments` allow).
- Latent logic bugs found once it compiled (12 failing tests): (1) parse_arms never consumed the `,` after `select`/`plural` — every select/plural message failed to parse; fixed. (2) `}}` at an arm boundary was eaten as a brace escape, so sub-messages ending in `…}}` hit end-of-input — redesigned with a `ParseContext` (Top/SelectArm/PluralArm): escapes only at top level, `}` always closes an arm (ICU semantics), `#` only in plural arms; module doc updated. (3) catalog.rs checked key grammar before value shape, so `[a]` sections reported InvalidKey instead of NestedTable — reordered. (4) audit rule 5 flagged extra arms for languages outside the plural table — now strict (exact CLDR set) only for known languages, `other`-only + tolerant otherwise, per its own doc. (5) locale `en_US` now maps to `InvalidSubtag` (message names BCP-47 hyphens) instead of `InvalidLanguage`. (6) refs.rs test expected document order; `toml::Table` iterates BTreeMap-sorted — test expectation corrected with a note (preserve_order would add indexmap = new dependency = Lead decision, not taken).
- Test-expectation repairs (detection still asserted; byte offsets/order only): `{}` → MissingName offset 1; plural DuplicateArm offset 19 (was inconsistent with the other three arm-error cases); doc 07 §6.2 select example given the mandatory `other` arm — crate rule vs. doc example flagged [VERIFY] in-code and below.
- Repaired corrupted fixtures tests/fixtures/refs/{unresolved,resolved}.toml (`[enu.leaf]]` invalid TOML → `[[menu.leaf]]`; unresolved's first ref pointed outside the fixture reference → `fixture.greeting`); audit.rs doc table now names the real fixture paths.
- Created tests/seed_integrity.rs (promised by lib.rs docs): shipped locales/ passes every audit rule; product.toml msgids (2) resolve (rule 4); Phase-1 seed keys present.
- ui/ stubs per card scope: .eslintrc.json (lumen/i18n-no-literals skeleton, settings reference cargo xtask l10n-audit + crates/localization), i18n-allowlist.json (`[]`), README.md (one paragraph: React app lands Phase 4, placeholders, no package.json). No package.json created.
- VERIFY: check/test/clippy -D warnings/fmt --check all green for -p localization (45 unit + 14 proof-matrix + 3 seed + 2 doc tests, 0 failed). JSON validity of ui/ files checked with python3.
- NOT run green: workspace-wide `cargo fmt --check` — currently fails inside crates/product-config/src/schema.rs (unexpected `}` at line 377), another agent's in-flight file (explicitly not mine to touch).

Stage Summary:
- 1.B3 COMPLETE (salvage): localization crate compiles, all 64 tests green, clippy/fmt clean under workspace lints; the five doc 07 §6.4 audit rules are enforced by tests/audit_proof_matrix.rs (fixtures repaired) plus tests/seed_integrity.rs over the shipped tree; ui/ lint stubs landed without package.json.
- [VERIFY — Lead reconciliation]: (a) doc 07 §6.2's select example omits the `other` arm the crate mandates (test adjusted, in-code marker); (b) refs extraction order is BTreeMap-sorted, not document order.
- Observation for the Lead (tools/ not mine to edit): xtask gate steps 6–8 skip on `ui/` *directory* absence — the dir now exists (pre-dated this card, empty), so a real gate run will attempt `pnpm --dir ui lint` (pnpm absent here). Precondition may need to become `ui/package.json` at 1.E-card time.
- Workspace `cargo fmt --check` blocked by in-flight product-config syntax error (other agent); localization-only fmt is clean. Nothing else blocked.

---
Task ID: 1.B1-R
Agent: Sub-agent (general-purpose), Wave 1.B salvage retry (Phase 1)
Task: Complete card 1.B1's remains after its context-deadline timeout (salvage per doc 13 §1.6): make `cargo test -p product-config` green without weakening validation, align the hw-decode enum with accepted ADR-001, and clear clippy/fmt. NO git commits.

Work Log:
- Reproduced with `cargo test -p product-config` (CARGO_TARGET_DIR=tmp/1b1r/target): 24 passed / 3 failed in fixtures_matrix (enum_hw_decode…, enum_subtitle_mode…, enum_crash_reports…). `--no-fail-fast` exposed 9 more latent failures the Lead's fail-fast run never reached: 1 golden_s35 + 8 semantic.
- Root cause A (the 3 named enum tests): the structural enum branch rendered legal values quoted (`must be one of: "auto", …`) while the crate's own contract (fixtures_matrix assertions, semantic injected-enum lists, §3.5 golden prose) renders them bare. schema.rs: bare vocabulary tokens in rule/expected; `found` stays a quoted literal via describe_value.
- Root cause B (hw test only, stale pre-ADR expectation): ADR-001 ACCEPTED 2026-10-19 (docs/adr/ADR-001-hardware-decode.md — hw-first H.264/HEVC via OS decoders); doc 07 §3.2 says the enum "grows only post-ADR-001". Grew the legal set to software-only | prefer-hardware | hardware-only in product.schema.json (+$comment citing ADR file+date) and model.rs HwDecodePolicy (+PreferHardware/HardwareOnly, ADR-cited doc comments). Fixture mutation now `hw_decode_policy = "hw-first"` (the ADR's feature-matrix row term — NOT config vocabulary; rejection stays strong); test renamed enum_hw_decode_policy_is_rejected_with_the_post_adr001_legal_values (+line assert); fixtures README map updated. Root product.toml keeps `software-only` (honest until the Phase 7 audited integration lands; rule F2 keeps features.hw-decode = false); its stale "PENDING ADR-001" comments (playback + features) rewritten to cite the acceptance.
- Root cause C (golden_s35, pre-existing): schema probe_max_bytes minimum was 1048576 (1 MiB) so the 1 MiB fixture sat AT the bound and validated clean; doc 07 §3.5's normative example and the x-lumen-hint both mandate a 16 MiB floor. minimum → 16777216 (a tightening) + $comment noting §2.9's "1 MiB..=64 MiB" comment is the outlier.
- Root cause D (cross_field_schema_version): schema pinned `schema_version` to `const: 1`, contradicting doc 07 §3.3's cross-field rule (== max(migrations.to), enforced by the semantic pass on the resolved config) and structurally bricking schema v2. Replaced with integer/minimum 1 + explanatory $comment.
- Root cause E (7 semantic tests): the semantic pass pushed found strings bare while the crate contract quotes them (structural pass, doc 07 §3.5). Added `quoted()` (same as describe_value's string arm) for short_name, id-consistency, injected-enum, catalog and F1 founds. Locales rule placeholder `<id>` → `<tag>` per doc 07 §2.4.
- Clippy `-D warnings` (36 errors, all pre-existing — 1.B1 never ran it): fixed with let-chains (collapsible_if ×15), assert!(false)→unreachable! everywhere (workspace denies panic!/todo!/unimplemented; a scratch probe confirmed unreachable! is allowed under the workspace lint block), derive(Default) for Urls/Migrations, Box<Diagnostic> on the two private load.rs helpers (result_large_err), inherent Schema::from_str → std::str::FromStr (should_implement_trait; callers use .parse::<Schema>()), manual_contains ×2, manual_clamp, useless_format, while_let_on_iterator, unwrap_or_default ×2, explicit_auto_deref, bool assert_eq!s, field-reassign-with-default, and an InjectedEnum struct (type_complexity). `#[allow(clippy::too_many_arguments)]` added to the third sibling fn diagnostic() (matches 1.B1's existing two). `cargo fmt` applied crate-wide.
- VERIFY: `cargo test -p product-config --no-fail-fast` → 99 passed, 0 failed (39 lib + 27 fixtures_matrix + 1 golden_s35 + 15 merge + 17 semantic). `cargo clippy -p product-config --all-targets -- -D warnings` → clean. `cargo fmt -p product-config --check` → clean. product.schema.json re-validated as JSON (python3); schema↔model enum sets agree (hw fixture pins all three legal values); pattern-coverage drift guard green; no other crate depends on product-config yet (tools/ grep), so the FromStr switch breaks nothing.

Stage Summary:
- 1.B1 COMPLETE (salvage): suite 99/99 green, clippy and fmt clean, no test deleted or weakened. The only legal-set change tracks accepted ADR-001; the only bound change tightens (probe 16 MiB floor per §3.5); schema_version const → cross-field rule restores doc 07 §3.3 exactly.
- Registry candidates for doc 10 §3.3 when the suppression registry lands: 3× clippy::too_many_arguments in schema.rs (param-struct refactor deferred; 1.B1 precedent).
- For the Lead (docs not mine to edit): doc 07 §2.8/§3.2 still sketch the pre-ADR single-value hw enum, `schema_version: const 1`, and §2.9's "1 MiB..=64 MiB" probe comment — all three now contradicted by the shipped schema; the §3.2 sketch also shows the old inherent from_str.
- The other ~40 fixtures still carry the inert pre-ADR comment block (regeneration is manual per fixtures README; only the hw fixture was regenerated). Workspace-wide `cargo fmt --check` is currently blocked by in-flight crates/theme-tokens (missing src/bin/tokens_gen.rs) — not this card's path; product-config itself is clean.
---
Task ID: 1.B4-R2
Agent: Sub-agent (general-purpose), Wave 1.B salvage retry #2 (Phase 1)
Task: Fix card 1.B4's 7 failing pipeline integration tests (salvage retry #2, after the 1.B4 and 1.B4-R context deadlines; the Lead had completed the crate + xtask bridge and verified regeneration idempotence). NO git commits.

Work Log:
- Reproduced with `cargo test -p theme-tokens --test pipeline` (CARGO_TARGET_DIR=tmp/1b4r2/target): 5 passed / 7 failed — six erroring with `Error: "expected a parse error"`, plus mini_source_is_valid_and_counts_survive at pipeline.rs:258 (left 7, right 8).
- Root cause 1 (all six expect_error tests — test-bug): the helper did `MINI.replacen("REPLACE_ME", mutation, 1)`, but MINI contains no REPLACE_ME marker, so every caller's pre-mutated source was silently discarded and the valid MINI parsed clean. The helper now parses the passed source directly; removed the eleven no-op `.replacen("REPLACE_ME", "", 1)` links from the callers.
- Root cause 2 (layer_rules_are_enforced — test-bug): the semantic-skips-alias mutation's replacement string was missing its closing JSON quote (`"{primitive.color.orange.500}`), which would yield a JSON syntax error instead of the layer rule. Quote restored; needle "must resolve through the alias layer" matches parse.rs's actual diagnostic (doc 08 §3.2).
- Root cause 3 (counts test — test-bug): MINI defines exactly 7 alias `$value`s; the expectation said 8. Corrected to 7 with an in-code justification comment (the parser counts one alias per `$value`).
- Every remaining needle verified against the real diagnostics (parse.rs / TokenError Display / Rgb8::parse_hex): layer rules, unknown reference, cycle chain, per-mode coverage, bad color, default-mode, $extensions, unsupported $type, unsupported semantic category — all match; no library bugs found; no check weakened.
- Clippy `-D warnings` was failing pre-existing (4 lints under this toolchain's clippy 1.99, none caused by my test edits — same situation 1.B1-R hit): fixed behavior-preservingly — color.rs `chunks_exact(2)` → `as_chunks::<2>()`, emit.rs int_plus_one (`+ 1 <=` → `<`), tokens_gen.rs 2× collapsible_if collapsed via edition-2024 let-chains. Scope note: the exception clause's file list omitted src/bin/, but the binding `--all-targets` clippy gate cannot pass otherwise.
- `cargo fmt` applied to the tests dir only (pipeline.rs + semantic_lint.rs, blessed by the method); src/ left untouched — this rustfmt 1.10 wants to rewrite the whole crate including src/generated/tokens.rs (forbidden), so the pre-existing src formatting drift is a toolchain-version mismatch, not this card's to fix.
- VERIFY: `cargo test -p theme-tokens` all targets green — 22 lib + 12 pipeline (was 5/7) + 2 semantic_lint + 0 bin + 0 doc = 36. `cargo clippy -p theme-tokens --all-targets -- -D warnings` clean. `cargo run -p theme-tokens --bin tokens-gen -- --check` exit 0 ("no drift — outputs match"; 92/70/70 tokens). `git diff --stat -- themes/ crates/theme-tokens/src/generated/` empty; generated outputs' mtimes (19:55) predate this session's edits (20:02+).

Stage Summary:
- 1.B4 COMPLETE (salvage retry #2): all 7 failures were test bugs — a dead mutation path through a nonexistent REPLACE_ME marker, one truncated JSON string literal, one off-by-one alias count. Fixed without weakening any doc 08 §3 assertion. Suite 36/36 green, clippy clean, tests dir fmt-clean, zero output drift.
- Lib/bin footprint for Lead review: 4 clippy-lint-only edits (src/color.rs, src/emit.rs, src/bin/tokens_gen.rs ×2) — behavior-preserving and drift-verified via `tokens-gen --check`, but the bin file sat outside the exception clause's literal list.
- Observation: crate-wide `cargo fmt --check` cannot go green under rustfmt 1.10 without rewriting src/generated/tokens.rs (Lead-verified committed output) — consider a rustfmt.toml or regenerate+recommit at integration time.

---
Task ID: 1.B4 (original) + 1.B4-R (retry) — salvage records
Agent: Sub-agents (general-purpose); both hit context deadline mid-card; Lead attribution
Task: theme-tokens crate + token pipeline per doc 08 §3 (card 1.B4).

Work Log:
- 1.B4 (original): wrote themes/tokens.json canonical source (92 primitive / 70 alias / 70 semantic tokens, 2 modes dark-default, 9 accents orange-default), library modules (color/model/parse), and the crate Cargo.toml documenting the subprocess-bridge [PROPOSAL] (xtask cannot path-depend on the crate without editing its frozen manifest). Died before: emit/lint modules completion, tokens-gen binary, generated outputs, tests, xtask wiring. Also violated the root-Cargo.toml freeze (added tmp/1b4/deptest member — scratch, later absent, breaking the workspace; Lead reverted).
- 1.B4-R (retry): completed emit.rs + lint.rs + src/bin/tokens_gen.rs + tests/{pipeline,semantic_lint}.rs + both generated outputs (tokens.css 240 ln, tokens.rs 212 ln). Ratified-bridge wiring NOT done; 7 pipeline tests failing at death.
- Lead completion (this session): wired tools/xtask/src/cmd/tokens.rs subprocess bridge (ratified the [PROPOSAL]); fixed clippy map_or-identity; fixed the fmt/regeneration fight by emitting #[rustfmt::skip] on the wrapped ACCENTS const in emit.rs (regeneration now fmt-stable); updated xtask cli test (tokens graduated from stub table).
- 1.B4-R2 (separate entry above): fixed the 7 pipeline test bugs (all test-side: dead REPLACE_ME mutation path, truncated literal, off-by-one count) + 4 behavior-preserving clippy fixes.

Stage Summary:
- 1.B4 COMPLETE via 1.B4-R + 1.B4-R2 + Lead completion. AC met: generated outputs committed + diff-tested (tokens-gen --check exit 0; double regeneration = empty git diff); semantic-only consumption lint fixture-proven; idempotence Lead-verified twice. Pipeline: 92/70/70 tokens, 9 accents, dark default — matches doc 08 §1 mined values (test resolved_values_match_doc_08_section_1_2 green).
- Process note: 3 launches consumed for 1 card — parallel triple-launch (1.B1+1.B3+1.B4) all hit the tool context deadline; safe concurrency for M-size implementation cards re-confirmed at 2 (matches Wave 1.A experience).

---
Task ID: 1.WAVE-LEAD
Agent: Lead
Task: Phase 1 launch session: housekeeping, Wave 1.A (1.A1–1.A3), Wave 1.B (1.B1, 1.B3, 1.B4; 1.B2 pending), salvage coordination, honest-gate fix.

Work Log:
- Grounded state: Phase 0 COMPLETE (tag phase-0-gate, exit report 13/13 PASS; stale session summary superseded — D-009 MIT + D-010 Figma-terms already recorded in the lost context). Cleaned environment-restore mode-noise (209 files 644→755, 0 content changes): chmod-restore + core.fileMode=false.
- Provisioned Rust 1.99.1 (stable, minimal profile + rustfmt/clippy) — cargo/rustc were absent from the sandbox.
- Wave 1.A: 1.A1 workspace skeleton (members=crates/* glob, workspace.lints per doc 10 §3.1, toolchain pin, template crate with cfg-gated lint canary) — committed b3141a6. 1.A2 CI skeleton (ci.yml: fmt/actionlint/deps + check/clippy/test × 3 OS, SHA-pinned actions, 1.E* stubs; actionlint 1.7.12 clean; green-run link honestly PENDING first push) + 1.A3 xtask gate v0 (doc 10 §2 10-step table, evidence/1 JSON report, frozen dispatch with pre-registered stubs) — committed fc9fd39 after consolidated Lead gate re-run.
- Wave 1.B: launched 1.B1/1.B3/1.B4 in parallel — ALL THREE hit tool context deadline (see 1.B4 record note). Salvage per doc 13 §1.6: 1.B1-R (3+9 fixture failures + 36 clippy fixed; ADR-001-aligned hw-decode policy enum; 99 tests), 1.B3-R (9 compile errors + 6 latent parser bugs + ui/ stubs; 64 tests), 1.B4 chain above.
- Honest-gate fix: ui steps' skip condition changed from bare ui/ dir to ui/package.json marker (1.B3's config stubs must not activate a pnpm gate); gate.rs doc table + 3 unit assertions + cli.rs integration assertion updated; drift-check + fmt-stability of generated outputs Lead-verified (cargo fmt → cargo xtask tokens → fmt --check clean → second regeneration empty diff).
- Full workspace verification: cargo test --workspace = 234 passed / 0 failed (localization 62, product-config 99, template 1, theme-tokens 36, xtask 38 — including the 7 cli integration tests); cargo xtask gate = SUCCESS 0 failed / 5 honest skips.

Stage Summary:
- Wave 1.A COMPLETE (3/3). Wave 1.B at 3/4 (1.B2 settings-schema pending — dep 1.B1 satisfied; launch next).
- Config spine standing: product.toml + product.schema.json + validator + precedence-merge v1 (1.B1); localization key grammar + 5 audit rules + en-US seed (1.B3); full token pipeline with committed generated outputs (1.B4).
- Next: 1.B2 (settings-schema + generators), then Lead-owned Wave 1.C (media-types I/II, testkit v0, ADR-005 decision).

---
Task ID: 1.B2
Agent: general-purpose implementation agent
Task: settings-schema crate + generators per doc 07 §5 (card 1.B2).

Work Log:
- Read AGENTS.md §3, worklog tail (1.WAVE-LEAD state), doc 13 §4 row 1.B2, doc 07 §4+§5 (full), doc 01 §2.3 settings-schema entry (name + "no internal deps"), doc 05 §5.3 (reserved CLI names), product-config public API + product.toml, theme-tokens/tokens-gen pattern + xtask frozen dispatch.
- Crate `crates/settings-schema` (L1, deps: serde/toml/thiserror — all preaudit **pass**, same family as product-config; no new deps → no records file needed): `diagnostic.rs` (rule+line+path+message contract), `model.rs` (SettingDef with all 10 mandated §5.2 fields + 4 [PROPOSAL] extensions; SettingType incl. list<T>/duration-ms/percent; shared `check_value` for G4 + E-DEFAULT), `parse.rs` (serde raw structs w/ deny_unknown_fields + LineIndex scanner for file:line diagnostics; D1 xor; E-FIELD/E-TYPE/E-FORMAT/E-DOMAIN/E-DEFAULT), `validate.rs` (D1 product-join: default_ref existence + type-check + same-area literal-dup heuristic [PROPOSAL per doc]; D2 keys; D3 flags/shorts vs doc 05 §5.3 reserved table+subcommands; D4 msgids; D5 orders/screens/ui-false-needs-advanced; D6 default_ref = product-decision slot [PROPOSAL interpretation; engine-slot owner table lands 4.E1]; E-GATE vs product [features] F1), `emit.rs` (G1 .d.ts types, G2 --help golden, G3 docs page — all pure fns, sorted (screen,section,order,key)), `runtime.rs` (G4: include_str!-embedded schema, validate_value = config.set-setting seam w/ invalid-argument + did-you-mean, schema_meta for get-setting), `src/bin/schema_gen.rs` (regeneration binary, --check drift mode, exit 0/1/2).
- Seed `schemas/settings.toml`: 27 entries / 14 areas (13 doc 07 §5.4 screens + `engine`), 24 default_refs into product.toml (window/defaults/telemetry/playback/limits) + 3 engine literals; [areas].known seeds the screen registry until layouts/settings-areas.toml lands [noted in file].
- xtask bridge `tools/xtask/src/cmd/schema_gen.rs`: subprocess `cargo run -p settings-schema --bin schema-gen` (Lead-ratified 1.B4 pattern), args forwarded verbatim, exit status forwarded.
- Tests: 43 lib (incl. one fixture per D-rule: D1×3 + xor, D2, D3×4, D4, D5×3, D6; committed seed validates against committed product.toml with zero findings) + 7 bin integration (committed --check exit 0; byte-for-byte idempotence ×2 runs; duplicate-option fixture → exit 2, error[D2], first-defined line, no outputs written; stale-output drift → exit 1; --help; missing-file I/O error).
- VERIFY: check/test/clippy -D warnings/fmt all green (fmt applied to my files only); cargo xtask schema-gen exit 0, second run byte-identical (md5 + --check exit 0, git diff --stat -- schemas/ empty); workspace 283 passed / 1 failed — the 1 is the pre-existing tools/xtask/tests/cli.rs stub assertion `("schema-gen","1.B2")` (not my file): graduation edit is the Lead's one-liner, same as the tokens precedent recorded in that file.

Stage Summary:
- 1.B2 complete: one settings schema (27 Phase-1-real entries, scales to the VLC census via flat [[setting]] arrays), D1–D6 all fixture-proven with actionable diagnostics, three committed deterministic generators (125/121/96 lines), runtime validation seam (validate_value/schema_meta) for product-config's config.set/get-setting, xtask subcommand live.
- Known Lead items: (a) cli.rs graduation one-liner above; (b) Cargo.lock updated by cargo (settings-schema deps added); (c) [VERIFY] items in seed: dynamic locales/themes enums, engine.max-event-rate default, engine.tick-interval-ms vs doc 05's playback.time-tick-interval-ms naming; (d) G1 emits types only — value table lands with 4.E1.

---
Task ID: 1.D2
Agent: Sub-agent (general-purpose), Wave 1.D
Task: plugin-api per doc 06 §1–2 (PluginKind, manifest types, host traits) — BLOCKED before any code was written by a mid-session sandbox reboot; this entry is the incident record plus the prepared design for relaunch.

Work Log:
- Card prep completed BEFORE the incident: read AGENTS.md; the pre-reboot worklog tail (state then: waves 1.A–1.C complete, 1.E3/1.E4/1.E5 landed = 1.E at 3/5, 1.D1 codec-api landed, workspace 401 tests green); doc 13 §1.4/§1.7; doc 06 §1–2 in full (the spec); media-types {ids,error,packet,frame,time} and codec-api {lib,decode} public surfaces (files present and read at session start).
- Design prepared for the relaunch (nothing written to disk): PluginKind with the 16 doc 06 §1.2 kinds, kebab-case serde names, §1.4 v1-extension policy (7 allowed / 7 refused; subtitle-parser and subtitle-renderer appear in NEITHER §1.4 list — plan: builtin-only [PROPOSAL]); host traits defined in plugin-api: Access + SourceStream, Demuxer, AudioFilter/VideoFilter (+ a FilterOutcome), SubtitleParser, PlaylistParser/PlaylistExporter (+ minimal Playlist vocabulary), MetadataProvider, UiPanel, ServiceDiscovery; Decoder/Encoder re-exported from codec_api, NOT redefined; audio-output/video-output (and subtitle-renderer) traits deliberately NOT defined here (doc 06 §1.2 rows 8/9/11 place them in L3/L4 domain crates) — all placement deltas to flag at the doc 06 §10 Q1–Q3 reconciliation.
- Manifest design: serde deny_unknown_fields on every table (§2.1 unknown-rejected); id grammar + 64-byte cap + lumen./ext. prefix-vs-origin rules (§2.2); hand-rolled semver-2.0 validation (semver crate NOT in the C5 grant); priority as Option with origin-aware effective default 100 builtin / 50 extension (§2.2 [PROPOSAL]); capabilities kind-key whitelist with first-listed-key-required [PROPOSAL] plus list rules (non-empty, deduplicated, lowercase, ≤32 B/entry, ≤64 entries total); permissions extension-only + network-host-allowlist implies http-fetch companion rule; provenance required-for-extension / absent-for-builtin; hash prefix "sha256:" checked at parse (doc 06 §2.3's own examples carry the placeholder "sha256:9f2c…(64 hex)" — strict 64-hex belongs to install-time verification, §5.7).
- Roundtrip pre-analysis of the 4 §2.3 fixtures: (d)'s inline comments cannot survive a serde roundtrip (comments are not data); (b)'s sample_rates array is line-wrapped in the doc (serializes single-line — normalized-equivalent); (c)/(d) write [dependencies] as an EMPTY TABLE while §2.2 says "array of plugin ids" — plan: lenient deserialize accepting array | empty table, empty normalized away on serialize; the toml serializer forces bare keys before table headers, so a non-empty deps array must sit right after `origin`.
- INCIDENT (root cause of the block): /tmp/boot-timeline.log shows a fresh start.sh at 2026-10-20T08:01:58Z (uptime 5.8s) — the sandbox rebooted ~3 minutes into this session. /home/z/my-project was re-initialized to commit 410d6cf (Oct 19 20:39 = Wave 1.B complete + ROADMAP edit); worklog.md now ends at the 1.B2 entry. The richer pre-reboot state just read (worklog entries through 1.D1; crates/media-types, crates/codec-api, crates/testkit present; crates/commands absent = 1.D3 not yet landed) is GONE from this environment: `git remote -v` is empty, `git fsck --lost-found` reports nothing, /tmp/my-project is the same wave-1B state. Per the pre-reboot worklog this also destroyed the Wave 1.C artifacts (media-types, testkit), 1.D1 (codec-api) and the 1.E3/1.E4/1.E5 artifacts (.github/workflows/wasm.yml, deny.toml, docs/dependencies/allowlist.md, xtask rebrand subcommand, ci.yml edits).
- Toolchain wiped as well: no ~/.cargo, no ~/.rustup; `cargo`/`rustc` not found after sourcing the cargo env. Restoration recipe is pinned in rust-toolchain.toml (1.99.1, minimal profile, rustfmt + clippy); the worklog 1.WAVE-1E-LEAD entry records the same prior incident and its fix.
- Why no partial delivery: 1.D2 hard-consumes 1.C2 + 1.D1 as the C5-granted path deps (media-types, codec-api) — both absent from the restored tree, so crates/plugin-api cannot compile; with no toolchain the mandatory C3 gate (fmt/check/clippy/test) cannot run and AGENTS.md §3.4 forbids unverifiable claims; writing the crate anyway would break `cargo check --workspace` for everyone (crates/* glob auto-membership + missing path deps). Zero files written outside this entry; nothing to revert.

Stage Summary:
- 1.D2 BLOCKED by environment fault, not by code or spec failure: the mid-session sandbox reboot (08:01:58Z) rolled the workspace back from "1.E at 3/5 + 1.D1, 401 tests" to commit 410d6cf (Wave 1.B complete) and wiped the Rust toolchain. Dependencies 1.C2/1.D1 no longer exist in-tree and are unrecoverable from inside this sandbox (no remote, no dangling objects, no second copy).
- Next steps (Lead): (1) re-provision rustup 1.99.1 per rust-toolchain.toml; (2) restore or redo the lost work per the Lead's own out-of-sandbox records — at minimum Wave 1.C (media-types, testkit) and 1.D1 (codec-api) before 1.D2 can build, plus the lost 1.E3/1.E4/1.E5 artifacts; (3) relaunch 1.D2 with the card unchanged (the design notes above shorten the relaunch).
- Honest-state note: the restored tree's worklog/Cargo.toml/Cargo.lock are self-consistent at the 1.B2-era state. This entry's description of the pre-reboot state is testimony from this session's reads, not from git history.

---
Task ID: 1.WAVE-RESTORE (Lead)
Agent: Lead
Task: Environment-incident recovery: sandbox reboot at 2026-10-20T08:01:58Z rolled the workspace back to commit 410d6cf (Wave 1.B complete), destroying commits bb84ef8 (wave-1E: 1.E3/1.E4/1.E5), 4d7c241 (wave-1C), 3ebcc3a (1.D1 codec-api), the toolchain, and the corresponding worklog entries. Recovery per protocol §1.6 + the 1.D2 agent's blocked report (its design notes survive below in its own entry).

Work Log:
- Re-provisioned rustup 1.99.1 (minimal + rustfmt + clippy) — exact pin match verified.
- RESTORED Wave 1.C from Lead context (the Lead authored both crates this session and retained the final post-fix content): crates/media-types (lib/ids/time/error/packet/frame + Cargo.toml with testkit dev-dep) + crates/testkit (Cargo.toml + lib.rs) + tests/testkit_wiring.rs. All post-landing fixes incorporated (LanguageTag per-subtag casing, file-scheme redaction leading slash, Duration ms component, LCG property-test bounds, underscore error codes, is_multiple_of, let-chains, "tolerance" typo).
- VERIFY (restoration): cargo test -p media-types -p testkit = 50 lib + 2 wiring + 5 testkit = 57 passed / 0 failed; clippy -D warnings = 0 errors; fmt --check clean. Byte-level drift from the lost 4d7c241 commit is possible (formatting-only); the test/verify surface is identical.
- NOT restorable from context (sub-agent re-implementations follow): codec-api 1.D1 (agent-authored, 1860 ln, Lead saw fragments only), wasm.yml (1.E3), deny.toml + docs/dependencies/allowlist.md + ci.yml cargo-deny activation (1.E5), xtask rebrand v0 + tests/rebrand + allowlist (1.E4). The 1.E4 Lead-salvage knowledge (rewrite rebuild bug fix pattern, clippy-compliance patterns) is retained by the Lead and will steer the relaunch.
- The 1.E/1.C/1.D1 worklog entries lost with the rollback are summarized in this entry; the git history divergence (lost SHAs bb84ef8/4d7c241/3ebcc3a) is recorded here for the phase-1 gate report.

Stage Summary:
- Wave 1.C restored and verified (57 tests green). Waves 1.E (3 cards) + 1.D1 flagged for re-implementation via sub-agents; 1.D2 design notes preserved (blocked agent's entry below); Lead proceeds with 1.D3 in parallel.
- Lesson recorded: commit-and-push cadence must survive sandbox reboots — the program's recovery now includes "Lead context is the last-resort artifact store"; nothing outside the sandbox exists.
