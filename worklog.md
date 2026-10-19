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
Task ID: 1.A2
Agent: general-purpose (timed out before reporting; Lead verified output and appended this entry on its behalf)
Task: Write docs/02-phase0-audit-plan.md — Phase 0 audit plan (baseline pinning, inventory methodology, feature-matrix schema, feasibility report structure, ADR-001/002/003 briefs, Phase 0 agent breakdown).

Work Log:
- Wrote docs/02-phase0-audit-plan.md (739 lines): objectives/exit gate, VLC 3.0.23 pinning procedure, inventory methodology, feature-matrix.toml schema with example rows, feasibility report structure, ADR briefs, Phase 0 agent breakdown, risks, open questions, traceability + [VERIFY] ledger appendices.
- Agent hit context deadline before final report/worklog append; Lead inspected the file and confirmed all required sections present with no leftover markers.

Stage Summary:
- Doc 02 delivered. [VERIFY] ledger (V1–V13) feeds Phase 0. Open questions for Lead in doc §9.
