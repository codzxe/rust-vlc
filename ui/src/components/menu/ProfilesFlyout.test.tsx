import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MenuItem } from "./MenuItem";
import { MenuPopup } from "./MenuPopup";
import { identityResolveLabel } from "./types";
import {
  buildProfileFlyout,
  createProfile,
  getProfilesState,
  memoryProfilesPersistence,
  resetProfilesForTests,
  selectProfile,
} from "./ProfilesFlyout";

/**
 * w26.i16 — the Profile flyout (Profiles.png §A10; h18 surface 10). Pins:
 *  1. the local store contract (seed, select, quick-create, persistence);
 *  2. the flyout row grammar (selected row w/ resting face + check slot,
 *     plain quiet rows, separator, "+ New" plus row);
 *  3. the MenuPopup integration (aria-current on the active profile, the
 *     comfortable 265px submenu class, activation fires the real seams).
 */

afterEach(() => {
  resetProfilesForTests(memoryProfilesPersistence);
});

describe("ProfilesFlyout store (lumen.profiles.v1)", () => {
  it("hydrates the frame seed (Movies/Shows/Music/Work) with Movies active", () => {
    const state = getProfilesState();
    expect(state.profiles.map((p) => p.name)).toEqual(["Movies", "Shows", "Music", "Work"]);
    expect(state.selectedId).toBe("movies");
  });

  it("select persists and unknown ids are a no-op (never a fabricated selection)", () => {
    expect(getProfilesState().selectedId).toBe("movies");
    act(() => {
      selectProfile("shows");
    });
    expect(getProfilesState().selectedId).toBe("shows");
    act(() => {
      selectProfile("nope");
    });
    expect(getProfilesState().selectedId).toBe("shows");
  });

  it("quick-create names Profile N (skipping taken names), selects it", () => {
    act(() => {
      createProfile("Profile");
    });
    let state = getProfilesState();
    // Seed 4 + quick-create → "Profile 5", selected on creation.
    expect(state.profiles.map((p) => p.name)).toContain("Profile 5");
    expect(state.selectedId).toBe(state.profiles[4]?.id);
    act(() => {
      selectProfile("movies");
      createProfile("Profile");
    });
    state = getProfilesState();
    // "Profile 5" is taken → the next quick-create skips to 6.
    expect(state.profiles.map((p) => p.name)).toContain("Profile 6");
  });

  it("a stored EMPTY list is valid user state — the seed never resurrects", () => {
    resetProfilesForTests({ load: () => ({ profiles: [], selectedId: null }), save: () => {} });
    expect(getProfilesState().profiles).toEqual([]);
    expect(getProfilesState().selectedId).toBeNull();
  });

  it("a corrupt store folds back onto the seed (never crashes)", () => {
    resetProfilesForTests({ load: () => null, save: () => {} });
    expect(getProfilesState().profiles.map((p) => p.name)).toEqual([
      "Movies",
      "Shows",
      "Music",
      "Work",
    ]);
  });
});

describe("buildProfileFlyout (Profiles.svg §A10 row grammar)", () => {
  const base = {
    profiles: [
      { id: "movies", name: "Movies" },
      { id: "shows", name: "Shows" },
      { id: "music", name: "Music" },
      { id: "work", name: "Work" },
    ],
    selectedId: "movies" as string | null,
    onSelect: vi.fn(),
    onCreate: vi.fn(),
    newLabel: "New",
  };

  it("rows: selected first with the resting face, plain rows, separator, + New", () => {
    const entries = buildProfileFlyout(base);
    expect(entries.map((e) => e.id)).toEqual([
      "profile-movies",
      "profile-shows",
      "profile-music",
      "profile-work",
      "profile-sep",
      "profile-new",
    ]);
    const first = entries[0] as { selected?: boolean };
    expect(first.selected).toBe(true); // frame row 1
    const others = entries.slice(1, 4);
    for (const entry of others) {
      const row = entry as { selected?: boolean; icon?: unknown };
      expect(row.selected).toBeFalsy(); // plain quiet rows
      expect(row.icon).toBeUndefined(); // no leading icon (empty check slot)
    }
    const last = entries[5] as { icon?: unknown; labelKey?: unknown };
    expect(last.icon).toBe("add"); // + New plus glyph
    expect(last.labelKey).toBe("New");
  });

  it("wire-up: row activation selects, + New creates", () => {
    const entries = buildProfileFlyout(base);
    const showsRow = entries[1] as { onSelect?: () => void };
    const newRow = entries[5] as { onSelect?: () => void };
    showsRow.onSelect?.();
    newRow.onSelect?.();
    expect(base.onCreate).toHaveBeenCalledTimes(1);
    // The profile rows call the host's onSelect with the profile id.
    expect(base.onSelect).toHaveBeenCalledWith("shows");
  });
});

describe("MenuPopup integration (the flyout as a real submenu)", () => {
  it("renders the selected profile with aria-current + the check slot, not a checkable", () => {
    const entries = buildProfileFlyout({
      profiles: getProfilesState().profiles,
      selectedId: getProfilesState().selectedId,
      onSelect: () => {},
      onCreate: () => {},
      newLabel: "New",
    });
    render(
      <MenuPopup
        entries={entries}
        resolveLabel={identityResolveLabel}
        onRequestClose={() => {}}
        onDismissAll={() => {}}
        idPrefix="test-flyout"
      />,
    );
    const selected = screen.getByRole("menuitem", { name: "Movies" });
    expect(selected).toHaveClass("lumen-menu__item--selected");
    expect(selected).toHaveAttribute("aria-current", "true");
    expect(selected).not.toHaveAttribute("aria-checked"); // current, not checkable
    expect(selected.querySelector(".lumen-menu__check")).not.toBeNull();
    // Plain rows keep the empty check slot (label x-alignment, frame x439).
    const plain = screen.getByRole("menuitem", { name: "Shows" });
    expect(plain).not.toHaveAttribute("aria-current");
    expect(plain.querySelector(".lumen-menu__check")).not.toBeNull();
    expect(plain.querySelector(".lumen-menu__icon")).toBeNull();
    // + New carries the plus glyph in the icon slot.
    const newRow = screen.getByRole("menuitem", { name: "New" });
    expect(newRow.querySelector(".lumen-menu__icon svg")).not.toBeNull();
  });

  it("MenuItem paints the selected-check grammar (aria-current, no aria-checked)", () => {
    const entry = { id: "movies", labelKey: "Movies", selected: true } as const;
    render(<MenuItem entry={entry} resolveLabel={identityResolveLabel} active />);
    const item = screen.getByRole("menuitem", { name: "Movies" });
    expect(item).toHaveAttribute("aria-current", "true");
    expect(item).not.toHaveAttribute("aria-checked");
    expect(item).toHaveClass("lumen-menu__item--selected");
  });
});
