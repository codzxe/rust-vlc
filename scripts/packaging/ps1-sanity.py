#!/usr/bin/env python3
"""ps1-sanity.py -- structural sanity check for the packaging .ps1
scripts (issue #56, cards 8.E1-8.E3, task W24-C; called by
scripts/packaging/gate.sh step 4).

WHY THIS EXISTS (honest scope): this sandbox has no pwsh, and the
packaging gate must still catch the cheap classes of PowerShell
breakage before the Windows runner does. This is a STRUCTURAL check,
NOT a parse [the real parse runs on the Windows runner -- VERIFY at
first live run]:

  1. ASCII-only source. Windows PowerShell 5.1 reads BOM-less files as
     ANSI, so any non-ASCII byte in a comment or string mojibakes on
     legacy hosts; PowerShell 7+ handles UTF-8, but pure ASCII is the
     one encoding that is correct under BOTH. (The packaging .ps1 files
     are kept ASCII by convention; this check enforces it.)
  2. Balanced (), [], {} OUTSIDE comments, strings and here-strings.
  3. Balanced here-strings: every @" / @' opener (last token on its
     line, per the PowerShell grammar) has a matching line-leading
     "@ / '@ terminator; nothing is left open at EOF.
  4. Balanced block comments <# ... #>.
  5. Every `param(` is the first statement of the file (only comments
     and blank lines before it) -- code before param is the classic
     PowerShell gotcha that silently breaks parameter binding.

Known limitations (deliberate): $(...) sub-expressions INSIDE double-
quoted strings are skipped, not balance-checked; backtick escapes are
honored in double-quoted strings; a `#` inside a string is content,
not a comment. All of these match how the checker walks the file, and
none of them can hide an unbalanced delimiter from rule 2.

Usage: python3 scripts/packaging/ps1-sanity.py <file.ps1> [...]
Exit 0 = structurally sane; 1 = findings (printed to stderr).
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE_CLOSE = {"hdq": '"@', "hsq": "'@"}


class Walker:
    """Character walker with (line, column) bookkeeping over `text`."""

    def __init__(self, text: str) -> None:
        self.text = text
        self.pos = 0
        self.line = 1
        self.col = 1

    def eof(self) -> bool:
        return self.pos >= len(self.text)

    def peek(self, ahead: int = 0) -> str:
        p = self.pos + ahead
        return self.text[p] if p < len(self.text) else ""

    def line_text(self) -> str:
        """The remainder of the current line (from col onward)."""
        end = self.text.find("\n", self.pos)
        if end == -1:
            end = len(self.text)
        return self.text[self.pos:end]

    def line_start(self) -> bool:
        return self.col == 1

    def advance(self, count: int = 1) -> None:
        for _ in range(count):
            if self.eof():
                return
            if self.text[self.pos] == "\n":
                self.line += 1
                self.col = 1
            else:
                self.col += 1
            self.pos += 1

    def skip_to_line_end(self) -> None:
        end = self.text.find("\n", self.pos)
        self.advance(len(self.text) - self.pos if end == -1 else end - self.pos)


def check(path: Path) -> list[str]:
    findings: list[str] = []
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return [f"{path}: unreadable as UTF-8: {exc}"]

    for lineno, line in enumerate(text.splitlines(), start=1):
        bad = next((ch for ch in line if ord(ch) > 0x7F), None)
        if bad is not None:
            findings.append(
                f"{path}:{lineno}: non-ASCII byte U+{ord(bad):04X} "
                f"(Windows PowerShell 5.1 reads BOM-less files as ANSI -- "
                f"keep .ps1 source ASCII)"
            )

    w = Walker(text)
    stack: list[tuple[str, int]] = []
    mode = "code"
    while not w.eof():
        ch = w.peek()
        nxt = w.peek(1)

        if mode == "code":
            if ch == "#":
                w.skip_to_line_end()
            elif ch == "<" and nxt == "#":
                w.advance(2)
                mode = "blockcomment"
            elif ch == '"':
                w.advance(1)
                mode = "dq"
            elif ch == "'":
                w.advance(1)
                mode = "sq"
            elif ch == "@" and nxt in ('"', "'") and w.line_text()[2:].strip() == "":
                # here-string opener: @" / @' as the last token on the line
                w.advance(2)
                w.skip_to_line_end()
                mode = "hdq" if nxt == '"' else "hsq"
            elif ch in "([{":
                stack.append((ch, w.line))
                w.advance(1)
            elif ch in ")]}":
                if not stack:
                    findings.append(
                        f"{path}:{w.line}: unbalanced '{ch}' (nothing open)"
                    )
                else:
                    opener, oline = stack.pop()
                    want = {"(": ")", "[": "]", "{": "}"}[opener]
                    if want != ch:
                        findings.append(
                            f"{path}:{w.line}: mismatched '{ch}' closes "
                            f"'{opener}' opened at line {oline}"
                        )
                w.advance(1)
            else:
                w.advance(1)
        elif mode == "blockcomment":
            if ch == "#" and nxt == ">":
                w.advance(2)
                mode = "code"
            else:
                w.advance(1)
        elif mode == "dq":
            if ch == '"':
                if nxt == '"':  # escaped ""
                    w.advance(2)
                else:
                    w.advance(1)
                    mode = "code"
            elif ch == "`":  # backtick escape
                w.advance(2)
            else:
                w.advance(1)
        elif mode == "sq":
            if ch == "'":
                if nxt == "'":  # escaped ''
                    w.advance(2)
                else:
                    w.advance(1)
                    mode = "code"
            else:
                w.advance(1)
        else:  # hdq / hsq
            if w.line_start() and w.line_text().startswith(HERE_CLOSE[mode]):
                w.advance(len(HERE_CLOSE[mode]))
                mode = "code"
            else:
                w.skip_to_line_end()
                w.advance(1)  # consume the newline

    for opener, oline in stack:
        findings.append(f"{path}:{oline}: '{opener}' never closed")
    if mode != "code":
        name = {"blockcomment": "block comment", "dq": 'double-quoted string',
                "sq": "single-quoted string", "hdq": 'here-string @"',
                "hsq": "here-string @'"}[mode]
        findings.append(f"{path}: file ends inside a {name}")

    # param( placement: only comments, blank lines and param-block
    # attributes ([CmdletBinding()] et al. -- legal and idiomatic right
    # before param) may precede it.
    lines = text.splitlines()
    for idx, line in enumerate(lines, start=1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("param("):
            break
        if stripped.startswith("[") and stripped.endswith(")]") or stripped == "]":
            continue  # parameter attribute (e.g. [CmdletBinding()])
        findings.append(
            f"{path}:{idx}: code precedes 'param(' (first statement rule) -- "
            f"PowerShell silently degrades parameter binding when param is "
            f"not first"
        )
        break

    return findings


def main() -> None:
    if len(sys.argv) < 2:
        print(__doc__.splitlines()[0])
        print("usage: ps1-sanity.py <file.ps1> [...]")
        raise SystemExit(2)
    all_findings: list[str] = []
    for arg in sys.argv[1:]:
        p = Path(arg)
        if not p.is_file():
            all_findings.append(f"{p}: not a file")
            continue
        all_findings.extend(check(p))
    if all_findings:
        for f in all_findings:
            print(f"ps1-sanity: {f}", file=sys.stderr)
        print(
            f"ps1-sanity: FAIL ({len(all_findings)} finding(s)) -- "
            f"structural check only; the real parse runs on the Windows "
            f"runner [VERIFY at first live run]",
            file=sys.stderr,
        )
        raise SystemExit(1)
    print("ps1-sanity: PASS (ASCII-only, delimiters balanced, param first)")


if __name__ == "__main__":
    main()
