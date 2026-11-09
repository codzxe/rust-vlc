# Phase 6 Exit Gate — Streaming & Network (waves 6.A–6.H) — DRAFT

Status: **DRAFT — PENDING LEAD DECISION**.
Verdict basis: issue ledger (#6–#15) + this draft's honest parity snapshot
(`cargo run -p xtask -- parity-report`, this session). Gate-line runs
(test/clippy/fmt/fuzz-registry) are **PENDING** — to be executed at gate
time, not asserted here.

## 1. Evidence chain (to be completed at gate)

| Gate | Result | Evidence |
|---|---|---|
| `cargo test --workspace --all-features` | PENDING | to run at gate |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PENDING | to run at gate |
| `cargo fmt --all -- --check` | PENDING | to run at gate |
| fuzz registry diff | PENDING | `scripts/fuzz_registry_check.sh` at gate |
| parity snapshot | 60/252 implemented-or-verified per desktop OS; web 0/252; strict verified-only 0/252 | `cargo run -p xtask -- parity-report` (this session, wave 13) |

## 2. Issue ledger — landed with evidence

| Issue | Scope | Evidence (`git log --oneline --grep=…`) |
|---|---|---|
| #6 | buffering model + reconnect core: watermarks, stall prediction, bounded retry/resume, generation-safe cancellation, HLS wall-clock pacing | `c6a401e` (merge wave 5), `e51a9e7`, `ddcac31` |
| #10 | HLS master playlists + variant switching with ABR hysteresis | `b045721` (merge wave 8), `d2de397` |
| #11 | HLS conformance corpus + X-TIMESTAMP-MAP + records | `372d9e1` (merge wave 11), `f009a1a` |
| #12 | DASH template resolution + MPD fuzz + ABR scheduling | `27b17e1` (merge wave 11), `93b7614` |
| #13 | streaming auth (basic + RFC 7616 digest, redacted credential store) + remote M3U/XSPF | `2834c5b` (merge wave 6), `844141f`, `f0f9a17` |
| #14 | synthetic network fault proxy (6.F netfuzz) | **HONEST: not in main.** `dc8bf7b` (6.F M1) + `4fabef4` ("wip: netfuzz final", 2026-11-09) live on branch `w10/14-netfuzz` (rebased on main `ed24e7d`); merge/review outstanding. NOT counted as landed until merged. |

## 3. Remaining (registered, not hidden)

* **#8 TLS** — pending Lead decision (ADR-011 input; network-tls-policy
  matrix rows). No verdict recorded here.
* **#15 RTSP/RTP** — **DEFERRED** with typed refusal (this wave's
  investigation: `docs/investigations/rtsp-rtp-feasibility.md`; refusal
  pinned by `crates/streaming/src/schemes.rs:557-577`, `:677-682`).
  Revisit trigger: ADR-001 landed on 3 OS + lead-customer demand.
* **6.H1 security findings** — 1 Medium (same-host https→http scheme
  downgrade followed) + 3 Low, fix sketches recorded in
  `docs/investigations/network-security-review.md`; recommend scheduling
  the Medium before exit sign-off.
* Protocol-access category: 0/11 implemented (all blocked/deferred rows
  now evidence-linked in `docs/feature-matrix.toml`).

## 4. Honest parity (this session's run)

`cargo run -p xtask -- parity-report` (wave 13, at `ed24e7d`):

* implemented-or-verified: **windows 60/252 (23.8%) · linux 60/252 (23.8%)
  · macos 60/252 (23.8%) · web 0/252 (0.0%)** — rows=252.
* verified-only (MP §1 strict definition, automated evidence):
  **0/252 on every platform** — the strict-verification debt is real and
  is the honest headline of this draft.

## 5. Verdict

**PENDING LEAD DECISION.**
