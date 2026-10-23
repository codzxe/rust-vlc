// Available locales — card 4.E2 language picker data source.
//
// REAL data only: the localization crate (crates/localization) resolves
// catalogs by scanning the repo `locales/` directory (load_locales_dir,
// catalog.rs); today that directory contains exactly one catalog — the
// reference locale en-US (doc 07 §6.3, REFERENCE_LOCALE in locale.rs). The
// schema entry global.language carries a [VERIFY] note for exactly this
// dynamic listing; when more locale packs land, grow this list from the same
// directory scan (a generated list is the follow-up once >1 catalog exists).

import type { MessageKey } from "../../../../i18n/messages";

/** Locale tags with a real catalog in `locales/` (the crate's source of truth). */
export const AVAILABLE_LOCALES: readonly string[] = ["en-US"];

/** doc 07 §6.3 — en-US is the reference catalog, always available. */
export const REFERENCE_LOCALE = "en-US";

/** Catalog label msgids per locale tag (raw tag shown when copy is missing). */
const LOCALE_LABEL_MSGIDS: Readonly<Record<string, MessageKey>> = {
  "en-US": "settings.locale.en-us",
};

/** Human label for a locale tag — the tag itself when no catalog copy exists. */
export function localeLabel(tag: string, m: (key: MessageKey) => string): string {
  const msgid = LOCALE_LABEL_MSGIDS[tag];
  return msgid === undefined ? tag : m(msgid);
}
