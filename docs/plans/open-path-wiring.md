# Open-Path Wiring — Investigation & Seam Design (6.OPW-0)

Status: IN PROGRESS (skeleton committed first; content filling in this session).
Branch: `wip/p6-opw0`. Card: 6.OPW-0 (Phase 6, Wave 1) — investigation/design, zero-code default.

## 1. Current open path map
(TODO: apps/cli main → run → dispatch → input → media-probe → playback-engine → demux host → codec host → output; the AVI/MJPEG-only gate and its typed refusal site, with file:line.)

## 2. Demux public API surface per container
(TODO: crates/demux/src/{mp4,mkv,ogg,wav,aiff,ts,ps,adts,es,ivf,avi} constructors/iterators, track/packet types, uniform vs bespoke.)

## 3. Codec dispatch surface
(TODO: how AVI/MJPEG picks decoders today; audio flac/mp3/aac/mp2/vorbis/alac/adpcm/pcm + video mjpeg/av1 exposed API; subtitle path status.)

## 4. The seam: per-container host registry
(TODO: crate location, trait shape, codec binding, honest refusal, single-file one-line registry entry.)

## 5. Per-container implementation-unit checklists
(TODO: mp4/mov/m4a, mkv/webm/mka, ogg/ogv, wav, aiff, ts, adts+aac, es(h264/hevc), ivf, ps, playlists — files, gates, corpus lanes, risks.)
