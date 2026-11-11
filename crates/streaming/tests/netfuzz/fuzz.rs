//! netfuzz/fuzz.rs — the issue #14 (M2) harness bodies for the two
//! network-facing fuzz targets, owned by the streaming wave agent:
//!
//! * [`fuzz_hls_playlist`] — hostile bytes through the **HLS
//!   interpretation stack**: the playlists crate's media/master
//!   playlist models, the ABR variant pick, and the
//!   `#EXT-X-TIMESTAMP-MAP` wall-clock mapping (the live-edge
//!   surface). The generic M3U/M3U8 *parse* pass underneath is the
//!   in-crate `playlists::fuzz::fuzz_playlist_m3u` harness the
//!   existing `fuzz_playlist_m3u` target already runs; this module
//!   layers the HLS-specific consumers on top.
//! * [`fuzz_http_headers`] — hostile bytes through the mock
//!   transport's **response-head grammar**
//!   (`streaming::mockhttp::fuzz_response_head`, the bounded typed
//!   parse banked with the M2 mockhttp extension) plus the
//!   request-side `Range:` header-value formatting under hostile
//!   numbers.
//!
//! The file lives under `crates/streaming/tests/` because the wave
//! agent owns neither `playlists/src` nor `streaming/src` — the same
//! ownership note that shaped the M1 `tests/netfault/mod.rs` support.
//! It is `#[path]`-included twice, verbatim, never rewritten:
//!
//! * by the libFuzzer shims `fuzz/fuzz_targets/fuzz_hls_playlist.rs`
//!   and `fuzz/fuzz_targets/fuzz_http_headers.rs` (each keeps one
//!   gate-visible in-crate harness call first — see those files);
//! * by the stable-toolchain sweep
//!   `crates/streaming/tests/netfuzz_sweep.rs`, the fuzz/README
//!   convention's runner while the nightly campaign wiring is the
//!   outstanding LEAD-DECISION.
//!
//! # The harness contract (fuzz/README: survival)
//!
//! No panic, no hang, no unbounded allocation. Malformed input must
//! surface as a typed error or a damage warning — every `Result` and
//! `Option` below is discarded exactly because the *verdict* is the
//! invariant, not the value. Integer overflow while walking attacker
//! bytes is a finding (release fuzz keeps `overflow-checks`), so the
//! arithmetic probes below stay inside the grammar's documented
//! domains and say so where they do.
//!
//! # Bounds (MP §12 / D-2)
//!
//! One pass per surface, output ≤ O(input). The lossy UTF-8 decode
//! allocates at most 3× input. Inputs over [`HLS_INPUT_CAP`] are
//! exercised as their prefix (a single call is therefore O(cap)); the
//! campaign's own `-max_len` governs real input sizes underneath. The
//! response-head parse is capped at the transport's own 64 KiB head
//! budget inside `fuzz_response_head` (over-limit answers the typed
//! `ProtocolViolation`, never an unbounded parse).

// Each libFuzzer shim includes this module whole but calls only its
// own twin; the sibling harness (and helpers only it uses) is dead
// code in that binary and alive in the sweep. The allow keeps both
// inclusions warning-clean; the sweep's own asserts stay honest.
#![allow(dead_code)]

/// The HLS harness's per-call input cap (256 KiB — two orders past
/// every committed fixture; keeps one call bounded regardless of what
/// the driver hands it).
pub const HLS_INPUT_CAP: usize = 256 * 1024;

/// The wall-clock probe domain for the inverse timestamp mapping: a
/// roam of ±(i64::MAX / 2·PTS_HZ) ms around the tag's own anchor
/// (±1.6 million years — far past the mapping's ±13.25 h contract,
/// still inside the i64 arithmetic `pts_for_wallclock_ms` does).
///
/// **Recorded finding (2026-11-09, this sweep)**: probing with a wall
/// value further than ~10^14 ms from the anchor overflows the
/// `(wallclock_ms - local_ms) * 90_000` intermediate — a panic under
/// `overflow-checks`, a silent wrap without them. The mapping's own
/// doc contracts it for values near anchors; the fuzz harness stays
/// inside the arithmetic's safe domain and pins that boundary here
/// for the follow-up (src is outside this wave's ownership).
const WALL_PROBE_ROAM: i64 = i64::MAX / (2 * 90_000);

/// One HLS-interpretation pass over `data` (see the module docs).
///
/// Never panics, never hangs, never allocates without a ceiling; the
/// verdicts (typed `HlsPlaylistError`s, damage warnings, `None`
/// mappings) are the contract, and are swallowed here on purpose.
pub fn fuzz_hls_playlist(data: &[u8]) {
    let bytes = &data[..data.len().min(HLS_INPUT_CAP)];
    let text = String::from_utf8_lossy(bytes).into_owned();
    // A resolvable absolute base: URI resolution must survive hostile
    // relative URIs (the model resolves every segment/variant URI
    // against it).
    let base = "https://fuzz.example/live/playlist.m3u8";

    // (1) The media-playlist model (VOD/live window grammar).
    if let Ok((media, _warnings)) = playlists::parse_media_playlist(Some(base), &text) {
        // Advisory touches: the fields the streaming sessions read.
        let _ = media.segments.len();
        let _ = media.target_duration;
        let _ = media.media_sequence;
        let _ = media.end_list;
        let _ = media.total_extinf_duration;
    }

    // (2) The master-playlist model + the ABR pick (6.B2b's pure
    // policy over attacker-described variants).
    if let Ok((master, _warnings)) = playlists::parse_master_playlist(Some(base), &text) {
        for bandwidth in [0u64, 1, 750_000, u64::MAX] {
            let _ = streaming::hls::master::select_variant_index(&master, bandwidth);
        }
    }

    // (3) The X-TIMESTAMP-MAP surface (the live edge's wall clock).
    if let Some(map) = streaming::hls::timestamp_map::TimestampMap::from_playlist_text(&text) {
        // PTS probes: any u64 — the mapping's wrap delta is wrapping
        // arithmetic over the 33-bit counter by construction.
        let pts = read_u64(data, 0);
        let _ = map.wallclock_ms_for_pts(pts);
        // Wall-clock probes: roamed around the tag's own anchor (see
        // WALL_PROBE_ROAM — the inverse map's i64 arithmetic is only
        // safe within that neighborhood of the anchor, which is also
        // far past its ±13.25 h contract).
        let roam = read_i64(data, 8).clamp(-WALL_PROBE_ROAM, WALL_PROBE_ROAM);
        let wall = map.local_ms.saturating_add(roam);
        let _ = map.pts_for_wallclock_ms(wall);
    }
}

/// One HTTP-header pass over `data` (see the module docs): the
/// response-head grammar plus the request-side `Range:` value
/// formatting. Same survival contract as [`fuzz_hls_playlist`].
pub fn fuzz_http_headers(data: &[u8]) {
    // (1) The response-head grammar: status line + `Name: value`
    // lines, bounded at the transport's own 64 KiB head budget,
    // typed verdicts (the socket path's exact grammar, split out by
    // the M2 mockhttp extension).
    let _ = streaming::mockhttp::fuzz_response_head(data);

    // (2) Request-side `Range:` header values under hostile numbers
    // (formatting only — the wire grammar for the ranged resource's
    // probe/resume requests; any u64 pair must format, never panic).
    let start = read_u64(data, 0);
    let end = read_u64(data, 8);
    let bounded = streaming::fetch::ByteRange::Bounded {
        start,
        end_exclusive: end,
    };
    let from = streaming::fetch::ByteRange::From { start };
    let _ = bounded.to_header_value();
    let _ = from.to_header_value();
}

/// A little-endian `u64` at `offset`, zero-padded (the probe-value
/// helper — attacker bytes become probe numbers, deterministically).
fn read_u64(data: &[u8], offset: usize) -> u64 {
    let mut buf = [0u8; 8];
    for (i, slot) in buf.iter_mut().enumerate() {
        if let Some(byte) = data.get(offset + i) {
            *slot = *byte;
        }
    }
    u64::from_le_bytes(buf)
}

/// A little-endian `i64` at `offset`, zero-padded (see [`read_u64`]).
fn read_i64(data: &[u8], offset: usize) -> i64 {
    read_u64(data, offset) as i64
}
