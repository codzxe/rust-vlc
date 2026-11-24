# AUDIT: tools/vta/ruopus/src/main.rs
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / functions | 135 / 3 (main, Dec::decode, hostile_suite) |
| File verdict | REAL (logic read; not executable in this sandbox — no cargo) |
| Reachable from shipped entry point? | NO — standalone harness crate (`tools/vta/ruopus`, own `[workspace]`, path-deps to crates/demux + plugin-api); never shipped, never in CI |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `cat tools/vta/ruopus/Cargo.toml`; `grep -n test_support crates/demux/src/avi/mod.rs` -> :121; RFC 7845 offset check (reasoning, see F-001) |

## 1. One-paragraph verdict
A genuine differential harness: it pumps the repo's own OggDemuxer, finds the OpusHead serial, decodes packets through ruopus (mono or multistream per mapping family), applies RFC 7845 pre-skip trimming, emits f32le, and runs a 6-case hostile-input suite with catch_unwind and exit 4 on panic. It cannot be executed here (no cargo in this sub-sandbox), and it has one real byte-offset bug — the OpusHead output-gain read lands on the wrong offset — plus the usual dev-tool argv panics. None of these affect shipped code.

## 2. What actually works (verified)
— Read: full demux→decode→trim→write pipeline present (L32-109); hostile suite L116-135 (truncated/garbage/zero/one-byte-toc/huge/huge-zeros, panic counting, exit 4).
— `grep -n "test_support" crates/demux/src/avi/mod.rs` -> L121 (the `demux::avi::test_support::mem_stream` dependency the harness imports exists).
— Standalone-crate isolation confirmed: tools/vta/ruopus/Cargo.toml has its own `[workspace]` and `publish = false`.

## 3. Findings
F-001 [S4] — OpusHead output-gain read at the wrong offset.
Where: tools/vta/ruopus/src/main.rs:63-65
Code:
```
    let preskip = u16::from_le_bytes([head[10], head[11]]) as usize;
    let gain_q7_8 = i16::from_le_bytes([head[14], head[15]]);
    let mapping_family = head[18];
```
What is wrong: RFC 7845 layout: pre-skip = bytes 10-11 (correct), input sample rate = bytes 12-15, output gain = bytes 16-17, mapping family = byte 18 (correct). The code reads gain from bytes 14-15 — the high half of the input sample rate — so the printed `gain=...q7.8` is garbage for any non-zero rate. Impact: debug eprintln only; the gain is never applied to samples (so no decode corruption from the misread) — but the harness also never applies output gain at all, which matters for differential fidelity if any input has gain ≠ 0. Confidence: HIGH — Verification: read (RFC 7845 §5.1 field table).
F-002 [S4] — argv handling panics instead of erroring.
Where: tools/vta/ruopus/src/main.rs:17-26
Code:
```
    let path = args[1].clone();
    ...
            "--out" => { out_path = Some(args[i + 1].clone()); i += 2; }
            "--trim-per-ch" => { trim_per_ch = Some(args[i + 1].parse().unwrap()); i += 2; }
```
What is wrong: no args check — zero args panics on `args[1]`; a trailing `--out`/`--trim-per-ch` panics on `args[i+1]`; a non-numeric trim panics in `unwrap()`. Dev tool, so limited blast radius, but a harness used for hostile-input work should refuse typed. Confidence: HIGH — Verification: read.
F-003 [S4] — Demux errors silently truncate the packet stream.
Where: tools/vta/ruopus/src/main.rs:41-47
Code:
```
        match ogg.next_event() {
            Ok(plugin_api::DemuxEvent::Packet(p)) => all.push((p.track, p.data.to_vec())),
            Ok(plugin_api::DemuxEvent::EndOfStream) => break,
            Ok(_) => continue,
            Err(_) => break,
        }
```
What is wrong: `Err(_) => break` ends collection without a word — a mid-file demux error produces a short PCM silently (only the DEBUG line's packet count hints at it). A differential could then compare fewer samples without a recorded reason. Confidence: HIGH — Verification: read.
F-004 [S4] — dead `let dec = ...; let mut dec = dec;` (L71-80) binding noise.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Dec::decode | 8-13 | REAL | — | maps both decoder kinds, error string formatting |
| main | 16-111 | REAL | S4 ×3 | F-001..F-003 live here |
| hostile_suite | 116-135 | REAL | — | 6 cases; catch_unwind; exit 4 on any panic |

## 5. Test honesty
The hostile suite would fail the process (exit 4) if a case panicked — genuine. Mutation reasoning: if ruopus panicked on truncated input, exit 4 fires; the suite is honest. Not executed (no cargo).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "OUR ogg demuxer -> ruopus decode -> raw f32le" | L1 | REAL-UNPROVEN | code path complete by read; execution blocked (no cargo) |

## 7. Dead / duplicated / unreachable code
F-004. The harness itself is unreachable from shipped builds by design (standalone crate).

## 8. Dependencies & flags
ruopus 0.1.2 (default-features=false, std), path deps demux + plugin-api. Not a workspace member (own [workspace]).

## 9. Open questions for the next agent
- Does any committed differential record depend on gain application (fixtures with non-zero OpusHead output gain)? Cross-check docs/dependencies/records/ruopus-0.1.2.md lanes.
