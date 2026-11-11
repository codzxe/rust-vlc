// Go-to-Time input parsing (issue #32, `ui-dialog-goto-time`).
//
// The accepted input grammar, matching VLC's "Go to time" field:
//   * bare seconds, optionally fractional — "90", "90.5";
//   * "mm:ss" — "1:30" (minutes may exceed 59: "90:00" is 5400 s);
//   * "hh:mm:ss" — "1:02:05".
// Anything else is honestly invalid (null): negatives (goto is a forward
// position), four-plus fields, non-numeric fields, fractional low fields,
// minutes/seconds ≥ 60 in the colon forms. Whitespace is trimmed.
//
// Pure data function — fully unit-tested here; the dialog owns only the
// rendering + the duration cross-check (parse first, then compare against
// the engine-reported duration).

/**
 * Parse a Go-to-Time input into seconds. `null` = invalid input (the
 * caller renders an honest field error; nothing is guessed or clamped).
 */
export function parseTimeInput(raw: string): number | null {
  const value = raw.trim();
  if (value.length === 0) return null;
  const parts = value.split(":");
  if (parts.length > 3) return null;

  if (parts.length === 1) {
    // Bare seconds — decimals allowed, sign never (a goto target is a
    // position, not a delta).
    if (!/^\d+(\.\d+)?$/.test(parts[0] ?? "")) return null;
    const seconds = Number(parts[0]);
    return Number.isFinite(seconds) ? seconds : null;
  }

  // Colon forms — every field is a non-negative integer; the two low
  // fields are bounded to 0..59 (VLC rejects "1:75" too).
  const fields: number[] = [];
  for (const part of parts) {
    if (!/^\d+$/.test(part)) return null;
    fields.push(Number(part));
  }
  const [major, minor, lowest = 0] = fields as [number, number, number?];
  if (minor > 59 || lowest > 59) return null;
  return parts.length === 2
    ? major * 60 + minor
    : major * 3600 + minor * 60 + lowest;
}
