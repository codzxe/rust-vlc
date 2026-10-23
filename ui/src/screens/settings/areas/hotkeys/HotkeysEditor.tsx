import { useEffect, useState } from "react";
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

/** A combo a binding currently claims: draft edit ▸ persisted override ▸ keymap default. */
const comboOf = (binding: HotkeyBinding, draft: Readonly<Record<string, string>>): string =>
  draft[binding.id] ?? binding.key;

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

  const isEdited = (id: string): boolean => (draft[id] ?? "") !== (overrides[id] ?? "");
  const dirtyIds = bindings.filter((b) => isEdited(b.id)).map((b) => b.id);
  const conflicted = detectConflicts(bindings, draft);

  useEffect(() => {
    if (dirtyIds.length === 0) return;
    const onBeforeUnload = (e: BeforeUnloadEvent): void => {
      e.preventDefault();
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, [dirtyIds.length]);

  return (
    <div className="settings-sections" data-settings-area="hotkeys">
      <section className="settings-section" data-section="bindings">
        <h2 className="settings-section__title">{m("settings.section.hotkeys.bindings")}</h2>
        <p className="settings-field__hint">{m("settings.hotkeys.defaults-note")}</p>
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
            {bindings.map((binding) => {
              const combo = comboOf(binding, draft);
              const command = binding.command ?? binding.recipe ?? binding.uiAction;
              const conflict = conflicted.has(binding.id);
              const changed = combo !== binding.key;
              return (
                <tr
                  key={binding.id}
                  className="hotkeys-row"
                  data-binding={binding.id}
                  data-conflict={conflict || undefined}
                  data-changed={changed || undefined}
                  data-disabled={binding.disabled || undefined}
                >
                  <td className="hotkeys-row__action">{m(binding.labelMsgid as MessageKey)}</td>
                  <td className="hotkeys-row__key">
                    <input
                      type="text"
                      className="hotkeys-row__input"
                      value={combo}
                      disabled={binding.disabled}
                      aria-label={m(binding.labelMsgid as MessageKey)}
                      placeholder={m("settings.hotkeys.edit-hint")}
                      data-testid={`hotkeys-input-${binding.id}`}
                      onChange={(e) =>
                        setDraft((prev) => ({ ...prev, [binding.id]: e.target.value }))
                      }
                    />
                    {conflict ? (
                      <p className="hotkeys-row__conflict" role="alert">
                        {m("settings.hotkeys.conflict")}
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
                        aria-label={m("settings.hotkeys.reset")}
                        title={m("settings.hotkeys.reset")}
                        data-testid={`hotkeys-reset-${binding.id}`}
                        onClick={() =>
                          setDraft((prev) => {
                            const next = { ...prev };
                            delete next[binding.id];
                            return next;
                          })
                        }
                      >
                        {m("settings.hotkeys.reset")}
                      </button>
                    ) : null}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>

        {dirtyIds.length > 0 ? (
          <div className="settings-applybar" data-testid="hotkeys-applybar">
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
      </section>
    </div>
  );
}
