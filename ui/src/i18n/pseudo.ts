// Pseudo-locale generator (card 4.H3, doc 07 §6.3).
//
// `pseudo` is a DETERMINISTIC transform of the en-US reference catalog used
// to prove layouts without a second real language:
//
//   * every string is bracket-wrapped — an unbracketed user-visible string
//     in a `?locale=pseudo` run is a hardcoded literal (hardcode proof);
//   * ASCII letters map to accented/tall glyphs (à ĝ Å Ň …) — glyph-height
//     and diacritic clipping become visible;
//   * a lengthening suffix pads every string to ≥1.5× its en-US length —
//     truncation/ellipsis-triggered layouts fail loudly;
//   * doc 07 §6.2 placeholders `{…}` (including nested plural/select arms)
//     and `{{ }}` escapes pass through VERBATIM — a transform that ate a
//     placeholder would corrupt the message-format contract.
//
// It is generated at runtime from the en-US catalog (never checked in as a
// second hand-written catalog — no drift possible) and is NOT a locales/
// pack: the language picker lists only real catalogs (locales.ts).
export const PSEUDO_LOCALE_TAG = "pseudo";

// Tall/accented stand-ins for a–z (upper case via String.toUpperCase).
const ACCENTED: Readonly<Record<string, string>> = {
  a: "à", b: "ƀ", c: "ç", d: "đ", e: "ê", f: "ƒ", g: "ĝ", h: "ĥ", i: "î",
  j: "ĵ", k: "ķ", l: "ļ", m: "ɱ", n: "ñ", o: "ô", p: "þ", q: "ǫ", r: "ř",
  s: "š", t: "ţ", u: "û", v: "ʋ", w: "ŵ", x: "ẋ", y: "ŷ", z: "ž",
};

/** Lengthening filler — appended until the string is ≥1.5× the source length. */
const FILLER = "äḷḷøçàṭë";

/** Deterministic pseudo transform of one en-US string. */
export function toPseudo(text: string): string {
  // 1. Brace-depth scan: lift every top-level {…} region out verbatim.
  //    Nested arms (`{n, plural, one {# item} …}`) stay inside their token;
  //    `{{…}}` escapes are simply two-deep tokens and survive whole. Tokens
  //    are masked with private-use sentinels (U+E000…U+E001) so the letter
  //    map can never touch them.
  const saved: string[] = [];
  let masked = "";
  let depth = 0;
  let token = "";
  for (const ch of text) {
    if (depth > 0) {
      token += ch;
      if (ch === "{") depth += 1;
      else if (ch === "}") {
        depth -= 1;
        if (depth === 0) {
          saved.push(token);
          masked += `\uE000${saved.length - 1}\uE001`;
          token = "";
        }
      }
    } else if (ch === "{") {
      depth = 1;
      token = "{";
    } else {
      masked += ch;
    }
  }
  if (token !== "") masked += token; // unterminated brace: pass through as-is

  // 2. Accent + tall-glyph the non-placeholder text.
  let accented = "";
  for (const ch of masked) {
    const lower = ACCENTED[ch.toLowerCase()];
    if (lower === undefined) accented += ch;
    else accented += ch === ch.toLowerCase() ? lower : lower.toUpperCase();
  }

  // 3. Lengthen to ≥1.5× the source length, bracket-wrap, restore tokens.
  let core = accented;
  const target = Math.ceil(text.length * 1.5);
  while (core.length < target) core += FILLER;
  return `[${core}]`.replace(/\uE000(\d+)\uE001/g, (_, i) => saved[Number(i)] ?? "");
}
