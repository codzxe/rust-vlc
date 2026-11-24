# CLAIM AUDIT: docs/audits/error-surface-20261023.md (333 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 8 REAL / 1 REAL-UNPROVEN / 0 / 0 / 0 / 0 / 3 UNVERIFIED (git commit baseline; clippy experiment re-run; 2026-10-23 counts for grown crates) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This is the strongest doc in my assignment: a self-auditing report whose every checkable
assertion is true, and whose ranked fix list was FULLY EXECUTED and is verifiable in today's
code. The raw-sweep method reproduces exactly on the five crates that have not grown since
2026-10-23 (input 37, packet-queue 3, audio-engine 2, video-engine 21 — byte-identical
counts to the doc's table), and the four crates that diverge (codecs 516 vs 72, demux 694
vs 200, media-clock 78 vs 36, playback-engine 11 vs 9) diverge because engine code landed
after the audit's dated baseline (`main @ a63fe22`). Fix items F-1 through F-5 all exist in
the tree today, each explicitly citing this audit: WiredKind enum (tracks.rs:156), contract
modules cfg-gated (input lib.rs:64,72), const-position `None => loop {}` cap
(limits.rs:15-31 "audit F-3"), the recycle-path regression test
(pipeline.rs:3282-3291 "Audit F-4"), and the check-panics xtask gate (check_panics.rs:1-2
"card 7-e, audit F-5 of docs/audits/error-surface-20261023.md"). The one gap: F-5's
mandate was "encoding this audit as CI" — `check-panics` is registered as an xtask
subcommand but is NOT wired into scripts/full_gate.sh or any .github/workflows/ file
(rg -> 0 hits); it appears only as an ad-hoc lane in docs/soak/soak-report-20261110.md:184.
The panic-surface gate exists but nothing in the merge-blocking path runs it.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Raw sweep: "452 raw matches" across nine engine crates | :21 | REAL at its date; 4 crates grew since | My re-run (rg count): input 37, packet-queue 3, audio-engine 2, video-engine 21 — EXACT match to doc table; playback-engine 11 (was 9), media-clock 78 (was 36), demux 694 (was 200), codecs 516 (was 72), media-probe 85 (was 72) — post-audit code growth (codec backfills). |
| 2 | "28 matches: 1 `expect` + 27 `unreachable!`; zero unwrap/panic/todo/unimplemented in non-test code" | :37-39 | REAL at date; F-1..F-4 since removed the worst | WiredKind retired the production expect (tracks.rs:150-156); F-4 deleted the recycle unreachable! (pipeline.rs:3282-3291 comment "The retired fallback ... would have panicked"). |
| 3 | "the workspace denies the panic surface at Cargo.toml:49-56" | :46-47 | REAL (line drifted) | Block exists; current position :53-66 (manifest grew: apps/desktop-tauri exclusion at :47-53). Same 11 deny entries. |
| 4 | "One sanctioned local allow ... playback-engine/src/pipeline.rs:1062" | :48-50 | WAS TRUE; now retired by F-1 | sed pipeline.rs:1062-1066 today: `for wired in [WiredKind::Video, WiredKind::Audio, WiredKind::Subtitle]` — the allow + expect are gone; WiredKind enum landed (F-1). |
| 5 | Clippy blind spot: `unreachable!` not covered by the panic deny (controlled /tmp experiment) | :52-61 | REAL-UNPROVEN (re-run) | The blind spot is architecturally confirmed by the follow-up doc docs/audits/error-surface-fixes-20261024.md:19-58 (seeded-violation proof); I did not rebuild the throwaway crate (cargo-build-budget). |
| 6 | packet-queue `cap()` unreachable at limits.rs:21-26, fix F-3 proposed | :119, :305-307 | REAL — FIXED | limits.rs:15-31 today: const-position `None => loop {}` form with comment "audit F-3, the video-engine policy.rs:186-190 precedent" and scoped #[allow(clippy::empty_loop)]. |
| 7 | audio-engine pipeline.rs:432-436 unreachable on fresh SharedBytes | :188 | REAL (still present) | sed :430-436 verbatim today: `None => unreachable!("fresh SharedBytes has refcount 1")` — doc-commented invariant, unchanged. |
| 8 | input/src/contract.rs 23 unreachable! executable-contract; fix F-2 proposes feature gate | :219-241 | REAL — FIXED | input lib.rs:64,72 `#[cfg(any(test, feature = "contract"))]` gates contract modules; lib.rs:32 comment cites "audit F-2". |
| 9 | media-probe contract.rs:79 same pattern | :253-260 | REAL — FIXED | Same cfg-gating treatment (fixes doc :12 F-2 row names both crates). |
| 10 | video-engine policy.rs:186-190 const-position `None => loop {}` "best practice" | :203-207 | REAL (verified as precedent) | limits.rs doc (:17) independently cites "the video-engine `policy.rs:186-190` precedent" — cross-confirmation of the pattern's existence. |
| 11 | F-5: "adding a grep-based step to the gate ... encoding this audit as CI" | :313-317 | PARTIAL: command exists, NOT CI-wired | tools/xtask/src/cmd/check_panics.rs exists (header: "the F-5 panic-surface gate"); registered in cmd/mod.rs:20; but rg over scripts/full_gate.sh + .github/workflows/*.yml -> 0 hits for check-panics. Only usage: docs/soak/soak-report-20261110.md:184 lane list. |
| 12 | Baseline: "`main @ a63fe22` (wave-5 dispatch base; branch `wip/w5-20`)" | :8 | UNVERIFIED (git) | No git in sandbox; commit/banch recorded in worklog docs. |

## 3. Contradictions
None found. Cross-checks: the follow-up closure doc
(docs/audits/error-surface-fixes-20261024.md) claims F-1..F-5 all "done" with commit
hashes — my independent code inspection confirms each fix is physically present today,
so the closure doc's claims and this doc's fix list agree with the tree.

## 4. Fabrication / inflation indicators
None. The doc self-reports its own blind spots (:319-331 "Honest gaps": direct-indexing
panics not dataflow-proven, overflow posture, mid-flight sibling branches) and its counts
reproduce exactly on unchanged crates.

## 5. Follow-through status (independent re-verification, 2026-11-24)
| Fix item | Claimed | Verified in tree today | Evidence |
|---|---|---|---|
| F-1 WiredKind total for_kind | done (fixes doc :13) | YES | tracks.rs:150-165 WiredKind enum; pipeline.rs:1062-1066 uses it |
| F-2 contract cfg-gating | done (:14) | YES | input lib.rs:32,64,72 |
| F-3 const-position cap | done (:15) | YES | limits.rs:15-31 |
| F-4 recycle-path descriptor | done (:16) | YES | pipeline.rs:3282-3291 regression test |
| F-5 check-panics gate | done (:17) | YES as command; NO in full_gate.sh/CI wiring | check_panics.rs + cmd/mod.rs:20; 0 gate/workflow references |
