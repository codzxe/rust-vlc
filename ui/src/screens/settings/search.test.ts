import { describe, expect, it } from "vitest";
import { useMsg } from "../../i18n/messages";
import type { MessageKey, Msg } from "../../i18n/messages";
import { keys as seedKeys } from "../../i18n/keys/w26.f01";
import { keys as gfixKeys } from "../../i18n/keys/w26.gfixB";
import { SETTINGS_AREAS } from "./generated/schema";
import type { SettingEntry } from "./generated/schema";
import { searchSettings } from "./search";

// useMsg is a plain lookup in the scaffold (no real hooks inside), but
// react-hooks/rules-of-hooks flags any non-hook call site — so each test
// builds its labeler the same way generated.test.ts does: the registered
// catalog first, then the lane catalogs' pre-registration keys (w26.f01 +
// w26.gfixB — the re-seed's labels live there until the Lead registers them).
const laneCatalog: Readonly<Record<string, string>> = { ...seedKeys, ...gfixKeys };

const makeLabelOf =
  (m: Msg) =>
  (entry: SettingEntry): string =>
    m(entry.description as MessageKey) ?? laneCatalog[entry.description] ?? "";

describe("settings search (card 4.E1)", () => {
  it("returns nothing for an empty/blank query (search inactive)", () => {
    expect(searchSettings(SETTINGS_AREAS, "", makeLabelOf(useMsg()))).toEqual([]);
    expect(searchSettings(SETTINGS_AREAS, "   ", makeLabelOf(useMsg()))).toEqual([]);
  });

  it("matches fields by label substring, case-insensitively", () => {
    const labelOf = makeLabelOf(useMsg());
    // w26.gfix-B: interface.window-* left the ui registry with the re-seed
    // (g10-P1-1: the interface frame shows no window-metric rows) — a live
    // frame row carries the case-insensitivity proof.
    const hits = searchSettings(SETTINGS_AREAS, "skin file", labelOf);
    const keys = hits.flatMap((h) => h.entries.map((e) => e.key));
    expect(keys).toContain("interface.skin-file");
    const upper = searchSettings(SETTINGS_AREAS, "SKIN FILE", labelOf);
    expect(upper.flatMap((h) => h.entries.map((e) => e.key))).toContain("interface.skin-file");
  });

  it("matches across areas and keeps schema order", () => {
    const hits = searchSettings(SETTINGS_AREAS, "languages", makeLabelOf(useMsg()));
    expect(hits.map((h) => h.area.id)).toEqual(["playback"]);
    expect(hits[0]?.entries.map((e) => e.key)).toEqual([
      "playback.preferred-audio-languages",
      "playback.preferred-subtitle-languages",
    ]);
  });

  it("hides feature-gated unexposed entries from results", () => {
    // "hardware decode policy" is gated on features.hw-decode = false.
    const hits = searchSettings(SETTINGS_AREAS, "hardware decode", makeLabelOf(useMsg()));
    expect(hits).toEqual([]);
  });

  it("reports an honest no-match for gibberish", () => {
    expect(searchSettings(SETTINGS_AREAS, "zzzz-not-a-setting", makeLabelOf(useMsg()))).toEqual([]);
  });
});
