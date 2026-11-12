// w26.b11 — the add-subtitles dialog tests (the four Folder states +
// subtitle_one/subtitle_multiple bodies + the sub filename seam): chrome,
// the three body modes, the control-row state machine, selection → Save
// payloads, honest disabled states and dismissal (Escape/✕/Cancel/scrim).

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AddSubtitlesDialog } from "./AddSubtitlesDialog";

afterEach(cleanup);

const MEDIA = [
  "The_Wand_S01E01.mp4",
  "The_Wand_S01E02.mp4",
  "The_Wand_S01E10.mp4",
];
const CANDIDATES = [
  "The_Wand_S01E01_English.srt",
  "The_Wand_S01E01_Spanish.srt",
  "The_Wand_S01E01_Japanese.srt",
  "The_Wand_S01E01_German.srt",
  "The_Wand_S01E01_Dutch.srt",
  "The_Wand_S01E01_French.srt",
  "The_Wand_S01E02.srt",
  "The_Wand_S01E10.srt",
];
const PATH = ["Videos", "Shows", "The Wand", "S01"];

type MountProps = Partial<React.ComponentProps<typeof AddSubtitlesDialog>>;

const mount = (props: MountProps = {}) =>
  render(
    <AddSubtitlesDialog
      open
      onClose={() => {}}
      mediaNames={MEDIA}
      candidates={CANDIDATES}
      folderPath={PATH}
      {...props}
    />,
  );

describe("AddSubtitlesDialog — chrome (Folder.svg)", () => {
  it("renders nothing while closed", () => {
    const { container } = render(<AddSubtitlesDialog open={false} onClose={() => {}} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders the title, the ✕ close and the footer pair", () => {
    mount();
    expect(screen.getByRole("dialog", { name: "Add Subtitles" })).not.toBeNull();
    expect(screen.getByRole("button", { name: "Close" })).not.toBeNull();
    expect(screen.getByRole("button", { name: "Cancel" })).not.toBeNull();
    expect(screen.getByRole("button", { name: "Save" })).not.toBeNull();
  });

  it("Escape and ✕ and Cancel all request close", () => {
    const onClose = vi.fn();
    mount({ onClose });
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledTimes(2);
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalledTimes(3);
  });

  it("w26.gfix-E (g19 P1-2): first Escape closes only the open dropdown — the dialog survives", () => {
    const onClose = vi.fn();
    mount({ onClose });
    // Open a pill dropdown (the frame's control row).
    fireEvent.click(screen.getByRole("button", { name: "Apply subtitles to" }));
    const listbox = screen.getByRole("listbox");
    // The Dialog focus-trap's NATIVE listener sees the Escape BEFORE the
    // shared Dropdown's React handler (bubble-phase ancestor of React's
    // delegation root) — the guard redirects the dismiss to the dropdown.
    fireEvent.keyDown(listbox, { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(screen.getByRole("dialog", { name: "Add Subtitles" })).not.toBeNull();
    expect(onClose).not.toHaveBeenCalled();
    // The Dropdown's own restore path returned focus to the trigger.
    expect(document.activeElement?.getAttribute("aria-label")).toBe("Apply subtitles to");
    // With no list open, the next Escape closes the dialog.
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("w26.gfix-E (g19 P1-2): the footer filename-matching dropdown scopes Escape the same way", () => {
    const onClose = vi.fn();
    mount({ onClose });
    fireEvent.click(screen.getByRole("button", { name: "Filename matching" }));
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe("AddSubtitlesDialog — control row state machine", () => {
  it("default: To This Folder + From Computer + breadcrumb + match dropdown (Folder.svg)", () => {
    mount();
    expect(screen.getByRole("button", { name: "Apply subtitles to" }).textContent).toContain(
      "To This Folder",
    );
    expect(screen.getByRole("button", { name: "Subtitle source" }).textContent).toContain(
      "From Computer",
    );
    expect(screen.getByText("Videos / Shows / The Wand / S01")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Filename matching" })).not.toBeNull();
  });

  it("online source swaps in the search field + language pill (Folder-3)", () => {
    mount({ initialSource: "online", initialDestination: "file" });
    expect(screen.getByRole("button", { name: "Subtitle source" }).textContent).toContain(
      "From OpenSubtitles",
    );
    expect(screen.getByLabelText("Search subtitles")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Subtitle language" }).textContent).toContain(
      "English",
    );
    expect(screen.queryByText("Videos / Shows / The Wand / S01")).toBeNull();
  });

  it("switching destination to file swaps the pairs review for the candidate picker", () => {
    mount();
    expect(screen.getByRole("list", { name: "Matched subtitle pairs" })).not.toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Apply subtitles to" }));
    fireEvent.click(within(screen.getByRole("listbox")).getByText("To This File"));
    expect(screen.queryByRole("list", { name: "Matched subtitle pairs" })).toBeNull();
    expect(screen.getByRole("listbox", { name: "Subtitle files" })).not.toBeNull();
    // w26.gfix-E (g19 P2-1): the match dropdown rides the footer in the PAIRS
    // mode it drives — Folder-2's candidate state has no match control.
    expect(screen.queryByRole("button", { name: "Filename matching" })).toBeNull();
  });
});

describe("AddSubtitlesDialog — Folder.svg matched-pairs review", () => {
  it("pairs every media file under the current strategy, natural order", () => {
    mount();
    const items = screen.getAllByRole("listitem");
    // E01 has six language variants — the FIRST natural candidate (Dutch)
    // pairs with it under "Title with extra characters".
    expect(items.map((item) => item.textContent)).toEqual([
      "The_Wand_S01E01.mp4The_Wand_S01E01_Dutch.srt",
      "The_Wand_S01E02.mp4The_Wand_S01E02.srt",
      "The_Wand_S01E10.mp4The_Wand_S01E10.srt",
    ]);
  });

  it("changing the strategy (sub filename.svg) re-pairs the review list", () => {
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Filename matching" }));
    fireEvent.click(within(screen.getByRole("listbox")).getByText("Exact title match"));
    // "extra"-only candidates (…_English) drop out under exact matching:
    // E01 has no plain .srt, E02/E10 keep theirs.
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(2);
    expect(items[0]!.textContent).toContain("The_Wand_S01E02.mp4");
    expect(items[1]!.textContent).toContain("The_Wand_S01E10.mp4");
  });

  it("Save is disabled while no pair matched and commits the pairs otherwise", () => {
    const onSave = vi.fn();
    const onClose = vi.fn();
    mount({ onSave, onClose, candidates: ["Unrelated.srt"] });
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    // Off → also nothing; switch strategy to Any title → pairs exist.
    fireEvent.click(screen.getByRole("button", { name: "Filename matching" }));
    fireEvent.click(within(screen.getByRole("listbox")).getByText("Any title"));
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        destination: "folder",
        source: "computer",
        matchMode: "any",
        pairs: [{ mediaName: "The_Wand_S01E01.mp4", subtitleName: "Unrelated.srt" }],
        selected: [],
      }),
    );
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe("AddSubtitlesDialog — Folder-2 multi-subtitle pick", () => {
  it("multi-selects candidates and commits them natural-ordered", () => {
    const onSave = vi.fn();
    mount({ initialDestination: "file", onSave });
    const save = screen.getByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    fireEvent.click(screen.getByText("The_Wand_S01E01_Spanish.srt"));
    fireEvent.click(screen.getByText("The_Wand_S01E01_English.srt"));
    expect(save).toBeEnabled();
    fireEvent.click(save);
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        destination: "file",
        source: "computer",
        selected: ["The_Wand_S01E01_English.srt", "The_Wand_S01E01_Spanish.srt"],
        pairs: [],
      }),
    );
  });
});

describe("AddSubtitlesDialog — Folder-3 online pick", () => {
  it("single-selects and replaces the pick", () => {
    mount({ initialDestination: "file", initialSource: "online" });
    const listbox = screen.getByRole("listbox", { name: "Subtitle files" });
    expect(listbox).not.toHaveAttribute("aria-multiselectable");
    fireEvent.click(screen.getByText("The_Wand_S01E01_English.srt"));
    expect(
      screen.getByRole("option", { name: /English/, selected: true }),
    ).not.toBeNull();
    fireEvent.click(screen.getByText("The_Wand_S01E01_Japanese.srt"));
    expect(
      screen.getByRole("option", { name: /Japanese/, selected: true }),
    ).not.toBeNull();
  });

  it("the search field filters the candidates honestly", () => {
    mount({ initialDestination: "file", initialSource: "online" });
    fireEvent.change(screen.getByLabelText("Search subtitles"), {
      target: { value: "spanish" },
    });
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(1);
    expect(options[0]!.textContent).toContain("The_Wand_S01E01_Spanish.srt");
  });
});
