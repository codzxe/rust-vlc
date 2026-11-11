// Bookmark-list ordering (w26.a18) — natural sort for the Bookmarks dialog.
//
// The W26 brief pins the list to NATURAL order ("1, 2, 10 — not lexical"):
// a plain string compare would put "Chapter 10" before "Chapter 2" and a
// lexical timecode compare would put "10:00" before "2:00". The row order is
// therefore: time ascending (numeric engine truth), labels compared
// digit-chunk-aware (natural), creation id as the final stable tie-break.
//
// Pure data functions — fully unit-tested here (naturalSort.test.ts); the
// dialog owns only the rendering.
import type { Bookmark } from "../../../generated/commands";

/**
 * Natural (digit-chunk-aware) string compare. Runs of digits compare
 * numerically ("2" < "10"), everything else byte-wise; shorter input sorts
 * first when one is a prefix ("Track 2" < "Track 2 remix"). Case-sensitive
 * on the non-digit chunks (labels are user data; a stable, predictable
 * order beats locale folding here).
 */
export function naturalCompare(a: string, b: string): number {
  let ia = 0;
  let ib = 0;
  while (ia < a.length && ib < b.length) {
    const ca = a.charAt(ia);
    const cb = b.charAt(ib);
    const aDigit = ca >= "0" && ca <= "9";
    const bDigit = cb >= "0" && cb <= "9";
    if (aDigit && bDigit) {
      // Digit run: compare numerically (strip leading zeros, then by length,
      // then byte-wise — no Number() so arbitrarily long runs stay exact).
      let ja = ia;
      while (ja < a.length && a.charAt(ja) >= "0" && a.charAt(ja) <= "9") ja += 1;
      let jb = ib;
      while (jb < b.length && b.charAt(jb) >= "0" && b.charAt(jb) <= "9") jb += 1;
      const runA = a.slice(ia, ja).replace(/^0+/, "") || "0";
      const runB = b.slice(ib, jb).replace(/^0+/, "") || "0";
      if (runA.length !== runB.length) return runA.length - runB.length;
      if (runA !== runB) return runA < runB ? -1 : 1;
      ia = ja;
      ib = jb;
      continue;
    }
    if (ca !== cb) return ca < cb ? -1 : 1;
    ia += 1;
    ib += 1;
  }
  // Prefix rule: the exhausted string sorts first.
  return (a.length - ia) - (b.length - ib);
}

/**
 * Bookmark row order (w26.a18 natural-sort pin): time ascending — numeric
 * engine truth, never a lexical timecode compare — then natural label order
 * (unnamed compares as ""), then the creation id as the stable final break.
 */
export function compareBookmarks(a: Bookmark, b: Bookmark): number {
  if (a.time !== b.time) return a.time - b.time;
  const labelA = a.label ?? "";
  const labelB = b.label ?? "";
  if (labelA !== labelB) return naturalCompare(labelA, labelB);
  return a.id - b.id;
}
