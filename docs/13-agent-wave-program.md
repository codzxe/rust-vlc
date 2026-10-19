# 13 — Agent Wave Program: the complete multi-agent execution plan

Status: planning artifact (Wave C, Task 1.C13). Authority: `docs/MASTER-PROMPT.md`
§15 (multi-agent protocol) and §16 (roadmap/gates) are primary; §17 defines done.
This document operationalizes them at scale: every phase 0–8 decomposed into
waves of small, launch-ready agent task cards — **200 task cards in 77 waves**.
Nothing here overrides AGENTS.md; it extends AGENTS.md §5 (task-ID grammar) and
§2 (assignment contract) with the standard clauses of §1.4.

How to use: the Lead launches a wave by copying each task-card row plus the
standard clauses (§1.4) verbatim into one agent assignment (one card per agent).
Wave tables are the program; prose exists only to bind rules.

## 1. Program principles

### 1.1 Why tasks are small

1. **Bounded context.** An agent reads the worklog, AGENTS.md, the 1–3 doc
   sections its card names, and nothing else. A card that needs the whole repo
   to be understood is a badly cut card.
2. **No timeouts.** Planning-wave agents writing whole 6,000-word docs hit
   context limits (worklog 1.A2 timed out and the Lead had to salvage it).
   Implementation cards are cut so an agent finishes well inside its budget:
   S = one bounded crate module or one doc section; M = one crate or one screen.
3. **Perfect detail per task.** The brief demands agents "implement perfectly
   and in detail". Detail is only achievable when the acceptance criteria are
   checkable in one sitting. Every card below carries checkable criteria.
4. **Reviewability.** The Lead must actually verify every output (§1.5). A
   300-line diff gets a real review; a 3,000-line diff gets a skim.
5. **Recoverability.** A failed or timed-out card costs one card. The timeout/
   failure protocol (§1.6) relaunches narrower; nothing else is at risk.

**Sizing rule (binding):** the size field is S/M/L, but **L is a planning smell,
never a launch state**. If a card reads L, the Lead splits it into more S/M
cards before the wave launches. Several phases below were re-cut for exactly
this reason (e.g. media-types is two cards; the H.264 work is parser +
integration + per-platform adapters in three phases).

### 1.2 Concurrency and conflict rules

Waves are the unit of concurrency. Within a wave, 2–6 agents may run in
parallel, bounded by:

- **R1 Exclusive ownership.** No two agents in a wave own the same file or the
  same crate. The Owns column is exhaustive; everything else is read-only.
- **R2 Lead-owned surfaces.** `crates/media-types`, `crates/codec-api`,
  `crates/commands`, `crates/plugin-api`, `product.toml`, `product.schema.json`,
  `schemas/`, `themes/` token source, `docs/feature-matrix.toml`, root
  `Cargo.toml`, `.github/workflows/*` are edited only by cards marked `[LEAD]`,
  or by exactly one agent holding explicit temporary ownership stated in its
  card. Phase 1 waves C/D are mostly `[LEAD]` for this reason.
- **R3 Registry merge points.** Multi-format crates (`codecs`, `demux`,
  `subtitles`, `playlists`) are partitioned into per-format ownership zones
  (e.g. `crates/codecs/src/audio/flac/**`). The shared registration file and
  Cargo feature list are **not** owned by format agents: each delivers a
  registration snippet in its report; the Lead merges at wave review (§1.5).
- **R4 Interfaces consumed must exist.** The Consumes column lists crates,
  traits, or doc sections that must already be merged. If an agent needs a
  change in a consumed interface, it stops and requests a specification
  decision (AGENTS.md §3.6); it never edits the interface.
- **R5 Integration belongs to the consumer.** Cross-crate wiring is a card in
  the same or a later wave that owns the consuming crate — never a parallel
  edit by the producer.
- **R6 One agent per card, one card per agent.** No card is split across agents
  without a new card ID; no agent holds two cards in one wave.
- **R7 Decision gates block, they don't fork.** `[DECISION-PENDING]` and
  `[CONDITIONAL]` cards (Phase 2 demuxer pair; H.264/HEVC integration; bitmap
  subtitles) are launched only after the named decision/ADR exists; the
  non-selected alternative is closed `not-selected` with a reason, never
  silently dropped.

### 1.3 Task-card format — fields every card carries

A card = **table row + standard clauses (§1.4)**. The Lead pastes both verbatim
into the agent assignment, satisfying the AGENTS.md §2 assignment contract.

| # | Field | Carried by |
|---|-------|------------|
| 1 | Task ID | table column (grammar §2) |
| 2 | Wave | the wave table the row lives in |
| 3 | Phase | wave-table heading (0–8, MP §16) |
| 4 | Type: `inv` investigation, `imp` implementation, `rev` review, `ver` verification | table column |
| 5 | Scope (1–3 sentences) | table column |
| 6 | Owned files/crates (exhaustive) | table column |
| 7 | Out of scope | standard clause C1 unless the row overrides |
| 8 | Interfaces consumed (must already exist) | table column |
| 9 | Acceptance criteria (checkable) | table column ("AC:") + standard clauses |
| 10 | Tests required | standard clause C2 (+FB/SB bundles) + row specifics |
| 11 | Commands to run | standard clause C3 + row specifics |
| 12 | Depends-on Task IDs | table column |
| 13 | Est. size (S/M/L; L must be split pre-launch) | table column |
| 14 | Report requirements | standard clause C4 |

### 1.4 Standard clauses (inherited by every card unless the row overrides)

- **C1 Out of scope (default):** everything not listed under Owns; in
  particular shared interfaces and root config (R2), other agents' files, CI
  workflows, `docs/feature-matrix.toml` (status changes go through the
  parity-tool outbox, doc 10 §7). Overrides require Lead-granted temporary
  ownership written into the card.
- **C2 Tests required (by type):** `imp` — unit tests in owned code including
  error paths; never weaken or delete a test; every defect fixed gets a
  regression test. `inv` — no code; an evidence file with every claim sourced
  or marked `[VERIFY]`. `rev` — a checklist verdict file. `ver` — reruns the
  documented evidence commands and writes an `evidence/` record per doc 10 §7
  (mock vs real labeled).
- **C3 Commands to run (doc 10 §2, verbatim gate):** `cargo fmt --all --
  --check`; `cargo check --workspace --all-targets --all-features`;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  `cargo test --workspace --all-features`; UI work adds lint, typecheck and
  tests; card specifics add e.g. fuzz runs. Report exact commands and actual
  results; never claim a command ran that did not.
- **C4 Report (≤200 words, AGENTS.md §3.9):** files changed; key decisions with
  `[PROPOSAL]`/`[VERIFY]` markers; tests + actual results; limitations; open
  questions. Plus the mandatory worklog append (AGENTS.md §4). Agents never
  mark their own work `verified` — the parity tool decides from evidence
  (MP §15).
- **C5 Dependency policy:** no new dependency without Lead approval; record
  language, source, transitive graph, build scripts, native links, license,
  provenance first (AGENTS.md §3.2, doc 11 method).
- **C6 Hygiene:** §1.7.
- **C7 Format bundle (`+FB`)** — every format/codec/protocol card additionally
  delivers: conformance pass on its fixture corpus; a registered fuzz target
  `fuzz_<kind>_<id>` with seed corpus and a 10-minute clean release-mode CI
  run (doc 04 §8.5); a filled per-format record `docs/formats/<id>.md` per the
  doc 04 §6 template; a feature-matrix status proposal via the parity-tool
  outbox; differential vs VLC 3.0.23 where lossless-comparable (doc 10 §6);
  `CORPUS.md` license entries for acquired fixtures.
- **C8 Screen bundle (`+SB`)** — every UI card additionally delivers:
  screenshot diff within documented tolerance vs baselines (doc 08 §6); i18n
  keys only (no literal user-visible text); all states (hover, pressed, focus,
  disabled, loading, error, empty) per doc 08 §4; keyboard reachability and
  ARIA labels; controls invoke real commands and reflect real engine state
  with truthful accepted-vs-completed semantics (doc 05 §1); one Playwright
  interaction test.

### 1.5 Review gate between waves (Lead, every wave)

A wave is finished only when, in order: (1) all ≤200-word reports collected;
(2) files verified to exist with no leftover `TODO`/`FIXME`/`CONT` markers
beyond the registered allowlist and no silent truncation; (3) worklog entries
appended in the AGENTS.md §4 format; (4) the C3 gate run green (or failures
triaged into defect cards); (5) cross-doc and cross-crate conflicts resolved —
the Lead edits shared surfaces itself; (6) registration snippets merged into
registry files (R3); (7) one wave-tagged commit (doc 15); (8) the §12 ledger
and per-card statuses updated. **The next wave launches only after step 8.**
Each wave also gets one Lead worklog entry (the *wave log*) recording cards
launched, accepted, rejected, and any §1.6 post-mortems.

### 1.6 Timeout and failure protocol

- **Timeout:** agent produces no report. The Lead inspects partial output:
  structure complete? markers clean? worklog appended? Then either accept the
  partial and relaunch the remainder as a narrower card, or revert. The
  relaunch keeps the Task ID with an `R1`/`R2` note in the wave log (§2).
- **Failure (rejected work):** out-of-scope edits, fabricated results, unsafe
  code, weakened tests, or wrong behavior. The Lead rejects with constructive
  feedback (MP §15), writes a one-line post-mortem into the wave log (what
  failed, why, fix), and relaunches narrower. Two consecutive failures on one
  card → the Lead re-plans it into smaller cards; it escalates to the user
  only if the blocker changes an interface, a policy, or a §2 fixed decision.
- **Blocked cards:** status `blocked` requires reason, investigation done,
  alternatives considered, and a next step (MP §17) — the Phase 5/6
  investigation cards exist precisely to produce these records honestly.

### 1.7 Agent hygiene (binding)

Read `worklog.md` plus only the docs/sections the card names. Write only owned
files. Append the worklog entry. No `git commit` in Phases 0–4 (Lead commits;
doc 15 governs any later branch rights). ≤200-word reports. Never self-verify.
Never edit shared interfaces or root config. Mark `[PROPOSAL]`/`[VERIFY]`.
Clean-room: no VLC source, comments, assets or branding; public specs and
observed behavior only. If you cannot finish, write an `## Unfinished` section
and report it (AGENTS.md §6).

## 2. Task ID grammar

Form: `<phase>.<wave><seq>` — phase `0`–`8`; wave a single letter `A`…`Z`
(then `AA`… if ever needed); seq `1`–`n` within the wave. Examples: `3.B2` =
Phase 3, wave B, task 2; `5.K4-I` = Phase 5, wave K, task 4, investigation.

- **Suffix `-I`**: investigation task (produces evidence/records, no product
  code beyond fixtures/tools the card names).
- **Suffix `-V`**: verification task (runs evidence commands; produces
  `evidence/` records; typically Lead-owned).
- **No suffix**: implementation or review tasks.
- **Relaunches:** same ID, wave log records `R1`, `R2`, … (§1.6).
- **`not-selected`:** program-local status extension (beyond MP §17's five) for
  decision-pair alternatives closed without execution, e.g. the Phase 2
  demuxer variant not chosen. Recorded with a reason. `[PROPOSAL]` for Lead
  ratification.
- This grammar **extends AGENTS.md §5**: the `<phase>-<subsystem>-<n>` form is
  retired in favor of wave-qualified IDs so every card maps to one wave, one
  ledger row, and one review gate. The Lead ratifies this supersession when
  committing this document (worklog note required).
- Card statuses: `not_started`, `in_progress`, `implemented`, `verified`,
  `blocked`, plus `not-selected` above. `verified` is set only by the Lead from
  parity-tool evidence (MP §15/§17), never by the executing agent.

## 3. Phase 0 — Audit (waves 0.A–0.D, 17 tasks)

Gate (MP §16): VLC 3.0.23 pinned; inventory from plugin categories, options,
preferences, menus, hotkeys; feasibility report; ADR-001/002/003 decided.
Detail doc: 02 (§7 defines the same 12-agent breakdown this expands — pinning
is split per platform, threat-model/corpus merged, matrix-freeze and gate made
explicit). Typical parallelism 3–5. All Phase 0 cards are clean-room
investigations of the pinned oracle and public data only.

### Wave 0.A — Baseline pinning (sequential root; everything depends on 0.A4)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 0.A1 | inv | Pin VLC 3.0.23 on Windows per doc 02 §2: official-source acquisition, hashes + signature result, install record, dump-channel smoke (`--version`, `--list`, `--list-plugins`). AC: pin-record section complete; every command recorded; hashes cross-checked vs official sidecar | `docs/baseline/vlc-3.0.23-windows.md`, `docs/baseline/raw/windows/` | doc 02 §2 | Lead pin confirmation | S |
| 0.A2 | inv | Same on Linux (record distro, package vs tarball choice, sandbox used). AC as 0.A1 | `docs/baseline/vlc-3.0.23-linux.md`, `raw/linux/` | doc 02 §2 | — | S |
| 0.A3 | inv | Same on macOS (record arch/universal). AC as 0.A1 | `docs/baseline/vlc-3.0.23-macos.md`, `raw/macos/` | doc 02 §2 | — | S |
| 0.A4 | inv | Consolidate the three pins into the master pin record; validate the oracle invocation contract (doc 02 §2.4) on all 3 OS; write `hardware-template.md`. AC: doc 02 §1.2 G1 evidence complete; platform discrepancies annotated | `docs/baseline/vlc-3.0.23.md`, `vlc-3.0.23.json`, `hardware-template.md` | 0.A1–0.A3 | 0.A1–0.A3 | S |

### Wave 0.B — Inventory extraction (parallel; needs the validated oracle)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 0.B1 | inv | Options & preferences dump on all 3 platforms (doc 02 §3.3): raw `--longhelp`/`--help-verbose` dumps → normalized `options-{os}.json`; three-way reconciliation; mapping to settings-schema seed categories. AC: G4 — normalized records, all diffs annotated | `docs/inventory/options-{windows,linux,macos}.json`, `options-summary.md`, `raw/options/` | 0.A4 | 0.A4 | M |
| 0.B2 | inv | Module/plugin census (doc 02 §3.2) on all 3 platforms; categorize every module into the seven capability categories (demuxers, decoders, access, filters, subtitle, playlist, services-discovery). AC: G3 — per-category counts; platform diffs annotated; unexplained diff = open defect | `docs/inventory/modules-{windows,linux,macos}.json`, `modules-summary.md`, `raw/modules/` | 0.A4 | 0.A4 | M |
| 0.B3 | inv | Menus, dialogs & hotkeys inventory (doc 02 §3.4–3.5) by text transcription only (no screenshots); produce `hotkeys.toml` seed; crosswalk to design-package screens; flag design-vs-VLC accelerator conflicts (doc 08 §1). AC: G6 — complete transcription, per-platform key sequences | `docs/inventory/ui-inventory.md`, `hotkeys.toml` | 0.A4, `design/` | 0.A4 | M |
| 0.B4 | inv | Formats & access inventory (doc 02 §3.6): map census modules to containers/codecs/subtitle/protocol families; probe-verify what the pinned oracle actually plays; usage-share estimates replace doc 04 `[VERIFY]` guesses (doc 04 §8.6 item 7). AC: G5 — every format mapped with probe evidence or explicit not-probe-verified | `docs/inventory/formats.{json,md}`, `raw/probe/` | 0.B2 | 0.B2 | M |

### Wave 0.C — Coverage, feasibility, platform (parallel; may start on public data, finalize vs 0.B4)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 0.C1-I | inv | Video codec coverage: pure-Rust availability verdict per inventoried video codec (doc 04 §4, §8.6 items 2 and 8 — rav1d asm-off proof plan, WebCodecs ADR scope). ADR-001 input. AC: every codec has a verdict with ≥2 sources; H.264/HEVC conclusion explicit | `docs/feasibility/sections/video-codecs.md` + evidence appendix | doc 04, 0.B4 | 0.B4* | M |
| 0.C2-I | inv | Audio + subtitle codec coverage (doc 04 §3, §5; §8.6 items 1, 5, 6 — Symphonia AAC/Opus status, truehdd conformance, oxisynth LGPL). AC: same verdict shape; ASS-rendering gap quantified | `docs/feasibility/sections/{audio-codecs,subtitles}.md` | doc 04, 0.B4 | 0.B4* | M |
| 0.C3-I | inv | Subsystem feasibility: demuxers, network/streaming, audio/video outputs, filters, playlists/services-discovery, transcode (doc 04 §8.6 items 3, 4, 9 — frag-MP4 corpus plan, `mpeg-ps` scope, Kate/FFV1/DVD ADRs). AC: every subsystem gets availability, risk, recommended phase | `docs/feasibility/sections/subsystems.md` | doc 01, doc 03, 0.B4 | 0.B4* | M |
| 0.C4-I | inv | Dependency pre-audit + WASM runtime candidates (ADR-002 evidence pack; doc 04 §8.6 item 10): per-candidate language/source/transitive graph/build scripts/native links/license/provenance; initial allowlist *format* per doc 11 method. AC: candidate table complete; pass/fail per hard requirement; method reproducible | `docs/adr/ADR-002-evidence.md`, `docs/dependencies/audit-method.md`, `docs/dependencies/allowlist-format.md` | docs 06, 09, 11 | — | M |
| 0.C5-I | inv | Platform & hardware capabilities per OS: webview capabilities (Tauri transparency, raw window handles), hw-decode APIs (VideoToolbox/MF/VA-API), audio/GPU OS bindings. ADR-001/003 inputs. AC: each ADR open question answered or explicitly `[VERIFY]`-flagged with follow-up | `docs/inventory/platform-{windows,linux,macos}.md` | doc 01, doc 08 | — | M |

### Wave 0.D — Synthesis, decisions, gate

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 0.D1-I | inv | Security threat-model draft (parsers, plugins, control server, oracle runs — doc 02 §6/0.A11) + corpus plan (fixture mapping per matrix row, provenance/license columns, malformed-generator spec, expectation format — doc 02 0.A12). AC: G12-shape; threat model covers MP §12 surfaces | `docs/inventory/security-phase0.md`, `docs/inventory/corpus-plan.md` | 0.C1–0.C5 drafts | 0.C1–0.C5 | M |
| 0.D2 | imp | Feature-matrix schema freeze + initial population (doc 02 §4): seed rows from 0.B inventories with expected initial statuses; freeze field set + validation rules; manual-checklist v0. AC: schema frozen (changes after this need Lead change-control per doc 02 §4.7); rows validate | `docs/feature-matrix.toml`, `docs/inventory/matrix-seed-notes.md` | 0.B1–0.B4 | 0.B1–0.B4 | M |
| 0.D3 | rev | `[LEAD]` Feasibility report assembly + ADR-001/002/003 drafting and decisions (doc 02 §6 briefs; hw-decode may be conditional software-first with per-API escalation). AC: three decided ADRs with rationale + conditions; feasibility report complete | `docs/feasibility-report.md`, `docs/adr/ADR-{001,002,003}*.md` | all 0.C, 0.D1, 0.D2 | 0.C1–0.C5, 0.D1, 0.D2 | M |
| 0.D4-V | ver | `[LEAD]` Phase 0 gate review vs doc 02 §1.2 exit checklist: every row ticked with evidence links; go/no-go; Phase 1 launch list. AC: exit report exists; no unchecked row | `docs/gates/phase0-exit-report.md` + worklog | all Phase 0 | 0.D3 | S |

## 4. Phase 1 — Foundation (waves 1.A–1.F, 20 tasks)

Gate (MP §16): workspace, product.toml, schemas, lints, CI, native-code audit,
command layer, plugin API, settings-schema generator, token pipeline — all
with tests per doc 10. Waves 1.C/1.D are Lead-owned interface work (R2): at
most one delegated shared-interface card in flight. Parallelism 2–4.

### Wave 1.A — Workspace bootstrap

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 1.A1 | imp | Workspace skeleton: root `Cargo.toml`, `rust-toolchain.toml`, `[workspace.lints]` exactly per doc 10 §3 (rust + clippy deny lists), crate template with `lints.workspace = true`, `.editorconfig`. AC: `cargo check`/`test` green on the template; lints proven inherited by a canary violation test | `Cargo.toml`, `rust-toolchain.toml`, `crates/_template/`, `.editorconfig` | docs 01, 10 | 0.D4-V | S |
| 1.A2 | imp | CI skeleton: GHA fast tier (fmt/check/clippy/test × windows/linux/macos), concurrency groups, actionlint; later-gate job stubs marked non-blocking. AC: green run artifact-linked on the skeleton commit | `.github/workflows/ci.yml` | 1.A1 | 1.A1 | S |
| 1.A3 | imp | xtask v0: `cargo xtask gate` runs the doc 10 §2 command table in order, captures per-command pass/fail, prints the machine-readable honest gate report. AC: gate fails when any command fails; report shape matches doc 10 §2 | `tools/xtask/**` | 1.A1 | 1.A1 | M |

### Wave 1.B — Config spine (parallel; separate crates)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 1.B1 | imp | product-config crate + `product.toml` + `product.schema.json` per doc 07 §2–3 (draft as starting point): load, syntax + semantic validation passes, actionable diagnostics, precedence merge v1 (product defaults + user). AC: validator rejects every doc 07 §3 fixture with field/line diagnostics; rebrand identity fields all present | `crates/product-config/**`, `product.toml`, `product.schema.json` | 1.A1 | 1.A1 | M |
| 1.B2 | imp | settings-schema crate + generators per doc 07 §5: entry format (10 fields), D1–D6 no-duplicate rules, generators → TS types, CLI `--help` text, docs page, runtime validation; `cargo xtask schema-gen` subcommand. AC: generator idempotence tests; duplicate-option fixture fails correctly | `crates/settings-schema/**`, `schemas/settings.toml` | 1.B1 | 1.B1 | M |
| 1.B3 | imp | localization crate per doc 07 §6: key grammar, `locales/en-US.toml` reference seed (Phase-1 keys), fallback chain, missing/duplicate-key + placeholder validation; i18n no-literal ESLint rule with allowlist. AC: the 5 doc 07 §6 CI audit rules enforced by fixtures | `crates/localization/**`, `locales/en-US.toml`, `ui/.eslintrc` stub | 1.A1 | 1.A1 | M |
| 1.B4 | imp | theme-tokens crate + token pipeline per doc 08 §3: tokens source from doc 08 §1 mined values; primitive→alias→semantic layers; generation → CSS variables + Rust constants (`xtask tokens`); 9 accents; light/dark maps. AC: generated outputs committed and diff-tested; semantic-only consumption lint | `crates/theme-tokens/**`, `themes/**`, xtask subcommand | doc 08, 1.A3 | 1.A3 | M |

### Wave 1.C — Core vocabulary `[LEAD]` (sequential)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 1.C1 | imp | `[LEAD]` media-types I per doc 01 §3.1–3.2: identity + timing (SourceUri, Track/Stream/Generation ids, Timestamp/TimeBase with Overflow/Discontinuity/Missing variants, Rate, Duration Known/Unknown). AC: type list complete; overflow/discontinuity unit tests | `crates/media-types/src/{ids,time}.rs` | 1.A1 | 1.A1 | M |
| 1.C2 | imp | `[LEAD]` media-types II per doc 01 §3.3–3.4: Packet/Frame, sample + pixel formats, minimal colorimetry, stage-tagged 7-kind error taxonomy with no-secrets rule. AC: taxonomy covers all stages; no sentinel values | `crates/media-types/src/{packet,frame,error}.rs` | 1.C1 | 1.C1 | M |
| 1.C3 | imp | testkit v0 per doc 03 §8: assert + fixture-path helpers, virtual-time trait stub, evidence-recording helper. AC: dev-dependency only — boundary check proves no production crate depends on it | `crates/testkit/**` | 1.C2 | 1.C2 | S |

### Wave 1.D — Contracts `[LEAD]` (mostly parallel once 1.C2 lands)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 1.D1 | imp | `[LEAD]` codec-api per doc 01 §2.4: Decoder/Encoder traits, capability descriptors, negotiated config, typed decode errors, threading/no-blocking contract in trait docs. AC: doc-sketch conformance; misuse compile-fail test where expressible | `crates/codec-api/**` | 1.C2 | 1.C2 | M |
| 1.D2 | imp | `[LEAD]` plugin-api per doc 06 §1–2: PluginKind enum, manifest types (id, kind, version, capabilities, priority, permissions, config_schema, api_version, dependencies, provenance), host traits (Demuxer, Access, Decoder, Filter, SubtitleParser, PlaylistParser, …). AC: the 4 doc 06 §2 example manifests parse + roundtrip | `crates/plugin-api/**` | 1.C2, 1.D1 | 1.C2, 1.D1 | M |
| 1.D3 | imp | `[LEAD]` commands I per doc 05 §1/§4: envelope (request_id, session, gen), typed results, error taxonomy + redaction, correlation and ordering invariants. AC: stale-generation rules unit-tested per doc 05 §4 | `crates/commands/src/{envelope,error,correlation}.rs` | 1.C2 | 1.C2 | M |
| 1.D4 | imp | `[LEAD]` commands II per doc 05 §2–3: catalog v1 (~65 commands, ~24 events) as data; JSON-schema emission; codegen → TS client + Rust host types. AC: golden-file tests for schema and generated code; catalog/schema drift check fails on mismatch | `crates/commands/src/catalog*`, xtask codegen, `ui/src/generated/` | 1.D3 | 1.D3 | M |

### Wave 1.E — Enforcement, CI, tooling (parallel)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 1.E1 | imp | xtask check-architecture per doc 01 §5: `boundaries.toml`, cargo-metadata edge check, cycle scan, unsafe scan, global-mutable-state scan, testkit-direction check. AC: seeded violations in scratch crates are all caught (fixture-based proof) | `tools/xtask` (subcommand), `boundaries.toml` | 1.A3, crates 1.B–1.D | 1.D4 | M |
| 1.E2 | imp | CI enforcement wiring: boundary-check job, schema-validation job (product + settings + feature-matrix), config-propagation job per doc 07 §9 C1–C5. AC: each job demonstrably fails on a seeded violation (evidence in report) | `.github/workflows/*` | 1.E1, 1.B1, 1.B2 | 1.E1 | S |
| 1.E3 | imp | wasm toolchain CI job per doc 09 §2: wasm32-unknown-unknown build of a canary crate + size-report artifact. AC: job green; budget numbers marked `[PROPOSAL]` pending doc 09 §12 decisions | `.github/workflows/wasm.yml` | 1.A2 | 1.A2 | S |
| 1.E4 | imp | xtask rebrand v0 per doc 07 §8: rewrite identity across crate paths/ids/window titles from `product.toml`; acceptance-test skeleton (Nova fixture: window title, CLI identity, About). AC: rebrand on Nova fixture passes skeleton assertions | `tools/xtask` (subcommand), `tests/rebrand/` skeleton | 1.B1 | 1.B1 | M |
| 1.E5 | imp | Dependency allowlist + native-code audit: `deny.toml` (licenses, advisories, sources), allowlist doc per doc 11 method, cargo-deny CI job. AC: job green on current deps; adding a native-linked crate fails it (fixture proof) | `deny.toml`, `docs/dependencies/allowlist.md`, CI job | 0.C4-I method, 1.A2 | 1.A2 | S |

### Wave 1.F — Gate

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 1.F1-V | ver | `[LEAD]` Phase 1 gate: full local gate + 3-OS CI green including all new jobs; ledger + statuses updated; Phase 2 launch list. AC: evidence record links each green run | `docs/gates/phase1-exit.md` + worklog | all Phase 1 | all | S |

## 5. Phase 2 — First vertical (waves 2.A–2.E, 17 tasks)

Gate (MP §16): one real file from open to decode to audio and video on screen,
through the command layer, from CLI and from the UI; includes the
video-in-webview prototype and the Figma token proof (token proof landed in
1.B4; prototype is 2.D3). **`[DECISION-PENDING]`**: the first-vertical
container pair — doc 03 §7 recommends AVI+MJPEG+PCM (self-build, zero new
dependencies); doc 04 §8.1 recommends MP4+PCM+MJPEG (symphonia-isomp4, real
sample tables). Both cards are written; **exactly one executes** (Lead decides
at wave-2.B launch); the other closes `not-selected`. Fixture card 2.A3
generates both so the decision blocks nothing. Parallelism 2–4.

### Wave 2.A — Input, probe, fixtures

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 2.A1 | imp | input crate per doc 03 §4.1: SourceUri resolution, file access, buffered reader with bounded memory, Seekable trait, position/length, typed errors (inaccessible, limit). AC: read/seek/EOF + truncation tests; no unbounded buffering | `crates/input/**` | media-types, plugin-api | 1.F1-V | M |
| 2.A2 | imp | media-probe crate per doc 03 §4.2: signature scoring over a bounded probe window, ProbeReport (format id, confidence, track hints). AC: probe fixtures (both container variants + garbage + truncated) classified; probe-depth bound tested | `crates/media-probe/**` | input, media-types, plugin-api | 2.A1 | M |
| 2.A3 | imp | Demo fixture generation (`xtask fixtures`): deterministic generators for AVI+MJPEG+PCM **and** MP4+MJPEG+PCM (decision-independent) + malformed/truncated derivatives; CC0, `CORPUS.md` entries. AC: fixtures byte-stable across runs (hash check) | `tools/xtask` (subcommand), `tests/fixtures/**` | 1.A3 | 1.A3 | M |

### Wave 2.B — Demux + first codecs (queue independent; demuxer pair decided)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 2.B1 | imp | packet-queue crate per doc 03 §4.4: bounded audio/video queues, backpressure signals, generation IDs with stale-drop. AC: producer/consumer tests incl. overflow backpressure and stale-generation drop proof | `crates/packet-queue/**` | media-types | 1.F1-V | M |
| 2.B2 | imp | `[DECISION-PENDING — variant A]` AVI demuxer (doc 03 §7): RIFF chunk walk, hdrl/strl parsing, idx1 index, MJPEG + PCM track extraction, seek-to-time. AC: generated AVI fixtures demux fully; `+FB`. Closes `not-selected` if the Lead picks MP4 | `crates/demux/src/avi/**` | input, plugin-api, media-types | 2.A1, 2.A3 | M |
| 2.B3 | imp | `[DECISION-PENDING — variant B]` MP4 demuxer via symphonia-isomp4 (or mp4parse `[VERIFY]`) behind the Demuxer trait: track boxes, stco/stsc sample tables, seek. AC: generated MP4 fixtures demux fully; `+FB`. Closes `not-selected` if the Lead picks AVI | `crates/demux/src/mp4/**` | input, plugin-api, media-types | 2.A1, 2.A3 | M |
| 2.B4 | imp | MJPEG decoder behind codec-api (jpeg-decoder or zune-jpeg — purity + output format `[VERIFY-P0]`). AC: decode fixtures match reference output hashes; `+FB` (fuzz_codec_mjpeg) | `crates/codecs/src/video/mjpeg/**` | codec-api, media-types | 2.A3 | M |
| 2.B5 | imp | PCM decoder (symphonia-codec-pcm or self-build `[VERIFY]`): s16/s24/s32/f32, channel layouts. AC: bit-exact output vs fixtures; `+FB` | `crates/codecs/src/audio/pcm/**` | codec-api, media-types | 2.A3 | S |

### Wave 2.C — Engine core (parallel on separate crates)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 2.C1 | imp | media-clock v0 per doc 03 §4.7: audio-master clock via sample counter, MediaTime mapping, pause/drift primitives. AC: monotonicity, pause, and rate tests on virtual time | `crates/media-clock/**` | media-types | 1.F1-V | M |
| 2.C2 | imp | playback-engine minimal state machine per doc 03 §4.6 + doc 05 §3: Idle→Opening→Buffering→Playing→Paused→Ended→Stopped + error states, session epochs, pipeline wiring demux→queues→decoders→sinks (seek/sync deliberately excluded — Phase 3). AC: transition-table tests; no invalid transition reachable | `crates/playback-engine/**` | media-types, packet-queue, codec-api, commands, media-clock | 2.B1, 2.C1 | M |
| 2.C3 | imp | Audio path v0: audio-engine (volume, channel map, straight mix) + audio-output (cpal `[VERIFY-P0]` purity; fallback: platform adapters per doc 01 §2.7) with the no-blocking callback contract and underrun counter. AC: sine fixture sink-buffer hash test; callback-budget test | `crates/audio-engine/**`, `crates/audio-output/**` | media-types, media-clock, plugin-api | 2.C1 | M |
| 2.C4 | imp | video-engine presentation v0 per doc 03 §4.10: frame scheduler (PTS vs clock), late-frame drop policy, presentation queue. AC: virtual-clock scheduling tests (in-order, drop-late) | `crates/video-engine/**` | media-types, media-clock | 2.C1 | M |

### Wave 2.D — Presentation, CLI, webview prototype

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 2.D1 | imp | renderer-wgpu v0: surface creation, RGBA8 upload (measured staging), sRGB present, vsync. AC: headless render of a synthetic frame produces the expected PNG hash | `crates/renderer-wgpu/**` | media-types, video-engine | 2.C4 | M |
| 2.D2 | imp | apps/cli: open/play/pause/seek/stop/info via the commands crate; stage-tagged honest error printing; `--help` generated from settings-schema. AC: scripted CLI session passes (open fixture → play → pause → seek → stop) incl. the unsupported-file error path | `apps/cli/**` | commands, playback-engine, demux + codecs impls | 2.C2, 2.B4, 2.B5 | M |
| 2.D3 | imp | Video-in-webview prototype per doc 08 §7 (ADR-003 input): variant B first (frame streaming to canvas via a minimal `apps/desktop` Tauri skeleton); variant A (native wgpu surface behind a transparent webview) only if B fails gates; test WebView2, WKWebView, WebKitGTK separately. AC: measured latency + compatibility matrix recorded; honest recommendation written for ADR-003 | `apps/desktop/**` (skeleton), `docs/prototypes/video-in-webview.md` | renderer-wgpu, video-engine, commands | 2.D1 | M |

### Wave 2.E — Integration and gate

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 2.E1 | imp | End-to-end integration test: open generated fixture → demux → decode → audio sink (null + real where hardware exists, labeled) + video render (headless) **through the command layer**; evidence per doc 10 §7. AC: green on 3 OS in CI; mock vs real evidence labeled | `tests/integration/first-vertical/**` | all Phase 2 impls | 2.D1, 2.D2 | M |
| 2.E2-V | ver | `[LEAD]` Phase 2 gate: first vertical plays on all 3 OS; webview prototype verdict recorded → ADR-003 finalized; evidence + ledger; Phase 3 launch list | `docs/gates/phase2-exit.md` + worklog | all Phase 2 | 2.E1 | S |

## 6. Phase 3 — Reliable engine (waves 3.A–3.F, 19 tasks)

Gate (MP §16): clocks, queues, generations, seeking, source replacement, A/V
sync, recovery. Detail doc: 03 (§5 generations, §6 seek flow, §8 testkit and
the 16-scenario mapping in doc 10 §6). No new formats by design (doc 04 §8.2).
Parallelism 2–4.

### Wave 3.A — Seek and cancellation

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 3.A1 | imp | Seek per doc 03 §6: the 7-step flow (pause ingestion → generation flush → demux re-anchor → pre-roll → resume → report) incl. the failed-seek contract D4 (rollback to prior position, typed error, never fake success). AC: boundary seeks (0, end, past-end, unseekable) tested; stale packets proven not to reappear | `crates/playback-engine/src/seek.rs`, demux-impl seek hooks, packet-queue flush API | playback-engine, demux impl, packet-queue | 2.E2-V | M |
| 3.A2 | imp | Source replacement: play-new-while-playing, session epoch bump, teardown ordering, stale decoder-output drain. AC: 50 rapid switches with no leak (bounded memory asserted) and no stale frames | `crates/playback-engine/src/session.rs` | 3.A1 | 3.A1 | M |
| 3.A3 | imp | Track switching mid-play: A/V/S selection via commands, decoder instance recycling, per-track queue drain, subtitle swap without restart. AC: switch tests on multi-track fixtures with event-sequence assertions | `crates/playback-engine/src/tracks.rs` | 3.A1 | 3.A1 | M |

### Wave 3.B — Sync and queues

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 3.B1 | imp | A/V sync hardening per doc 03 §4.7/§4.10: drift measurement (audio-master), correction policy (bounded video drop/repeat, bounded audio resample), discontinuity handling. AC: synthetic drift scenarios converge; no event storms | `crates/media-clock/**`, `crates/video-engine/**`, `crates/audio-engine/**` | 3.A1 | 3.A1 | M |
| 3.B2 | imp | Clock backends: video-master and external-master modes, master-loss fallback policy, Ended-state seek-back. AC: master-switch tests; discontinuity unit tests | `crates/media-clock/src/backends.rs` | 3.B1 | 3.B1 | S |
| 3.B3 | imp | Queue backpressure tuning: per-kind limits from config, stall/resume thresholds, starvation metrics events. AC: property tests with synthetic producers/consumers — no deadlock, bounded memory under burst (doc 03 §8) | `crates/packet-queue/**`, playback-engine wiring | 3.A2 | 3.A2 | M |
| 3.B4 | imp | Event completeness per doc 05 §3: all ~24 catalog events emitted with correct payloads; ordering invariants (doc 05 §4); throttling/coalescing policy. AC: the doc 05 §7 contract suite passes against the engine | `crates/playback-engine/src/events.rs`, `crates/commands` test-side only | commands catalog | 3.A3, 1.D4 | 3.A3 | M |

### Wave 3.C — State machine and recovery

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 3.C1 | imp | State-machine completeness: every doc 05 §3 transition incl. error re-entry, Buffering re-entry, stop-during-opening, reopen cycles. AC: model-based exhaustive transition-table test | `crates/playback-engine/src/state.rs` | 3.B4 | 3.B4 | M |
| 3.C2 | imp | Recovery — media level: corrupt stream (bounded skip-and-report), truncated file (clean Ended + warning), missing file, unsupported format (known-format info in error; never fabricated frames/silence/duration). AC: malformed-corpus scenario tests; regression test per defect | demux impls, codecs, playback-engine error paths | 3.C1 | 3.C1 | M |
| 3.C3 | imp | Recovery — environment: interrupted network source (retry window → typed network error), audio device loss (suspend + reopen policy per doc 03 proposal), renderer surface loss (recreate). AC: fault-injection tests via testkit synthetic sources | `crates/playback-engine/src/recovery.rs`, audio-output, renderer hooks | 3.C1 | 3.C1 | M |

### Wave 3.D — testkit v1 and property suites

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 3.D1 | imp | testkit v1: virtual clock (deterministic time control), synthetic media sources with scripted packet timelines, synthetic decoders. AC: engine tests rewritten on the virtual clock run deterministic and <10 s | `crates/testkit/**` | 3.C1 | 3.C1 | M |
| 3.D2 | imp | testkit fault injection: flaky source (drop/delay/corrupt at offsets), device-loss simulator, clock-jump injector. AC: 3.C2/3.C3 scenarios run deterministically on the kit | `crates/testkit/src/faults.rs` | 3.D1 | 3.D1 | S |
| 3.D3 | imp | Property suites per doc 03 §8: timestamp arithmetic (overflow/discontinuity), queue invariants, generation monotonicity, seek roundtrips. AC: suites in CI; every shrunk counterexample committed as a regression | `tests/property/**` | 3.D1, 3.D2 | 3.D2 | M |

### Wave 3.E — Scenarios, differential, soak

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 3.E1 | imp | Scenario suite part 1 — doc 10 §6 scenarios 1–8 (open, play, pause, seek, stop, repeated seeks, A/V sync, track switching) with pass criteria + evidence artifacts. AC: green in CI on 3 OS | `tests/integration/scenarios/01-08/**` | 3.D3 | 3.D3 | M |
| 3.E2 | imp | Scenario suite part 2 — scenarios 9–16 (corrupt, missing, interrupted network, device loss, long, short, reopen cycles, exit during playback). AC: green; exit-during-playback leaves no temp state | `tests/integration/scenarios/09-16/**` | 3.E1 | 3.E1 | M |
| 3.E3 | imp | Differential-vs-VLC harness v1 per doc 10 §6: run pinned VLC 3.0.23 externally on corpus; compare decoded-PCM MD5 for lossless paths; non-comparable list recorded; mock runs separated from real-oracle runs. AC: per-fixture report produced | `tools/bench-vs-vlc/**`, differential tests | 0.A4 oracle, 3.E1 | 3.E1 | M |
| 3.E4 | imp | Soak harness + first run: 4 h loop (open/play/seek/switch/reopen), memory-slope measurement (<5 MB/h target `[PROPOSAL]`), leak triage list. AC: harness wired to nightly tier; first report committed | `tests/soak/**` | 3.E2 | 3.E2 | M |

### Wave 3.F — Baseline and gate

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 3.F1 | imp | Perf baseline capture per doc 12: startup, probe latency, time-to-first-frame/sound, seek latency, CPU/GPU, peak memory, dropped frames, underruns on the Phase-2 format set, with hardware manifest. AC: baseline numbers committed (no targets claimed yet) | `benches/**`, `docs/perf/baseline.md` | 3.E3 | 3.E3 | M |
| 3.F2-V | ver | `[LEAD]` Phase 3 gate: 16/16 scenarios green on 3 OS; soak slope within target; differential harness operational. AC: evidence record + ledger + Phase 4 launch list | `docs/gates/phase3-exit.md` + worklog | all Phase 3 | 3.F1 | S |

## 7. Phase 4 — Desktop product (waves 4.A–4.I, 25 tasks)

Gate (MP §16): Figma-exact UI, layouts from config, themes, localization,
playlists, settings, plugins (builtin side), rebrand test. Detail docs: 08
(components, states, screenshot harness), 07 (settings, l10n, rebrand), 05
§8 (keymaps/menus as data). `+SB` bundle applies to every UI card. Parallelism
3–5 (UI cards own disjoint component/screen zones; the shared primitives
registry is Lead-merged per R3).

### Wave 4.A — UI foundation

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 4.A1 | imp | ui/ workspace scaffold per doc 08 §8: Vite + React + TS strict, vitest + RTL, Playwright, ESLint (i18n-no-literals, boundary imports); generated command client wired from 1.D4 codegen + mock transport; routing skeleton for design screens. AC: lint + typecheck + tests green in a CI UI job | `ui/**` (scaffold) | commands codegen | 3.F2-V | M |
| 4.A2 | imp | Token pipeline wiring per doc 08 §3: components consume semantic tokens only (lint-enforced); light/dark + 9 accents switching; CSS variables + Rust constants generated. AC: token snapshot tests; theme-switch E2E | `ui/src/tokens/**`, `themes/**` consumption | 1.B4, 4.A1 | 4.A1 | M |

### Wave 4.B — Primitive library (per doc 08 §4; disjoint zones)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 4.B1 | imp | Primitives: Button, Toggle, IconButton + form inputs (Text, Number, Checkbox, Radio, Switch) with all states (hover/pressed/focus/disabled/loading/error). AC: `+SB`; per-component RTL tests + visual snapshots | `ui/src/components/{button,form}/**` | 4.A2 | 4.A2 | M |
| 4.B2 | imp | Primitives: Slider (scrub, keyboard steps, live-vs-committed), Seekbar (buffer ranges, 8×24 knob per design), Volume. AC: `+SB`; pointer + keyboard interaction tests; truthful pending-seek states per doc 08 §5 | `ui/src/components/slider/**` | 4.B1 | 4.B1 | M |
| 4.B3 | imp | Primitives: Dropdown/Select, MenuBar, ContextMenu, Tooltip (accelerators from keymap data, submenus, focus trap). AC: `+SB`; keyboard-nav + ARIA tests | `ui/src/components/menu/**` | 4.B1 | 4.B1 | M |
| 4.B4 | imp | Primitives: Dialog/Sheet, Tabs, ScrollArea, virtualized List/Table, icon system (1.5-stroke set, state variants). AC: `+SB`; virtualization perf test at 10k rows | `ui/src/components/{surface,list,icon}/**` | 4.B1 | 4.B1 | M |

### Wave 4.C — Player surface

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 4.C1 | imp | Player screen shell: video canvas area, chrome show/auto-hide timing per design, fullscreen + ESC, empty state; panel/menu structure from layout config (doc 07 §7). AC: `+SB` | `ui/src/screens/player/**`, `config/layouts/` consumption | 4.B4 | 4.B4 | M |
| 4.C2 | imp | Control bar: transport (play/pause/stop with engine-state mapping per doc 08 §5), seekbar (accepted vs completed), volume/mute, track menus (A/V/S from real track lists). AC: `+SB`; every control invokes a real command, reflects real state, surfaces real errors | `ui/src/screens/player/controls/**` | 4.C1 | 4.C1 | M |
| 4.C3 | imp | Playback UX states: loading, buffering (progress), unsupported (format info shown), error (stage-tagged, actionable), first-run empty. AC: `+SB`; state-matrix component tests; no fabricated success | `ui/src/screens/player/states/**` | 4.C1 | 4.C1 | S |

### Wave 4.D — Playlist and media info

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 4.D1 | imp | playlists crate v0 (M3U read/write only) + Playlist panel + queue semantics (play-next/play-later, shuffle/repeat, drag reorder). AC: M3U roundtrip + fuzz-lite; queue behavior matches the command catalog; `+SB` | `crates/playlists/src/m3u/**`, `ui/src/screens/playlist/**` | 4.C2 | 4.C2 | M |
| 4.D2 | imp | Media Info screen per design: streams, codecs, metadata from media-info commands; unknowns shown as "Unknown", never invented. AC: `+SB`; fixture-driven rendering tests | `ui/src/screens/media-info/**` | 4.C2 | 4.C2 | S |

### Wave 4.E — Settings (framework + 3 area groups covering the 13–14 design areas)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 4.E1 | imp | Settings framework per doc 07 §5: screens generated from settings-schema (TS generator from 1.B2), search, dirty-state + apply/revert, restart-required badges, reset-to-default. AC: generated-screen snapshot test detects schema drift | `ui/src/screens/settings/**` (framework) | 1.B2, 4.B1 | 4.B4 | M |
| 4.E2 | imp | Settings group 1: Interface, Appearance (theme/accent), Language, Hotkeys editor (hotkeys.toml data + command IDs; conflict detection). AC: `+SB`; every option bound to a real setting key | settings screens (group 1 modules) | 4.E1 | 4.E1 | M |
| 4.E3 | imp | Settings group 2: Audio (device, exclusivity), Video (display, deinterlace), Subtitles/OSD. AC: `+SB`; config-never-invents — absent capabilities render disabled with reason | settings screens (group 2 modules) | 4.E1 | 4.E1 | M |
| 4.E4 | imp | Settings group 3: Input & Codecs (cache, hw-decode toggle per ADR-001 status), Playlist & services discovery, Privacy & Network, Updates/About/Advanced. AC: `+SB`; feature-exposure gating per product.toml F1/F2 rules | settings screens (group 3 modules) | 4.E1 | 4.E1 | M |

### Wave 4.F — Secondary surfaces and a11y

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 4.F1 | imp | Onboarding flow per design screens: first-run, privacy defaults opt-in-off, skippable, state persisted. AC: `+SB`; no telemetry default-on | `ui/src/screens/onboarding/**` | 4.E2 | 4.E2 | S |
| 4.F2 | imp | Player variants: Miniplayer + Music mode per design exports; variant switching via command/layout. AC: `+SB` | `ui/src/screens/player/variants/**` | 4.C2 | 4.C2 | M |
| 4.F3 | imp | Customize panels per design: Crop/Aspect, Video Colors, Essential, HDR — fields present only when the engine supports them; no dead controls. AC: `+SB`; unsupported filter fields hidden per feature exposure | `ui/src/screens/player/customize/**` | 4.C2 | 4.C2 | M |
| 4.F4 | imp | Keyboard navigation + focus + a11y pass across all screens: tab order, focus rings from tokens, ARIA labels from i18n keys, shortcuts from keymaps data. AC: axe/Playwright a11y suite green on every screen | `ui/**` (pass, no new screens) | all 4.* screens | 4.F1–4.F3 | M |

### Wave 4.G — Fidelity harness

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 4.G1 | imp | Screenshot-diff harness per doc 08 §6: `xtask visual` renders screens at defined viewports/DPI, diffs vs baselines within documented tolerance; per-OS baseline sets; flakiness controls. AC: harness detects a seeded 2-px regression; PR-lite + nightly modes | `tools/xtask` (subcommand), `tests/visual/**` | 4.* screens | 4.F4 | M |
| 4.G2 | imp | Baseline capture + first full matrix run: all screens × 3 OS × DPI set; drive remediation until within tolerance (or documented OS-text exceptions per doc 08 §6). AC: full-matrix green | `tests/visual/baselines/**` | 4.G1 | 4.G1 | M |

### Wave 4.H — Shell, rebrand, localization

| ID | Type | Scope | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 4.H1 | imp | Tauri shell packaging: apps/desktop composition root (engine wiring, single `lumen_dispatch` per doc 05 §5.1), window config from product.toml, drag-and-drop file open, native file dialogs. AC: dev + release builds launch and play a fixture on 3 OS (CI launch test per doc 10) | `apps/desktop/**` | all | 4.G2 | M |
| 4.H2 | imp | Rebrand acceptance test per doc 07 §8: Nova fixture → rebrand → window title, installer identity, About, CLI identity, logs, docs all reflect Nova. AC: automated assertions pass — the MP §5 acceptance proof | `tools/xtask` (rebrand), `tests/rebrand/**` | 1.E4, 4.H1 | 4.H1 | M |
| 4.H3 | imp | Localization completion: en-US 100 % key coverage (no missing/duplicate; placeholder validation), pseudo-locale locale proving no clipping/hardcoding, generated docs + CLI `--help` regenerated from schema. AC: CI l10n rules green; pseudo-locale screenshots within layout tolerance | `locales/**`, ui l10n wiring | 4.F4 | 4.F4 | M |

### Wave 4.I — Gate

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 4.I1-V | ver | `[LEAD]` Phase 4 gate: visual matrix within tolerance; rebrand test green; config-propagation proof; l10n rules green; packaging launch tests on 3 OS. AC: evidence + ledger + Phase 5 launch list | `docs/gates/phase4-exit.md` + worklog | all Phase 4 | 4.H2, 4.H3 | S |

## 8. Phase 5 — Breadth (waves 5.A–5.R, 45 tasks — the longest phase)

Gate (MP §16): containers, audio and video codecs, subtitles, filters, one
verified increment at a time. Sequencing follows doc 04 §8.3 exactly (P0
first). Rules for this phase: (1) every format card carries the `+FB` bundle;
(2) per-format ownership zones + registry snippets per R3 — several format
cards run in parallel per wave; (3) `[CONDITIONAL]` cards launch only if the
named decision/ADR landed; (4) `-I` cards produce honest records and
decisions, not code — blocked outcomes are planned, not hidden (MP §14).
Parallelism 2–5.

### Waves 5.A–5.B — Lossless audio core and Ogg family

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.A1 | imp | FLAC decoder (symphonia-claxon or claxon `[VERIFY purity]`) with SEEKTABLE seeking + binary-search fallback. AC: `+FB`; bit-exact vs reference decoder; differential vs VLC | `crates/codecs/src/audio/flac/**` | codec-api | 4.I1-V | M |
| 5.A2 | imp | WAV + AIFF native: full RIFF/WAVE variants (extensible, oddities `[VERIFY]`) + AIFF COMM/SSND. AC: `+FB`; truncated-header error paths tested | `crates/demux/src/{wav,aiff}/**` | demux plumbing | 4.I1-V | M |
| 5.A3 | imp | MP3 decoder (symphonia) + ID3v2/VBRI/Xing metadata + gapless handling. AC: `+FB`; conformance corpus; differential vs VLC (PCM MD5); honest VBR-seek accuracy note in record | `crates/codecs/src/audio/mp3/**` | codec-api | 4.I1-V | M |
| 5.B1 | imp | Ogg container polish: physical/logical stream mapping, chained streams, granule discontinuities. AC: `+FB` | `crates/demux/src/ogg/**` | demux plumbing | 4.I1-V | M |
| 5.B2 | imp | Vorbis decoder (symphonia or lewton `[VERIFY]`) + Vorbis comments. AC: `+FB`; differential vs VLC on lossless-comparable corpus | `crates/codecs/src/audio/vorbis/**` | codec-api, 5.B1 | 5.B1 | M |

### Waves 5.C–5.D — MP4 family and Matroska

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.C1 | imp | MP4/MOV container full: fragmented MP4 (moof/trun — doc 04 §8.6 item 3 corpus pass), chapter refs, edts/elst, 3gp quirks. AC: `+FB`; frag-MP4 + MOV corpora demux + seek | `crates/demux/src/mp4/**` (extension) | 2.B3 outcome | 4.I1-V | M |
| 5.C2 | imp | AAC-LC decoder (symphonia — coverage per doc 04 §8.6 item 1) + ADTS/ADIF raw access. HE-AAC honestly recorded as partial/absent. AC: `+FB`; conformance vectors pass for LC | `crates/codecs/src/audio/aac/**`, `crates/demux/src/adts/**` | codec-api | 5.C1 | M |
| 5.D1 | imp | Matroska/WebM demux full: EBML, cues-based seeking, block lacing, multi-track; WebM payload set minus VP9 (recorded). AC: `+FB` | `crates/demux/src/mkv/**` | demux plumbing | 4.I1-V | M |
| 5.D2 | imp | Matroska extras + records: attachments, chapters, editions, tags; conformance corpus; differential vs VLC. AC: `+FB` complete record | `crates/demux/src/mkv/**` (extras), `docs/formats/container-matroska.md` | 5.D1 | 5.D1 | M |

### Wave 5.E — AV1 flagship

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.E1 | imp | AV1 decode via rav1d with the asm feature OFF per doc 04 §4.3: purity proof (no .S/asm compiled), integration behind codec-api, IVF/Ogg delivery plumbing. AC: `+FB`; purity evidence in record | `crates/codecs/src/video/av1/**` | codec-api | 4.I1-V | M |
| 5.E2 | imp | AV1 conformance + differential + record: full conformance run, differential vs VLC where comparable, speed-honesty report (asm-off cost measured per doc 12). AC: `+FB` record complete | `docs/formats/video-av1.md`, conformance tests | 5.E1 | 5.E1 | M |

### Waves 5.F–5.G — H.264 and HEVC (hw-first per ADR-001; adapters land in 7.B)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.F1 | imp | H.264/AVC bitstream parser (self-build, clean-room from spec): NAL units, SPS/PPS, slice headers, SEI — sufficient for probing, track info, hw-feed, and honest errors; no software-decode claim. AC: `+FB` (parser fuzz); conformance streams parse with field-level assertions | `crates/codecs/src/video/h264/parser/**` | media-types, codec-api | 4.I1-V | M |
| 5.F2 | imp | `[CONDITIONAL on ADR-001]` H.264 decode-path integration: hw-decode trait consumption point, parser→adapter plumbing; fallback = typed unsupported-on-platform error + truthful matrix row. AC: `+FB`; adapters plug in at 7.B1–7.B3 | `crates/codecs/src/video/h264/**`, `docs/formats/video-h264.md` | 5.F1, ADR-001 | 5.F1 | M |
| 5.G1 | imp | HEVC bitstream parser (same clean-room pattern: VPS/SPS/PPS, NAL). AC: `+FB` (parser fuzz) | `crates/codecs/src/video/hevc/parser/**` | 5.F1 | 5.F1 | M |
| 5.G2 | imp | `[CONDITIONAL: executes only if the H.264 hw path landed]` HEVC decode-path integration + record, mirroring 5.F2 | `crates/codecs/src/video/hevc/**`, `docs/formats/video-hevc.md` | 5.G1, 5.F2 | 5.G1 | M |

### Wave 5.H — MPEG video

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.H1 | imp | MPEG-1/2 video decoder: assess pure-Rust availability `[VERIFY-P0]`; implement, or record `blocked` with investigation evidence. AC: `+FB` either way | `crates/codecs/src/video/mpeg12/**` | codec-api | 4.I1-V | M |
| 5.H2-I | inv | MPEG-4 ASP (DivX/Xvid) decision: pure-Rust availability, self-build effort (L → split proposal for a later wave), patent note. AC: verdict + plan or honest blocked record | `docs/formats/video-mpeg4asp.md` + matrix proposal | doc 04 | — | S |
| 5.H3 | imp | MJPEG polish: AVI1.1 field handling, 4:2:2/4:2:0 edge cases; record to `verified` incl. differential. AC: `+FB` | `crates/codecs/src/video/mjpeg/**` | 2.B4 | 2.B4 | M |

### Wave 5.I — MPEG systems containers

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.I1 | imp | MPEG-TS demux (mpeg2ts-reader `[VERIFY]` or self-build): PSI/SI (PAT/PMT/SDT), PES timing, discontinuities, seek by timing scan. AC: `+FB`; multi-program + corrupted-TS corpora | `crates/demux/src/ts/**` | demux plumbing | 4.I1-V | M |
| 5.I2 | imp | MPEG-PS/VOB demux (mpeg-ps crate — scope per doc 04 §8.6 item 4): packs/system headers, DVD-VOB quirks; AC-3/DTS/LPCM private streams exposed (decode pending 5.K4-I). AC: `+FB` | `crates/demux/src/ps/**` | demux plumbing | 5.I1 | M |
| 5.I3 | imp | Elementary-stream wrappers: raw .264/.265/.mp3/.aac access + IVF (AV1). AC: `+FB` | `crates/demux/src/{es,ivf}/**` | 5.F1, 5.A3 | 5.F1 | S |

### Wave 5.J — AVI family and audio breadth

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.J1 | imp | AVI full: if MP4 won the Phase-2 decision, implement AVI now from the 2.B2 card; else polish (OpenDML, interleaving edge cases). AC: `+FB` | `crates/demux/src/avi/**` | Phase-2 decision record | 4.I1-V | M |
| 5.J2 | imp | ADPCM breadth: MS-ADPCM, IMA, Yamaha, OKI/Dialogic variants (set per doc 04 `[VERIFY]`). AC: `+FB` | `crates/codecs/src/audio/adpcm/**` | codec-api | 4.I1-V | M |
| 5.J3 | imp | ALAC decoder (`[VERIFY]` pure) + MP2 decoder. AC: `+FB`; differential for ALAC (lossless) | `crates/codecs/src/audio/{alac,mp2}/**` | codec-api | 4.I1-V | M |

### Wave 5.K — Audio tail: hardening + decisions

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.K1 | imp | TrueHD hardening (truehdd crate): conformance vs reference streams (doc 04 §8.6 item 5), honest downmix policy. AC: `+FB` | `crates/codecs/src/audio/truehd/**` | codec-api | 4.I1-V | M |
| 5.K2-I | inv | Opus decision: symphonia-codec-opus publication status (doc 04 §8.6 item 1); contribute-vs-self-build-vs-blocked verdict + plan | `docs/formats/audio-opus.md` | doc 04 | — | S |
| 5.K3-I | inv | ASF/WMA decision: container feasibility, WMA decode availability (likely blocked); verdict + records | `docs/formats/{container-asf,audio-wma}.md` | doc 04 | — | S |
| 5.K4-I | inv | AC-3 + DTS decision: passthrough policy, self-build effort (L), OS-decoder option per the ADR-001 analog; verdict + plan or blocked records | `docs/formats/audio-{ac3,dts}.md` | doc 04, ADR-001 | — | S |

### Wave 5.L — Subtitle pipeline foundation

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.L1 | imp | Subtitle overlay pipeline: compositor contract on video-engine/renderer (safe margins, scaling with video, z-order), timing on the shared clock. AC: synthetic cue render tests (position, duration, overlap) | `crates/subtitles/src/compositing/**`, video-engine + renderer hooks | 4.I1-V | 4.I1-V | M |
| 5.L2 | imp | Cue model: shared cue types, positioning/alignment primitives, text-style tokens (subset `[PROPOSAL]`). AC: roundtrip + timing edge cases (zero-duration, overlap, out-of-range) | `crates/subtitles/src/model/**` | 5.L1 | 5.L1 | S |

### Wave 5.M — Subtitle formats, wave 1

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.M1 | imp | SRT parser (encoding detection incl. UTF-16, malformed tolerance) + render path. AC: `+FB` (fuzz_sub_srt) | `crates/subtitles/src/srt/**` | 5.L2 | 5.L2 | S |
| 5.M2 | imp | ASS/SSA parser — plain-text subset per the doc 04 §5 decision (styling deferred, recorded). AC: `+FB` | `crates/subtitles/src/ass/**` | 5.L2 | 5.L2 | M |
| 5.M3 | imp | WebVTT parser (cues, settings subset, NOTE/STYLE blocks). AC: `+FB` | `crates/subtitles/src/vtt/**` | 5.L2 | 5.L2 | M |

### Wave 5.N — Subtitle formats, wave 2 `[CONDITIONAL — bitmap/CC decision]`

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.N1 | imp | `[CONDITIONAL]` PGS (HDMV) parse + palette render. AC: `+FB` | `crates/subtitles/src/pgs/**` | 5.L1 | 5.M1 | M |
| 5.N2 | imp | `[CONDITIONAL]` DVB subtitles + VobSub (IDX/SUB) parse + render. AC: `+FB` | `crates/subtitles/src/{dvb,vobsub}/**` | 5.N1 | 5.N1 | M |
| 5.N3 | imp | `[CONDITIONAL]` CC 608/708 extraction (from VBI/user-data/SEI) + render. AC: `+FB` | `crates/subtitles/src/cc/**` | 5.F1 | 5.F1 | M |

### Wave 5.O — Playlist parsers

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.O1 | imp | M3U8 attributes + M3U polish (from 4.D1); PLS + XSPF parsers. AC: `+FB` (fuzz per parser) | `crates/playlists/src/{m3u,pls,xspf}/**` | 4.D1 | 4.D1 | M |
| 5.O2 | imp | CUE sheets: parse (REM/INDEX/gaps), album-oriented gapless handoff semantics. AC: `+FB` | `crates/playlists/src/cue/**` | 5.O1 | 5.O1 | M |
| 5.O3 | imp | Playlist probing + directory loading: playlist-vs-media detection, recursive directory enumeration behind the access trait. AC: records updated; bounded enumeration | `crates/playlists/src/probe.rs`, input hooks | 5.O1 | 5.O1 | S |

### Wave 5.P — Video tail decisions (honest records)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.P1-I | inv | VP8/VP9 decision: no pure-Rust decoder exists (doc 04 §4.5); options = blocked record vs WebCodecs-analog (web column) vs L self-build; verdict | `docs/formats/video-vp8-vp9.md` | doc 04, doc 09 | — | S |
| 5.P2-I | inv | Theora decision (self-build L vs usage share from Phase 0) + blocked/deferred records for WMV/VC-1, ProRes/DNxHD, screen codecs, Kate, FFV1, DVD-Video (ADR list), MIDI `[PROPOSAL]` (oxisynth LGPL review, rustysynth needs). AC: every named format has a matrix row + record with status and next step | `docs/formats/*` + matrix proposals | doc 04 §8.4 | — | M |

### Wave 5.Q — Cross-format hardening

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.Q1 | imp | Fuzz-target sweep: every Phase-5 parser registered with doc 04 §8.5 naming; seed corpora committed; 10-min clean CI runs; coverage gates wired to feature-audit. AC: registry-vs-implementation diff empty | `fuzz/**`, CI job | all 5.* | 5.O*, 5.P* | M |
| 5.Q2-V | ver | Records + differential + blocked-registry audit: every Phase-5 format has a filled record, a correct matrix status via the parity tool, differential evidence where comparable; blocked formats have honest records. AC: audit report with zero missing rows | `docs/formats/**` audit + matrix | 5.Q1 | 5.Q1 | M |

### Wave 5.R — Perf snapshot and gate

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 5.R1 | imp | Perf-vs-VLC breadth snapshot per doc 12: new codecs measured (decode throughput, TTF, seek) vs baselines; honest slower-pure-decoder reporting. AC: numbers + hardware manifest committed | `benches/**`, `docs/perf/breadth.md` | 5.Q2-V | 5.Q2-V | M |
| 5.R2-V | ver | `[LEAD]` Phase 5 gate: P0/P1 format set verified-or-honestly-recorded; fuzz registry complete; records complete. AC: evidence + ledger + Phase 6 launch list | `docs/gates/phase5-exit.md` + worklog | 5.R1 | 5.R1 | S |

## 9. Phase 6 — Network & streaming (waves 6.A–6.I, 17 tasks)

Gate (MP §16): HTTP(S), HLS, DASH, RTSP where possible, buffering, reconnect,
authentication, fuzzing. Detail docs: 03 (input traits), 09 §6 (web I/O shares
the fetch+Range model), 12 §12 of MP (untrusted input). All network responses
are untrusted input. Parallelism 2–3.

### Wave 6.A — Streaming core

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 6.A1 | imp | streaming crate core: HTTP(S) client (ureq or reqwest — purity/async policy per doc 01 §4.2 `[VERIFY]`) with range requests, connect/read deadlines, bounded redirects with an explicit policy, TLS verification always on; typed network errors. AC: mock-server tests (redirect loops, timeouts, 416 range) | `crates/streaming/**` | input traits, plugin-api, media-types | 5.R2-V | M |
| 6.A2 | imp | Seekable network resource: byte-range abstraction over the input crate (sparse buffering, read-ahead window, bounded cache per product.toml limits). AC: seek/rewind tests incl. the no-range-server fallback (linear scan) | `crates/streaming/src/ranged.rs` + input hooks | 6.A1 | 6.A1 | M |
| 6.A3 | imp | Scheme registry + records: http/https access plugins; ftp decision `[PROPOSAL: blocked v1]`; URL parsing/normalization; credential-free logging. AC: `+FB` | `crates/streaming/src/schemes.rs`, `docs/formats/network-http.md` | 6.A1 | 6.A1 | S |

### Wave 6.B — HLS

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 6.B1 | imp | HLS v1: media playlists (EXTINF segments, live sliding window, target duration), segment fetch scheduling, discontinuity handling. AC: `+FB` (fuzz_demux_hls); VOD + live mock-server scenarios | `crates/streaming/src/hls/media.rs` | 6.A2 | 6.A2 | M |
| 6.B2 | imp | HLS master playlists: variant selection by bandwidth (policy `[PROPOSAL]`), alternate audio/subtitle renditions, manual quality override via config. AC: variant switch preserves A/V sync | `crates/streaming/src/hls/master.rs` | 6.B1 | 6.B1 | M |
| 6.B3 | imp | HLS records + conformance: iETF/Apple test vectors; differential vs VLC where lossless; live-edge behavior recorded. AC: `+FB` record complete | `docs/formats/streaming-hls.md`, tests | 6.B2 | 6.B2 | S |

### Wave 6.C — DASH

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 6.C1 | imp | DASH: MPD parser (periods, adaptation sets, representations; static + dynamic; SegmentTemplate/SegmentList). AC: `+FB` (fuzz_demux_mpd); schema-validity tolerance for broken servers | `crates/streaming/src/dash/mpd.rs` | 6.B1 | 6.B1 | M |
| 6.C2 | imp | DASH scheduling + adaptation: buffer-based ABR (`[PROPOSAL]` algorithm), segment timelines, availability-time sync for live; records + conformance (DASH-IF vectors subset). AC: `+FB` | `crates/streaming/src/dash/sched.rs`, `docs/formats/streaming-dash.md` | 6.C1 | 6.C1 | M |

### Wave 6.D — Buffering and reconnect

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 6.D1 | imp | Progressive adaptive buffering: buffer model with stall prediction (`[PROPOSAL]`), watermark fill/drain, rebuffer metrics events. AC: synthetic bandwidth traces (3G/lossy) meet stall targets or honestly miss them in the report | `crates/streaming/src/buffer.rs`, playback-engine hooks | 6.A2 | 6.A2 | M |
| 6.D2 | imp | Reconnect + network-error surfacing: bounded retry with backoff (`[PROPOSAL]`), resume-from-offset, generation-safe cancellation; UI states (reconnecting/failed) via events. AC: interrupted-network scenarios from the suite pass | `crates/streaming/src/reconnect.rs`, events | 6.D1 | 6.D1 | M |

### Wave 6.E — Authentication and remote playlists

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 6.E1 | imp | Authentication: HTTP basic + digest (`[PROPOSAL]`) for access; credential-store policy (OS keychain `[VERIFY]` vs config); redaction tests prove no secrets in logs/errors | `crates/streaming/src/auth.rs` | 6.A1 | 6.A1 | M |
| 6.E2 | imp | Playlist-from-network: remote M3U/XSPF open, relative-URI resolution, redirects; HTML directory-listing parse decision `[PROPOSAL]`. AC: `+FB` | `crates/streaming/src/remote_playlist.rs` + playlists hooks | 6.B3, 5.O1 | 6.B3 | S |

### Wave 6.F — Fuzzing and integration

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 6.F1 | imp | Network fuzz targets: HLS/DASH/MPD parsers + HTTP header/framing edge cases (doc 10 §6 registry). AC: seeds committed; 10-min clean runs | `fuzz/**` | 6.B*, 6.C* | 6.C2 | M |
| 6.F2 | imp | Interrupted-network integration scenarios: synthetic proxy (drop/delay/reset at byte offsets); suite subset (scenario 11 + live-window edges). AC: green; mock evidence labeled | `tests/integration/network/**` | 6.D2, 6.C2 | 6.D2 | M |

### Wave 6.G — RTSP investigation and blocked-protocol registry

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 6.G1-I | inv | RTSP investigation + blocked-protocol registry: pure-Rust RTSP/RTP client feasibility (interleaved-TCP only?), honest verdict — likely `blocked` v1, recorded; udp/rtp/satellite/other blocked rows with reasons incl. the web-build column per doc 09 §10 | `docs/formats/network-rtsp.md`, `docs/formats/network-registry.md` + matrix proposals | docs 04, 09 | 6.A3 | S |

### Wave 6.H — Security review

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 6.H1 | ver | Network security review per doc 11 §7: SSRF (loopback/link-local policy), redirect trust, TLS config, path traversal in playlist URIs, header injection. AC: findings → defect cards or risk-register entries; no silent acceptance | `docs/security/network-review.md` + fix cards | all 6.* | 6.F2, 6.G1-I | M |

### Wave 6.I — Gate

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 6.I1-V | ver | `[LEAD]` Phase 6 gate: streaming scenarios green; security findings triaged; records complete. AC: evidence + ledger + Phase 7 launch list | `docs/gates/phase6-exit.md` + worklog | all Phase 6 | 6.H1 | S |

## 10. Phase 7 — Platform & advanced (waves 7.A–7.L, 24 tasks)

Gate (MP §16): devices, hardware decode, recording, conversion, stream
output, WASM extensions, web build parity. Detail docs: 01 §2.7 (platform
adapters), 06 §5 (extension host), 09 (web). Note: the codec-side H.264/HEVC
hw *integration* landed in 5.F/5.G; this phase builds the OS adapters and the
truthful reporting around them. Parallelism 2–4.

### Wave 7.A — Platform adapters (one per OS; parallel)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.A1 | imp | platform-windows: WASAPI audio device enumeration + change notifications, GPU adapter enumeration, file-dialog glue; every unsafe block documented with invariants (OS-ABI exception per MP §9). AC: device-enum tests on the windows runner; unsafe audit clean | `crates/platform-windows/**` | platform traits (doc 01 §2.7) | 6.I1-V | M |
| 7.A2 | imp | platform-linux: PipeWire/PulseAudio (`[VERIFY]` crates) device enum + notifications, GPU via wgpu adapter, xdg-desktop-portal dialogs; same audit bar. AC: as 7.A1 on linux | `crates/platform-linux/**` | same | 6.I1-V | M |
| 7.A3 | imp | platform-macos: CoreAudio devices, Metal layer glue, NSPanel dialogs; same audit bar. AC: as 7.A1 on macos | `crates/platform-macos/**` | same | 6.I1-V | M |

### Wave 7.B — Hardware decode adapters `[ADR-001]`

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.B1 | imp | VideoToolbox hw decode adapter: H.264 (+ HEVC if 5.G2 executed) via VT; capability detection; truthful hw-active reporting events; graceful fallback to the typed unsupported error. ADR-001 compliance: exact API list + safety review. AC: hw fixture decode on mac hardware, evidence labeled real | `crates/codecs/src/video/h264/hw/videotoolbox.rs`, platform-macos glue | 5.F2, 7.A3 | 5.F2, 7.A3 | M |
| 7.B2 | imp | Media Foundation hw decode adapter (same shape, Windows; MF hw decoder + DXVA paths per ADR). AC: as 7.B1 on windows | `crates/codecs/src/video/h264/hw/mediafoundation.rs`, platform-windows glue | 5.F2, 7.A1 | 5.F2, 7.A1 | M |
| 7.B3 | imp | VA-API hw decode adapter (same shape, Linux; device-node discovery, permission-failure handling). AC: as 7.B1 on linux | `crates/codecs/src/video/h264/hw/vaapi.rs`, platform-linux glue | 5.F2, 7.A2 | 5.F2, 7.A2 | M |

### Wave 7.C — Device hotplug and output fallback

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.C1 | imp | Device hotplug + output fallback: audio-device-lost → reopen/new-device mid-play; GPU/backend fallback chain (auto-negotiated wgpu backend → software present); capability report surfaced to UI. AC: hotplug tests with simulated device events | `crates/playback-engine` hooks, audio-output, renderer-wgpu, platforms | 7.A1–7.A3 | 7.A1–7.A3 | M |

### Wave 7.D — Device discovery

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.D1 | imp | device-discovery crate + panels: local volumes/removable media with events; UPnP/DLNA investigation verdict `[VERIFY]` (implement subset or record blocked); config exposure per docs 06/07; UI panels consume discovery events. AC: discovery events tested; blocked outcomes recorded honestly | `crates/device-discovery/**`, `ui/src/panels/discovery/**` | 7.C1 | 7.C1 | M |

### Wave 7.E — Transcode, recording, conversion

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.E1 | imp | transcode graph core: decode→filter→encode pipeline model, job lifecycle (progress/cancel), bounded resources; v1 encoders = WAV PCM + stream-copy only. AC: job model tests; no unbounded queues | `crates/transcode/**` | codecs, demux | 6.I1-V | M |
| 7.E2 | imp | Recording: record-to-WAV from any decodable input; MP4 remux (H.264/AAC stream copy only, no transcode claim); honest matrix rows. AC: recorded output verified by our own demux+decode | `crates/transcode/src/record.rs` | 7.E1 | 7.E1 | M |
| 7.E3 | imp | Conversion UI + stream-output subset: convert dialog (WAV / MP4-remux targets only, per feature exposure), job queue; stream-output = local file/pipe ES sink `[PROPOSAL]`. AC: no capability shown that is not implemented | `ui/src/screens/convert/**`, transcode wiring | 7.E2 | 7.E2 | M |

### Wave 7.F — WASM extension host (per doc 06 §5 + ADR-002)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.F1 | imp | plugin-host-wasm runtime: chosen runtime per ADR-002 (wasmtime/wasmi `[VERIFY-P0]`), linear-memory/no-host-pointers model, import dispatch (`lumen:v1.*`), API versioning with deprecation window. AC: hello-world extension runs and reports its manifest | `crates/plugin-host-wasm/src/{runtime,imports}.rs` | plugin-api, commands | ADR-002 | M |
| 7.F2 | imp | Permissions + resource limits: capability enforcement on imports (fs-sandboxed-read, http-fetch + allowlist, ui-surface, device-query), memory/fuel ceilings, I/O quotas. AC: every doc 06 §9 escalation test is denied | `crates/plugin-host-wasm/src/{policy,limits}.rs` | 7.F1 | 7.F1 | M |
| 7.F3 | imp | `.lumenext` package + install flow: package format (manifest + module + signature `[PROPOSAL scheme]`), user-initiated install/remove, provenance + trust checks, no-auto-exec proof test. AC: doc 06 §5 install-flow tests pass | `crates/plugin-host-wasm/src/package.rs`, apps glue | 7.F1 | 7.F1 | M |
| 7.F4 | imp | Crash isolation: trap handling → plugin `Disabled` + backoff policy, playback-unaffected proof, resource-exhaustion containment; crash-injection suite wired into the conformance kit. AC: trap-loop and OOM extensions cannot kill playback | `crates/plugin-host-wasm/src/isolation.rs`, conformance kit | 7.F2 | 7.F2 | M |

### Wave 7.G — Example extensions

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.G1 | imp | Example extensions: metadata provider (reads tags via declared permission) + UI panel (command-layer-only protocol per doc 06 §5); both pass the conformance kit and serve as living documentation. AC: kit green; docs walk through both | `plugins/extensions/{metadata-provider,ui-panel}/**` | 7.F4 | 7.F4 | M |

### Wave 7.H — Web build (per doc 09; one card per pillar)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.H1 | imp | apps/web v1, engine-in-Worker: wasm32 build of the engine core, cooperative pump/slice budgets (doc 09 §3), MessagePort client, capability negotiation + honest `threading: single|pooled(n)` report. AC: engine boots and plays the first-vertical fixture headless in CI | `apps/web/**` (worker + build) | engine crates, commands | 6.I1-V | M |
| 7.H2 | imp | Web rendering: RenderSurface → WebGPU with WebGL2 fallback (doc 09 §4), frame-upload path, DPR handling, backend report. AC: headless-webgl tests + manual matrix rows recorded | `apps/web/src/render/**` | 7.H1 | 7.H1 | M |
| 7.H3 | imp | Web audio: AudioWorklet AudioSink + sample-counter clock, drift/resample policy, autoplay gesture-required substate (`[PROPOSAL]` per doc 09 §5). AC: clock stability tests; gesture state surfaced as event | `apps/web/src/audio/**` | 7.H1 | 7.H1 | M |
| 7.H4 | imp | Web I/O: fetch+Range seekability detection, File/Blob no-upload path, blocked capabilities typed (no raw UDP/RTSP/discs per doc 09 §10). AC: seekability matrix on mock servers; blocked rows typed not crashed | `apps/web/src/io/**` | 7.H1 | 7.H1 | M |

### Wave 7.I — Browser matrix and web conformance

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.I1 | ver | Browser matrix + web conformance: run the doc 05 §7 command-layer suite in Chrome/Firefox/Safari incl. iOS constraints; per-browser results recorded against BCD-cited expectations (doc 09 §8); web feature-matrix column honesty pass (blocked/unsupported with reasons). AC: matrix report committed; no claim beyond evidence | `docs/web/browser-matrix.md`, matrix updates | 7.H2–7.H4 | 7.H2–7.H4 | M |

### Wave 7.J — Control server

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.J1 | imp | control-protocol + apps/control-server v1 (`[PROPOSAL]` phase per doc 01): loopback-only bind, versioned WS, token handshake, deny-by-default sensitive ops, request validation; conformance tests. Deep security review happens in 8.A2. AC: contract suite passes; non-loopback bind refused | `crates/control-protocol/**`, `apps/control-server/**` | commands | 6.I1-V | M |

### Wave 7.K — Web packaging

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.K1 | imp | Web packaging + budgets: static bundle, COOP/COEP header decision applied (doc 09 §9 off-by-default `[PROPOSAL]`), size-budget enforcement (3/6 MB `[PROPOSAL]`), per-codec tree-shake proof. AC: budget check fails the build on breach | `apps/web` build + CI job | 7.I1 | 7.I1 | S |

### Wave 7.L — Gate

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 7.L1-V | ver | `[LEAD]` Phase 7 gate: hw-decode truth table verified; WASM sandbox proofs green; web parity column honest; control server first-pass secured. AC: evidence + ledger + Phase 8 launch list | `docs/gates/phase7-exit.md` + worklog | all Phase 7 | 7.K1 | S |

## 11. Phase 8 — Hardening & release (waves 8.A–8.H, 16 tasks)

Gate (MP §16): compatibility, security, soak, performance versus VLC,
packaging, honest parity report. Detail docs: 11 §7/§9 (security, SBOM), 12 §9
(final perf report), 10 §7–§8 (evidence, regression protocol), MP §17 (done
definition). Parallelism 2–4.

### Wave 8.A — Security sweep (per doc 11 §7)

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 8.A1 | ver | Security sweep — parsers: fuzz-corpus expansion, triage of all accumulated findings, probe/demux/decode hardening review. AC: every finding fixed or risk-registered with owner + next step | `docs/security/sweep-parsers.md` + fix cards | 7.L1-V | M |
| 8.A2 | ver | Security sweep — plugins, control server, network: sandbox-escape review, permission-model audit, loopback/token review, secret-redaction verification. AC: same shape | `docs/security/sweep-platform.md` + fix cards | 7.L1-V | M |
| 8.A3 | ver | Security sweep — supply chain: cargo-deny/vet clean, SBOM regeneration (doc 11 §9), license compliance final (incl. design-asset risk R-07), provenance record. AC: audit jobs green; SBOMs attached to artifacts | `docs/security/sweep-supplychain.md` | 1.E5 | M |

### Wave 8.B — Compatibility matrix

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 8.B1 | ver | Full-corpus compatibility matrix run: every supported format × 3 OS + web; evidence per doc 10 §7, mock vs real labeled. AC: complete matrix with no empty claimed-supported cells | matrix + `evidence/` | 8.A1–8.A3 | 8.A1–8.A3 | M |
| 8.B2 | imp | Gap triage + status flips: every gap either fixed (bounded), or status honestly `partial`/`blocked` with reason and next step (MP §17 blocked definition). AC: parity tool applies all transitions; zero unexplained gaps | matrix + parity tool | 8.B1 | 8.B1 | M |

### Wave 8.C — Soak and stress completion

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 8.C1 | ver | Soak/stress completion + stability: full soak suite (4 h × scenarios), reopen-cycle stress, memory slopes vs target, LSAN nightly clean, flaky-test quarantine resolved or removed, regression-guard audit (doc 10 §8). AC: soak reports committed; quarantine empty or scheduled | `tests/soak/**` reports | 8.B2 | 8.B2 | M |

### Wave 8.D — Final performance report

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 8.D1 | ver | Perf-vs-VLC final benchmarks per doc 12 §9: all metrics × all codecs × 3 OS on reference hardware; honest comparisons incl. pure-Rust-slower cases. AC: numbers + manifests committed; no silent re-runs | `docs/perf/final.md` + benches | 8.C1 | 8.C1 | M |
| 8.D2 | imp | Perf report + bounded fixes: top-3 regressions vs baseline fixed with no correctness trades; report finalized. AC: fixes regression-tested; report reflects post-fix numbers | perf + fixes | 8.D1 | 8.D1 | M |

### Wave 8.E — Packaging per platform

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 8.E1 | imp | Packaging Windows: installer (MSI/WiX `[VERIFY]` toolchain purity), code signing (certificate availability `[VERIFY]` — escalate if absent), launch test, upgrade path | `packaging/windows/**` | 8.D2 | 8.D2 | M |
| 8.E2 | imp | Packaging macOS: .dmg, codesign + notarization, universal-binary decision, launch test | `packaging/macos/**` | 8.D2 | 8.D2 | M |
| 8.E3 | imp | Packaging Linux + SBOM all platforms: AppImage/deb/rpm (`[VERIFY]` toolchain), final SBOM + checksums per doc 11 §9 | `packaging/linux/**`, SBOMs | 8.E1, 8.E2 | 8.E1, 8.E2 | M |

### Wave 8.F — Parity statement

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 8.F1 | imp | parity-report tool finalization per doc 10 §7 + MP §17: status transitions strictly from the evidence outbox; matrix-vs-claims diff; no self-verification path exists in code. AC: attempting a manual status flip fails | `tools/parity-report/**` | 8.B2 | 8.B2 | M |
| 8.F2 | imp | Honest parity statement: user-facing report (verified/partial/unsupported/blocked with evidence links) + known-limits doc final (Lua extensions, DVD/BD, DVB, patent notes per MP §14). AC: every sentence traceable to evidence | `docs/parity-statement.md`, `docs/known-limits.md` | 8.F1 | 8.F1 | S |

### Wave 8.G — Docs and release checklist

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 8.G1 | ver | Docs completeness sweep: format records complete, ADR register current, user docs + troubleshooting, README claims audited (no "all formats" claims anywhere). AC: audit checklist all green | `docs/**` audit report | 8.F2 | 8.F2 | M |
| 8.G2 | imp | Release checklist + RC smoke: version stamping from product.toml, RC builds on 3 OS + web, launch/play/install/uninstall tests, update-URL wiring. AC: checklist executable end-to-end on the RC | `docs/release-checklist.md`, RC pipeline | 8.E3, 8.G1 | 8.E3, 8.G1 | M |

### Wave 8.H — Program exit

| ID | Type | Scope & acceptance criteria | Owns | Consumes | Deps | Sz |
|---|---|---|---|---|---|---|
| 8.H1-V | ver | `[LEAD]` Phase 8 gate + program exit review vs the MP §17 done definition: language audit passed; product.toml + schemas drive the product with a passing rebrand test; UI within Figma tolerance; plugins load and sandbox correctly; web + desktop both play real media through the same command layer; CI merge-blocking on all platforms; parity report states exactly what is verified, partial, unsupported, blocked. AC: final evidence record; program closed or follow-up program drafted | `docs/gates/phase8-exit.md` + worklog | 8.G2 | 8.G2 | S |

## 12. Wave execution ledger

One wave-unit = one full Lead cycle (launch → agents run → review gate →
commit → ledger update). Waves are sequential; agents within a wave are
concurrent. Wall-clock is therefore counted in wave-units, **not calendar
time** — no dates are promised. A rework factor of 1.25–1.5× (§1.6
relaunches, remediation cards like 4.G2) yields ~96–115 effective wave-units.

| Phase | Waves | Task cards | Cumulative | Typical parallelism | Wave-units (nominal) |
|---|---|---|---|---|---|
| 0 — Audit | 0.A–0.D (4) | 17 | 17 | 3–5 | 4 |
| 1 — Foundation | 1.A–1.F (6) | 20 | 37 | 2–4 (C/D mostly `[LEAD]`) | 6 |
| 2 — First vertical | 2.A–2.E (5) | 17 | 54 | 2–4 | 5 |
| 3 — Reliable engine | 3.A–3.F (6) | 19 | 73 | 2–4 | 6 |
| 4 — Desktop product | 4.A–4.I (9) | 25 | 98 | 3–5 | 9 |
| 5 — Breadth | 5.A–5.R (18) | 45 | 143 | 2–5 | 18 |
| 6 — Network & streaming | 6.A–6.I (9) | 17 | 160 | 2–3 | 9 |
| 7 — Platform & advanced | 7.A–7.L (12) | 24 | 184 | 2–4 | 12 |
| 8 — Hardening & release | 8.A–8.H (8) | 16 | 200 | 2–4 | 8 |
| **Total** | **77 waves** | **200** | — | **2–6 bound** | **77 nominal** |

Notes on the count:

- 200 cards ≥ 150 required by the brief; every card is S or M (§1.1 sizing
  rule); no L card is launchable.
- Exactly one of the `[DECISION-PENDING]` pair 2.B2/2.B3 executes, so Phase 2
  runs 16 cards and the program executes ~199.
- `[CONDITIONAL]` cards (5.F2, 5.G2, 5.N1–5.N3) execute only when their
  decision/ADR lands; otherwise they are recorded as blocked with the
  investigation attached — the wave still plans them (doc 04 honesty).
- Parallelism is bounded by §1.2 conflicts, not by ambition: Phase 5 runs 2–5
  concurrent format agents because of the R3 ownership-zone rule; Phase 1 C/D
  runs lower because shared interfaces serialize.
- **Delta log:** the Lead may split, merge, or add cards between waves (each
  delta recorded here with date and reason). Phase gates (MP §16) are fixed;
  the card list is a living plan. Deltas so far: none (initial issue, Wave C).

Ledger maintenance is part of the wave gate (§1.5 step 8): per-card status
(`not_started`/`in_progress`/`implemented`/`verified`/`blocked`/
`not-selected`), evidence links, and delta entries.

## 13. Program rules of engagement

**Launching a wave (Lead).** (1) Confirm the previous wave's gate entry is
green (§1.5). (2) For each card: paste the table row + standard clauses
(§1.4) + hygiene block (§1.7) verbatim into one agent assignment — one card
per agent, batched as parallel Task calls. (3) State the temporary-ownership
grants explicitly if any R2 exception applies. (4) Record the launch list in
the wave log (worklog entry). The Lead never paraphrases a card loosely;
if a card is wrong, fix the card in this document first (delta log), then
launch.

**Verifying a wave (Lead).** Files exist; no leftover markers; worklog entries
well-formed; the C3 gate green; 10–25 % of diffs read in full (100 % for
shared-interface and security-relevant cards); card-specific commands re-run
where doubt exists; registration snippets merged; cross-doc conflicts
resolved by editing the shared surface directly. Reject with constructive
feedback and a post-mortem line otherwise (§1.6).

**Committing.** Lead-only, one commit per wave, message
`wave <phase>.<letter>: <summary> (cards <ids>)` per doc 15. Agents do not
commit in Phases 0–4; doc 15 governs any later branch-commit grants; merge to
`main` is always Lead-only.

**Definition-of-done wiring (MP §17).** A card is `implemented` when its
acceptance criteria pass and the wave gate is green. It is `verified` only
when the parity tool has evidence (automated run, platform-labeled, linked in
the ledger) — agents never self-verify. A phase closes only at its `-V` gate
card with recorded evidence on each claimed platform. Cycle reporting
(Implemented / Changed / Verified / Compatibility / Dependency compliance /
Known gaps / Risks / Next action) happens at every phase gate and is compiled
from the ledger, not from memory.

**Escalation to the user (Lead → user).** Only for: changes to MP §2 fixed
decisions; ADR-level ambiguity that cannot be resolved from docs (e.g.
signing-certificate availability, reference-VM availability, design-package
license R-07); legal/patent questions; anything the docs genuinely do not
cover. Everything else is decided by the Lead inside the documented space and
recorded in the worklog.

**Re-planning.** Between waves the Lead may re-cut cards (delta log, §12).
If a phase's gate cannot pass with the current card set, the Lead drafts the
missing cards — the program is designed to absorb them — rather than
weakening the gate. Failing checks are fixed before work counts as done
(MP §17); tests are never weakened or deleted to pass (AGENTS.md §3.5).

---

### Traceability

- MP §15 (protocol): §1.2 conflicts, §1.3 card fields = assignment contract,
  §13 engagement; MP §16 (gates): phase sections 3–11 + `-V` gate cards;
  MP §17 (done/statuses): §2 statuses, §12 ledger, §13 DoD wiring.
- MP §5 (config-driven): waves 1.B, 4.E/H; MP §6 (plugins): waves 1.D2, 7.F/G;
  MP §7 (UI fidelity): waves 4.B–4.G; MP §8 (engine): waves 2–3; MP §9
  (Rust-only/deps): 1.E5, 8.A3, `[VERIFY]` markers throughout; MP §10 (web):
  1.E3, 7.H–7.K; MP §11 (perf): 3.F1, 5.R1, 8.D; MP §12 (security): 0.D1,
  6.H1, 8.A; MP §13 (testing/CI): 1.A/E, 3.E, 5.Q1, 8.B/C; MP §14 (limits):
  5.P, 6.G, 8.F2.
- Doc 02 §7 → §3; doc 03 §5–§8 → §6; doc 04 §8 → §8; doc 05 → 1.D, 3.B4,
  7.I; doc 06 §5 → 7.F/G; doc 07 → 1.B, 4.E/H; doc 08 → 1.B4, 4.*; doc 09 →
  7.H–7.K; doc 10 → §1.4 clauses + waves 3.E, 5.Q, 8.B/C; docs 11/12 → 8.A,
  8.D; doc 14 (risks) and doc 15 (git) are cross-referenced at each gate.

Open questions for the Lead (beyond each doc's own list): (1) ratify the §2
grammar supersession of AGENTS.md §5's `<phase>-<subsystem>-<n>` form;
(2) ratify the `not-selected` status extension; (3) confirm wave 2.B demuxer
decision timing (before wave 2.B launch); (4) confirm the rework-factor
assumption (1.25–1.5×) used for wall-clock estimates; (5) confirm the 8.A–8.C
`ver`-type cards may open fix cards ad hoc within Phase 8 waves (delta log).




