//! The 6.A2a end-to-end proof: a demuxer reads real network media
//! through [`RangedResource`] and gets the same bytes the file on disk
//! yields.
//!
//! A real ffmpeg-generated WAV fixture (the committed
//! `wav_pcm_s16le_44k1_st.wav`) is served over a real socket by the
//! mock HTTP server (Range honored), opened through
//! [`RangedResource`] as a `plugin_api::SourceStream`, demuxed by our
//! wav demuxer, and the concatenated packet payload is checked three
//! ways:
//!
//! 1. **byte-exact against the file on disk** — the payload equals
//!    the fixture's `data` chunk (parsed straight from the on-disk
//!    RIFF structure; strictly stronger than a digest comparison);
//! 2. **MD5 against the recorded ffmpeg truth** (the
//!    `ffmpeg -c copy -f md5` digest measured when the fixture suite
//!    was pinned — RFC 1321 implemented test-side, pinned against
//!    RFC 1321's own A.5 vectors first);
//! 3. **byte-exact against the same fixture demuxed through the
//!    input crate's `FileAccess`** — the local-file access stack and
//!    the network ranged resource agree through the whole demuxer.
//!
//! A small chunk size (1 KiB over a ~46 KiB file) forces dozens of
//! range requests, so the demuxer's seek-heavy header walk genuinely
//! exercises the window cache and the ranged fetch path — and the
//! request log proves it.

use std::sync::Arc;

use demux::wav::WavDemuxer;
use plugin_api::{Access, DemuxEvent, Demuxer, SourceStream};
use streaming::fetch::HttpFetch;
use streaming::mockhttp::{MockConfig, MockServer};
use streaming::ranged::{RangedConfig, RangedResource};
use testkit::Must;

/// The fixture under test (an ffmpeg product; truth measured in the
/// demux fixture suite): 44.1 kHz stereo pcm_s16le, 8 820 frames,
/// ffmpeg `-c copy -f md5` payload digest `dcaccb18…`.
const FIXTURE: &str = "generated/valid/wav_pcm_s16le_44k1_st.wav";
/// The ffmpeg-measured MD5 of the demuxed packet stream (recorded in
/// crates/demux/tests/wav_aiff_fixtures.rs when the suite was pinned).
const FFMPEG_MD5: &str = "dcaccb18f7fc858b87684ad3b25bc7c5";

/// Drain a started demuxer, returning the concatenated packet payload.
fn drain(wav: &mut WavDemuxer) -> Vec<u8> {
    let mut payload = Vec::new();
    loop {
        match wav.next_event() {
            Ok(DemuxEvent::Packet(packet)) => payload.extend_from_slice(&packet.data),
            Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => unreachable!("the fixture demuxes cleanly: {err}"),
        }
    }
    payload
}

/// Extract the `data` chunk payload from on-disk RIFF/WAVE bytes (the
/// file-on-disk truth; test-side: a wrong shape is a broken fixture).
fn wav_data_chunk(bytes: &[u8]) -> &[u8] {
    assert!(bytes.len() >= 12, "RIFF header present");
    assert_eq!(&bytes[0..4], b"RIFF", "RIFF magic");
    assert_eq!(&bytes[8..12], b"WAVE", "WAVE form");
    let mut off = 12usize;
    while off + 8 <= bytes.len() {
        let id = &bytes[off..off + 4];
        let size = u32::from_le_bytes(
            bytes[off + 4..off + 8]
                .try_into()
                .unwrap_or_else(|_| unreachable!("fixed 4-byte slice")),
        ) as usize;
        let body = off + 8;
        if id == b"data" {
            let end = body + size;
            assert!(end <= bytes.len(), "data chunk within the file");
            return &bytes[body..end];
        }
        off = body + size + (size & 1); // chunks are word-aligned
    }
    unreachable!("the fixture carries a data chunk");
}

/// Serve the fixture bytes over a real socket, open them through the
/// ranged resource, demux, and prove the payload three ways.
#[test]
fn wav_over_http_demuxes_to_the_on_disk_truth() {
    let bytes = testkit::fixture_bytes(FIXTURE).must();
    let server = MockServer::spawn(MockConfig::body(bytes.clone()))
        .unwrap_or_else(|e| unreachable!("spawn: {e}"));
    let fetch: Arc<dyn HttpFetch> = Arc::new(server.client());

    // 1 KiB chunks over a ~46 KiB file: dozens of range requests.
    let config = RangedConfig {
        window_bytes: 4096,
        chunk_bytes: 1024,
    };
    let stream = RangedResource::open(fetch, &server.url("media.wav"), config)
        .unwrap_or_else(|e| unreachable!("open: {e}"));
    assert_eq!(stream.size(), Some(bytes.len() as u64));
    let mut wav = WavDemuxer::new(Box::new(stream)).unwrap_or_else(|e| unreachable!("{e}"));
    let tracks = wav.start().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].codec.as_str(), "pcm_s16le");
    let net_payload = drain(&mut wav);

    // (1) byte-exact against the file on disk's data chunk.
    assert_eq!(
        net_payload,
        wav_data_chunk(&bytes),
        "network-demuxed payload == the on-disk data chunk"
    );

    // (2) MD5 against the recorded ffmpeg truth.
    assert_eq!(md5_hex(&net_payload), FFMPEG_MD5);

    // (3) the same fixture through the input crate's file access.
    let path = testkit::fixture_path(FIXTURE).must();
    let uri =
        media_types::ids::SourceUri::parse(&format!("file://{}", path.to_str().must())).must();
    let file_stream = input::FileAccess::new().open(&uri).must();
    let mut file_wav = WavDemuxer::new(file_stream).unwrap_or_else(|e| unreachable!("{e}"));
    file_wav.start().unwrap_or_else(|e| unreachable!("{e}"));
    let file_payload = drain(&mut file_wav);
    assert_eq!(net_payload, file_payload, "network and file paths agree");

    // The request log proves the ranged path really did the work:
    // many range requests, all ranged, no re-fetch storm.
    let requests = server.requests();
    assert!(
        requests.len() >= 30,
        "dozens of range requests over a {}-byte file (got {})",
        bytes.len(),
        requests.len()
    );
    for req in &requests {
        let range = req
            .header("range")
            .unwrap_or_else(|| unreachable!("Range sent"));
        assert!(
            range.starts_with("bytes="),
            "every request is a ranged request: {range}"
        );
    }
}

// ---------------------------------------------------------------------------
// RFC 1321 MD5, test-side (no external deps; C5) — the same shape the
// demux fixture suite uses. Pinned against RFC 1321's own A.5 vectors
// before any differential assertion relies on it.
// ---------------------------------------------------------------------------

/// MD5 digest of `data`, lowercase hex.
fn md5_hex(data: &[u8]) -> String {
    // K[i] = floor(|sin(i+1)| * 2^32) — computed, not transcribed.
    let k: Vec<u32> = (0..64)
        .map(|i| (((i as f64) + 1.0).sin().abs() * 4_294_967_296.0) as u32)
        .collect();
    let s: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let mut a0: u32 = 0x6745_2301;
    let mut b0: u32 = 0xefcd_ab89;
    let mut c0: u32 = 0x98ba_dcfe;
    let mut d0: u32 = 0x1032_5476;

    // Padded message: 0x80, zeros, 8-byte little-endian bit length.
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let mut off = 0;
    while off < msg.len() {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            let at = off + i * 4;
            *word = u32::from_le_bytes(
                msg[at..at + 4]
                    .try_into()
                    .unwrap_or_else(|_| unreachable!("fixed 4-byte slice")),
            );
        }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f2 = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f2.rotate_left(s[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
        off += 64;
    }

    let mut out = Vec::with_capacity(16);
    for word in [a0, b0, c0, d0] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.iter().map(|b| format!("{b:02x}")).collect()
}

/// RFC 1321's own A.5 test vectors — the pin before the differential
/// assertion above trusts this implementation.
#[test]
fn md5_matches_rfc1321_vectors() {
    assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(md5_hex(b"a"), "0cc175b9c0f1b6a831c399e269772661");
    assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(
        md5_hex(b"message digest"),
        "f96b697d7cb7938d525a2f31aaf161d0"
    );
    assert_eq!(
        md5_hex(b"abcdefghijklmnopqrstuvwxyz"),
        "c3fcd3d76192e4007dfb496cca67e13b"
    );
    assert_eq!(
        md5_hex(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"),
        "d174ab98d277d9f5a5611c2c9f419d9f"
    );
    assert_eq!(
        md5_hex(
            b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
        ),
        "57edf4a22be3c955ac49da2e2107b67a"
    );
}
