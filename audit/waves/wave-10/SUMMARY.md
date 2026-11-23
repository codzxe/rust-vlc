# WAVE 10 SUMMARY — streaming, network, access, playlists, sout, discovery

| Field | Value |
|---|---|
| Agents | A01–A10 all complete |
| Files audited | 128, 100% covered |
| Findings | S0:0 S1:6 S2:17 S3:~90 S4:~130 |
| Tree purity | clean |

## THE URL ANSWER (§8.10 hostile question)
`lumen open http://...` -> run.rs:126 -> host.rs:890 -> demux_host.rs:362
-> open_http_source:474 -> SchemeRegistry::new() = bare UreqClient (ureq
3.5.0 + rustls, verification always-on, webpki roots) -> RangedResource
(linear-scan windows) -> media_probe (container signatures only) ->
containers/demux. NO retry, NO reconnect, NO clock. TLS is REAL but a
real handshake is REAL-UNPROVEN (all e2e drive the test MockClient over
real loopback sockets; the 2 live ureq tests are permanently #[ignore]d).
- REAL schemes: file, http, https. Everything else (ftp/sftp/rtsp/rtp/
  udp/data/gopher/mms/mmsh/smb/rtmp...) = typed SchemeError::Unsupported.
- .m3u8/.mpd URLs: NOT a typed refusal — silently die at probe (no HLS
  signature in media_probe). Raw .ts URL would play.
- Entire playlists + transcode + device-discovery crates: zero shipped
  dependents. streaming/{dash,hls,reconnect,buffer,auth,remote_playlist,
  clock}: unreachable. Finished-but-orphaned stacks.

## Worst findings
1. [S1, EXECUTED] dash/resolve.rs div-by-zero panics at L735
   (@duration="0") and L410 (@d="0" + negative @r), exit 101 — falsifies
   fuzz.rs's "panic-free" contract (W10-A08).
2. [S1] hls/master.rs absorb/serve usize underflow panic when one
   segment body > max_retained_bytes (config-accepted input); likely
   replicated in media.rs/live.rs (W10-A10; media.rs confirmed S2
   W10-A09).
3. [S1, EXECUTED] timestamp_map.rs:195 i64 overflow on 9-digit-year LOCAL
   field — panic with checks, silent wrap in release; fuzzer-reachable
   (W10-A06).
4. [S2] transcode/queue.rs interrupt() lost-wakeup race (notifies condvar
   without mutex) — cancelled job can hang (W10-A01).
5. [S2] ranged.rs maps EVERY HttpStatus (incl. 403/404) to
   Network{ConnectionLost} which the engine retries as weather (W10-A02).
6. [S2, probe-proven] XSPF extension containers NOT opaque —
   <app:location> in <extension> hijacks entry location; "extension
   content ignored" doc FALSE (W10-A08).
7. [S2] auth.rs redirect credential gate is host-only (port ignored)
   (W10-A03); AuthFetch 100% unwired; Basic+Digest only, no Bearer.
8. [S2 cluster] netfault proxy tests fabricate errors AFTER fully
   successful fetches (faults are fake, user-space) (W10-A03/A05);
   fetch.rs Url::join refuses legal relative Location (breaks shipped
   redirect follower) (W10-A06); sched.rs dynamic+None -> Wait{0.0} spin
   (W10-A04).
9. [S2] transcode reality: zero encoders -> "transcode" = decode->PCM->WAV
   + verbatim copy + h264/aac remux; honest internally, name oversells
   (W10-A07); mp4_remux exists and works (executed, self-referential
   oracle) (W10-A09).
10. [KEY] Matrix row network-dash "partial" inflated: zero playback path
    exists. hls/abr/cue rows are library-island code (W10-A04 et al.).
