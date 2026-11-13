// w26.i16 — the Profile flyout (family w26.h18, surface 10: Profiles.png
// 6.4 NO → the real row grammar; analysis §A10).
//
// Frame (Profiles.svg, `submenu "Profile"`): panel 265×249 = 8 + 48×5 + 1
// separator; row 1 (the selected profile) = the resting #1F1F1F face with the
// orange check IN THE ICON SLOT (check 14×11 sw2 #D37918 @ x409–423 — the
// selected profile REPLACES its icon with the check); rows 2–4 plain quiet
// labels (no icon, no check); a #222222 separator after row 4; row 5 "+ New"
// with a 14×14 #8F8F8F plus glyph. Labels per the render read: Movies /
// Shows / Music / Work / New (SVG layer ids are "Option" placeholders —
// [VERIFY] via the §5 vision loop; g20/h18/a19 all transcribed the same).
//
// DATA HONESTY (the playlistsStore precedent, `lumen.playlists.v1`): there is
// no engine profile command in the catalog, so profiles are LOCAL device data
// persisted under `lumen.profiles.v1` — frame-seeded on first run (the seed
// never resurrects once the user edits), selection and quick-creates persist,
// and no row claims engine capability. Real seams only: select (persists the
// active profile), "+ New" (quick-create: persists + selects). The frame's
// rows ship as data through MenuPopup's existing submenu machinery — no new
// popup grammar (menu.css owns all visuals).

import { useSyncExternalStore } from "react";
import type { MenuEntry } from "./types";

/** One local media-library profile (a named collection marker). */
export interface MediaProfile {
  readonly id: string;
  readonly name: string;
}

/** The persisted shape (`lumen.profiles.v1`). */
export interface ProfilesState {
  readonly profiles: readonly MediaProfile[];
  /** The active profile (the flyout's checked row); null = none selected. */
  readonly selectedId: string | null;
}

export interface ProfilesPersistence {
  load(): { profiles?: unknown; selectedId?: unknown } | null;
  save(state: ProfilesState): void;
}

const STORAGE_KEY = "lumen.profiles.v1";

/**
 * Seed fixture — the frame rows (Profiles.svg flyout rows 1–4), same
 * structural-fixture precedent as playlistsStore's SEED (playlist.svg /
 * Player.png rows). Seeded only when the store is absent; user edits persist,
 * so the seed never resurrects. The active profile defaults to row 1
 * (Movies) — the frame's checked row.
 */
const SEED: readonly MediaProfile[] = [
  { id: "movies", name: "Movies" },
  { id: "shows", name: "Shows" },
  { id: "music", name: "Music" },
  { id: "work", name: "Work" },
];
const SEED_SELECTED_ID = "movies";

export const localStorageProfilesPersistence: ProfilesPersistence = {
  load() {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw === null) return null;
      const parsed: unknown = JSON.parse(raw);
      if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return null;
      return parsed as { profiles?: unknown; selectedId?: unknown };
    } catch {
      return null; // corrupt store behaves like "never saved" — never crash
    }
  },
  save(state) {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
      // Storage unavailable (private mode/quota) — state stays in-memory.
    }
  },
};

export const memoryProfilesPersistence: ProfilesPersistence = {
  load() {
    return null; // test double: always the seed fixture
  },
  save() {
    /* test double */
  },
};

/** Coerce one unknown record into a MediaProfile, or null when malformed. */
function coerceProfile(value: unknown): MediaProfile | null {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.id !== "string" || typeof record.name !== "string" || record.name === "") return null;
  return { id: record.id, name: record.name };
}

/** Absent/corrupt store folds back onto the seed rows; a stored EMPTY list is
 * valid user state and stays empty — it never resurrects. */
function hydrate(): ProfilesState {
  const loaded = persistence.load();
  if (loaded === null || !Array.isArray(loaded.profiles)) {
    return { profiles: SEED, selectedId: SEED_SELECTED_ID };
  }
  const profiles = loaded.profiles.map(coerceProfile).filter((p): p is MediaProfile => p !== null);
  const selectedId = typeof loaded.selectedId === "string" && profiles.some((p) => p.id === loaded.selectedId)
    ? loaded.selectedId
    : null;
  return { profiles, selectedId };
}

// --- external store (no provider; the flyout host reads it reactively) ---

/* w26.k02 (j02 J02-11): hydrate() runs at MODULE IMPORT and used to touch
 * window.localStorage unguarded (unlike Dropdown.tsx's typeof-window seam) —
 * any non-DOM import of this module (SSR/node) crashed at import time. In a
 * windowless environment the store rides the memory seam + seed. */
let persistence: ProfilesPersistence =
  typeof window === "undefined" ? memoryProfilesPersistence : localStorageProfilesPersistence;
let state: ProfilesState = hydrate();
const listeners = new Set<() => void>();

function commit(next: ProfilesState): void {
  state = next;
  persistence.save(next);
  for (const listener of listeners) listener();
}

const subscribe = (listener: () => void): (() => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

const getSnapshot = (): ProfilesState => state;

/** Non-hook read of the current store state (tests + non-React hosts). */
export function getProfilesState(): ProfilesState {
  return state;
}

/** Reactive read of the local profiles + the active id. */
export function useMediaProfiles(): ProfilesState {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

/** Select the active profile (the flyout row click). Unknown id: no-op. */
export function selectProfile(id: string): void {
  if (!state.profiles.some((p) => p.id === id)) return;
  if (state.selectedId === id) return;
  commit({ profiles: state.profiles, selectedId: id });
}

/** Stable client id for a quick-created profile (collision-safe per device). */
function nextId(): string {
  return `pr-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

/**
 * Quick-create (the "+ New" row): a real persisted profile named
 * `<baseName> N` (N skips taken names), selected on creation. `baseName` is
 * the host-resolved i18n string ("Profile") — generated names stay localizable.
 */
export function createProfile(baseName: string): MediaProfile {
  const base = baseName.trim() === "" ? "Profile" : baseName.trim();
  const taken = new Set(state.profiles.map((p) => p.name));
  let n = state.profiles.length + 1;
  while (taken.has(`${base} ${n}`)) n += 1;
  const profile: MediaProfile = { id: nextId(), name: `${base} ${n}` };
  commit({ profiles: [...state.profiles, profile], selectedId: profile.id });
  return profile;
}

/** Test-only: swap the seam and rewind to the hydrated seed. */
export function resetProfilesForTests(seam: ProfilesPersistence = localStorageProfilesPersistence): void {
  persistence = seam;
  state = hydrate();
  for (const listener of listeners) listener();
}

/**
 * The flyout's rows as menu DATA (Profiles.svg §A10, comfortable density):
 * selected row first with the resting selected face + check in the icon slot
 * (the selected profile REPLACES its icon with the check — MenuItem renders
 * the empty check slot when a row has no icon, and menu.css paints the ✓ on
 * the selected face), the remaining profiles as plain quiet rows, a #222222
 * separator, then "+ New" (plus glyph 14×14 #8F8F8F). Profile names are
 * dynamic strings — resolve with the identity seam (the recents pattern).
 */
export function buildProfileFlyout({
  profiles,
  selectedId,
  onSelect,
  onCreate,
  newLabel,
}: {
  profiles: readonly MediaProfile[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onCreate: () => void;
  /** Resolved i18n label for the "+ New" row (the plus glyph is the "+"). */
  newLabel: string;
}): MenuEntry<string>[] {
  const rows: MenuEntry<string>[] = profiles.map((profile) => ({
    id: `profile-${profile.id}`,
    labelKey: profile.name,
    selected: profile.id === selectedId,
    onSelect: () => onSelect(profile.id),
  }));
  return [
    ...rows,
    { id: "profile-sep", kind: "separator" },
    { id: "profile-new", labelKey: newLabel, icon: "add", onSelect: onCreate },
  ];
}
