# CLAIM AUDIT: docs/dependencies/records/wgpu-30.0.1.md
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Claims extracted | 11 |
| REAL: 6 · REAL-UNPROVEN: 3 · SHALLOW: 0 · STUB: 0 · PAPER-ONLY: 0 · FALSE: 0 · UNREACHABLE: 0 · STALE: 2 (counted under S4 below) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This is the most honest document I have audited in this wave: it records a *decision NOT to import*, lists exactly what it could not run (cargo-deny absent), and every code-level and environment-level claim I could re-probe today reproduced. The only defects are two staleness items (deny.toml gained a scoped libloading wrapper after the record's date; and the earlier allowlist-format "1 record" universe) plus three network-bound claims I cannot re-run.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "This record's verdict: conditional (E2), NOT admitted in 2.D1 — the card's software-path fallback is executed instead." | wgpu-30.0.1.md:17-19 | **REAL** | `rg -c 'name = "wgpu"' Cargo.lock` → exit 1 (no wgpu in lockfile); `crates/renderer-wgpu/Cargo.toml` [dependencies] = `media-types`, `video-engine` (path only) — no wgpu. |
| 2 | "renderer-wgpu v0 ships the complete renderer architecture behind the internal `Compositor` seam, std-only" | :93-94 | **REAL** | renderer-wgpu/Cargo.toml deps section (quoted above) has zero external crates; src/ has backend.rs/surface.rs/renderer.rs/staging.rs/synthetic.rs + tests. |
| 3 | "`PresentMode::wgpu_present_mode()` (the exact wgpu::PresentMode variant names)" | :100-101 | **REAL** | crates/renderer-wgpu/src/surface.rs:60 `pub fn wgpu_present_mode(self) -> &'static str`, with tests :356-358 asserting "Fifo"/"Mailbox"/"Immediate". |
| 4 | "`SurfaceFormat::wgpu_texture_format()` (`Bgra8UnormSrgb`/`Rgba8UnormSrgb`)" | :101-103 | **REAL** | crates/renderer-wgpu/src/color.rs:45-48 — `SurfaceFormat::Bgra8Srgb => "Bgra8UnormSrgb"`, `Rgba8Srgb => "Rgba8UnormSrgb"`. |
| 5 | "`Renderer::with_backend` injection point (surface + compositor pair)" | :103 | **REAL** | crates/renderer-wgpu/src/renderer.rs:67 `pub fn with_backend(`. |
| 6 | AC "met by the software path … committed SHA-256 (`1466e74a…eb30bf` … `tests/png_hash.rs`)" | :113-118 | **REAL** | crates/renderer-wgpu/tests/png_hash.rs:73 `"1466e74a10fefa702f517f2fe09db4da97a12a7c40f2334a95834ac992eb30bf"` — exact prefix/suffix match. |
| 7 | Environment: "/dev/dri does not exist", "/etc/vulkan/icd.d/ exists and is empty", no /usr/share/vulkan/icd.d | :23-27 | **REAL** (re-probed today) | `ls /dev/dri` → "No such file or directory"; `ls -la /etc/vulkan/icd.d/` → total 0 (empty); `ls /usr/share/vulkan/icd.d/` → does not exist. |
| 8 | "cargo-deny is not installed in this sandbox session (the 1.E5-R pinned binary is gone)" | :68-69 | **REAL** (still true today) | `which cargo-deny` → rc=1, empty output. |
| 9 | "wgpu 30.0.1 is the newest crates.io release (2026-10). Default features = std, parking_lot, dx12, metal, gles, vulkan, wgsl, webgpu." | :55-56 | **REAL-UNPROVEN** | Requires crates.io API — no network in sandbox; not re-runnable. Nothing in-repo contradicts it. |
| 10 | "default-features normal-edge graph on Linux resolves to 80 unique packages … `libloading v0.8.9` is in that closure … via `ash v0.38` … and `dlib v0.5`" | :57-61 | **REAL-UNPROVEN** | Requires `cargo tree` on a throwaway project with wgpu downloaded — cargo-build budget / network. Cannot re-run; no in-repo counter-evidence. |
| 11 | "`deny.toml` bans `libloading` unconditionally — the workspace's own cargo-deny gate would fail the import as-is." | :63-64 | **REAL-UNPROVEN (conclusion) / STALE (wording)** | Current deny.toml:189 = `{ name = "libloading", wrappers = ["libappindicator-sys"] }` — a *scoped* ban, not unconditional; the wrapper exception postdates the record (deny.toml comment: "ADR-004 admission exception (issue #67, 2026-11-13)", record gathered 2026-10-20/21). The conclusion still holds: ash/dlib paths are not the allowed wrapper, so the import would still trip the ban — but that is reasoning, not a run (cargo-deny absent, claim 8). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "deny.toml bans libloading unconditionally" (:63) | deny.toml:180-189: "ONE scoped wrapper. libloading enters the LOCKFILE only through tray-icon … Any OTHER path to libloading still trips the ban" | Both describe the same net effect for wgpu (ash/dlib path trips); the word "unconditionally" was true at record time (2026-10-20/21) and is stale after the 2026-11-13 ADR-004 carve-out. S4 staleness, not a lie. |

## 4. Fabrication / inflation indicators
- None found. The record explicitly withholds claims it could not run (§2: "the ban violation is established at the manifest level above, not by a live gate run — recorded honestly"; evidence posture ¶: "The wgpu tarball itself was NOT inspected").
- The AC sentence "The wgpu path has no executable evidence here and none is claimed" (:118-119) matches repo reality (no wgpu anywhere in the lockfile).

## 5. Matrix rows
N/A — not a matrix chunk.

## 6. What actually works (verified)
- `ls /dev/dri`, `ls /etc/vulkan/icd.d/`, `which cargo-deny` (all exit 0/1 as cited) — environment claims reproduce today.
- `rg` over renderer-wgpu sources + tests/png_hash.rs — every code-level integration-point claim lands on a real symbol.

## 7. Open questions for next agent
- Claim 10 (80-package closure, libloading via ash) is the load-bearing reason wgpu was deferred; if a Lead ever waves it through on this record alone, it stays unverified — the owed §6 R1–R10 record has not landed (records/ has no full wgpu R1–R10 record as of this audit).
- oxideav-ac3-0.0.11.md (sibling record, another agent's domain) — check its honesty markers against this one's standard.
