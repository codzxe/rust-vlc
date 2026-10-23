import { describe, expect, it } from "vitest";
import { useMsg } from "../../i18n/messages";
import type { MessageKey, Msg } from "../../i18n/messages";
import { SETTINGS_AREAS } from "./generated/schema";
import type { SettingEntry } from "./generated/schema";
import { searchSettings } from "./search";

// useMsg is a plain lookup in the scaffold (no real hooks inside), but
// react-hooks/rules-of-hooks flags any non-hook call site — so each test
// builds its labeler the same way generated.test.ts does.
const makeLabelOf =
  (m: Msg) =>
  (entry: SettingEntry): string =>
    m(entry.description as MessageKey);

describe("settings search (card 4.E1)", () => {
  it("returns nothing for an empty/blank query (search inactive)", () => {
    expect(searchSettings(SETTINGS_AREAS, "", makeLabelOf(useMsg()))).toEqual([]);
    expect(searchSettings(SETTINGS_AREAS, "   ", makeLabelOf(useMsg()))).toEqual([]);
  });

  it("matches fields by label substring, case-insensitively", () => {
    const labelOf = makeLabelOf(useMsg());
    const hits = searchSettings(SETTINGS_AREAS, "window width", labelOf);
    const keys = hits.flatMap((h) => h.entries.map((e) => e.key));
    expect(keys).toContain("interface.window-width");
    const upper = searchSettings(SETTINGS_AREAS, "WINDOW WIDTH", labelOf);
    expect(upper.flatMap((h) => h.entries.map((e) => e.key))).toContain("interface.window-width");
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
