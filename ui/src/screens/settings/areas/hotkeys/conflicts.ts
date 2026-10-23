// Conflict detection for the hotkeys editor (card 4.E2, doc 07 §5).
//
// Two bindings conflict when they claim the SAME normalized key combination
// in the SAME activation mode (a "player-focused" combo never collides with
// an "always" one). Normalization reuses card 4.C1's seam
// (ui/src/screens/player/keymap.ts normalizeBindingKey) — one spelling
// grammar everywhere; unparseable spellings never conflict (never guessed).

import { normalizeBindingKey } from "../../../player/keymap";

export interface ConflictCandidate {
  readonly id: string;
  readonly key: string;
  readonly mode?: string;
}

/**
 * ids of every binding whose (mode, combo) group has more than one member.
 * The caller marks ALL rows of a conflicting group (inline warning on both).
 */
export function detectConflicts(bindings: readonly ConflictCandidate[], keys: Readonly<Record<string, string>>): ReadonlySet<string> {
  const groups = new Map<string, string[]>();
  for (const binding of bindings) {
    const combo = keys[binding.id] ?? binding.key;
    const normalized = normalizeBindingKey(combo);
    if (normalized === null) continue; // unparseable — never guessed as equal
    const mode = binding.mode ?? "";
    const groupKey = `${mode}|${normalized.key}+${[...normalized.modifiers].sort().join("+")}`;
    const bucket = groups.get(groupKey);
    if (bucket === undefined) groups.set(groupKey, [binding.id]);
    else bucket.push(binding.id);
  }
  const conflicted = new Set<string>();
  for (const ids of groups.values()) {
    if (ids.length > 1) for (const id of ids) conflicted.add(id);
  }
  return conflicted;
}
