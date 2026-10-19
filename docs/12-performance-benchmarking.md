# 12 — Performance Engineering & Benchmarking Program

| Field | Value |
|---|---|
| Task | 1.C12 (Wave 1, planning document — program design, no implementation) |
| Status | implemented (awaiting Lead review) |
| Authoritative source | `docs/MASTER-PROMPT.md` (MP) — **§11 primary**; §1 (parity = measured), §2 (clean-room oracle rule), §8 (engine requirements informing perf), §13 (differential oracle, corpus), §16 (Phase 8 gate), §17 (Verified = commands + actual numbers) |
| Companions | doc 01 (architecture, ADR-001 register), doc 02 §2 (VLC pin record, oracle invocation contract M1–M4, hardware template), doc 03 §2/§4.4/§4.9/§4.10/§4.11/§9 (pipeline, queues, RT audio rules, scheduler, renderer, threading), doc 04 §7 (honest gap register — decoder perf reality), doc 05 §2–§4 (command/event catalog — the measurement surface), doc 09 §2.4/§3/§4/§5 (web perf constraints), doc 10 §4/§5/§6.5/§6.6/§9 (CI placement, corpus, soak, differential, dashboards) |
| Inventions | Marked `[PROPOSAL]`; collected in §10 with the decisions requested from the Lead |
| Verification | Tooling facts re-verified via network on 2026-10-19 (crates.io API; doc.rust-lang.org). Everything else unverified is `[VERIFY]`. No code was written; no benchmark was run. |

**Scope.** This document defines the entire performance program: principles, the
metrics catalogue, the `tools/bench-vs-vlc` harness, the baseline protocol,
provisional targets, the architecture-for-performance plan, profiling, soak
perf, and reporting. **Out of scope:** running any of it (Phase 0 fixes
methodology and the hardware-recording format only — doc 02 §1; Phase 2+
captures first real numbers; Phase 8 executes the full program, MP §16), and
micro-optimization decisions (made per-function with profiling evidence, §7).

---

## 1. Principles (Trace: MP §11, §8, §17)

### 1.1 Baseline first, then targets

No performance target in this project is set before a recorded baseline exists.
Numbers in §5 are explicitly provisional `[PROPOSAL]` and exist only so the
program has a shape; each is replaced by an evidence-derived target the moment
the corresponding baseline lands. The order is fixed: (1) pin VLC 3.0.23 and
the hardware (doc 02 §2), (2) capture the baseline per metric per corpus item
per platform (§4), (3) set targets as ratios of, or absolute caps derived from,
that baseline, (4) only then optimize. "Fast" is a measurement, not an
adjective (MP §11 heading).

### 1.2 Same files, same hardware, pinned versions

Every comparison against VLC 3.0.23 uses the pinned binary (doc 02 §2.3 pin
record, hash-asserted before every invocation, drift aborts the run), the same
corpus files (content-hashed, §3.4), and the same recorded machine (doc 02
hardware template). A number without its environment fingerprint is not
evidence and cannot appear in any report (MP §17).

### 1.3 The honest-reporting rule

Pure-Rust software decoders may start **slower** than VLC's heavily tuned,
assembly-accelerated decoders (doc 04 §7: H.264/HEVC software decode is a
registered gap; rav1d may be adopted with asm off). The rule: report the gap
honestly and improve it with evidence. We publish losses next to wins, name the
decoder that produced every run (§6.4), and never re-run until we like the
number (run selection rules are fixed in §3.6 before the run happens).

### 1.4 Never trade correctness for a benchmark

Non-negotiable, from MP §8 ("never fabricate frames, silence, durations") and
MP §11: no fabricated frames, no silence-padding to hide underruns, no skipped
validation, no benchmark-only fast paths, no corpus-tuned special cases. The
harness drives Lumen through the **same command layer** the UI uses (doc 05
§1.1) — the CLI host is a host, not a back door — and §3.9 lists the
anti-gaming rules enforced mechanically. A faster wrong answer is a defect, and
defects get regression tests (doc 10 §8.1).

### 1.5 Engine-reported, not UI-estimated

Latency metrics are computed from **engine-stamped event timestamps**, never
from when the UI happened to render an event. Doc 05's event envelope carries
`seq`/`session`/`gen`/`origin` but no timestamp; §2.4 proposes the additive
envelope field `ts_ns` (monotonic engine clock) that makes every metric below
engine-reported. Until that proposal is accepted, the harness stamps events at
command-layer receipt and labels those numbers `host-measured` — a strictly
weaker claim.

---

## 2. Metrics catalogue

### 2.1 How to read the catalogue

Each metric is defined by its **endpoints** (two engine-observable points),
**unit**, **owning stage** (doc 05 `Stage`), the **Lumen measurement points**
(doc 05 events/commands), and the **VLC measurement points** (doc 02 §2.4
oracle modes M1–M4). Counters that are not interval-derived are surfaced
through a proposed additive stats command (§2.4). All wall-clock latencies are
**medians with p95**, over ≥10 repetitions (§3.6).

| # | Metric | Endpoints (start → stop) | Unit | Owning stage | Lumen points (doc 05) | VLC points (doc 02 §2.4) |
|---|---|---|---|---|---|---|
| M-01 | Startup time | process spawn → command layer ready | ms | host/bootstrap | process spawn (harness `Instant`) → first successful `system.get-engine-info` response `[PROPOSAL: defines "ready"]` | spawn → first rc-interface reply (M4) `[VERIFY rc timing granularity]` |
| M-02 | Probe latency | open accepted → format identified | ms | Probe | `session.open` response (`Accepted`) → proposed `session.probe-completed { format, gen }` (§2.4) | rc `add` → verbose-log demuxer line (M1 grammar) `[VERIFY]` |
| M-03 | Time-to-first-frame (TTF) | open accepted → first video frame presented | ms | VideoOutput + Renderer | `session.open` `Accepted` → proposed `video.first-frame-presented { pts, gen }` (§2.4) | log-line delta to first display (M4) `[VERIFY: no clean VLC signal; see §2.3 note]` |
| M-04 | Time-to-first-sound (TTFS) | open accepted → first non-silent audio output | ms | AudioOutput | `Accepted` → first callback that drains real samples, counted by the sink (doc 03 §4.9 counter) and surfaced via stats (§2.4) | log-line delta to first audio buffer sent (M4) `[VERIFY]` |
| M-05 | Seek latency | seek command accepted → first target frame presented | ms | Demux + Decode + VideoOutput | `playback.seek` `Accepted` → `timing.seek-completed { gen, position }` — definition pinned in §2.3 | rc `seek` → `get_time` reflects target (M4, polling ≤ 10 ms) `[VERIFY]` |
| M-06 | Playback CPU | steady-state window | % core-time (user+sys, and per-thread) | all (attribution per doc 03 §9 thread roles) | external sampler (§3.2) + per-thread `/proc/<pid>/task/*/stat` on Linux | identical external sampler — same method, both processes |
| M-07 | GPU use | steady-state window | % GPU-engine time + VRAM bytes | Renderer | external per-process GPU counters (§3.2) | identical method |
| M-08 | Memory (peak + steady) | whole run | MiB RSS (peak; steady = median of last 60 s) | all | external RSS sampler + engine-internal `queue-resident-bytes` and allocator counters (§2.4) | external RSS sampler only |
| M-09 | Dropped video frames | steady-state window | count and % of decode-eligible frames | VideoOutput (doc 03 §4.10 drop policy) | drop counter in video-engine, split `late-dropped` vs `policy-skipped`, via stats (§2.4) | rc `stats` decoded/displayed/lost `[VERIFY exact fields on 3.0.23]` |
| M-10 | Audio underruns | whole run | count of callback starvations (doc 03 `SinkEvent::Underrun`) | AudioOutput | underrun counter via stats (§2.4); must be 0 in soak (§5) | rc `stats` output counters `[VERIFY]` |
| M-11 | Rebuffer frequency | network-source runs | episodes/min after initial fill | Input + Demux (buffering policy) | `session.state-changed` `playing→buffering` transitions with `gen`, minus the initial episode | log/status polling (M1) `[VERIFY]` |
| M-12 | Energy proxy `[PROPOSAL]` | steady-state window | J/min (package or platform energy) | all | RAPL (`/sys/class/powercap`) on Linux; `powermetrics` on macOS (privileged, manual tier); none on Windows v1 | identical method |

### 2.2 Cold vs warm, and window definitions

- **Startup (M-01)** is reported twice: **warm** (immediate re-launch, OS page
  cache hot — the common user experience) and **cold** (page cache evicted
  before launch: `posix_fadvise(DONTNEED)` over the binary on Linux
  `[VERIFY: privilege-free]`; a reboot protocol on the manual tier for macOS/
  Windows). Cold runs are N=5 minimum; they are noisy and never gate anything
  (§5), they inform.
- **Seek (M-05)** is reported per seek kind: local-file warm seek (target
  within demux cache), local-file cold seek (target far away, queue flush +
  refill), and network seek. Frame-step seeks are excluded from M-05 and
  tracked in the testkit only.
- **Steady-state window (M-06..M-09)** = the 60 s interval starting at the
  first frame + 5 s (TTF excluded, so decode warm-up does not pollute
  steady-state numbers). Soak runs use the whole-run version (§8).

### 2.3 Measurement notes and known asymmetries

- **VLC-side first-frame/first-sound (M-03/M-04)** have no clean engine signal
  in 3.0.23; the M4 method is verbose-log wall-clock deltas
  (`-I dummy --verbose 2 --play-and-exit`). This is an **asymmetric
  measurement** (our engine-side stamp vs their log line). It is honest if the
  log line is emitted at the same pipeline point; the Phase 2 baseline
  procedure (§4.1) must validate which log lines correspond to "first picture
  displayed" and "first audio buffer written", record the exact grammar in the
  harness, and annotate every published M-03/M-04 comparison with the method.
  `[VERIFY: log grammar on 3.0.23 — same open question as doc 02 M1.]`
- **Seek-completed definition (M-05).** Doc 05 §3.2 defines
  `timing.seek-completed` as the terminal event for the newest seek generation
  but does not pin *when* it fires. This document proposes: seek-completed is
  emitted when the **first frame at/after the target position is presented**
  (video media) or the **first audio sample at/after target is output**
  (audio-only). This makes M-05 exactly `seek-completed.ts − Accepted.ts`.
  `[PROPOSAL — doc 05 §3.2 wording change; needs Lead sign-off.]`
- **M-06/M-07/M-08 use identical external methods on both players.** Process-
  and GPU-level counters are measured by the harness the same way for both
  processes — the comparison is fair even where absolute attribution is
  platform-dependent.
- **M-12** exists because "energy" is where decode efficiency shows up last;
  it is report-only in v1 (no target), and only where measurable without
  degrading the machine (RAPL reading is passive; `powermetrics` needs sudo →
  manual tier).

### 2.4 Instrumentation deltas requested from doc 05 `[PROPOSAL]`

All additive per doc 05 §6.1 rules; all carry `gen` where pipeline-scoped:

1. **Envelope field `ts_ns`** (§1.5) — monotonic engine clock stamp on every
   event; the foundation of engine-reported metrics.
2. **Event `session.probe-completed { format, container, gen }`** — M-02.
3. **Event `video.first-frame-presented { pts, gen }`** and **counter**
   `audio.first-sound` — M-03/M-04.
4. **Command `system.get-stats { session? }`** — cumulative counters:
   `frames_decoded`, `frames_presented`, `frames_late_dropped`,
   `frames_policy_skipped`, `audio_underruns`, `audio_first_sound_ms`,
   `queue_resident_bytes { audio, video }`, `rebuffer_count`. Snapshot class
   (S). Counters are also readable by testkit in-process without the command
   layer (doc 10 §7.1 labels apply: in-process = test evidence, through-host =
   contract evidence).

---

## 3. Benchmark harness — `tools/bench-vs-vlc`

### 3.1 Position and clean-room safety

`tools/bench-vs-vlc` is an xtask-adjacent binary in the tools/ area of MP §4.
Clean-room rule (MP §2): VLC runs **externally** as a reference oracle,
invoked only through the recorded modes of doc 02 §2.4, version-asserted
before every invocation, never vendored, never linked. The harness never reads
VLC source; the only VLC-derived artifacts are its observable outputs (rc
replies, log lines, exit codes). The harness itself is pure Rust, in-repo, CI-
built like any other crate.

### 3.2 Driving Lumen

The harness embeds the engine **through the command layer only** (doc 05 §1.1),
via the same `crates/commands` catalog the UIs use, hosted by a bench host
(apps/cli in `--script` mode `[PROPOSAL]`, or a dedicated `bench-host` binary
in tools/ that reuses `crates/testkit`'s host scaffolding). No engine-internal
APIs, no testkit back doors — if a metric cannot be measured through the
contract, the contract gains a field (§2.4), not the harness a shortcut.
External samplers (CPU/GPU/RSS, §2.1 M-06..M-08) run as harness child threads
polling OS counters at 100–250 ms `[PROPOSAL: interval]`, writing into the
run record.

### 3.3 Driving VLC 3.0.23 (the automation surface)

Two surfaces, both already anticipated by doc 02 §2.4:

- **Interactive: the rc interface** (`vlc -I rc`), for seek-latency and
  long-running scenarios — commands `add`, `seek`, `get_time`, `is_playing`,
  `status`, `stats` `[VERIFY: exact command set and reply grammar on 3.0.23;
  doc 02 M1 flags this same verification]`. Chosen because it is a stable,
  documented, socket/stdio-attachable control surface that exists on all three
  platforms.
- **Batch: `-I dummy --play-and-exit --verbose 2`** for startup, TTF, TTFS and
  whole-run metrics, with wall-clock deltas taken from the verbose log and the
  process clock `[VERIFY: log timestamp availability/precision per platform]`.

Selection rule `[PROPOSAL]`: rc for anything requiring mid-playback
interaction; `--play-and-exit` for launch-to-first-output timing (rc itself
adds startup cost and would pollute M-01). Both modes are recorded in the run
record; the pin guard (doc 02 §2.4) runs before every invocation. For
software-vs-software fairness VLC is pinned to software decode
(`--avcodec-hw=none` `[VERIFY exact 3.0.23 flag]`) — mirroring doc 10 §6.6 —
and a separate hw-vs-hw suite runs where both sides have OS decoders (§6.4).

### 3.4 Bench corpus

`tests/fixtures/bench/` `[PROPOSAL: location; reuses doc 10 §5 licensing
ledger and generation pipeline]`. Dimensions: **format** (containers × codecs)
× **bitrate tier** (low/mid/high) × **resolution tier** (audio-only, 480p,
720p, 1080p, 2160p) × **duration class** (10 s clips for latency metrics, 60 s
for steady-state, loopable long items for soak). Concrete codec membership
grows with the codec roadmap (doc 04 §8) — the corpus is pinned per phase, and
every item records its generator parameters. Rules:

- **Synthetic by default** where licensing is unclear (doc 10 §5.3): generated
  by the fixture generator with recorded parameters; content-hashed (SHA-256
  in the corpus manifest); identical files served to both players.
- **CI budget**: total corpus ≤ 2 GiB, per-item ≤ 60 s / 150 MiB
  `[PROPOSAL — numbers]`; a `--quick` subset (≤ 200 MiB) covers P0 formats for
  interactive/label runs (§3.8).
- **Tier tags** map to §5 targets: e.g. `video/h264-1080p-mid.mkv` is the
  reference item for the 1080p seek target.
- Real-world captures (e.g. broadcast streams) enter only through the doc 10
  §5.2 acquisition ledger.

### 3.5 Environment control

- **Machine spec recorded** once per machine via doc 02's hardware template
  (CPU, cores, freq range, RAM, GPU, OS build, compositor on Linux, driver
  versions, audio device); the machine gets a stable `machine_id` referenced by
  every run.
- **Governor pinned** to performance on Linux (`cpupower frequency-set -g
  performance` `[VERIFY: available on runner]`); Windows high-performance
  power scheme; macOS has no user governor — recorded as a caveat, macOS runs
  are accepted with higher noise (§5.3).
- **Warmup**: ≥ 2 discarded full runs per player per scenario before recording.
- **Repetitions**: N ≥ 10 for latency metrics (M-01..M-05), N ≥ 3 for
  steady-state resource metrics (M-06..M-08), N ≥ 2 full soak runs per release
  (§8). Each repetition is a **fresh process** (no cross-run caching of decode
  state).
- **Statistics**: median + p95 + IQR per metric per item. Run-level variance
  (IQR/median) is reported next to every number.
- **Background-noise budget** `[PROPOSAL]`: before each session the harness
  samples 30 s of system idle; if idle CPU > 5% or a latency metric's
  within-session CV exceeds 15%, the session is flagged `noisy` and repeated
  once; if still noisy, the machine is quarantined (its baselines are not
  updated until investigated). Recorded runs never get deleted for being slow —
  they get *explained* or the session is voided wholesale under the noise rule.

### 3.6 Output format and regression detection

Every run writes `results/<run-id>/` (gitignored, archived to the evidence
store per doc 10 §4.5):

```
results/<run-id>/
  manifest.json     # run-id, date, machine_id, env fingerprint, corpus manifest
                    # hash, Lumen build (git SHA + cargo profile + features),
                    # VLC pin (version string + artifact hash), mode (rc|batch),
                    # active decoders per item (from system.get-engine-info)
  per-item/*.json   # one record per corpus item × player: raw samples,
                    # median/p95/IQR per metric, counters, sampler series
  summary.json      # cross-item rollups per tier
  profiles/         # flamegraphs / trace captures when the run captures them (§7)
```

**Baselines** (the distilled medians, not the raw runs) are committed under
`docs/baselines/perf/<platform>/<suite>.json` `[PROPOSAL — see §4.2 on the
directory-name reconciliation with doc 02's docs/baseline/]`, each carrying the
manifest fields that make it reproducible.

**Regression detection** `[PROPOSAL]` is noise-aware, not a flat threshold: a
metric regresses when the new median worsens by more than
`max(flat%, 2× historical σ)` where flat% is 5 % for M-01..M-05, 7.5 % for
M-06..M-08. Beyond that: **flagged** (report + issue). **Hard** regression at
> 20 % on any latency metric, or any nonzero→worse count metric (M-09/M-10 on
  the soak suite), escalates per §5.2. Count metrics moving 0 → > 0 in steady
  state is always hard.

### 3.7 Web suite

The same scenario definitions run against `apps/web` through the
`@lumen/wasm-client` (doc 09 §7) in Playwright-driven browsers, with
web-specific metrics added: module fetch + compile + instantiate (startup
analog), size budget conformance (doc 09 §2.4: 3/6 MB), single-thread vs pooled
threading mode as separate suites (doc 09 §3), and frame-upload-path
attribution where the browser exposes it. Browser runs are a **separate
baseline family** — never averaged with desktop numbers (§5.3).

### 3.8 CI placement — the no-CI-by-default policy

Benchmarks are **not** PR-blocking by default. Placement:

- **Nightly, dedicated hardware**: the full suite runs on the reference
  machines (self-hosted runners per doc 10 §4.1 `[PROPOSAL: runner class]`),
  results archived, regressions filed per §3.6.
- **PR-blocking exceptions**: (a) the `perf-check` label — auto-suggested when
  a PR touches a hot-path crate (`renderer-wgpu`, `audio-engine`, `codecs`,
  `demux`, `packet-queue`, `input`, `playback-engine`) — triggers the `--quick`
  latency suite on one reference machine and blocks on **hard** regressions
  only; (b) in-tree `criterion` micro-benchmarks (benches/ per MP §4) run in
  CI as **advisory** comments only.
- **Release gate**: the Phase 8 exit includes a clean full-suite run per
  claimed platform with no unresolved hard regressions (MP §16).
- Rationale: shared CI runners have uncontrolled noise; numbers from them are
  not evidence (§1.2). Nothing slow, flaky, or hardware-dependent belongs on
  the merge path (doc 10 §10 philosophy).

### 3.9 Anti-gaming rules

1. The bench host uses the production command layer; engine internals are not
   reachable (enforced by the same boundary check as doc 05 §7.4 bypass
   prevention).
2. No benchmark-conditional code paths: a build flag grep in CI fails on
   `cfg(feature = "bench")` in engine crates `[PROPOSAL]`.
3. Corpus items are content-hashed at run time; the manifest hash is asserted.
4. Each repetition is a fresh process; no warm decode caches across runs.
5. Drop/underrun counters are read from the engine, not inferred from timing.
6. The harness code is reviewable in-repo; results link the exact harness
   commit.

---

## 4. Baseline protocol

### 4.1 Capture procedure (Phase 0 and Phase 2)

- **Phase 0** (doc 02 §1 explicitly scopes this): pin VLC 3.0.23 per platform
  (doc 02 §2.2–2.3), write the hardware templates for the reference machines,
  and fix this protocol. No performance numbers are produced.
- **Phase 2** (first vertical, MP §16): capture the **VLC-only baseline** for
  the metrics that exist on the first-slice corpus (M-01..M-05, M-08) on each
  reference platform, plus the first **Lumen** numbers on the same items —
  establishing both sides of the first honest comparison and validating the
  M-03/M-04 VLC log-line method (§2.3).
- **Phase 3+**: extend per corpus tier as formats land; each codec that gains
  a decode path gets its corpus items baselined before its optimization work
  starts (§1.1).
- **Phase 8**: full per-platform re-capture on the release build, which
  becomes the published performance-vs-VLC report (§9.2).

Every capture session records: date, machine_id, OS/driver state, corpus
manifest hash, VLC pin, Lumen build, harness commit, governor setting, noise
check result. A baseline without these fields is invalid.

### 4.2 Where baselines live

- `docs/baselines/perf/<platform>/<suite>.json` — committed, human-reviewable
  distilled medians + p95 + σ, with the full manifest fields.
- `results/` — raw per-run artifacts, gitignored, archived per doc 10 §4.5
  evidence store; each baseline entry links its run-id.
- **Reconciliation needed**: doc 02 §2.3 places the VLC pin in
  `docs/baseline/` (singular). This document proposes `docs/baselines/`
  (plural) as the parent for `vlc-3.0.23.*` + `perf/` — the Lead picks one
  spelling and doc 02/12 align (§10 OQ-3).

### 4.3 Re-baselining rules

- **New VLC patch version**: never silently. The baseline stays 3.0.23 unless
  the Lead re-pins with a new pin record (doc 02 §2.5). A 3.0.24 comparison
  may be run *in addition* and is always labeled separately.
- **New hardware**: new `machine_id`, new baseline set; the old set is kept
  and marked superseded (historical comparisons stay valid for their machine).
- **Environment drift** (OS upgrade, driver update, compositor change on
  Linux): the change is annotated in the machine record; if any P0 metric
  shifts > 10 % on a no-code-change re-validation run `[PROPOSAL: threshold]`,
  the baseline is re-captured and the shift is documented in the perf report —
  old and new are never mixed in one trend line.
- **Scheduled re-validation**: monthly no-code-change run of the quick suite
  per machine; drift beyond noise resets the baseline (this is what keeps
  regression detection honest over months).

---

## 5. Targets and budget

### 5.1 Provisional target table `[PROPOSAL]`

Explicitly provisional until baselines exist (§1.1); every target is either an
**absolute cap** or a **ratio to VLC 3.0.23 on the same item/machine**. R = the
recorded VLC median for that item.

| Metric | v1 provisional target (desktop, reference hw) | Phase 8 goal |
|---|---|---|
| M-01 startup (warm) | < 800 ms to command-layer ready; cold < 2.5× warm | < 500 ms / 2× |
| M-02 probe latency (local) | < 100 ms per corpus container | < 1.5× R |
| M-03 TTF | ≤ 2.0× R and < 1500 ms absolute (1080p mid-tier) | ≤ 1.3× R |
| M-04 TTFS | ≤ 2.0× R and < 300 ms absolute | ≤ 1.3× R |
| M-05 seek (local, warm) | ≤ 2.0× R and < 500 ms p95 (480p/1080p tiers) | ≤ 1.3× R, < 300 ms |
| M-06 steady CPU | ≤ 2.0× R software-decoded P0 formats; ≤ 1.3× R on hw-decode paths | ≤ 1.5× R sw / 1.15× hw |
| M-07 GPU | report-only v1 | within 1.5× R |
| M-08 memory | peak RSS ≤ 1.5× R; steady bounded by queue config (§6.5) | ≤ 1.2× R |
| M-09 dropped frames | 0 in steady window, ≤ 1080p30 tiers on reference hw | 0 through 2160p hw |
| M-10 audio underruns | 0 in every soak run (hard) | 0 |
| M-11 rebuffer | 0 engine-caused rebuffers on LAN-hosted corpus at 2× bitrate headroom | 0 |
| M-12 energy | report-only | report-only |

Web targets are a separate family (§3.7): startup budget = fetch+compile+
instantiate within the doc 09 §2.4 size budgets on mid-tier hardware; TTF/TTFS
compared only against the web build of prior releases (there is no VLC-on-web
oracle), recorded per browser tier per doc 09 §8.

### 5.2 Escalation ladder when a target is missed

1. **Profile** the failing scenario with the platform toolchain (§7); the
   profile artifact is attached to the miss record.
2. **Optimize** under the evidence rules (profile-first, before/after numbers,
   PR template §7.4); repeat until target met or gains exhaust.
3. **Document the gap** if it is fundamental (e.g. pure-Rust H.264 software
   decode vs VLC's assembly-tuned FFmpeg — doc 04 §7 gap #1): write the gap
   into the perf report with the profile, the measured ratio, and the
   compensating path (hardware decode per ADR-001, resolution-tier guidance);
   set a revised provisional target **with Lead approval**, recorded beside the
   original so the history is visible.
4. Never: relax correctness/validation to move the number (§1.4), re-define
   the metric, or compare against a differently-configured oracle.

### 5.3 Platform caveats

- **macOS ARM (Apple Silicon)**: no governor control; performance-core/
  efficiency-core migration inflates variance — macOS medians carry the noise
  caveat, p95 is secondary, and M-06 attributes per-thread so P-core vs E-core
  residency is visible. ARM and x86 machines are separate baselines, never
  averaged.
- **Linux**: compositor recorded (X11 vs Wayland present paths differ);
  reference machines run a pinned compositor during sessions.
- **Windows**: WPR/ETW overhead noted for profiled runs — timing numbers come
  from un-profiled runs, profiling runs never feed the baseline.
- **Browsers** (doc 09 §8): per-browser baselines; single-thread v1 wasm vs
  desktop are different families; Safari/Firefox differences (WebGPU vs
  WebGL2, doc 09 §4.4) are recorded per run, since the active renderer backend
  changes M-06/M-07 legitimately.

---

## 6. Architecture-for-performance plan (Trace: MP §11)

### 6.1 Zero-copy frame path — design sketch and the copy ledger

Target path (native desktop), building on doc 03 §2/§4.4/§4.11:

```
input ring (pooled, ref-counted byte blocks)
  │  demuxer parses headers from borrowed slices — no copy for header work
  ▼
Packet { PacketBuf }            ← pooled, ref-counted; aliases the ring block
  │                               when the payload is contiguous
  ▼
Decoder (software) → FrameHandle  ← planes from a per-session FramePool;
  │                                 reference frames codec-owned, pooled
  ▼
video-engine scheduler → PresentRequest { FrameHandle, params }   (move, no copy)
  ▼
renderer-wgpu: CPU→GPU upload    ← the single allowed frame copy (doc 03
                                    §4.11(e)); zero-copy via layout-compatible
                                    import where the backend supports it
                                    [VERIFY: per-backend capability, recorded
                                    per platform]
```

Audio side: decoder → audio-engine conversion **in place** on pooled buffers →
SPSC ring → the one `memcpy` into the callback buffer that doc 03 §4.9(c)
mandates.

**Copy ledger** `[PROPOSAL]`: a reviewed, in-repo table enumerating every copy
in the frame/sample path — network read → input ring; non-contiguous packet
extraction (e.g. stream-splice/unescape cases); decoder-internal management;
CPU→GPU upload when import is unsupported; the audio callback copy; the web
paths below. Each entry carries its justification and its measured cost (from
profiling, §7). Rule: a new copy requires a ledger entry in the same PR;
copies with no measurable cost justification get removed.

Web variants (doc 09 §4.2): wasm-linear-memory → GPU (one copy),
`ImageBitmap` transferable handoff (zero additional on the main thread),
WebCodecs `VideoFrame` → `GPUExternalTexture` (zero-copy import). The
video-in-webview canvas-streaming fallback (doc 01/08 Phase 2 prototype
option) is explicitly a copy-ful path and, if chosen, gets a ledger entry with
its measured cost — that measurement is an input to the prototype decision.

### 6.2 GPU color conversion

Per doc 03 §4.11(e): YUV→RGB conversion happens in WGSL shaders on the GPU;
scaling/aspect in the same pass; subtitle alpha-compositing on-GPU (doc 03
§4.11 [PROPOSAL]). The CPU conversion path exists only as the testkit
reference implementation. naga translates the shaders once per backend (doc 09
§4.1), so the web build shares the identical conversion code — one code path
to optimize, everywhere. M-07 tracks whether the GPU budget stays within the
target while doing so.

### 6.3 SIMD policy — measure first, vectorize with evidence

- **Hot-loop registry** (candidates, confirmed by profiling before any work):
  PCM format conversion (i16/i24/f32), channel remap and interleaving, volume
  ramps (audio-engine, doc 03 §4.8); resampler inner loop; MDCT/IMDCT in
  transform codecs (AAC, Vorbis); FLAC fixed predictors and LPC; MP3 synthesis
  filters; probe scoring loops (media-probe, doc 03 §4.2). `[PROPOSAL — the
  registry itself; membership moves with evidence.]`
- **Method per function**: (1) portable scalar baseline with conformance
  vectors and differential tests; (2) `criterion` micro-benchmark (verified:
  crates.io, updated 2026-02-04); (3) vectorized implementation; (4) keep it
  only with ≥ 15 % micro-bench gain `[PROPOSAL: number]` **and** either a
  measurable end-to-end effect on an M-metric or a documented
  critical-path argument; (5) differential test vector-vs-scalar over random +
  corpus data — bit-exact for integer paths, documented tolerance for float
  paths where reduction order differs.
- **Toolchain choice**: `std::simd` (`portable_simd`) is **still
  nightly-only** (verified 2026-10-19: it remains in the unstable book,
  doc.rust-lang.org/unstable-book/library-features/portable-simd.html) and the
  project pins a stable toolchain (`rust-toolchain.toml`) → unavailable until
  stabilized. Candidate safe, pure-Rust portable-SIMD crates, verified on
  crates.io (2026-10-19): `wide` (updated 2026-09-14), `pulp` (updated
  2026-06-20). Any choice goes through the doc 11 dependency audit (transitive
  graph, internal-unsafe review, license); selection is a Lead decision (§10).
- **Runtime feature detection** for ISA dispatch; scalar fallback always
  builds and is always tested (it is the reference).
- Decoder reality check: for AV1, doc 04's rav1d-with-asm-off posture (if
  adopted) means the SIMD program is our main lever; for H.264/HEVC software
  decode the gap-register reality (§5.2 step 3) applies — we do not pretend
  SIMD closes an assembly-sized gap.

### 6.4 Hardware decode and truthful decoder attribution (Trace: ADR-001)

Every benchmark run records **which decoder produced it**: the harness queries
`system.get-engine-info` (doc 05 §2.12 — `decoders[].active`,
`hardware: true` + backend name) at run start, on `tracks.changed`, and at run
end; the manifest stores the last-wins decoder per track. Consequences:

- Software-vs-software comparisons pin VLC to software decode (§3.3); results
  are labeled `sw-vs-sw`.
- A separate `hw-vs-hw` suite runs where both sides use OS decoders
  (VideoToolbox on macOS; Media Foundation/D3D11 on Windows; VA-API on Linux)
  — these compare our *integration* quality, not the decoders.
- Any published number states the decoder that produced it (§9.3). A run where
  Lumen silently fell back from hw to sw mid-run (device loss) is marked
  `degraded` and excluded from medians — reported, never hidden.

### 6.5 Bounded memory as an invariant, not an optimization

Bounded queues (doc 03 §4.4) are a correctness invariant (MP §8) that the perf
program **asserts**, never tunes per benchmark: the harness reads
`queue_resident_bytes` (§2.4) during every scenario and fails the run if a
queue exceeds its configured bound; wasm `Memory` maximum from
`product.toml` `resource_limits` (doc 09 §2.4) is asserted the same way. A
"performance win" obtained by raising bounds is a config change through the
normal settings path, visibly recorded — not a benchmark trick.

### 6.6 Allocation discipline in realtime paths (Trace: doc 03 §4.9, MP §8)

- **Audio callback**: zero allocation, enforced by testkit's fault-injecting
  allocator + lock tracer (doc 03 §4.9(c)) — these gates run in the perf suite
  too, not only in unit tests.
- **Pre-allocated pools**: packet pool and frame pool sized at session open
  (from negotiated format/resolution), grown only at documented events
  (resolution change), never in steady state; pool growth is a counter the
  harness watches (growth during steady state = defect).
- **Steady-state allocation rate ≈ 0** as a measured property: debug-build
  runs with the counting allocator report allocs/s during the steady window;
  nonzero rates on the decode/present path get filed as defects.
  `[PROPOSAL: counting-allocator wiring in testkit.]`

---

## 7. Profiling program

### 7.1 Toolchain per platform

| Platform | CPU time | Allocations/memory | Notes |
|---|---|---|---|
| Linux | `perf record -g` + `flamegraph` crate/cargo-flamegraph (verified: crates.io, updated 2026-08-12) | `dhat` (verified; last crate update 2024-02 — slower cadence, noted), heaptrack, valgrind | `iai-callgrind` (verified: updated 2025-07-30) gives deterministic instruction counts for CI-stable micro-comparison; LSan nightly per doc 10 §6.5 |
| macOS | Instruments via `cargo-instruments` (verified: updated 2026-06-04): Time Profiler template `[VERIFY current template names on shipping Xcode]`; `sample` CLI | Instruments Allocations/Leaks template (manual tier) | GUI-tier tooling: runs on the manual tier, artifacts attached to miss records |
| Windows | ETW: Windows Performance Recorder/Analyzer (WPR/WPA), `xperf` CLI `[VERIFY current names/availability]` | WPA heap analysis | profiled runs never feed timing baselines (§5.3) |
| Web/wasm | Chrome DevTools Performance (wasm frames `[VERIFY: DWARF symbolization status for Rust wasm]`); Firefox Profiler (profiler.firefox.com) | Chrome Memory panel; heap snapshots `[VERIFY wasm heap attribution]` | per-browser; artifacts stored under results/<run>/profiles/ |

### 7.2 Flamegraph integration and retention

`cargo xtask profile <scenario>` `[PROPOSAL: xtask subcommand]` wraps the bench
harness scenarios with the platform profiler, saving flamegraph SVGs (plus
perf.data/ETL raw where small enough) under `results/<run-id>/profiles/`.
Every target miss (§5.2) and every optimization PR links one. The dashboard
(doc 10 §9.3) gains a perf row: latest medians vs baseline, open misses, soak
slope.

### 7.3 Profile-before-optimize — enforcement

The rule is procedural and review-enforced (AGENTS §3.9 puts review gating
with the Lead): **no optimization PR merges without a cited profile, and no
performance claim without numbers**. Concretely: the PR template section
(§7.4) is mandatory for hot-path crates (§3.8 list); reviewers reject
"this should be faster" changes with no artifact, and reject number-free
claims exactly as doc 10 §2.4 rejects unverified test claims. Mechanization
beyond the checklist is a follow-up (§10 OQ-7).

### 7.4 Optimization PR template `[PROPOSAL]`

```markdown
## Performance change
- Symptom / metric:        <M-## or micro-bench, scenario, corpus tier>
- Profile evidence:        <tool + command + artifact link — REQUIRED before change>
- Change:                  <approach, in one paragraph>
- Evidence after:          <exact bench command + before/after median, p95, env>
- End-to-end effect:       <M-metric delta, or "none measured — rationale">
- Correctness:             <conformance + differential tests run, fuzz target executed>
- Ledger:                  <copy ledger / hot-loop registry entries updated, if any>
- Risks:                   <portability, fallback paths, bounds>
```

---

## 8. Soak and endurance performance (Trace: doc 10 §6.5)

### 8.1 Scenarios and assertions

Built on doc 10 §6.5 (4 h playback, looping playlists, seek-storm, stress)
with the perf-owned extensions:

- **Memory slope**: RSS + queue-resident-bytes sampled every 60 s; linear
  regression slope < 5 MB/h `[PROPOSAL — doc 10's number, frozen after first
  baseline]`; hard fail on any queue-bound violation (§6.5).
- **Queue stability**: queue-depth histograms for minute 1 vs minute 230 —
  distributions must not drift (KS-test or simple quantile comparison
  `[PROPOSAL: method]`).
- **Reopen cycles**: 1 000 × open → play 5 s → stop → close on mixed corpus;
  thread and fd counts (testkit counters, doc 03 §8) flat; RSS within ±5 % of
  the post-first-cycle level.
- **Device-loss cycles** (audio): repeated `VanishingSink`-driven loss on the
  soak tier; underrun counter must stay 0 outside the loss windows
  (doc 03 §4.9 policy).
- **CPU drift**: steady-state CPU sampled hourly; > 10 % drift up = defect
  (scheduler degradation, leak-ish growth in work).

### 8.2 Leak detection method

Layered: (1) testkit thread/fd counters per scenario (always on); (2) nightly
LeakSanitizer run of parser crates on Linux (doc 10 §6.5 `[PROPOSAL — nightly
toolchain component]`); (3) `dhat` allocation-site diff between soak minute 1
and minute 30 — any site with monotonic growth is filed; (4) macOS Instruments
Allocations mark-based persistent-growth check on the manual tier, per
release. Soak evidence is labeled per doc 10 §7.1 (real-hardware vs container
runs differ — soak in containers measures memory/thread behavior, not timing).

### 8.3 Results ledger

`results/soak/ledger.json` `[PROPOSAL]` — append-only, one entry per run:
date, machine_id, build, duration, scenarios, slope, drift, verdicts, artifact
run-id. The dashboard consumes it (doc 10 §9.3 "soak memory slope" row). A
release requires the latest ledger entry per platform to be green.

---

## 9. Reporting

### 9.1 Cycle-report integration (Trace: MP §17, doc 10 §9.1)

Cycle reports gain a **Performance** block whenever the cycle touches
hot-path crates or claims perf work: metric, suite command, actual medians/
p95 vs baseline, env fingerprint, artifact link. "Verified" means the exact
command and actual numbers — e.g. `cargo xtask bench --suite latency --quick
--runs 10 → results/2026-XX-XX-a; TTF 1080p-mid: 412 ms (baseline VLC 388 ms,
prior Lumen 505 ms)`. A perf claim with no run-id is not verified, full stop.

### 9.2 The Phase 8 performance-vs-VLC report

The Phase 8 gate artifact (MP §16): per metric × per platform × per corpus
tier, Lumen vs VLC 3.0.23, with decoder attribution (§6.4), method notes
(§2.3 asymmetries), and the ✓ / partial / ✗ vocabulary mirroring doc 04 §7 —
including honest ✗ rows for fundamental software-decode gaps with their
documented rationale (§5.2 step 3).

### 9.3 Public honesty rules (marketing-adjacent claims)

1. No "faster than VLC" claim without the recorded corpus, hardware, builds
   and both medians — claims are quoted as: metric, corpus tier, machine,
   Lumen build vs VLC 3.0.23 pin, decoder attribution.
2. Losses are published with wins in the same table; selective quoting of
   only favorable tiers is a violation.
3. Hw-vs-sw mixed comparisons are labeled as such or not made.
4. Web claims name the browser and threading mode (doc 09 §3).
5. These rules bind README, release notes, and any site copy, exactly as
   parity claims bind (MP §1 "never claim … beyond that evidence").

---

## 10. Open questions for the Lead (decisions requested, per AGENTS §3.6)

1. **Doc 05 envelope `ts_ns`** (§2.4) — the single highest-leverage
   instrumentation decision; without it every latency metric stays
   host-measured.
2. **The §2.4 event/command additions** (`session.probe-completed`,
   `video.first-frame-presented`, `system.get-stats`) — additive per doc 05
   §6, but they widen the public contract; approve or restrict to a
   testkit-gated feature.
3. **Baseline directory naming**: `docs/baselines/` (plural, this doc) vs
   doc 02's `docs/baseline/` — pick one.
4. **Reference hardware**: which machines per platform are the pinned
   benchmark machines, who maintains them, and are self-hosted nightly
   runners in scope (doc 10 §4.1 assumed GitHub-hosted)?
5. **VLC automation surface** confirmation (rc vs `--play-and-exit` split,
   §3.3) — shares doc 02 M1/M4 `[VERIFY]` items; the Phase 2 baseline must
   settle the log grammar.
6. **SIMD crate choice** (`wide` vs `pulp` vs waiting for `std::simd`
   stabilization) — dependency-audit implications; decision can wait for the
   first profiled hot loop.
7. **Regression thresholds and enforcement** (§3.6 numbers; whether
   `perf-check` label runs block, and whether nightly hard regressions block
   `main` or only releases).
8. **Energy proxy scope** (M-12): is privileged tooling (`powermetrics`) on
   the manual tier acceptable, and is RAPL-based reporting wanted at all in
   v1?
9. **Bench-corpus generation tooling**: doc 10 §5.3's generator pipeline —
   confirm the approved generator (and that a dev-only ffmpeg/ffmpeg-like
   dependency for fixture synthesis is clean-room-acceptable, never shipped).
10. **Browser perf tier**: which browsers get full perf baselines (doc 09 §8
    matrix is functional) — proposal: Chromium + Firefox nightly, Safari
    manual tier.

— End of 12-performance-benchmarking.md —

