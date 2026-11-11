/**
 * naturalCompare / naturalSort — natural (numeric) ordering for list titles
 * and filenames: "Bookmark 1, 2, 10", never the lexical "1, 10, 2".
 *
 * This is the 2026-11-09 v2 flow brief's "natural sort" fix that MUST survive
 * the W25 reskin — numbered playlists, bookmarks and episode files must keep
 * reading in human order.
 *
 * Hand-rolled (zero-new-deps rule): both strings are chunked into maximal
 * digit / non-digit runs. Digit runs compare by numeric value — compared as
 * the zero-trimmed run's LENGTH first, then digit-wise, so arbitrarily long
 * numbers order correctly without BigInt. Text runs compare
 * case-insensitively, with a case-sensitive tie-break so ordering is total
 * and deterministic. Numerically-equal digit runs ("01" vs "1") tie-break to
 * the fewer leading zeros first (Finder-style).
 */

interface Chunk {
  digits: boolean;
  text: string;
}

const isDigit = (char: string | undefined): boolean => char !== undefined && char >= "0" && char <= "9";

/** Split into maximal digit / non-digit runs: "S01E2" → [S][01][E][2]. */
function chunkify(value: string): Chunk[] {
  const chunks: Chunk[] = [];
  let start = 0;
  let digits = isDigit(value[0]);
  for (let i = 1; i <= value.length; i++) {
    if (i === value.length || isDigit(value[i]) !== digits) {
      chunks.push({ digits, text: value.slice(start, i) });
      start = i;
      digits = isDigit(value[i]);
    }
  }
  return chunks;
}

/** Leading zeros trimmed: "007" → "7" (numeric value; "000" → "0"). */
const trimZeros = (digits: string): string => {
  const trimmed = digits.replace(/^0+/, "");
  return trimmed === "" ? "0" : trimmed;
};

/**
 * Natural-order comparator for strings. Returns a negative number when `a`
 * sorts before `b`, positive when after, 0 when equal (Array.sort contract).
 *
 * Two passes: numeric + case-insensitive first (so "bookmark 2" sorts before
 * "Bookmark 10" — case never outranks a number), then a case-sensitive
 * tie-break so ordering is total and deterministic.
 */
export function naturalCompare(a: string, b: string): number {
  const ac = chunkify(a);
  const bc = chunkify(b);
  const primary = compareChunks(ac, bc, false);
  return primary !== 0 ? primary : compareChunks(ac, bc, true);
}

function compareChunks(ac: Chunk[], bc: Chunk[], caseSensitive: boolean): number {
  const count = Math.max(ac.length, bc.length);
  for (let i = 0; i < count; i++) {
    const x = ac[i];
    const y = bc[i];
    if (x === undefined) return -1; // shorter string (a prefix) sorts first
    if (y === undefined) return 1;
    if (x.digits !== y.digits) return x.digits ? -1 : 1; // digit runs before text runs
    if (x.digits) {
      const nx = trimZeros(x.text);
      const ny = trimZeros(y.text);
      if (nx.length !== ny.length) return nx.length - ny.length; // magnitude
      if (nx !== ny) return nx < ny ? -1 : 1; // equal length → lexical IS numeric
      if (x.text.length !== y.text.length) return x.text.length - y.text.length; // fewer leading zeros first
    } else {
      const tx = caseSensitive ? x.text : x.text.toLowerCase();
      const ty = caseSensitive ? y.text : y.text.toLowerCase();
      if (tx !== ty) return tx < ty ? -1 : 1;
    }
  }
  return 0;
}

/**
 * Natural-sorted copy of `items` (the input is never mutated; the sort is
 * stable — equal keys keep their relative order). `read` extracts the sort
 * key; the default stringifies, so `naturalSort(["b10", "b2"])` works
 * directly.
 */
export function naturalSort<T>(items: readonly T[], read: (item: T) => string = (item): string => String(item)): T[] {
  return [...items].sort((x, y) => naturalCompare(read(x), read(y)));
}
