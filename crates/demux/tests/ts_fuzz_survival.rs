//! Fuzz-lite survival for the TS packet layer + PSI (card 5.I1a):
//! deterministic LCG mutations over a synthetic multi-program
//! transport stream (well under 200 KB) — every round must produce a
//! typed outcome, never a panic, never a hang, and the demuxer must
//! stay deterministic across replays of the same mutation.
//!
//! The corpus lanes (on-disk, not committed) get a second lane when
//! present: the first 64 KiB of a real ffmpeg-authored TS, mutated
//! the same way.

use demux::ts::test_support::{
    mem_stream, pat_body, pmt_body, psi_packets, psi_section, sdt_body, stream_of, ts_packet,
};
use demux::ts::{TsDemuxer, TsStats};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// The seeded LCG (deterministic across runs and platforms).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 16
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

/// A deterministic ~3 KB multi-program TS: two programs, video+audio
/// and audio+data, SDT names, CAT, PES-looking payloads, one PCR.
fn base_stream() -> Vec<u8> {
    let pat = psi_section(0x00, 7, 0, &pat_body(&[(1, 0x1000), (2, 0x1100)]));
    let pmt1 = psi_section(
        0x02,
        1,
        0,
        &pmt_body(0x100, &[], &[(0x1B, 0x100, &[]), (0x0F, 0x101, &[])]),
    );
    let pmt2 = psi_section(
        0x02,
        2,
        0,
        &pmt_body(
            0x200,
            &[0x09, 0x00],
            &[(0x03, 0x200, &[]), (0x06, 0x201, &[])],
        ),
    );
    let sdt = psi_section(
        0x42,
        7,
        0,
        &sdt_body(0x11, &[(1, "FFmpeg", "Service01"), (2, "P2", "Two")]),
    );
    let cat = psi_section(0x01, 1, 0, &[0x09, 0x04, 0xFF, 0xFF, 0xFF, 0xFF]);
    let mut packets = Vec::new();
    packets.extend(psi_packets(0x0011, 0, &[sdt]));
    packets.extend(psi_packets(0x0001, 0, &[cat]));
    packets.extend(psi_packets(0x0000, 0, &[pat]));
    packets.extend(psi_packets(0x1000, 0, &[pmt1]));
    packets.extend(psi_packets(0x1100, 0, &[pmt2]));
    for i in 0..8u8 {
        packets.push(ts_packet(
            0x100,
            i % 2 == 0,
            i,
            Some((90_000 * u64::from(i), 0)),
            &[0x55; 150],
        ));
        packets.push(ts_packet(0x101, i == 0, i, None, &[0x66; 150]));
        packets.push(ts_packet(0x200, i == 0, i, None, &[0x77; 150]));
    }
    stream_of(&packets)
}

/// Run one stream through start+drain; returns the outcome (typed).
fn survive(bytes: &[u8]) -> Result<TsStats, media_types::EngineError> {
    let stream = mem_stream(bytes.to_vec())?;
    let mut demuxer = TsDemuxer::new(stream)?;
    demuxer.start()?;
    loop {
        if matches!(demuxer.next_event()?, DemuxEvent::EndOfStream) {
            break;
        }
    }
    Ok(demuxer.stats())
}

#[test]
fn mutations_never_panic_and_stay_deterministic() {
    let base = base_stream();
    assert!(base.len() < 200_000, "the fuzz input stays small");
    let mut rng = Lcg(0x5EED_5EED_5EED_5EED);
    let mut outcomes_ok = 0;
    for round in 0..512 {
        let mut bytes = base.clone();
        // 1–8 mutations per round.
        let mutations = 1 + rng.below(8);
        for _ in 0..mutations {
            match rng.below(4) {
                0 => {
                    // Flip a byte (biased into packet headers/PSI).
                    let at = rng.below(bytes.len());
                    bytes[at] ^= (rng.next() as u8) | 0x01;
                }
                1 => {
                    // Truncate.
                    let at = rng.below(bytes.len());
                    bytes.truncate(at);
                }
                2 => {
                    // Splice garbage (sync-loss class).
                    let at = rng.below(bytes.len());
                    let garbage = vec![0x00u8; 1 + rng.below(190)];
                    bytes.splice(at..at, garbage);
                }
                _ => {
                    // Wild header: randomize a packet's first 5 bytes.
                    let pkt = rng.below(bytes.len() / 188 + 1);
                    let at = pkt * 188;
                    if at + 5 <= bytes.len() {
                        for i in 0..5 {
                            bytes[at + i] = rng.next() as u8;
                        }
                    }
                }
            }
        }
        // The survival contract: a typed outcome, no panic, no hang.
        let first = survive(&bytes);
        let second = survive(&bytes);
        assert!(
            first.is_ok() == second.is_ok(),
            "round {round}: outcome kind must be deterministic"
        );
        match (&first, &second) {
            (Ok(a), Ok(b)) => {
                outcomes_ok += 1;
                assert_eq!(a, b, "round {round}: deterministic across replays");
            }
            (Err(a), Err(b)) => {
                assert_eq!(
                    a.context.code, b.context.code,
                    "round {round}: typed refusals replay identically"
                );
            }
            _ => {}
        }
        // Sanity: the pristine stream survives cleanly.
        if round == 0 {
            let clean = survive(&base).must();
            assert_eq!(clean.psi_crc_failures, 0);
            assert!(clean.pes_packets > 0);
        }
    }
    assert!(
        outcomes_ok > 256,
        "most mutations must stay walkable, {outcomes_ok} did"
    );
}

#[test]
fn truncation_sweep_over_packet_boundaries() {
    let base = base_stream();
    // Every prefix length that lands near a packet boundary (and a
    // few inside) must produce a typed outcome.
    for cut in (0..base.len()).step_by(61) {
        let outcome = survive(&base[..cut]);
        if cut < 188 {
            // Below one packet: not a TS (or empty-stream refusal).
            if let Err(err) = outcome {
                assert!(
                    err.context.code == "demux.ts.not-ts"
                        || err.context.code == "demux.ts.no-pat"
                        || err.context.code == "demux.source.empty",
                    "unexpected code {} at cut {cut}",
                    err.context.code
                );
            }
        }
    }
}

/// The real-corpus lane (skip-with-msg when the corpus is absent):
/// the first 64 KiB of a real ffmpeg TS, mutated the same way.
#[test]
fn corpus_head_mutations_survive() {
    let dir = std::path::PathBuf::from(
        std::env::var("LUMEN_CORPUS")
            .unwrap_or_else(|_| "/tmp/my-project/corpus-v4/media".to_string()),
    );
    let path = dir.join("0091-bframe-storm.ts");
    if !path.exists() {
        eprintln!(
            "SKIP (loud): corpus head absent at {} — fuzz lane not run",
            path.display()
        );
        return;
    }
    let whole = std::fs::read(&path).must();
    let head: Vec<u8> = whole.into_iter().take(64 * 1024).collect();
    let mut rng = Lcg(0x00C0_FFEE_0011);
    for _ in 0..256 {
        let mut bytes = head.clone();
        for _ in 0..(1 + rng.below(6)) {
            let at = rng.below(bytes.len());
            match rng.below(3) {
                0 => bytes[at] ^= (rng.next() as u8) | 0x01,
                1 => bytes.truncate(at),
                _ => {
                    let pkt = rng.below(bytes.len() / 188 + 1);
                    let at = pkt * 188;
                    if at + 4 <= bytes.len() {
                        bytes[at + 1] = rng.next() as u8; // TEI/PUSI/PID bits
                    }
                }
            }
        }
        // Typed outcome only; a truncated head may honestly refuse
        // with no-pat (the scan window ran out mid-PES).
        if let Err(err) = survive(&bytes) {
            assert!(
                err.context.code == "demux.ts.not-ts" || err.context.code == "demux.ts.no-pat",
                "unexpected refusal {}",
                err.context.code
            );
        }
    }
}
