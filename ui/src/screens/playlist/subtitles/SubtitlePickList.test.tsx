// w26.b11 — candidate picker tests (subtitle one.svg / subtitle multiple.svg
// row + listbox): render anatomy, natural ordering, selection states and
// the full keyboard model (roving arrows, Home/End, Enter/Space toggle,
// single vs multi semantics).

import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SubtitlePickList } from "./SubtitlePickList";
import { SubtitlePickRow } from "./SubtitlePickRow";

afterEach(cleanup);

const CANDIDATES = [
  "The_Wand_S01E01_English.srt",
  "The_Wand_S01E01_Spanish.srt",
  "The_Wand_S01E01_Japanese.srt",
];

/** The same candidates in NATURAL order (English < Japanese < Spanish) — the
 * listbox renders them sorted, so keyboard targets index into this. */
const SORTED_CANDIDATES = [
  "The_Wand_S01E01_English.srt",
  "The_Wand_S01E01_Japanese.srt",
  "The_Wand_S01E01_Spanish.srt",
];

/** Stateful mount so controlled selection flows like the real host. */
function StatefulPickList({
  multiple,
  onReport,
}: {
  multiple: boolean;
  onReport: (selected: readonly string[]) => void;
}) {
  const [selected, setSelected] = useState<readonly string[]>([]);
  return (
    <SubtitlePickList
      candidates={CANDIDATES}
      selected={selected}
      onSelectedChange={(next) => {
        setSelected(next);
        onReport(next);
      }}
      multiple={multiple}
    />
  );
}

describe("SubtitlePickRow — frame anatomy", () => {
  it("renders the filename with option semantics and the selected band + check", () => {
    const { container, rerender } = render(
      <SubtitlePickRow id="row" fileName="The_Wand_S01E01.srt" selected={false} />,
    );
    const row = screen.getByText("The_Wand_S01E01.srt").closest("div");
    expect(row).not.toBeNull();
    expect(row).not.toHaveClass("lumen-plsubs-row--selected");
    expect(container.querySelector(".lumen-plsubs-row__check")).toBeNull();

    rerender(<SubtitlePickRow id="row" fileName="The_Wand_S01E01.srt" selected />);
    expect(row).toHaveClass("lumen-plsubs-row--selected");
    expect(container.querySelector(".lumen-plsubs-row__check")).not.toBeNull();
  });

  it("renders the frame's subtitle-file glyph (aria-hidden)", () => {
    const { container } = render(
      <SubtitlePickRow id="row" fileName="a.srt" selected={false} />,
    );
    expect(container.querySelector(".lumen-plsubs-row__icon svg")).not.toBeNull();
  });

  it("spreads optionProps (role, aria-selected, handlers)", () => {
    const onClick = vi.fn();
    render(
      <SubtitlePickRow
        id="row"
        fileName="a.srt"
        selected
        optionProps={{ role: "option", "aria-selected": true, tabIndex: 0, onClick }}
      />,
    );
    const option = screen.getByRole("option", { selected: true });
    fireEvent.click(option);
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

describe("SubtitlePickList — render + ordering", () => {
  it("renders candidates in NATURAL order (E2 before E10)", () => {
    render(
      <SubtitlePickList
        candidates={["Show_E10.srt", "Show_E2.srt"]}
        selected={[]}
        onSelectedChange={() => {}}
      />,
    );
    const options = screen.getAllByRole("option");
    expect(options.map((option) => option.textContent)).toEqual(["Show_E2.srt", "Show_E10.srt"]);
  });

  it("exposes a labelled multi-select listbox with aria-activedescendant", () => {
    render(
      <SubtitlePickList
        candidates={CANDIDATES}
        selected={[]}
        onSelectedChange={() => {}}
        multiple
        label="Subtitle files"
      />,
    );
    const listbox = screen.getByRole("listbox", { name: "Subtitle files" });
    expect(listbox).toHaveAttribute("aria-multiselectable", "true");
    expect(listbox.getAttribute("aria-activedescendant")).toMatch(/-opt-0$/);
  });

  it("renders the honest empty state when no candidates exist", () => {
    render(<SubtitlePickList candidates={[]} selected={[]} onSelectedChange={() => {}} />);
    expect(screen.getByText("No subtitle files found")).not.toBeNull();
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});

describe("SubtitlePickList — selection states", () => {
  it("multi mode appends and removes on click (subtitle multiple.svg)", () => {
    const onSelectedChange = vi.fn();
    render(
      <SubtitlePickList
        candidates={CANDIDATES}
        selected={[]}
        onSelectedChange={onSelectedChange}
        multiple
      />,
    );
    fireEvent.click(screen.getByText("The_Wand_S01E01_English.srt"));
    expect(onSelectedChange).toHaveBeenLastCalledWith(["The_Wand_S01E01_English.srt"]);

    // Selected rows carry aria-selected + the check band class.
    render(
      <SubtitlePickList
        candidates={CANDIDATES}
        selected={["The_Wand_S01E01_English.srt"]}
        onSelectedChange={() => {}}
        multiple
      />,
    );
    expect(
      screen.getByRole("option", { name: /English/, selected: true }).className,
    ).toContain("lumen-plsubs-row--selected");
  });

  it("multi mode removes an already-selected row on click", () => {
    const onSelectedChange = vi.fn();
    render(
      <SubtitlePickList
        candidates={CANDIDATES}
        selected={["The_Wand_S01E01_English.srt", "The_Wand_S01E01_Spanish.srt"]}
        onSelectedChange={onSelectedChange}
        multiple
      />,
    );
    fireEvent.click(screen.getByText("The_Wand_S01E01_English.srt"));
    expect(onSelectedChange).toHaveBeenLastCalledWith(["The_Wand_S01E01_Spanish.srt"]);
  });

  it("single mode replaces the selection (subtitle one.svg / Folder-3)", () => {
    const onSelectedChange = vi.fn();
    render(
      <SubtitlePickList
        candidates={CANDIDATES}
        selected={["The_Wand_S01E01_English.srt"]}
        onSelectedChange={onSelectedChange}
      />,
    );
    expect(screen.getByRole("listbox")).not.toHaveAttribute("aria-multiselectable");
    fireEvent.click(screen.getByText("The_Wand_S01E01_Japanese.srt"));
    expect(onSelectedChange).toHaveBeenLastCalledWith(["The_Wand_S01E01_Japanese.srt"]);
  });
});

describe("SubtitlePickList — keyboard model (APG listbox)", () => {
  const mount = (multiple = true) => {
    const onSelectedChange = vi.fn();
    render(
      <SubtitlePickList
        candidates={CANDIDATES}
        selected={[]}
        onSelectedChange={onSelectedChange}
        multiple={multiple}
      />,
    );
    return { onSelectedChange, listbox: screen.getByRole("listbox") };
  };

  it("starts focused on the first row (roving tabindex)", () => {
    const { listbox } = mount();
    const options = screen.getAllByRole("option");
    expect(options[0]).toHaveFocus();
    expect(options[0]).toHaveAttribute("tabindex", "0");
    expect(options[1]).toHaveAttribute("tabindex", "-1");
    expect(listbox.getAttribute("aria-activedescendant")).toBe(options[0]!.id);
  });

  it("ArrowDown/ArrowUp move the roving focus and aria-activedescendant", () => {
    mount();
    const options = screen.getAllByRole("option");
    fireEvent.keyDown(options[0]!, { key: "ArrowDown" });
    expect(options[1]).toHaveFocus();
    fireEvent.keyDown(options[1]!, { key: "ArrowUp" });
    expect(options[0]).toHaveFocus();
    // Wraps at both ends.
    fireEvent.keyDown(options[0]!, { key: "ArrowUp" });
    expect(options[2]).toHaveFocus();
    fireEvent.keyDown(options[2]!, { key: "ArrowDown" });
    expect(options[0]).toHaveFocus();
  });

  it("Home/End jump to the first/last row", () => {
    mount();
    const options = screen.getAllByRole("option");
    fireEvent.keyDown(options[0]!, { key: "End" });
    expect(options[2]).toHaveFocus();
    fireEvent.keyDown(options[2]!, { key: "Home" });
    expect(options[0]).toHaveFocus();
  });

  it("Enter and Space toggle the active row in multi mode", () => {
    const onReport = vi.fn();
    render(<StatefulPickList multiple onReport={onReport} />);
    const options = screen.getAllByRole("option");
    // The user focuses row 2 first (click/tab), then presses the keys.
    fireEvent.focus(options[1]!);
    fireEvent.keyDown(options[1]!, { key: " " });
    expect(onReport).toHaveBeenLastCalledWith([SORTED_CANDIDATES[1]]);
    fireEvent.keyDown(options[1]!, { key: "Enter" });
    expect(onReport).toHaveBeenLastCalledWith([]);
  });

  it("Enter selects the active row in single mode", () => {
    const onReport = vi.fn();
    render(<StatefulPickList multiple={false} onReport={onReport} />);
    const options = screen.getAllByRole("option");
    fireEvent.keyDown(options[0]!, { key: "ArrowDown" });
    fireEvent.keyDown(options[1]!, { key: "Enter" });
    expect(onReport).toHaveBeenLastCalledWith([SORTED_CANDIDATES[1]]);
  });
});
