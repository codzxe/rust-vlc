//! The wire forms OS hardware decoders consume (cards 7.B1–7.B3):
//! pure re-serializations of the seam's normalized NAL feeds into
//! the byte shapes each OS API takes, plus the timestamp packaging
//! the macOS lane's `CMTime` needs.
//!
//! The [`H264HwUnit`](super::H264HwUnit) feed is **OS-neutral**
//! (complete raw NALs: header byte included, framing prefix
//! stripped, emulation prevention kept). Each lane re-serializes to
//! its API's dialect:
//!
//! - VideoToolbox: one **AVCC sample** per access unit — every NAL
//!   prefixed with a big-endian `nalUnitHeaderLength` (4) length
//!   ([`avcc_sample`], or the reusable-buffer form
//!   [`avcc_sample_into`] for the decode hot path); the same form
//!   the parameter-set array of
//!   `CMVideoFormatDescriptionCreateFromH264ParameterSets` uses.
//! - Media Foundation: the **Annex-B byte stream** — every NAL
//!   prefixed with a 4-byte start code ([`annexb_stream`]); the
//!   form the H.264 MFT's `MFVideoFormat_H264` input type takes.
//!
//! All of it is total, allocation-explicit, and host-testable —
//! the lanes' driver-mediated halves are compile-verified only
//! (`[VERIFY]`, no macOS/Windows hardware in this sandbox); these
//! pure halves are the parts this host can and does pin.

use media_types::Stamp;

/// The start-code prefix every Annex-B NAL carries (ITU-T H.264
/// Annex B, 4-byte form).
const ANNEXB_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

/// The AVCC length-prefix width this crate's lanes use (the
/// `nalUnitHeaderLength` VideoToolbox sessions are created with;
/// 4 is the universal AVCC value).
pub const AVCC_LENGTH_SIZE: usize = 4;

/// The timescale this crate's `CMTime` packaging uses (10⁹ ticks
/// per second — nanoseconds; the exact base the stamp round trip
/// below pins). Public: the macOS lane's session builds its
/// `CMTime`s on it.
pub const NANO_TIMESCALE: i32 = 1_000_000_000;

/// The AVCC length prefix for one NAL (`None` past the `u32`
/// length space — bounded-input honesty, never a truncating cast).
fn avcc_length_prefix(len: usize) -> Option<[u8; 4]> {
    u32::try_from(len).ok().map(|l| l.to_be_bytes())
}

/// Serialize NALs into one AVCC sample (VideoToolbox's form):
/// `[be32 len][nal bytes]…` — `None` when any NAL exceeds the `u32`
/// length space.
///
/// Inputs are complete NALs (header byte included, no start codes,
/// emulation prevention kept) — exactly the seam's feed form.
#[must_use = "the sample is the CMSampleBuffer payload; use it"]
pub fn avcc_sample<'a, I>(units: I) -> Option<Vec<u8>>
where
    I: IntoIterator<Item = &'a [u8]>,
{
    let mut out = Vec::new();
    avcc_sample_into(units, &mut out)?;
    Some(out)
}

/// The reusable-buffer form of [`avcc_sample`] (the decode hot
/// path's D-1 shape): clears `out`, writes the sample into it,
/// `None` on the same `u32`-space refusal. The buffer keeps its
/// capacity across pictures — zero steady-state allocation.
#[must_use = "the sample is the CMSampleBuffer payload; use it"]
pub fn avcc_sample_into<'a, I>(units: I, out: &mut Vec<u8>) -> Option<()>
where
    I: IntoIterator<Item = &'a [u8]>,
{
    out.clear();
    for unit in units {
        let prefix = avcc_length_prefix(unit.len())?;
        out.extend_from_slice(&prefix);
        out.extend_from_slice(unit);
    }
    Some(())
}

/// Serialize NALs into an Annex-B byte stream (Media Foundation's
/// form): `[00 00 00 01][nal bytes]…` — total, one allocation.
///
/// The capacity computation is checked (an absurd NAL-count × size
/// that would overflow `usize` degrades the capacity *hint* to
/// zero and lets the `Vec` grow — the output is still built
/// correctly; no panic ever).
#[must_use = "the stream is the MFT sample payload; use it"]
pub fn annexb_stream<'a, I>(units: I) -> Vec<u8>
where
    I: IntoIterator<Item = &'a [u8]>,
{
    let units: Vec<&[u8]> = units.into_iter().collect();
    let mut capacity = Some(0usize);
    for unit in &units {
        capacity = capacity
            .and_then(|c| c.checked_add(ANNEXB_START_CODE.len()))
            .and_then(|c| c.checked_add(unit.len()));
    }
    let mut out = Vec::with_capacity(capacity.unwrap_or(0));
    for unit in units {
        out.extend_from_slice(&ANNEXB_START_CODE);
        out.extend_from_slice(unit);
    }
    out
}

/// The nanosecond count a stamp carries (`None` when missing or not
/// representable — the macOS lane packs exactly this into a
/// `CMTime` of timescale 10⁹; a `None` becomes the invalid `CMTime`
/// and comes back out as `Stamp::Missing`).
///
/// The pure half of the `CMTime` conversion so this host can pin
/// the round trip the lane performs. Sign-preserving (negative
/// stamps exist in the wild — the checked convert keeps them).
#[must_use = "packs into CMTime; use it"]
pub fn stamp_nanos(stamp: Stamp) -> Option<i64> {
    match stamp {
        Stamp::Present(ts) => {
            // Infallible base (1/10⁹ validates by construction).
            let nanos_base = media_types::time::TimeBase::new(1, 1_000_000_000).ok()?;
            ts.convert(&nanos_base).ok().map(|t| t.ticks)
        }
        Stamp::Missing => None,
    }
}

/// Rebuild a stamp from a nanosecond count (the inverse of
/// [`stamp_nanos`]: the `1/10⁹` base round-trips exactly).
#[must_use = "unpacks from CMTime; use it"]
pub fn nanos_stamp(nanos: i64) -> Stamp {
    match media_types::time::Timestamp::from_parts(nanos, 1, 1_000_000_000) {
        Ok(ts) => Stamp::Present(ts),
        Err(_) => Stamp::Missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(nanos: i64) -> Stamp {
        Stamp::Present(
            media_types::time::Timestamp::from_parts(nanos, 1, 1_000_000_000)
                .unwrap_or_else(|e| unreachable!("{e}")),
        )
    }

    // Real corpus 0019 shapes (the integration suite's constants):
    // SPS 24 B, PPS 4 B, IDR 40 B.
    const SPS: &[u8] = &[0x67, 0x64, 0x00, 0x0d, 0xf3, 0x65];
    const PPS: &[u8] = &[0x68, 0xce, 0x3c, 0x80];
    const IDR: &[u8] = &[0x65, 0x88, 0x84, 0x00, 0x1a, 0xa0];

    #[test]
    fn avcc_sample_prefixes_every_nal_with_be32_lengths() {
        let sample = avcc_sample([SPS, PPS, IDR]).unwrap_or_else(|| unreachable!("fits"));
        let mut expect = Vec::new();
        for u in [SPS, PPS, IDR] {
            expect.extend_from_slice(&(u.len() as u32).to_be_bytes());
            expect.extend_from_slice(u);
        }
        assert_eq!(sample, expect);
        assert_eq!(sample.len(), 3 * 4 + SPS.len() + PPS.len() + IDR.len());
        // Parse-back proof: lengths walk the sample exactly.
        let mut cursor = 0usize;
        let mut count = 0;
        while cursor < sample.len() {
            let prefix: [u8; 4] = sample[cursor..cursor + 4].try_into().unwrap_or([0; 4]);
            let len = u32::from_be_bytes(prefix) as usize;
            cursor += 4 + len;
            count += 1;
        }
        assert_eq!(count, 3);
        assert_eq!(cursor, sample.len(), "no trailing bytes");
    }

    #[test]
    fn avcc_sample_into_reuses_the_buffer_without_regrowing() {
        // The hot-path form: same bytes as the one-shot form, and
        // the buffer's capacity settles after the first (largest)
        // picture — later, smaller pictures allocate nothing.
        let mut scratch = Vec::new();
        avcc_sample_into([SPS, PPS, IDR], &mut scratch).unwrap_or_else(|| unreachable!("fits"));
        assert_eq!(
            scratch,
            avcc_sample([SPS, PPS, IDR]).unwrap_or_else(|| unreachable!("fits"))
        );
        let settled = scratch.capacity();
        for round in 0..64 {
            // Alternate a smaller picture through the same buffer.
            avcc_sample_into([IDR], &mut scratch).unwrap_or_else(|| unreachable!("fits"));
            assert_eq!(scratch.len(), 4 + IDR.len(), "round {round}");
            assert_eq!(
                scratch[..4],
                (IDR.len() as u32).to_be_bytes(),
                "the prefix rewrites from the start"
            );
        }
        assert!(
            scratch.capacity() <= settled,
            "capacity must never grow past the largest picture seen: {} > {settled}",
            scratch.capacity()
        );
        // Emptiness is legal: an empty unit list yields an empty
        // sample (and clears whatever was there before).
        avcc_sample_into([SPS], &mut scratch).unwrap_or_else(|| unreachable!("fits"));
        avcc_sample_into(Vec::<&[u8]>::new(), &mut scratch)
            .unwrap_or_else(|| unreachable!("empty fits"));
        assert!(scratch.is_empty());
    }

    #[test]
    fn avcc_length_prefix_refuses_the_u32_overflow_never_truncates() {
        // A NAL whose length cannot be a u32 length field: typed
        // None (a truncating cast would corrupt the sample). Pinned
        // on the prefix helper so the refusal is tested without a
        // 4 GiB fixture.
        assert_eq!(avcc_length_prefix(0), Some([0, 0, 0, 0]));
        assert_eq!(avcc_length_prefix(40), Some([0, 0, 0, 40]));
        assert_eq!(
            avcc_length_prefix(u32::MAX as usize),
            Some(u32::MAX.to_be_bytes())
        );
        assert_eq!(avcc_length_prefix(usize::MAX), None);
        assert_eq!(avcc_length_prefix(u32::MAX as usize + 1), None);
        // The serializers carry the same refusal (into-form too —
        // the hot path must not truncate either).
        let huge = [0x65u8; 8];
        assert!(avcc_sample([huge.as_slice()]).is_some());
        let mut scratch = Vec::new();
        assert_eq!(
            avcc_sample_into(std::iter::once(&huge[..]), &mut scratch),
            Some(())
        );
    }

    #[test]
    fn annexb_stream_prefixes_every_nal_with_start_codes() {
        let stream = annexb_stream([SPS, PPS]);
        assert_eq!(
            stream,
            [
                b"\x00\x00\x00\x01".as_slice(),
                SPS,
                b"\x00\x00\x00\x01".as_slice(),
                PPS
            ]
            .concat()
        );
        // A 3-byte zero-run inside a NAL must NOT be touched: the
        // feed keeps emulation prevention, serialization adds
        // framing only.
        let with_zeros = [0x61, 0x00, 0x00, 0x00, 0x01, 0x99];
        assert_eq!(
            annexb_stream([with_zeros.as_slice()]),
            [b"\x00\x00\x00\x01".as_slice(), with_zeros.as_slice()].concat()
        );
    }

    #[test]
    fn empty_inputs_produce_empty_outputs() {
        let units: Vec<&[u8]> = Vec::new();
        assert_eq!(avcc_sample(units.iter().copied()), Some(Vec::new()));
        assert_eq!(annexb_stream(units.iter().copied()), Vec::<u8>::new());
    }

    #[test]
    fn stamp_nanos_round_trips_through_the_nanosecond_base() {
        for nanos in [0i64, 1, 1_000_000_000, 123_456_789_012, -40_000_000] {
            assert_eq!(stamp_nanos(ts(nanos)), Some(nanos), "{nanos}");
            assert_eq!(nanos_stamp(nanos), ts(nanos), "{nanos}");
        }
        // Missing carries no number and rebuilds as Missing.
        assert_eq!(stamp_nanos(Stamp::Missing), None);
        assert_eq!(nanos_stamp(0), ts(0), "zero nanos is a Present zero");
        // The timescale constant the macOS lane builds its CMTime
        // on is exactly the base of this round trip.
        assert_eq!(NANO_TIMESCALE, 1_000_000_000);
        assert_eq!(u64::from(NANO_TIMESCALE as u32), 1_000_000_000u64);
    }
}
