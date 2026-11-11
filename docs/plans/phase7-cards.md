# Phase 7 card-to-issue breakdown (issue #22 close package)

**Owner:** W18-NN (docs-only). **Source of truth:** `docs/13-agent-wave-program.md` §10
(waves 7.A–7.L, lines 692–782, 24 cards). Every item below derives its scope and
AC from that section — no new scope invented here. The Lead has converted this
table into GitHub issues #40–#51 (created 2026-11-09, one per row, in table
order); umbrella issue #22 closes on the Phase 7 exit gate (item 12 / #51).

**Purpose:** group the 24 Phase 7 cards into 12 issue-sized, dependency-ordered work
items so parallelism (MP says 2–4 concurrent agents) maps cleanly onto issue
assignment, and identify what can start immediately.

## Reality deltas since doc 13 §10 was written

The wave tables are followed verbatim; these deltas are *de-risks*, not scope changes:

1. **TLS is landed and activated** (issue #39, ADR-011, ureq+rustls; commits
   `849be1f`/`913008d`): `crates/streaming` now has a proven https + Range-fetch
   path with activation tests and mockhttp. This de-risks 7.H4 (browser
   fetch+Range seekability mirrors the native design), 7.F2's `http-fetch`
   allowlisted import (native transport precedent), and 7.D1's UPnP/DLNA probing.
2. **Streaming stack is mature** (wave 17: HLS media, ogg native_seek fix,
   bounded buffering): the bounded-backpressure and seekability-detection
   patterns that 7.E1 (transcode jobs) and 7.H4 (web I/O) need already exist
   in-tree to copy, not reinvent.
3. **ADR-002 is accepted** (wasmi, Phase 0 gate) — 7.F1's `[VERIFY-P0]`
   "wasmtime/wasmi" question is **resolved: wasmi**. No investigation needed.
4. **ADR-013 (VA-API design brief) is drafted, status *proposed*** — it must be
   ratified by the Lead **before** the 7.B3 VA-API lane starts (per its own
   mandate). The 7.B1 VideoToolbox lane is the sequence anchor and is not blocked.
5. **Phase 6 gate is done** (`docs/gates/phase6-exit.md`) — every `6.I1-V`
   dependency in §10 is satisfied; nothing in Phase 7 waits on Phase 6.
6. Confirmed still-missing targets: crates `platform-{windows,linux,macos}`,
   `device-discovery`, `transcode`, `plugin-host-wasm`, `control-protocol`;
   `apps/web`, `apps/control-server`, `plugins/extensions/`,
   `crates/codecs/src/video/h264/hw/`.

## Work items (tracked as issues #40–#51)

| # | Issue | Title | Cards | Size | Depends on | Start |
|---|---|---|---|---|---|---|
| 1 | #40 | Platform adapter crates (Windows/Linux/macOS) | 7.A1–7.A3 | L | 6.I1-V ✅ | **ready now** |
| 2 | #41 | Hardware decode adapters (VT / MF / VA-API) | 7.B1–7.B3 | L | item 1, 5.F2 ✅ | sequential |
| 3 | #42 | Device hotplug, output fallback & discovery | 7.C1, 7.D1 | L | item 1 | sequential |
| 4 | #43 | Transcode graph core | 7.E1 | M | 6.I1-V ✅ | **ready now** |
| 5 | #44 | Recording, conversion UI & stream-output subset | 7.E2, 7.E3 | L | item 4 | sequential |
| 6 | #45 | plugin-host-wasm v1 (runtime, policy, packages) | 7.F1–7.F3 | L | ADR-002 ✅ | **ready now** |
| 7 | #46 | Plugin crash isolation & example extensions | 7.F4, 7.G1 | L | item 6 | sequential |
| 8 | #47 | apps/web v1 — engine-in-Worker | 7.H1 | M | 6.I1-V ✅ | **ready now** |
| 9 | #48 | Web render, audio & I/O pillars | 7.H2–7.H4 | L | item 8 | sequential |
| 10 | #49 | Browser matrix, web conformance & packaging budgets | 7.I1, 7.K1 | M | item 9 | sequential |
| 11 | #50 | control-protocol & apps/control-server v1 | 7.J1 | M | 6.I1-V ✅ | **ready now** |
| 12 | #51 | Phase 7 exit gate (LEAD) | 7.L1-V | S | all | last |

Card coverage: 3+3+2+1+2+3+2+1+3+2+1+1 = **24/24**. Issue #22 names 7 missing
crates; items 1 (×3), 3 (`device-discovery`), 4 (`transcode`), 6
(`plugin-host-wasm`), 11 (`control-protocol`) plus items 8/11 (apps) cover all.

### Item bodies (paste-ready drafts)

**1 — Platform adapter crates (Windows/Linux/macOS)** *(issue #40; 7.A1, 7.A2, 7.A3; L)*
Create `crates/platform-windows`, `crates/platform-linux`, `crates/platform-macos`
implementing the platform traits (doc 01 §2.7), one lane per OS, parallel:
Windows — WASAPI audio device enumeration + change notifications, GPU adapter
enumeration, file-dialog glue; Linux — PipeWire/PulseAudio (`[VERIFY]` crates)
device enum + notifications, GPU via wgpu adapter, xdg-desktop-portal dialogs;
macOS — CoreAudio devices, Metal layer glue, NSPanel dialogs. Every unsafe block
documented with invariants (OS-ABI exception per MP §9). *Acceptance:* device-enum
tests green on each OS runner (windows/linux/macos); unsafe audit clean per lane.
*Deps:* 6.I1-V ✅. *Note:* single tracking issue with three checklists; the Lead
may split per-OS for parallel assignment. **De-risked:** no prerequisites remain.

**2 — Hardware decode adapters (VideoToolbox / Media Foundation / VA-API)** *(issue #41; 7.B1, 7.B2, 7.B3; L)*
Implement the ADR-001 hw decode adapters under `crates/codecs/src/video/h264/hw/`
with platform glue: VideoToolbox (macOS; H.264 + HEVC if 5.G2 executed), Media
Foundation + DXVA paths (Windows), VA-API (Linux; device-node discovery,
permission-failure handling). Each: capability detection, truthful hw-active
reporting events, graceful fallback to the typed unsupported error; exact API
list + safety review per ADR-001. *Acceptance:* hw fixture decode on real
mac/windows/linux hardware, evidence labeled real. *Deps:* 5.F2 ✅, item 1.
*Delta:* ADR-013 is drafted but **proposed** — ratify before the VA-API lane;
VT lane (7.B1) is the sequence anchor.

**3 — Device hotplug, output fallback & discovery** *(issue #42; 7.C1, 7.D1; L, ordered C1→D1)*
C1: audio-device-lost → reopen/new-device mid-play in `playback-engine` hooks +
`audio-output`; GPU/backend fallback chain (auto-negotiated wgpu backend →
software present) in `renderer-wgpu`; capability report surfaced to UI; hotplug
tests with simulated device events. D1: `crates/device-discovery` + UI panels
(`ui/src/panels/discovery`): local volumes/removable-media events; UPnP/DLNA
investigation verdict `[VERIFY]` (implement subset or record blocked); config
exposure per docs 06/07. *Acceptance:* hotplug tests with simulated events;
discovery events tested; blocked outcomes recorded honestly. *Deps:* item 1.
*Delta:* in-tree https (ADR-011) is available for any UPnP/DLNA probing — state
the verdict against that capability.

**4 — Transcode graph core** *(issue #43; 7.E1; M)*
Create `crates/transcode`: decode→filter→encode pipeline model, job lifecycle
(progress/cancel), bounded resources; v1 encoders = WAV PCM + stream-copy only.
*Acceptance:* job model tests; no unbounded queues. *Deps:* 6.I1-V ✅; codecs +
demux exist. **Ready now; de-risked** by the mature streaming stack's bounded
backpressure patterns — reuse them.

**5 — Recording, conversion UI & stream-output subset** *(issue #44; 7.E2, 7.E3; L, ordered E2→E3)*
E2: `crates/transcode/src/record.rs` — record-to-WAV from any decodable input;
MP4 remux (H.264/AAC stream copy only, no transcode claim); honest matrix rows;
recorded output verified by our own demux+decode. E3: convert dialog
(`ui/src/screens/convert`, WAV / MP4-remux targets only per feature exposure),
job queue; stream-output = local file/pipe ES sink `[PROPOSAL]`. *Acceptance:*
no capability shown that is not implemented. *Deps:* item 4.

**6 — plugin-host-wasm v1 (runtime, permissions, packaging)** *(issue #45; 7.F1, 7.F2, 7.F3; L, ordered F1→F2→F3)*
Create `crates/plugin-host-wasm` per doc 06 §5: F1 — **wasmi** runtime (ADR-002
accepted — the doc-13 `[VERIFY-P0]` is resolved), linear-memory/no-host-pointers
model, `lumen:v1.*` import dispatch, API versioning with deprecation window;
hello-world extension runs and reports its manifest. F2 — capability enforcement
on imports (fs-sandboxed-read, http-fetch + allowlist, ui-surface,
device-query), memory/fuel ceilings, I/O quotas; every doc 06 §9 escalation test
denied. F3 — `.lumenext` package format (manifest + module + signature
`[PROPOSAL scheme]`), user-initiated install/remove, provenance + trust checks,
no-auto-exec proof; doc 06 §5 install-flow tests pass. *Deps:* plugin-api ✅,
ADR-002 ✅. **Ready now; heavily de-risked** — runtime decision done, and the
native http-fetch import can lean on the in-tree https client (ADR-011).

**7 — Plugin crash isolation & example extensions** *(issue #46; 7.F4, 7.G1; L)*
F4: trap handling → plugin `Disabled` + backoff policy, playback-unaffected
proof, resource-exhaustion containment; crash-injection suite wired into the
conformance kit — trap-loop and OOM extensions cannot kill playback. G1: example
extensions `plugins/extensions/{metadata-provider,ui-panel}` (command-layer-only
protocol per doc 06 §5) passing the conformance kit as living documentation;
docs walk through both. *Deps:* item 6 (esp. F2).

**8 — apps/web v1 — engine-in-Worker** *(issue #47; 7.H1; M)*
Create `apps/web`: wasm32 build of the engine core, cooperative pump/slice
budgets (doc 09 §3), MessagePort client, capability negotiation + honest
`threading: single|pooled(n)` report. *Acceptance:* engine boots and plays the
first-vertical fixture headless in CI. *Deps:* 6.I1-V ✅; engine crates +
commands exist. **Ready now.**

**9 — Web render, audio & I/O pillars** *(issue #48; 7.H2, 7.H3, 7.H4; L, three parallel pillars after H1)*
H2: RenderSurface → WebGPU with WebGL2 fallback (doc 09 §4), frame-upload path,
DPR handling, backend report; headless-webgl tests + manual matrix rows. H3:
AudioWorklet AudioSink + sample-counter clock, drift/resample policy,
autoplay-gesture-required substate `[PROPOSAL]` (doc 09 §5); clock stability
tests, gesture surfaced as event. H4: fetch+Range seekability detection,
File/Blob no-upload path, blocked capabilities typed (no raw UDP/RTSP/discs per
doc 09 §10); seekability matrix on mock servers, blocked rows typed not
crashed. *Deps:* item 8. *Delta:* mirror `crates/streaming`'s Range/
seekability-detection design (incl. its mockhttp harness) for the browser path.

**10 — Browser matrix, web conformance & packaging budgets** *(issue #49; 7.I1, 7.K1; M, ordered I1→K1)*
I1 (ver): run the doc 05 §7 command-layer suite in Chrome/Firefox/Safari incl.
iOS constraints; per-browser results recorded against BCD-cited expectations
(doc 09 §8); web feature-matrix column honesty pass (blocked/unsupported with
reasons); matrix report committed, no claim beyond evidence. K1: static bundle,
COOP/COEP header decision applied (doc 09 §9, off-by-default `[PROPOSAL]`),
size-budget enforcement (3/6 MB `[PROPOSAL]`), per-codec tree-shake proof;
budget check fails the build on breach. *Deps:* item 9.

**11 — control-protocol & apps/control-server v1** *(issue #50; 7.J1; M)*
Create `crates/control-protocol` + `apps/control-server` (`[PROPOSAL]` phase per
doc 01): loopback-only bind, versioned WS, token handshake, deny-by-default
sensitive ops, request validation; conformance tests. Deep security review is
explicitly deferred to 8.A2. *Acceptance:* contract suite passes; non-loopback
bind refused. *Deps:* 6.I1-V ✅; commands crate exists. **Ready now.**

**12 — Phase 7 exit gate (LEAD)** *(issue #51; 7.L1-V; S, ver)*
`docs/gates/phase7-exit.md` + worklog: hw-decode truth table verified; WASM
sandbox proofs green; web parity column honest; control server first-pass
secured. AC: evidence + ledger + Phase 8 launch list. *Deps:* all items. This is
the `[LEAD]`-owned closure condition — now tracked as issue #51 (the final
gate issue), not an implementation issue.

## Ready-to-start now (no missing-crate prerequisite)

- **7.A1, 7.A2, 7.A3** — they *are* the new crates; only prereq (6.I1-V) is done.
- **7.E1** — `transcode` is new but consumes only existing codecs/demux.
- **7.F1** (entry point of item 6; F2/F3 follow in-crate) — ADR-002 decided.
- **7.H1** — `apps/web` is new but consumes existing engine crates.
- **7.J1** — `control-protocol`/`apps/control-server` new, consumes commands.

That is 5 parallel entry lanes ≈ the MP's 2–4 parallelism (item 1 counts as up
to 3 lanes if split per-OS).

## Sequential (blocked on a missing crate or an earlier card)

- 7.B1–7.B3 → after 7.A1–7.A3 (VA-API lane also gated on ADR-013 ratification).
- 7.C1 → after A-wave; 7.D1 → after 7.C1.
- 7.E2 → after 7.E1; 7.E3 → after 7.E2.
- 7.F2 → after 7.F1; 7.F3 → after 7.F1; 7.F4 → after 7.F2; 7.G1 → after 7.F4.
- 7.H2, 7.H3, 7.H4 → after 7.H1 (then mutually parallel).
- 7.I1 → after 7.H2–7.H4; 7.K1 → after 7.I1.
- 7.L1-V → after all.
