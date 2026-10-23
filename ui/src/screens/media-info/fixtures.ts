// Card 4.D2 — fixture-driven tests: fixtures shaped EXACTLY like the
// `media.get-info` command result type (generated MediaInfo). Three cases:
// full multi-stream media, sparse (missing fields everywhere), and live
// audio-only. Consumed by the panel/screen tests via FakeTransport.

import type { MediaInfo } from "../../client/transport";

/** Rich case: video ×2 (one fully-specified, one sparse), audio ×2,
 * subtitles ×2 (stream + external), metadata tags, chapters. */
export const fullMediaInfo: MediaInfo = {
  title: "Big Buck Bunny",
  container: "Matroska (MKV)",
  duration: { kind: "known", time: 635_000 },
  isLive: false,
  seekable: true,
  source: { kind: "file", path: "/media/big-buck-bunny.mkv" },
  metadata: {
    ARTIST: "Blender Foundation",
    DATE: "2008",
    ENCODER: "Lavf61.1.100",
  },
  chapters: [
    {
      index: 0,
      timeStart: 0,
      duration: { kind: "known", time: 635_000 },
      title: "Feature",
    },
  ],
  tracks: [
    {
      id: 1,
      kind: "video",
      isSelected: true,
      isDefault: true,
      origin: "stream",
      language: null,
      name: null,
      codec: { description: "H.264 (AVC)", fourcc: "avc1", profile: "High", level: "4.1" },
      video: { width: 1920, height: 1080, frameRate: 23.976, bitDepth: 8, hdr: false },
    },
    {
      id: 2,
      kind: "video",
      isSelected: false,
      isDefault: false,
      origin: "stream",
      language: null,
      name: "Commentary",
      codec: { description: null, fourcc: "hev1", profile: null, level: null },
      video: { width: 1280, height: 720, frameRate: null, bitDepth: null, hdr: true },
    },
    {
      id: 3,
      kind: "audio",
      isSelected: true,
      isDefault: true,
      origin: "stream",
      language: "eng",
      name: "Surround 5.1",
      codec: { description: "E-AC-3", fourcc: "eac3", profile: null, level: null },
      audio: { channels: 6, sampleRate: 48_000, bitDepth: null, codecParams: null },
    },
    {
      id: 4,
      kind: "audio",
      isSelected: false,
      isDefault: false,
      origin: "stream",
      language: null,
      name: null,
      codec: { description: null, fourcc: null, profile: null, level: null },
      audio: { channels: 2, sampleRate: null, bitDepth: null, codecParams: null },
    },
    {
      id: 5,
      kind: "subtitle",
      isSelected: true,
      isDefault: false,
      origin: "stream",
      language: "eng",
      name: null,
      codec: { description: "SubRip (SRT)", fourcc: null, profile: null, level: null },
    },
    {
      id: 6,
      kind: "subtitle",
      isSelected: false,
      isDefault: false,
      origin: "external-subtitle",
      language: null,
      name: "forced.de.srt",
      codec: { description: null, fourcc: null, profile: null, level: null },
    },
  ],
};

/** Sparse case: every optional field absent, unknown duration, no metadata,
 * no chapters — the honest-unknown rendering proof. */
export const sparseMediaInfo: MediaInfo = {
  title: null,
  container: null,
  duration: { kind: "unknown" },
  isLive: false,
  seekable: true,
  source: { kind: "url", url: "https://example.org/stream" },
  metadata: {},
  chapters: [],
  tracks: [
    {
      id: 1,
      kind: "video",
      isSelected: true,
      isDefault: true,
      origin: "stream",
      language: null,
      name: null,
      codec: { description: null, fourcc: null, profile: null, level: null },
      video: { width: 0, height: 0, frameRate: null, bitDepth: null, hdr: false },
    },
    {
      id: 2,
      kind: "audio",
      isSelected: true,
      isDefault: true,
      origin: "stream",
      language: null,
      name: null,
      codec: { description: null, fourcc: null, profile: null, level: null },
      audio: { channels: 2, sampleRate: null, bitDepth: null, codecParams: null },
    },
  ],
};

/** Live audio-only case (radio): infinite duration, live, unseekable. */
export const liveAudioMediaInfo: MediaInfo = {
  title: "SomaFM Groove Salad",
  container: "MP3 stream",
  duration: { kind: "infinite" },
  isLive: true,
  seekable: false,
  source: { kind: "url", url: "https://ice1.somafm.com/groovesalad-128-mp3" },
  metadata: { GENRE: "Ambient / Downtempo" },
  chapters: [],
  tracks: [
    {
      id: 1,
      kind: "audio",
      isSelected: true,
      isDefault: true,
      origin: "stream",
      language: "eng",
      name: null,
      codec: { description: "MP3", fourcc: ".mp3", profile: null, level: null },
      audio: { channels: 2, sampleRate: 44_100, bitDepth: null, codecParams: null },
    },
  ],
};
