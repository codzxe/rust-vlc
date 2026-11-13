import { memo, useCallback, useEffect, useMemo, useState } from "react";
import type { ReactElement } from "react";
import { Button } from "../../../../components/button/Button";
import { useMsg } from "../../../../i18n/messages";
import type { MessageKey } from "../../../../i18n/messages";
import { HOTKEY_BINDINGS } from "./generated/bindings";
import type { HotkeyBinding } from "./generated/bindings";
import { detectConflicts } from "./conflicts";
import { localStorageHotkeyOverrides } from "./overrides";
import type { HotkeyOverridesStore } from "./overrides";
import "./hotkeys.css";

// HotkeysEditor — card 4.E2 (doc 07 §5 hotkeys editor). A table of bindings
// from the REAL keymap data (keymaps/default.toml via the generated module —
// hotkey entries are dynamic data, not schema-static [[setting]] rows, which
// is why this area is a hand-written extension beside the generated screens).
//
// Editing writes DRAFT state; Apply commits the overrides to the persistence
// seam; conflict detection marks every row of a duplicated (mode, combo)
// group with an inline warning; reset-to-default is per binding.
//
// w26.i15 (h11 #5): the editor renders inside the controls area's DIALOG
// (Keyboard_Shortcuts.svg window grammar — areas/controls.tsx opens it from
// the "Edit Keyboard Shortcuts" action row). Structure: a scrollable body
// (the table) + the dirty-gated apply bar as the dialog's footer; the old
// inline-fold presentation (nested .settings-sections root, sentence-case
// h2, defaults note) is gone — the dialog's top bar carries the title.
//
// w26.k13 (j13 P2-6, the re-render hotspot): the numeric-aware ordering
// (141 bindings × a catalog lookup each) and the conflict detection are
// MEMOIZED (they recompute only when the bindings data or the draft
// actually changes — not on every unrelated render), and each row is a
// memoized component fed stable callbacks — a keystroke in one of the 141
// controlled inputs re-renders THAT row, not the whole table's subtrees.

/** A combo a binding currently claims: draft edit ▸ persisted override ▸ keymap default. */
const comboOf = (binding: HotkeyBinding, draft: Readonly<Record<string, string>>): string =>
  draft[binding.id] ?? binding.key;

/** Label with its trailing number stripped ("Play Bookmark 10" → "Play Bookmark"). */
const labelPrefix = (label: string): string => label.replace(/[0-9]+\s*$/u, "").trim();

/** Trailing number of a label, or -1 when it ends in no number. */
const trailingNumber = (label: string): number => {
  const match = /[0-9]+\s*$/u.exec(label);
  return match === null ? -1 : Number(match[0]);
};

/**
 * Numeric-aware ordering for NUMBERED SERIES (w26.i15, h11 #5: the table used
 * to list "Play Bookmark 1, 10, 2…"). Only runs of consecutive rows whose
 * labels share one prefix and end in numbers are reordered — the curated
 * keymap order of everything else is untouched. (The same rule the
 * /keyboard-shortcuts reference dialog applies, w26.i17.)
 */
export function orderBindingsNumericAware(
  bindings: readonly HotkeyBinding[],
  labelOf: (key: MessageKey) => string,
): readonly HotkeyBinding[] {
  const labels = bindings.map((b) => labelOf(b.labelMsgid as MessageKey));
  const result = [...bindings];
  let start = 0;
  while (start < result.length) {
    const prefix = labelPrefix(labels[start] ?? "");
    let end = start + 1;
    while (end < labels.length && labelPrefix(labels[end] ?? "") === prefix) end++;
    if (end - start > 1) {
      const run = result
        .slice(start, end)
        .map((binding, offset) => ({ binding, number: trailingNumber(labels[start + offset] ?? "") }))
        .sort((a, b) => a.number - b.number);
      for (let offset = 0; offset < run.length; offset++) result[start + offset] = run[offset]!.binding;
    }
    start = end;
  }
  return result;
}

/**
 * One memoized binding row (w26.k13, j13 P2-6): every prop is a primitive or
 * a stable callback, so a draft keystroke re-renders only the row whose
 * combo changed — the other 140 rows skip their subtree diff entirely.
 */
const HotkeysRow = memo(function HotkeysRow({
  binding,
  combo,
  conflict,
  changed,
  label,
  placeholder,
  resetLabel,
  conflictLabel,
  onComboChange,
  onReset,
}: {
  binding: HotkeyBinding;
  combo: string;
  conflict: boolean;
  changed: boolean;
  label: string;
  placeholder: string;
  resetLabel: string;
  conflictLabel: string;
  onComboChange: (id: string, value: string) => void;
  onReset: (id: string) => void;
}): ReactElement {
  const command = binding.command ?? binding.recipe ?? binding.uiAction;
  return (
    <tr
      className="hotkeys-row"
      data-binding={binding.id}
      data-conflict={conflict || undefined}
      data-changed={changed || undefined}
      data-disabled={binding.disabled || undefined}
    >
      <td className="hotkeys-row__action">{label}</td>
      <td className="hotkeys-row__key">
        <input
          type="text"
          className="hotkeys-row__input"
          value={combo}
          disabled={binding.disabled}
          aria-label={label}
          placeholder={placeholder}
          data-testid={`hotkeys-input-${binding.id}`}
          onChange={(e) => onComboChange(binding.id, e.target.value)}
        />
        {conflict ? (
          <p className="hotkeys-row__conflict" role="alert">
            {conflictLabel}
          </p>
        ) : null}
      </td>
      <td className="hotkeys-row__command">{command ?? "—"}</td>
      <td className="hotkeys-row__mode">{binding.mode ?? "—"}</td>
      <td className="hotkeys-row__reset">
        {changed ? (
          <button
            type="button"
            className="settings-field__reset"
            aria-label={resetLabel}
            title={resetLabel}
            data-testid={`hotkeys-reset-${binding.id}`}
            onClick={() => onReset(binding.id)}
          >
            {resetLabel}
          </button>
        ) : null}
      </td>
    </tr>
  );
});

export function HotkeysEditor({
  bindings = HOTKEY_BINDINGS,
  store = localStorageHotkeyOverrides,
}: {
  bindings?: readonly HotkeyBinding[];
  store?: HotkeyOverridesStore;
}): ReactElement {
  const m = useMsg();
  const [overrides, setOverrides] = useState<Record<string, string>>(() => ({ ...store.load() }));
  const [draft, setDraft] = useState<Record<string, string>>(() => ({ ...overrides }));

  const isEdited = useCallback(
    (id: string): boolean => (draft[id] ?? "") !== (overrides[id] ?? ""),
    [draft, overrides],
  );
  const dirtyIds = useMemo(
    () => bindings.filter((b) => isEdited(b.id)).map((b) => b.id),
    [bindings, isEdited],
  );
  // w26.k13 (j13 P2-6): the conflict set recomputes only when the bindings
  // data or the DRAFT changes — the O(n) scan no longer runs on every render.
  const conflicted = useMemo(() => detectConflicts(bindings, draft), [bindings, draft]);
  // w26.k13 (j13 P2-6): the numeric-aware ordering (141 × catalog lookups +
  // the run sort) is memoized on the bindings data and the resolver identity
  // (useMsg's closure is per-render; the memo then recomputes exactly when a
  // locale switch could change the labels — never per keystroke).
  const ordered = useMemo(
    () => orderBindingsNumericAware(bindings, (key) => m(key)),
    [bindings, m],
  );

  // Stable row callbacks (w26.k13): setDraft is stable, so these never
  // change identity — the memoized rows keep their props equal on re-render.
  const onComboChange = useCallback((id: string, value: string) => {
    setDraft((prev) => ({ ...prev, [id]: value }));
  }, []);
  const onReset = useCallback((id: string) => {
    setDraft((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
  }, []);

  useEffect(() => {
    if (dirtyIds.length === 0) return;
    const onBeforeUnload = (e: BeforeUnloadEvent): void => {
      e.preventDefault();
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, [dirtyIds.length]);

  return (
    <>
      <div className="hotkeys-editor__body">
        <table className="hotkeys-table" data-testid="hotkeys-table">
          <thead>
            <tr>
              <th scope="col">{m("settings.hotkeys.column.action")}</th>
              <th scope="col">{m("settings.hotkeys.column.key")}</th>
              <th scope="col">{m("settings.hotkeys.column.command")}</th>
              <th scope="col">{m("settings.hotkeys.column.mode")}</th>
              <th scope="col"><span className="visually-hidden">{m("settings.hotkeys.reset")}</span></th>
            </tr>
          </thead>
          <tbody>
            {ordered.map((binding) => (
              <HotkeysRow
                key={binding.id}
                binding={binding}
                combo={comboOf(binding, draft)}
                conflict={conflicted.has(binding.id)}
                changed={comboOf(binding, draft) !== binding.key}
                label={m(binding.labelMsgid as MessageKey)}
                placeholder={m("settings.hotkeys.edit-hint")}
                resetLabel={m("settings.hotkeys.reset")}
                conflictLabel={m("settings.hotkeys.conflict")}
                onComboChange={onComboChange}
                onReset={onReset}
              />
            ))}
          </tbody>
        </table>
      </div>

      {dirtyIds.length > 0 ? (
        <div className="settings-applybar hotkeys-editor__applybar" data-testid="hotkeys-applybar">
          <span className="settings-applybar__count">
            {m("settings.unsaved-changes")} ({dirtyIds.length})
          </span>
          <Button
            variant="primary"
            onClick={() => {
              const next: Record<string, string> = {};
              for (const b of bindings) {
                const combo = comboOf(b, draft);
                if (combo !== b.key && combo !== "") next[b.id] = combo;
              }
              setOverrides(next);
              store.save(next);
              setDraft({ ...next });
            }}
          >
            {m("settings.hotkeys.apply")}
          </Button>
          <Button variant="default" onClick={() => setDraft({ ...overrides })}>
            {m("settings.hotkeys.revert")}
          </Button>
        </div>
      ) : null}
    </>
  );
}
