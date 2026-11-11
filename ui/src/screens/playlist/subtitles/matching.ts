// w26.b11 — subtitle filename matching (pure logic).
//
// Frames: "sub filename.svg" pins the strategy selector (Off / Any title /
// Contains title / Title with extra characters / Exact title match) and
// "Add Subtitles to Playlist/Folder.svg" pins its product: the matched-pairs
// review list (media file ↔ subtitle file per row, natural order).
//
// Matching normalizes like VLC's auto-detect: strip the extension, strip
// release tags (everything from the first tag separator in [_.- ] space),
// lowercase, collapse separators. Strategies then compare the media TITLE
// against the subtitle's normalized stem:
//   off        — never match (the honest empty review list)
//   any-title  — both sides non-empty after normalization
//   contains   — one normalized stem contains the other
//   extra      — subtitle stem equals the title OR extends it with a
//                separator-delimited suffix ("Title.English" ← the
//                Folder-2 candidates "The_Wand_S01E01_English.srt")
//   exact      — normalized stems are identical
//
// Candidate ordering is NATURAL everywhere (naturalSort from the list
// primitives — "S01E2" before "S01E10", never lexical).

import { naturalSort } from "../../../components/list";

/** The five strategies of sub filename.svg (order = the frame's row order). */
export const MATCH_MODES = ["off", "any", "contains", "extra", "exact"] as const;

export type MatchMode = (typeof MATCH_MODES)[number];

/** Default per the frame: "Title with extra characters" is the checked row. */
export const DEFAULT_MATCH_MODE: MatchMode = "extra";

/** Subtitle extensions VLC auto-detects (doc 05 §2 subtitle track kinds). */
const SUBTITLE_EXTENSIONS: readonly string[] = ["srt", "ass", "ssa", "vtt", "sub", "idx", "ttml", "sbv"];

/** True when a candidate file looks like a subtitle (by extension). */
export function isSubtitleFile(fileName: string): boolean {
  const dot = fileName.lastIndexOf(".");
  if (dot <= 0) return false;
  return SUBTITLE_EXTENSIONS.includes(fileName.slice(dot + 1).toLowerCase());
}

/** Split "The_Wand_S01E01_English.srt" → stem "The_Wand_S01E01_English" + "srt". */
export function splitExtension(fileName: string): { stem: string; extension: string } {
  const dot = fileName.lastIndexOf(".");
  if (dot <= 0) return { stem: fileName, extension: "" };
  return { stem: fileName.slice(0, dot), extension: fileName.slice(dot + 1) };
}

/**
 * Normalize a stem for matching: lowercase, fold every [_.- ] run into one
 * space, trim. "The_Wand_S01E01_English" → "the wand s01e01 english".
 */
export function normalizeStem(stem: string): string {
  return stem
    .toLowerCase()
    .replace(/[\s_.-]+/g, " ")
    .trim();
}

/**
 * Release-tag title: the part of the stem before the first tag group
 * (quality/source markers like 720p, web, x264 …). Conservative — only
 * well-known tag words are cut so real titles are never clipped.
 */
const TAG_WORDS: readonly string[] = [
  "720p", "1080p", "2160p", "480p",
  "hdtv", "web", "webrip", "web-dl", "bluray", "brrip", "dvdrip", "hdrip",
  "x264", "x265", "h264", "h265", "hevc", "avc",
  "aac", "ac3", "eac3", "dts", "flac",
  "proper", "repack", "internal",
];

/** "the wand s01e01 720p web" → "the wand s01e01" (title before tags). */
export function titleOfStem(normalized: string): string {
  const words = normalized.split(" ");
  const cut = words.findIndex((word) => TAG_WORDS.includes(word));
  return cut > 0 ? words.slice(0, cut).join(" ") : normalized;
}

/** Does `subtitleFile` match `mediaTitle` (a display title or filename)? */
export function matchesMode(mode: MatchMode, mediaTitle: string, subtitleFile: string): boolean {
  if (mode === "off") return false;
  const { stem } = splitExtension(subtitleFile);
  const sub = normalizeStem(stem);
  const media = titleOfStem(normalizeStem(splitExtension(mediaTitle).stem));
  if (media === "" || sub === "") return false;
  switch (mode) {
    case "any":
      return true;
    case "contains":
      return sub.includes(media) || media.includes(sub);
    case "extra": {
      // Equal, or the subtitle adds separator-delimited words after the
      // full title ("… s01e01 english" extends "… s01e01").
      if (sub === media) return true;
      return sub.startsWith(`${media} `);
    }
    case "exact":
      return sub === media;
  }
}

/** One matched pair (Folder.svg row: media file ↔ subtitle file). */
export interface SubtitlePair {
  readonly mediaName: string;
  readonly subtitleName: string;
}

/**
 * Build the matched-pairs review list (Folder.svg body): for every media
 * entry (natural order by name) take the FIRST subtitle candidate that
 * matches under `mode` (candidates natural-sorted first — deterministic).
 */
export function pairSubtitles(
  mediaNames: readonly string[],
  candidates: readonly string[],
  mode: MatchMode,
): SubtitlePair[] {
  const sortedCandidates = naturalSort(
    candidates.filter((name) => isSubtitleFile(name)),
    (name) => name,
  );
  const used = new Set<string>();
  const pairs: SubtitlePair[] = [];
  for (const mediaName of naturalSort(mediaNames, (name) => name)) {
    const candidate = sortedCandidates.find(
      (name) => !used.has(name) && matchesMode(mode, mediaName, name),
    );
    if (candidate === undefined) continue;
    used.add(candidate);
    pairs.push({ mediaName, subtitleName: candidate });
  }
  return pairs;
}
