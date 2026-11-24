# AUDIT: fuzz/corpus/fuzz_demux_wav/seed_wav_rf64.wav (parts 1-5, L1-3235)
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 35,394 bytes / 8478de97ce85374b10989f9c63a0d6855b14a5ff3d185a26078680894c08d4f0 |
| Type | MBWF/RF64 audio, stereo 44100 Hz |
| File verdict | REAL-UNPROVEN (valid RF64 seed; the fuzz target that consumes it is never executed — see F-001) |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 header parse; rg consumers; rg baseline |

## 1. Verdict
Structurally valid RF64: magic RF64, RIFF size field 0xFFFFFFFF (canonical RF64 sentinel), ds64 chunk at 12 with riffSize=35,386, dataSize=35,280; fmt PCM (tag 1) 2ch/44,100Hz/16-bit; data size field 0xFFFFFFFF sentinel (RF64 back-compat), actual payload 35,280 bytes = 35,394−106−8. Consistent. Seed for fuzz_demux_wav (target declared in fuzz/Cargo.toml `[[bin]] ... test=false`, path fuzz_targets/fuzz_demux_wav.rs).
## 2. Findings
F-001 [S3]
Where: fuzz/Cargo.toml (all 35 `[[bin]]` entries, e.g. lines ~295-315 pattern: `test = false` / `doc = false` / `bench = false`); audit/runs/baseline/ (no cargo-fuzz run exists in any baseline file — `rg "cargo-fuzz|libfuzzer" audit/runs/baseline/*` → no matches)
What is wrong: no committed evidence that any fuzz target ever ran with this seed; corpus seeds exist but the lane that consumes them has zero execution record. UNVERIFIED (cargo absent; no orchestrator fuzz baseline to cite). Confidence: HIGH that it is unexecuted in the committed baselines.
## 3. What actually works (verified)
python3 header parse (fields above); the seed file is referenced as the target's corpus dir per fuzz/Cargo.toml:306 comment ("Seeds: fuzz/corpus/<target>/").
