# i18n keys — per-lane catalogs (wave 26)

Parallel-safe pattern (CONTRACT §4.5): each lane adds genuinely NEW message
keys in its own file here (`<task-id>.ts`) exporting

```ts
export const keys = { "screen.eq.preamp": "Preamp" } as const;
```

The Lead registers these into `ui/src/i18n/messages.ts` (the MessageKey union
+ catalog) after each wave merge. Agents never edit `messages.ts` directly.
Reuse existing keys whenever the copy already exists — grep first.
