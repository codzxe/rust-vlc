// w26.b11 — matching logic tests (sub filename.svg strategies + the
// Folder.svg pairing): every strategy branch, natural order, tag-stripping
// and the deterministic first-candidate pairing.

import { describe, expect, it } from "vitest";
import {
  DEFAULT_MATCH_MODE,
  MATCH_MODES,
  isSubtitleFile,
  matchesMode,
  normalizeStem,
  pairSubtitles,
  splitExtension,
  titleOfStem,
} from "./matching";

const MEDIA = "The_Wand_S01E01.mp4";

describe("matching — extensions + normalization", () => {
  it("recognizes subtitle extensions (case-insensitive) and rejects others", () => {
    expect(isSubtitleFile("a.SRT")).toBe(true);
    expect(isSubtitleFile("b.en.vtt")).toBe(true);
    expect(isSubtitleFile(MEDIA)).toBe(false);
    expect(isSubtitleFile("noext")).toBe(false);
  });

  it("splits the extension off the stem", () => {
    expect(splitExtension("The_Wand_S01E01_English.srt")).toEqual({
      stem: "The_Wand_S01E01_English",
      extension: "srt",
    });
    expect(splitExtension("plain")).toEqual({ stem: "plain", extension: "" });
  });

  it("normalizes separator runs to single spaces, lowercase", () => {
    expect(normalizeStem("The_Wand-S01E01.english")).toBe("the wand s01e01 english");
    expect(normalizeStem("  spaced__out. ")).toBe("spaced out");
  });

  it("cuts the release-tag tail off a normalized title", () => {
    expect(titleOfStem("the wand s01e01 720p web x264")).toBe("the wand s01e01");
    // No tags → the whole stem is the title.
    expect(titleOfStem("the wand s01e01")).toBe("the wand s01e01");
  });
});

describe("matching — the five strategies (sub filename.svg)", () => {
  it("pins the frame option order and the checked default", () => {
    expect(MATCH_MODES).toEqual(["off", "any", "contains", "extra", "exact"]);
    expect(DEFAULT_MATCH_MODE).toBe("extra");
  });

  it("off never matches", () => {
    expect(matchesMode("off", MEDIA, "The_Wand_S01E01.srt")).toBe(false);
  });

  it("any matches any non-degenerate pair", () => {
    expect(matchesMode("any", MEDIA, "anything.srt")).toBe(true);
    expect(matchesMode("any", "", "anything.srt")).toBe(false);
  });

  it("contains matches either direction", () => {
    expect(matchesMode("contains", MEDIA, "The_Wand_S01E01_English.srt")).toBe(true);
    expect(matchesMode("contains", "The_Wand_S01E01_Long_Cut.mp4", "The_Wand_S01E01.srt")).toBe(true);
    expect(matchesMode("contains", MEDIA, "Unrelated.srt")).toBe(false);
  });

  it("extra matches the title with separator-delimited suffixes only", () => {
    expect(matchesMode("extra", MEDIA, "The_Wand_S01E01.srt")).toBe(true);
    expect(matchesMode("extra", MEDIA, "The_Wand_S01E01_English.srt")).toBe(true);
    // A partial-title stem is NOT an extension of the full title.
    expect(matchesMode("extra", MEDIA, "The_Wand.srt")).toBe(false);
    expect(matchesMode("extra", MEDIA, "The_Wand_S01E0.srt")).toBe(false);
  });

  it("exact matches normalized stems only", () => {
    expect(matchesMode("exact", MEDIA, "the wand s01e01.srt")).toBe(true);
    expect(matchesMode("exact", MEDIA, "The_Wand_S01E01_English.srt")).toBe(false);
  });
});

describe("pairSubtitles — the Folder.svg review list", () => {
  it("pairs media with the first matching candidate, natural order both sides", () => {
    const media = [
      "The_Wand_S01E10.mp4",
      "The_Wand_S01E02.mp4",
      "The_Wand_S01E01.mp4",
    ];
    const candidates = [
      "The_Wand_S01E10.srt",
      "The_Wand_S01E02.srt",
      "The_Wand_S01E01.srt",
    ];
    const pairs = pairSubtitles(media, candidates, "exact");
    expect(pairs.map((pair) => pair.mediaName)).toEqual([
      "The_Wand_S01E01.mp4",
      "The_Wand_S01E02.mp4",
      "The_Wand_S01E10.mp4",
    ]);
    expect(pairs[0]).toEqual({ mediaName: "The_Wand_S01E01.mp4", subtitleName: "The_Wand_S01E01.srt" });
  });

  it("orders candidates naturally (E2 before E10) when several match", () => {
    const pairs = pairSubtitles(
      ["Show_E2.mp4"],
      ["Show_E10.srt", "Show_E2.srt"],
      "extra",
    );
    expect(pairs).toEqual([{ mediaName: "Show_E2.mp4", subtitleName: "Show_E2.srt" }]);
  });

  it("skips unmatched media, filters non-subtitle candidates and never reuses one", () => {
    const pairs = pairSubtitles(
      ["A.mp4", "B.mp4", "C.mp4"],
      ["A.srt", "A.txt", "B.srt"],
      "any",
    );
    expect(pairs).toEqual([
      { mediaName: "A.mp4", subtitleName: "A.srt" },
      { mediaName: "B.mp4", subtitleName: "B.srt" },
    ]);
  });

  it("returns an honest empty list for off / no candidates", () => {
    expect(pairSubtitles([MEDIA], ["The_Wand_S01E01.srt"], "off")).toEqual([]);
    expect(pairSubtitles([MEDIA], [], "extra")).toEqual([]);
  });
});
