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
