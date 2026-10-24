//! ALAC fuzz-lite survival (`+FB`, card 5.J3a): seeded-LCG mutations
//! of REAL decoded-corpus bytes (the committed ffmpeg-generated
//! fixtures' packets + cookie), pinned on the three survival
//! properties — no panic, no hang, decode is Ok-or-typed — plus
//! determinism across runs. Committed fuzz inputs stay under 200 KB
//! (the mono-16 fixture + the first packets of the stereo fixtures,
//! re-extracted from the committed .m4a at run time through the real
//! MP4 demuxer — nothing synthetic).

mod common;

use std::path::{Path, PathBuf};

use demux::mp4::Mp4Demuxer;
use media_types::ids::SourceUri;
use plugin_api::{Access, DemuxEvent, Demuxer};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/alac")
        .join(name)
}

fn open_via_input(
    path: &Path,
) -> Result<Box<dyn plugin_api::SourceStream>, media_types::EngineError> {
    let uri = match SourceUri::parse(&format!("file://{}", path.to_str().unwrap_or(""))) {
        Ok(uri) => uri,
        Err(err) => unreachable!("fixture path is a valid file URI ({path:?}): {err}"),
    };
    input::FileAccess::new().open(&uri)
}

/// Demux a fixture to (cookie, first `max` packet payloads).
fn corpus_bytes(name: &str, max: usize) -> (Vec<u8>, Vec<Vec<u8>>) {
    let path = fixture_path(name);
    let stream = open_via_input(&path).unwrap_or_else(|e| unreachable!("open: {e}"));
    let mut mp4 = Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("new: {e}"));
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let cookie = mp4
        .sample_entry_bytes(tracks[0].id)
        .unwrap_or_else(|| unreachable!("sample entry"))
        .to_vec();
    let mut packets = Vec::new();
    loop {
        match mp4
            .next_event()
            .unwrap_or_else(|e| unreachable!("event: {e}"))
        {
            DemuxEvent::Packet(p) => {
                if packets.len() < max {
                    packets.push(p.data.as_slice().to_vec());
                }
            }
            DemuxEvent::EndOfStream => break,
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => continue,
        }
    }
    (cookie, packets)
}

/// The seeded LCG (the crate's fuzz-survival convention): cheap,
/// deterministic, reproducible.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() >> 33) as usize % n.max(1)
    }
}

/// Decode one mutated packet through the free surface — Ok or a
/// TYPED engine error (`Corrupt`/`Unsupported`/`Decoder` — the only
/// kinds the taxonomy emits), never a panic. Returns `true` for Ok.
fn try_decode(packet: &[u8], cookie: &[u8]) -> bool {
    match codecs::audio::alac::decode_frame(packet, cookie) {
        Ok(_) => true,
        Err(e) => {
            assert!(
                matches!(
                    e.kind,
                    media_types::ErrorKind::Corrupt { .. }
                        | media_types::ErrorKind::Unsupported { .. }
                        | media_types::ErrorKind::Decoder { .. }
                ),
                "unmapped engine error kind: {:?}",
                e.kind
            );
            false
        }
    }
}

#[test]
fn mutated_packets_never_panic_or_hang() {
    let (cookie16, packets16) = corpus_bytes("alac_mono16.m4a", 4);
    let (cookie24, packets24) = corpus_bytes("alac_stereo24.m4a", 3);
    let corpora: [(&Vec<u8>, &Vec<Vec<u8>>); 2] =
        [(&cookie16, &packets16), (&cookie24, &packets24)];
    let mut lcg = Lcg(0x5eed_a1ac);
    let mut ok = 0usize;
    let mut typed = 0usize;
    for round in 0..512 {
        let (cookie, packets) = &corpora[round % corpora.len()];
        let base = &packets[lcg.below(packets.len())];
        let mut bytes = base.clone();
        match round % 4 {
            0 => {
                // Single-bit flips (biased into the element header).
                let flips = 1 + lcg.below(8);
                for _ in 0..flips {
                    let byte = lcg.below(bytes.len());
                    bytes[byte] ^= 1u8 << (lcg.below(8));
                }
            }
            1 => {
                // Truncations.
                bytes.truncate(lcg.below(bytes.len() + 1));
            }
            2 => {
                // Wild header bytes + a burst overwrite.
                let at = lcg.below(bytes.len());
                bytes[at] = u8::try_from(lcg.next()).unwrap_or(0xff);
                let end = (at + 1 + lcg.below(16)).min(bytes.len());
                for b in &mut bytes[at..end] {
                    *b = u8::try_from(lcg.next()).unwrap_or(0xff);
                }
            }
            _ => {
                // Splice another packet's tail on (boundary chaos).
                let other = &packets[lcg.below(packets.len())];
                let at = lcg.below(bytes.len());
                let take = 1 + lcg.below(other.len().min(64));
                let tail = other[other.len() - take..].to_vec();
                bytes.splice(at.., tail);
            }
        }
        let first = try_decode(&bytes, cookie);
        if first {
            ok += 1;
        } else {
            typed += 1;
        }
        // Determinism: the same bytes answer the same way.
        assert_eq!(try_decode(&bytes, cookie), first);
    }
    // The mutations must bite: with 512 rounds over real frames, both
    // outcomes occur (a purely-Ok or purely-Err run means the harness
    // tested nothing).
    assert!(
        ok > 0 && typed > 0,
        "ok={ok} typed={typed} — mutations must bite"
    );
}

#[test]
fn mutated_cookies_are_typed_not_panics() {
    let (cookie, _) = corpus_bytes("alac_mono16.m4a", 1);
    let (_, packets) = corpus_bytes("alac_mono16.m4a", 1);
    let packet = &packets[0];
    let mut lcg = Lcg(0xa1ac_5eed);
    let mut rejected = 0usize;
    for round in 0..256 {
        let mut bytes = cookie.clone();
        match round % 3 {
            0 => {
                let at = lcg.below(bytes.len());
                bytes[at] ^= 1u8 << (lcg.below(8));
            }
            1 => bytes.truncate(lcg.below(bytes.len() + 1)),
            _ => {
                let extra = 1 + lcg.below(8);
                let filler = u8::try_from(lcg.next()).unwrap_or(0);
                bytes.extend(std::iter::repeat_n(filler, extra));
            }
        }
        match codecs::audio::alac::decode_frame(packet, &bytes) {
            Ok(_) => {}
            Err(_) => rejected += 1,
        }
    }
    assert!(rejected > 0, "cookie mutations must be rejected");
}

#[test]
fn empty_and_edge_packets_are_typed() {
    let (cookie, packets) = corpus_bytes("alac_stereo16.m4a", 1);
    let packet = &packets[0];
    // Empty packet: typed structure damage on the free surface.
    assert!(codecs::audio::alac::decode_frame(&[], &cookie).is_err());
    // Single stray bytes / the all-0x00 and all-0xFF shapes.
    for byte in [0x00u8, 0xFF, 0x80, 0x7F] {
        let edge = [byte; 8];
        assert!(codecs::audio::alac::decode_frame(&edge, &cookie).is_err());
    }
    // The clean packet still decodes after all the abuse above (the
    // free surface is stateless).
    assert!(codecs::audio::alac::decode_frame(packet, &cookie).is_ok());
}
