# AUDIT: tests/integration/first-vertical/evidence/final-frame.png
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 307,541 bytes / 698af4ee249d8b31925ad958ba49ca4dcf1a530aa949c6aab30a8ed93cb553db |
| Type / dims | PNG 320x240, 8-bit RGBA non-interlaced |
| File verdict | REAL — but STALE vs the code that produces it |
| Reachable from shipped entry point? | NO — evidence artifact |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | sha256 batch; `file`; rg consumers; baseline check |

## 1. One-paragraph verdict
The committed "final frame" screenshot evidence of the first vertical: a real 320x240 RGBA PNG (dims match the fixture media's 320x240, asserted by tests/integration/first-vertical/tests/evidence.rs:171-174). Guarded by `final_frame_is_current` (evidence.rs:152-175): a fresh run-to-end must produce BYTE-IDENTICAL PNG bytes, and the run-to-end transcript must name the committed frame's sha256 + 320x240 extent. That guard FAILS in the committed baseline (cargo-test-workspace.txt:2767 `test final_frame_is_current ... FAILED`) — the committed PNG has drifted from what the current code produces; the artifacts need `--ignored regen` (evidence.rs:181-183).
## 2. Findings
F-001 [S2]
Where: this file vs audit/runs/baseline/cargo-test-workspace.txt:2767, 2781
Code (baseline): `test final_frame_is_current ... FAILED` / `thread 'final_frame_is_current' (27978) panicked at tests/integration/first-vertical/tests/evidence.rs:162:5:`
What is wrong: the committed evidence artifact contradicts the current code output — the "evidence" the ledger cites does not match a fresh run (one of the repo's 2 failing tests). Why it matters: evidence/ artifacts are cited by docs as proof the first vertical works; a drifted frame means the proof is stale until regenerated. What correct looks like: run `cargo test -p first-vertical --test evidence -- --ignored regen` and commit. Confidence: HIGH — Verification: ran (baseline grep, no cargo needed).
## 3. What actually works (verified)
PNG is a real image with the asserted 320x240 extent; consumer test exists and DID catch the drift (the guard works — the failure is proof of honesty, not of a vacuous test).
