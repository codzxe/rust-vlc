// DemoTransport — the FRIENDLY simulated engine for the BROWSER DEV SHELL
// ONLY (Solstice app-shell workstream, 2026-11-08).
//
// Scope contract, stated honestly:
//   * This module exists so `npm run dev` (browser shell) shows a living app:
//     opening media, a playback clock, playlist semantics, track selection,
//     volume/mute — all speaking the EXACT 1.D4 wire protocol
//     (src/generated/commands.ts) and emitting the same Event union a real
//     engine emits. It is a UI-side simulation, NOT an engine: no bytes are
//     decoded, no clocks are wall-locked, no audio leaves the speakers.
//   * The desktop host injects the real transport (TauriTransport,
//     client/tauri-transport.ts) at the main.tsx seam — that wiring is a
//     separate GitHub issue. Nothing in this file fabricates engine
//     capability: commands the demo cannot simulate (filesystem playlist
//     save/load, config.*, plugins, engine info) answer with the typed
//     `feature-not-exposed` error envelope — the same honesty rule
//     MockTransport follows, applied per-command instead of globally.
//
// TIMING UNITS — a deliberate, documented choice: the player state seam
// (screens/player/context.tsx) documents `durationSeconds`/`position` in
// SECONDS and reads `Duration.time` / `timing.time-tick.position` directly,
// so the demo transport speaks seconds on every timing surface (snapshot,
// duration-known, time-tick, current-time, seek). The media-info fixtures
// carry millisecond `Duration.time` (formatDurationMs); the demo converts
// the seeded fixtures to seconds so the PLAYER — the surface this transport
// exists to light up — renders truthful timecodes. The ms-vs-seconds split
// between the player and media-info screens is a pre-existing
// inconsistency flagged to the lead; it predates this module.

import type { MessageKey } from "../i18n/messages";
import { fullMediaInfo, liveAudioMediaInfo } from "../screens/media-info/fixtures";
import type {
  CommandListing,
  CommandRequest,
  CommandResponse,
  Event,
  MediaInfo,
  PlaybackState,
  PlaylistItemId,
  RepeatMode,
  SeekTarget,
  SourceSpec,
  StateSnapshot,
  TrackInfo,
  TrackSelections,
  TrackSelector,
} from "../generated/commands";
import type { CommandTransport } from "./transport";

// ---------------------------------------------------------------------------
// Demo clip catalog (DATA, not UI copy — titles render via open.clip.* keys)
// ---------------------------------------------------------------------------

/** One pickable demo clip; `titleKey` renders through useMsg(). */
export interface DemoClip {
  readonly id: "bunny" | "ambient" | "live" | "radio";
  readonly titleKey: MessageKey;
  readonly source: SourceSpec;
}

/** The four clips the open-media dialog offers (see screens/open-media). */
export const DEMO_CLIPS: readonly DemoClip[] = [
  { id: "bunny", titleKey: "open.clip.bunny", source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" } },
  { id: "ambient", titleKey: "open.clip.ambient", source: { kind: "file", path: "/media/demo/ambient-loop.flac" } },
  { id: "live", titleKey: "open.clip.live", source: { kind: "url", url: "https://demo.lumen.dev/live/edge-hsl.m3u8" } },
  { id: "radio", titleKey: "open.clip.radio", source: { kind: "url", url: "https://ice1.somafm.com/groovesalad-128-mp3" } },
];

/** A remembered opening (the open-media dialog's "Recent" section). */
export interface DemoRecentEntry {
  readonly source: SourceSpec;
  readonly title: string | null;
}

// ---------------------------------------------------------------------------
// Seeded MediaInfo (adapted from the 4.D2 fixtures; durations → seconds)
// ---------------------------------------------------------------------------

const knownSeconds = (time: number): { kind: "known"; time: number } => ({ kind: "known", time });

const AMBIENT_AUDIO: TrackInfo = {
  id: 1,
  kind: "audio",
  isSelected: true,
  isDefault: true,
  origin: "stream",
  language: null,
  name: "Stereo",
  codec: { description: "FLAC", fourcc: "fLaC", profile: null, level: null },
  audio: { channels: 2, sampleRate: 44_100, bitDepth: 16, codecParams: null },
};

const LIVE_VIDEO: TrackInfo = {
  id: 1,
  kind: "video",
  isSelected: true,
  isDefault: true,
  origin: "stream",
  language: null,
  name: null,
  codec: { description: "H.264 (AVC)", fourcc: "avc1", profile: "Main", level: "3.1" },
  video: { width: 1280, height: 720, frameRate: 30, bitDepth: 8, hdr: false },
};

const LIVE_AUDIO: TrackInfo = {
  id: 2,
  kind: "audio",
  isSelected: true,
  isDefault: true,
  origin: "stream",
  language: "eng",
  name: null,
  codec: { description: "AAC", fourcc: "mp4a", profile: "LC", level: null },
  audio: { channels: 2, sampleRate: 48_000, bitDepth: null, codecParams: null },
};

/** Big Buck Bunny — the 4.D2 rich fixture (635 s), duration in seconds. */
const bunnyInfo: MediaInfo = { ...fullMediaInfo, duration: knownSeconds(635) };

/** Ambient loop — audio-only, known 180 s (built like the sparse fixture). */
const ambientInfo: MediaInfo = {
  title: "Ambient loop",
  container: "FLAC",
  duration: knownSeconds(180),
  isLive: false,
  seekable: true,
  source: { kind: "file", path: "/media/demo/ambient-loop.flac" },
  metadata: { ARTIST: "Lumen demo reel" },
  chapters: [],
  tracks: [AMBIENT_AUDIO],
};

/** Live edge — HLS: infinite, live, unseekable. */
const liveInfo: MediaInfo = {
  title: "Live edge",
  container: "HLS stream",
  duration: { kind: "infinite" },
  isLive: true,
  seekable: false,
  source: { kind: "url", url: "https://demo.lumen.dev/live/edge-hls.m3u8" },
  metadata: {},
  chapters: [],
  tracks: [LIVE_VIDEO, LIVE_AUDIO],
};

/** Night radio — the 4.D2 live-audio fixture (infinite duration, verbatim). */
const radioInfo: MediaInfo = liveAudioMediaInfo;

const CLIP_INFOS: ReadonlyMap<string, MediaInfo> = new Map([
  ["/media/demo/big-buck-bunny.mkv", bunnyInfo],
  ["/media/demo/ambient-loop.flac", ambientInfo],
  ["https://demo.lumen.dev/live/edge-hsl.m3u8", liveInfo],
  ["https://ice1.somafm.com/groovesalad-128-mp3", radioInfo],
]);

const sourceKey = (source: SourceSpec): string =>
  source.kind === "file" ? source.path : source.url;

const sourceTitle = (source: SourceSpec): string | null => {
  const info = CLIP_INFOS.get(sourceKey(source));
  if (info !== undefined) return info.title ?? null;
  if (source.kind === "file") {
    const base = source.path.split("/").pop();
    return base !== undefined && base.length > 0 ? base : null;
  }
  try {
    return new URL(source.url).hostname;
  } catch {
    return null;
  }
};

/** Synthesized info for sources outside the clip catalog (network streams). */
function infoForSource(source: SourceSpec): MediaInfo {
  const seeded = CLIP_INFOS.get(sourceKey(source));
  if (seeded !== undefined) return { ...seeded, source };
  const isNetwork = source.kind === "url";
  return {
    title: sourceTitle(source),
    container: isNetwork ? "Network stream" : null,
    duration: { kind: "infinite" },
    isLive: isNetwork,
    seekable: false,
    source,
    metadata: {},
    chapters: [],
    tracks: isNetwork ? [LIVE_AUDIO] : [AMBIENT_AUDIO],
  };
}

// ---------------------------------------------------------------------------
// The command listing (drives the CustomizePanels capability gate)
// ---------------------------------------------------------------------------

const LISTED_COMMANDS: readonly string[] = [
  "session.open",
  "session.close",
  "playback.play",
  "playback.pause",
  "playback.toggle-play-pause",
  "playback.stop",
  "playback.seek",
  "playback.set-rate",
  "playback.set-position",
  "playback.step-frame",
  "tracks.list",
  "tracks.get-info",
  "tracks.set-video-track",
  "tracks.set-audio-track",
  "tracks.set-subtitle-track",
  "tracks.set-secondary-audio",
  "audio.set-volume",
  "audio.get-volume",
  "audio.set-mute",
  "media.get-info",
  "media.get-current-time",
  "media.get-duration",
  "media.get-chapters",
  "media.set-chapter",
  "playlist.enqueue",
  "playlist.enqueue-many",
  "playlist.play-item",
  "playlist.next",
  "playlist.prev",
  "playlist.skip",
  "playlist.remove",
  "playlist.move",
  "playlist.clear",
  "playlist.set-shuffle",
  "playlist.set-repeat",
  "system.get-state-snapshot",
  "system.list-commands",
];

const COMMAND_LISTINGS: readonly CommandListing[] = LISTED_COMMANDS.map((id) => ({
  class: "s",
  deprecated: null,
  id,
  requires: [],
  since: "0.1.0",
}));

// ---------------------------------------------------------------------------
// Internal playlist item
// ---------------------------------------------------------------------------

interface DemoPlaylistItem {
  readonly id: PlaylistItemId;
  readonly source: SourceSpec;
  readonly title: string | null;
}

/** Distributive Omit (plain Omit collapses the Event union). */
type DistributiveOmit<T, K extends keyof never> = T extends unknown ? Omit<T, K> : never;

/** An Event draft before the transport stamps its monotonic `seq`. */
type EventDraft = DistributiveOmit<Event, "seq">;

// ---------------------------------------------------------------------------
// DemoTransport
// ---------------------------------------------------------------------------

/** Playback-clock tick: 500 ms of simulated media time per interval. */
const TICK_MS = 500;
/** Fixed (jitter-free) opening choreography timings. */
const OPENING_PROGRESS_MS = 160;
const OPENING_PLAY_MS = 280;
const OPENING_INFO_MS = 320;
/** Pause before auto-advancing to the next playlist item at end-of-media. */
const AUTO_ADVANCE_MS = 480;
/** Recent-openings cap (the open-media dialog's "Recent" section). */
const RECENT_LIMIT = 6;

/**
 * The browser dev-shell transport. See the module header for the honesty
 * contract. Deterministic by construction: fixed delays, sequential
 * playlist advancement (the shuffle FLAG is reported truthfully; a real
 * re-order is engine work), no jitter anywhere.
 */
export class DemoTransport implements CommandTransport {
  private readonly listeners = new Set<(event: Event) => void>();
  private readonly timers = new Set<ReturnType<typeof setTimeout>>();

  // Wire bookkeeping
  private generation = 0;
  private seq = 0;
  private session: number | null = null;

  // Engine state
  private state: PlaybackState = "idle";
  private info: MediaInfo | null = null;
  private position = 0;
  private rate = 1;
  private volume = 0.8;
  private muted = false;
  private selections: TrackSelections = {};

  // Playlist state
  private items: DemoPlaylistItem[] = [];
  private currentId: PlaylistItemId | null = null;
  private shuffle = false;
  private repeat: RepeatMode = "off";
  private nextPlaylistId = 1;

  // Open-media history
  private recent: DemoRecentEntry[] = [];

  constructor() {
    // One interval for the transport's whole life; tick() is inert unless
    // state === "playing" (the clock body checks before advancing).
    const clock = setInterval(() => this.tick(), TICK_MS);
    this.timers.add(clock);
  }

  /** Releases the clock + pending choreography (test hygiene, teardown). */
  dispose(): void {
    for (const timer of this.timers) clearTimeout(timer);
    this.timers.clear();
    this.listeners.clear();
  }

  /** The open-media dialog's "Recent" section (most recent first). */
  get recentOpenings(): readonly DemoRecentEntry[] {
    return [...this.recent].reverse();
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    const gen = ++this.generation;
    const { requestId } = request;
    const respond = (result: CommandResponse["result"]): CommandResponse => ({
      gen,
      requestId,
      result,
      session: this.session ?? undefined,
    });
    const notExposed = (): CommandResponse =>
      respond({
        error: {
          code: "feature-not-exposed",
          details: { cmd: request.cmd },
          message: "demo transport: command not simulated",
          stage: "control",
        },
      });

    switch (request.cmd) {
      // --- system -------------------------------------------------------
      case "system.list-commands":
        return respond({
          completed: { kind: "command-listings", commands: [...COMMAND_LISTINGS] },
        });
      case "system.get-state-snapshot":
        return respond({ completed: { kind: "state-snapshot", snapshot: this.snapshot() } });
      case "system.get-engine-info":
        return notExposed();

      // --- session ------------------------------------------------------
      case "session.open":
        this.openSource(
          request.args.source,
          request.args.options?.startPaused === true,
          request.args.options?.startTime ?? 0,
        );
        return respond({ accepted: { gen, session: this.session ?? undefined } });
      case "session.close":
        this.position = 0;
        this.transition("idle");
        return respond({ completed: { kind: "unit" } });

      // --- playback -----------------------------------------------------
      case "playback.play":
        this.play();
        return respond({ completed: { kind: "unit" } });
      case "playback.pause":
        if (this.state === "playing") this.transition("paused");
        return respond({ completed: { kind: "unit" } });
      case "playback.toggle-play-pause":
        if (this.state === "playing") this.transition("paused");
        else this.play();
        return respond({ completed: { kind: "unit" } });
      case "playback.stop":
        this.position = 0;
        if (this.state !== "idle") this.transition("stopped");
        return respond({ completed: { kind: "unit" } });
      case "playback.seek":
        this.seek(request.args.target);
        return respond({ completed: { kind: "unit" } });
      case "playback.set-rate":
        this.rate = request.args.rate;
        this.emit({ kind: "playback.rate-changed", rate: request.args.rate });
        return respond({ completed: { kind: "applied-rate", rate: request.args.rate } });
      case "playback.set-position":
        this.seek({ kind: "percent", fraction: request.args.fraction });
        return respond({ completed: { kind: "unit" } });
      case "playback.step-frame":
        this.seek({
          kind: "relative",
          deltaMs: request.args.direction === "next" ? 1000 / 24 : -1000 / 24,
        });
        return respond({ completed: { kind: "unit" } });

      // --- tracks -------------------------------------------------------
      case "tracks.list":
        return respond({
          completed: { kind: "track-list", tracks: this.info?.tracks ?? [] },
        });
      case "tracks.get-info": {
        const track = this.info?.tracks.find((t) => t.id === request.args.id);
        return track === undefined
          ? respond({
              error: {
                code: "not-found",
                details: { id: String(request.args.id) },
                message: "demo transport: no such track",
                stage: "control",
              },
            })
          : respond({ completed: { kind: "track-info", info: track } });
      }
      case "tracks.set-video-track":
        return respond(this.applySelector("video", request.args.selector));
      case "tracks.set-audio-track":
        return respond(this.applySelector("audio", request.args.selector));
      case "tracks.set-subtitle-track":
        return respond(this.applySelector("subtitle", request.args.selector));
      case "tracks.set-secondary-audio":
        return respond(this.applySelector("secondaryAudio", request.args.selector));

      // --- audio --------------------------------------------------------
      case "audio.set-volume":
        this.volume = Math.max(0, Math.min(1, request.args.volume));
        return respond({ completed: { kind: "audio-volume", volume: this.volume } });
      case "audio.get-volume":
        return respond({ completed: { kind: "audio-volume", volume: this.volume } });
      case "audio.set-mute":
        this.muted = request.args.muted;
        return respond({ completed: { kind: "applied-bool", value: this.muted } });

      // --- playlist -----------------------------------------------------
      case "playlist.enqueue":
        return respond(this.enqueue(request.args.source, request.args.title ?? null));
      case "playlist.enqueue-many": {
        const ids: PlaylistItemId[] = [];
        for (const item of request.args.items) {
          const result = this.enqueue(item.source, item.title ?? null);
          if ("completed" in result && result.completed.kind === "playlist-item") {
            ids.push(result.completed.id);
          }
        }
        return respond({ completed: { kind: "playlist-items", ids } });
      }
      case "playlist.play-item": {
        const item = this.items.find((entry) => entry.id === request.args.id);
        if (item === undefined) {
          return respond({
            error: {
              code: "not-found",
              details: { id: String(request.args.id) },
              message: "demo transport: no such playlist item",
              stage: "playlist",
            },
          });
        }
        this.setCurrent(item.id);
        this.openSource(item.source, false, 0);
        return respond({ accepted: { gen, session: this.session ?? undefined } });
      }
      case "playlist.next":
        this.stepPlaylist(1);
        return respond({ completed: { kind: "unit" } });
      case "playlist.prev":
        this.stepPlaylist(-1);
        return respond({ completed: { kind: "unit" } });
      case "playlist.skip":
        this.stepPlaylist(request.args.n);
        return respond({ completed: { kind: "unit" } });
      case "playlist.remove":
        return respond(this.removeItem(request.args.id));
      case "playlist.move":
        return respond(this.moveItem(request.args.id, request.args.toIndex));
      case "playlist.clear":
        this.items = [];
        this.currentId = null;
        this.position = 0;
        this.transition("idle");
        this.emit({ kind: "playlist.changed", change: { kind: "cleared" } });
        return respond({ completed: { kind: "unit" } });
      case "playlist.set-shuffle":
        this.shuffle = request.args.enabled;
        // Honest simulation: the FLAG flips and is reported; the demo keeps
        // sequential advancement (a real shuffle re-order is engine work).
        this.emit({
          kind: "playlist.changed",
          change: { kind: "shuffle-set", enabled: this.shuffle },
        });
        return respond({ completed: { kind: "applied-bool", value: this.shuffle } });
      case "playlist.set-repeat":
        this.repeat = request.args.mode;
        this.emit({
          kind: "playlist.changed",
          change: { kind: "repeat-set", mode: this.repeat },
        });
        return respond({ completed: { kind: "applied-repeat", mode: this.repeat } });

      // --- media --------------------------------------------------------
      case "media.get-info":
        if (this.info === null || this.state === "idle") {
          return respond({
            error: {
              code: "not-found",
              details: {},
              message: "demo transport: no media open",
              stage: "input",
            },
          });
        }
        return respond({ completed: { kind: "media-info", info: this.info } });
      case "media.get-current-time":
        return respond({
          completed: {
            kind: "current-time",
            position: this.position,
            time: Math.round(this.position * 1000),
            buffered: this.bufferedRanges(),
          },
        });
      case "media.get-duration":
        return respond({
          completed: { kind: "duration", duration: this.info?.duration ?? { kind: "unknown" } },
        });
      case "media.get-chapters":
        return respond({
          completed: { kind: "chapters", chapters: this.info?.chapters ?? [] },
        });
      case "media.set-chapter": {
        const chapter = (this.info?.chapters ?? [])[request.args.index];
        if (chapter === undefined) {
          return respond({
            error: {
              code: "out-of-range",
              details: { index: String(request.args.index) },
              message: "demo transport: no such chapter",
              stage: "input",
            },
          });
        }
        this.seek({ kind: "absolute", time: chapter.timeStart / 1000 });
        return respond({ completed: { kind: "unit" } });
      }

      // --- honestly not simulated ---------------------------------------
      default:
        return notExposed();
    }
  }

  // -------------------------------------------------------------------------
  // Snapshot + events
  // -------------------------------------------------------------------------

  private snapshot(): StateSnapshot {
    return {
      audioDelayMs: 0,
      devices: [
        { id: "demo-stereo", isDefault: true, name: "Demo stereo out" },
        { id: "demo-hdmi", isDefault: false, name: "Demo HDMI" },
      ],
      duration: this.info?.duration ?? { kind: "unknown" },
      muted: this.muted,
      playlist: {
        current: this.currentId,
        itemCount: this.items.length,
        repeat: this.repeat,
        shuffle: this.shuffle,
      },
      rate: this.rate,
      selections: this.selections,
      state: this.state,
      subtitle: {},
      subtitleDelayMs: 0,
      time: this.state === "idle" ? null : Math.round(this.position * 1000),
      tracks: this.info?.tracks ?? [],
      videoDelayMs: 0,
      volume: this.volume,
    };
  }

  private bufferedRanges(): { end: number; start: number }[] {
    if (this.info === null) return [];
    if (this.info.duration.kind === "known") {
      return [{ start: 0, end: this.info.duration.time }];
    }
    // Live edge: a 30 s sliding window behind the playhead.
    return [{ start: Math.max(0, this.position - 30), end: this.position }];
  }

  private emit(draft: EventDraft): void {
    this.seq += 1;
    const event = { ...draft, seq: this.seq, session: this.session ?? undefined } as Event;
    for (const listener of this.listeners) {
      listener(event);
    }
  }

  /** Fire an event after a fixed delay (the opening choreography). */
  private emitIn(ms: number, draft: EventDraft): void {
    this.schedule(ms, () => this.emit(draft));
  }

  /** setTimeout with dispose-tracked hygiene. */
  private schedule(ms: number, run: () => void): void {
    const timer = setTimeout(() => {
      this.timers.delete(timer);
      run();
    }, ms);
    this.timers.add(timer);
  }

  private transition(to: PlaybackState): void {
    const from = this.state;
    this.state = to;
    this.emit({ kind: "session.state-changed", from, to });
  }

  private transitionIn(ms: number, to: PlaybackState): void {
    this.schedule(ms, () => this.transition(to));
  }

  // -------------------------------------------------------------------------
  // Opening + the playback clock
  // -------------------------------------------------------------------------

  private openSource(source: SourceSpec, startPaused: boolean, startTime: number): void {
    this.info = infoForSource(source);
    this.position = startTime;
    this.selections = {};
    this.session = 1;
    this.rememberOpening(source);

    // Opening choreography (fixed timings, no jitter): the accepted response
    // returns first; the state machine then walks
    //   idle → opening → (progress) → playing/paused, followed by the info
    // burst (media.info-ready, tracks.list-ready, duration-known, first
    // time-tick) — the same event set a real engine emits for a new session.
    this.transition("opening");
    this.emitIn(OPENING_PROGRESS_MS, { kind: "session.opening-progress", progress: 0.5 });
    this.emitIn(OPENING_PLAY_MS, { kind: "session.buffering", progress: 0.9 });
    this.transitionIn(OPENING_PLAY_MS + 40, startPaused ? "paused" : "playing");
    this.schedule(OPENING_INFO_MS, () => this.emitInfoBurst());
  }

  private emitInfoBurst(): void {
    const info = this.info;
    if (info === null) return;
    this.emit({ kind: "media.info-ready", info });
    if (info.tracks.length > 0) {
      this.emit({ kind: "tracks.list-ready", tracks: info.tracks });
    }
    if (info.duration.kind === "known") {
      this.emit({ kind: "timing.duration-known", duration: info.duration.time });
    }
    this.emit({
      kind: "timing.time-tick",
      position: this.position,
      time: Math.round(this.position * 1000),
    });
  }

  private play(): void {
    if (this.state === "paused") {
      this.transition("playing");
      return;
    }
    if (this.state === "stopped" || this.state === "ended" || this.state === "error") {
      this.position = 0;
      this.transition("playing");
      return;
    }
    if (this.state === "idle" && this.currentId !== null) {
      const item = this.items.find((entry) => entry.id === this.currentId);
      if (item !== undefined) this.openSource(item.source, false, 0);
    }
  }

  private seek(target: SeekTarget): void {
    if (this.info === null) return;
    const known = this.info.duration.kind === "known" ? this.info.duration.time : null;
    let next = this.position;
    switch (target.kind) {
      case "absolute":
        next = target.time;
        break;
      case "relative":
        next = this.position + target.deltaMs / 1000;
        break;
      case "percent":
        next = (known ?? 0) * target.fraction;
        break;
      case "chapter": {
        const chapter = this.info.chapters[target.index];
        if (chapter !== undefined) next = chapter.timeStart / 1000;
        break;
      }
    }
    next = known !== null ? Math.max(0, Math.min(known, next)) : Math.max(0, next);
    this.position = next;
    this.emit({
      kind: "timing.seek-completed",
      position: Math.round(next * 1000),
      seekKind: "command",
    });
    this.emit({
      kind: "timing.time-tick",
      position: next,
      time: Math.round(next * 1000),
    });
  }

  private tick(): void {
    if (this.state !== "playing" || this.info === null) return;
    this.position += (TICK_MS / 1000) * this.rate;
    const known = this.info.duration.kind === "known" ? this.info.duration.time : null;

    if (known !== null && this.position >= known) {
      this.emit({ kind: "playback.end-of-media" });
      if (this.repeat === "one") {
        this.position = 0;
        this.emit({ kind: "timing.seek-completed", position: 0, seekKind: "command" });
        this.emit({ kind: "timing.time-tick", position: 0, time: 0 });
        return;
      }
      this.transition("ended");
      const index = this.items.findIndex((entry) => entry.id === this.currentId);
      const hasNext =
        this.currentId !== null &&
        this.items.length > 0 &&
        (this.repeat === "all" || index < this.items.length - 1);
      if (hasNext) {
        // Auto-advance after a fixed pause (wrap-around honors repeat "all").
        this.schedule(AUTO_ADVANCE_MS, () => this.stepPlaylist(1));
      }
      return;
    }

    this.emit({
      kind: "timing.time-tick",
      position: this.position,
      time: Math.round(this.position * 1000),
    });
  }

  // -------------------------------------------------------------------------
  // Playlist helpers
  // -------------------------------------------------------------------------

  private enqueue(source: SourceSpec, title: string | null): CommandResponse["result"] {
    const id = this.nextPlaylistId;
    this.nextPlaylistId += 1;
    this.items = [...this.items, { id, source, title: title ?? sourceTitle(source) }];
    this.emit({ kind: "playlist.changed", change: { kind: "items-added", ids: [id] } });
    return { completed: { kind: "playlist-item", id } };
  }

  private setCurrent(id: PlaylistItemId | null): void {
    this.currentId = id;
    if (id !== null) {
      this.emit({ kind: "playlist.changed", change: { kind: "current-changed", id } });
    }
  }

  /**
   * Move the current pointer by `delta` items (wraps at both ends) and open
   * the target item — next/prev start playback in every media player the
   * demo imitates; with an empty playlist this is an honest no-op.
   */
  private stepPlaylist(delta: number): void {
    if (this.items.length === 0) return;
    const index = this.items.findIndex((entry) => entry.id === this.currentId);
    const nextIndex =
      index < 0
        ? delta >= 0
          ? 0
          : this.items.length - 1
        : (index + delta + this.items.length) % this.items.length;
    const item = this.items[nextIndex];
    if (item === undefined) return;
    this.setCurrent(item.id);
    this.openSource(item.source, false, 0);
  }

  private removeItem(id: PlaylistItemId): CommandResponse["result"] {
    const index = this.items.findIndex((entry) => entry.id === id);
    if (index < 0) {
      return {
        error: {
          code: "not-found",
          details: { id: String(id) },
          message: "demo transport: no such playlist item",
          stage: "playlist",
        },
      };
    }
    this.items = this.items.filter((entry) => entry.id !== id);
    this.emit({ kind: "playlist.changed", change: { kind: "items-removed", ids: [id] } });
    if (this.currentId === id) {
      const successor = this.items[Math.min(index, this.items.length - 1)];
      this.setCurrent(successor === undefined ? null : successor.id);
      if (successor === undefined) {
        this.position = 0;
        this.transition("idle");
      }
    }
    return { completed: { kind: "unit" } };
  }

  private moveItem(id: PlaylistItemId, toIndex: number): CommandResponse["result"] {
    const from = this.items.findIndex((entry) => entry.id === id);
    if (from < 0) {
      return {
        error: {
          code: "not-found",
          details: { id: String(id) },
          message: "demo transport: no such playlist item",
          stage: "playlist",
        },
      };
    }
    const items = [...this.items];
    const [item] = items.splice(from, 1);
    const to = Math.max(0, Math.min(toIndex, items.length));
    items.splice(to, 0, item!);
    this.items = items;
    this.emit({ kind: "playlist.changed", change: { kind: "moved", from, id, to } });
    return { completed: { kind: "unit" } };
  }

  private applySelector(
    kind: "video" | "audio" | "subtitle" | "secondaryAudio",
    selector: TrackSelector,
  ): CommandResponse["result"] {
    this.selections = { ...this.selections, [kind]: selector.kind === "id" ? selector.id : null };
    this.emit({
      kind: "tracks.changed",
      trackKind: kind === "secondaryAudio" ? "secondary-audio" : kind,
      selected: selector.kind === "id" ? selector.id : null,
    });
    return { completed: { kind: "applied-selector", selector } };
  }

  private rememberOpening(source: SourceSpec): void {
    const entry: DemoRecentEntry = { source, title: sourceTitle(source) };
    this.recent = [
      entry,
      ...this.recent.filter((prev) => sourceKey(prev.source) !== sourceKey(source)),
    ].slice(0, RECENT_LIMIT);
  }
}

/** Type guard: does this transport expose the demo "recent" list? */
export function isDemoTransport(transport: CommandTransport): transport is DemoTransport {
  return transport instanceof DemoTransport;
}
