// w26.b09 — playlists LIBRARY store (the /playlist screen's data seam).
//
// OWNERSHIP: ui/src/screens/playlist/** (wave-b playlist core). The app
// shell's playlists sidebar renders structural fixture rows today (a10 lane);
// this store is the real seam it can adopt post-wave (log w26.b09).
//
// HONESTY: the engine command catalog (doc 05 §2.9) has no `playlist.list`
// and no create/remove commands for SAVED playlists — the queue
// (`playlist.changed`, PlaylistProvider) is engine truth, these saved
// playlists are LOCAL device data (persisted `lumen.playlists.v1`, same
// pattern as onboardingState). Rows never claim engine playback: the
// context-menu Play entry stays disabled-honest until a catalog command
// exists. File names recorded by the create dialog are names only — the
// media-rows browse view (wave-b thumbnail/media lane) owns rendering them.
//
// Persistence mirrors onboardingState: interface + localStorage impl +
// memory test double, useSyncExternalStore, absent/corrupt store folds back
// onto the seed fixture (frame rows), never crashes.

import { useSyncExternalStore } from "react";

/** A saved local playlist (new playlist.svg "Save" output). */
export interface SavedPlaylist {
  /** Stable client id (never an engine id — no catalog command exists). */
  readonly id: string;
  /** User title (dialog input; non-empty by the Save gate). */
  readonly title: string;
  /** Creation time (ms epoch) — future "date added" sorts. */
  readonly createdAt: number;
  /** File names recorded by the create dialog (names only, no paths). */
  readonly itemNames: readonly string[];
}

export interface PlaylistsPersistence {
  load(): { playlists?: unknown } | null;
  save(state: PlaylistsState): void;
}

/** The persisted shape (`lumen.playlists.v1`). */
export interface PlaylistsState {
  readonly playlists: readonly SavedPlaylist[];
}

const STORAGE_KEY = "lumen.playlists.v1";

/**
 * Seed fixture — the frame rows (playlist.svg "Cool Playlist"; Player.svg
 * sidebar "Cool Songs"/"Anime to Watch"), same structural-fixture precedent
 * as the shell sidebar (a10). Seeded only when the store is absent; user
 * edits (removals) persist, so the seed never resurrects.
 */
const SEED: readonly SavedPlaylist[] = [
  { id: "cool-playlist", title: "Cool Playlist", createdAt: 0, itemNames: [] },
  { id: "cool-songs", title: "Cool Songs", createdAt: 0, itemNames: [] },
  { id: "anime-to-watch", title: "Anime to Watch", createdAt: 0, itemNames: [] },
];

export const localStoragePlaylistsPersistence: PlaylistsPersistence = {
  load() {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw === null) return null;
      const parsed: unknown = JSON.parse(raw);
      if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return null;
      return parsed as { playlists?: unknown };
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

export const memoryPlaylistsPersistence: PlaylistsPersistence = {
  load() {
    return null; // test double: always the seed fixture
  },
  save() {
    /* test double */
  },
};

/** Coerce one unknown record into a SavedPlaylist, or null when malformed. */
function coercePlaylist(value: unknown): SavedPlaylist | null {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.id !== "string" || typeof record.title !== "string" || record.title === "") return null;
  const createdAt = typeof record.createdAt === "number" ? record.createdAt : 0;
  const itemNames = Array.isArray(record.itemNames)
    ? record.itemNames.filter((name): name is string => typeof name === "string")
    : [];
  return { id: record.id, title: record.title, createdAt, itemNames };
}

/** Absent/corrupt store folds back onto the seed rows; a stored EMPTY list
 * is valid user state (all seeds removed) and stays empty — it never
 * resurrects. */
function hydrate(): PlaylistsState {
  const loaded = persistence.load();
  if (loaded === null || !Array.isArray(loaded.playlists)) return { playlists: SEED };
  const playlists = loaded.playlists
    .map(coercePlaylist)
    .filter((p): p is SavedPlaylist => p !== null);
  return { playlists };
}

// --- external store (no provider; the screen + future shell sidebar read it) ---

let persistence: PlaylistsPersistence = localStoragePlaylistsPersistence;
let state: PlaylistsState = hydrate();
const listeners = new Set<() => void>();

function commit(next: PlaylistsState): void {
  state = next;
  persistence.save(state);
  for (const listener of listeners) listener();
}

const subscribe = (listener: () => void): (() => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

const getSnapshot = (): PlaylistsState => state;

/** Reactive read of the saved playlists (natural sort is the screen's job). */
export function usePlaylists(): PlaylistsState {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

/** Client id for a new playlist (stable, collision-safe within this device). */
function nextId(): string {
  return `pl-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

/** Create a playlist (new playlist.svg "Save") — title must be non-empty. */
export function createPlaylist(input: { title: string; itemNames?: readonly string[] }): SavedPlaylist {
  const title = input.title.trim();
  if (title === "") throw new Error("createPlaylist requires a non-empty title");
  const playlist: SavedPlaylist = {
    id: nextId(),
    title,
    createdAt: Date.now(),
    itemNames: input.itemNames ?? [],
  };
  commit({ playlists: [...state.playlists, playlist] });
  return playlist;
}

/** Remove a saved playlist (row context menu "Remove"). Unknown id: no-op. */
export function removePlaylist(id: string): void {
  if (!state.playlists.some((p) => p.id === id)) return;
  commit({ playlists: state.playlists.filter((p) => p.id !== id) });
}

/** Test-only: swap the seam and rewind to the hydrated seed. */
export function resetPlaylistsForTests(seam: PlaylistsPersistence = localStoragePlaylistsPersistence): void {
  persistence = seam;
  state = hydrate();
  for (const listener of listeners) listener();
}
