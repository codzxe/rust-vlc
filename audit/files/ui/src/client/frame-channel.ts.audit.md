# ui/src/client/frame-channel.ts — the webview receive half (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| File | `ui/src/client/frame-channel.ts` (448 LOC) — the w31.4 shim + LUF1 decoder |
| Reachability | lumen-tauri binary → ui/dist → MediaSurface's `FrameCanvasSurfaceLive` (line 126 `new FrameChannelShim(backend, { onRecord })`) → `transformCallback` registration + `window._<id>` → shell's `frame_channel.rs` ships `InvokeResponseBody::Raw` → the tauri channel protocol reaches this shim's `handleCallback` → `drain` → `resolveBytes` → `onRecord(record)` → MediaSurface's decode+blit. |
| Verdict | **Real, disciplined port of the @tauri-apps/api Channel mechanism, and the LUF1 decoder is a genuine byte-mirror of wire.rs.** It is also the layer with the most runtime-unverifiable surface in the whole GUI chain: every tauri-protocol shape claim (2.1.x vs 2.12.x eval targets, fetch-token form) is pinned only against mocked internals. |

## Per-function table

| Unit | Lines | What it does (sed-verified) |
|---|---|---|
| `LUF1` consts | 39-52 | MAGIC `0x4c 0x55 0x46 0x31`, VERSION 1, MIN_HEADER_LEN 48, MAX 1024, payload cap 1 GiB, dimension cap 65_535, FORMAT_RGBA8=1 / FORMAT_BGRA8=2 — matches `apps/desktop/src/wire.rs:70-76` (rg-verified parity of constants). |
| `parseLuf1Header` | 112-172 | strict little-endian header parse: magic/version/format/reserved/geometry/payload_len == w·h·4; u64 fields through the safe-integer guard (94-103). |
| `decodeLuf1Record` | 185-204 | one-record form: truncated / trailing-bytes refusals; payload = subarray(headerLen..total). |
| `detectFrameChannelBackend` | 244-249 | `globalThis.__TAURI_INTERNALS__` presence probe (invoke+transformCallback both functions). |
| `FrameChannelShim` | 298-447 | constructor registers via `transformCallback` (315-317) AND `window._<id>` (323-329, configurable so 2.1.x redefinition is safe); `handleCallback` (348-378) serves both `{message, index}` (2.12.x) and `{message, id}` (2.1.x) frames, honors `{end: true}`, drops stale indices; `drain` (387-415) is a guarded in-order loop; `resolveBytes` (418-447) accepts ArrayBuffer / typed view / fetch-token string (`plugin:__TAURI_CHANNEL__|fetch` with `Tauri-Channel-Id` header, lines 433-437); `close` (333-345) deregisters both. |

Key quotes (sed-verified):
- Line 212: `export const FETCH_CHANNEL_DATA_COMMAND = "plugin:__TAURI_CHANNEL__|fetch";`
- Lines 28-36 (honest limits): "verified against tauri 2.1.1 and 2.12.5 (the shipped Cargo.lock version) channel sources — in 2.12.x the shell's own eval'd script performs the fetch before `runCallback` hands the callback an ArrayBuffer, so the token-string branch is the DEFENSIVE path ... No webview exists in this sandbox (apps/desktop-tauri/INFRA-BLOCKED.md), so the runtime shapes are pinned by unit tests against mocked internals".

## Findings

- **S2 — protocol-shape risk concentrated here.** The 2.12.5 claim rests on reading tauri's source at two versions, not on a run: if the shipped shell's injected script ever hands the callback a token string without the shell performing the fetch (the exact drift the comment fears), the shim's fetch requires a capability grant (`plugin:__TAURI_CHANNEL__|fetch` — see tauri.conf.json audit, S4 open question). The failure mode is loud (typed diagnostic per message, lines 401-410) but total: zero frames, dark canvas, console-only evidence. UNVERIFIED at runtime.
- **S3 — `handleCallback` deduplicates the two registration routes** (transformCallback target + `window._<id>`) into one ordering queue — but nothing deduplicates a tauri that delivers the SAME index through both routes with different payload wrappers: `pending.set(index, ...)` (line 376) last-write-wins. The stale-index guard (372-375) protects against replay of older indices only. Theoretical; no committed test covers double-delivery of one index. Recorded.
- **S4 — `nextIndex` advances past a failed fetch** (402-410): a message skipped for a fetch error is dropped forever (the pending map moves on). Intentional ("never a stall"), matches the 2.12.x api class's documented behavior. One frame lost per fetch failure, uncounted beyond the diagnostic.
- **S4 — `close()` is invoked on the `{end: true}` marker** (365-370): the shell's channel teardown closes the consumer; a re-mount re-binds. Consistent with the shell's "rebuild on every bind" (app.rs:191-193).

## Test honesty

`frame-channel.test.ts` (498 LOC): decoder byte-mirror conformance (119-241), header parser refusals (242-303), shim ordering/pending/end/close against a fake internals (304-474), the probe (475+). The suite honestly labels the mocked posture in the module header (lines 1-11). Same ceiling as every UI test: no webview, no runtime.

## Doc claims

- Header claims byte-mirror status vs `wire.rs` — verified by this audit: the constants and record layout match `wire.rs:18-38` exactly (offsets 0/4/6/8/9/10/12/16/20/24/32/40/48 in both files).

## Dead code / deps / flags

- No dead exports. No `@tauri-apps/api` import (the allowlist rule, lines 13-16) — the hand-rolled posture is documented and consistent with `tauri-transport.ts`.

## Open questions

1. `Object.defineProperty(globalThis, this.propName, { writable: false, configurable: true })` (323-329): on a 2.1.x shell `transformCallback` already defined the prop — redefining a non-writable-but-configurable prop is legal ES; verified against spec knowledge, not a runtime 2.1.x shell (none exists here). Dormant.
