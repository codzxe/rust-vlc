# W14-A05 — corrected paren-form macro probe (bang is part of macro syntax).
import io
src = io.open("/home/z/my-project/audit-workspace/apps/desktop-tauri/src/app.rs", encoding="utf-8").read()
source = "\n".join(l for l in src.splitlines() if not l.lstrip().startswith("//"))
LB, RB, BANG = chr(91), chr(93), "!"
HANDLER = "generate_handler" + BANG
GOOD = ".invoke_handler(tauri::generate_handler" + BANG + LB + "lumen_dispatch" + RB + ")"
assert GOOD in source

def scan(text):
    handler = text.find(HANDLER)
    if handler == -1:
        return "PANIC unreachable: no generate_handler! in app.rs code"
    array_open = text[handler:].find(LB)
    if array_open == -1:
        return "PANIC unreachable: no '[' after the site"
    array_close = text[handler + array_open:].find(RB)
    if array_close == -1:
        return "PANIC unreachable: no ']' closing the array"
    body = text[handler + array_open + 1 : handler + array_open + array_close]
    return [e for e in (s.strip() for s in body.split(",")) if e]

paren = source.replace(GOOD,
    ".invoke_handler(tauri::generate_handler" + BANG + "(lumen_dispatch, sneaky_cmd))")
res = scan(paren)
print("paren-form generate_handler!(a, b) at app.rs site ->", res,
      "=> test FAILS LOUDLY" if isinstance(res, str) or res != ["lumen_dispatch"] else "=> BYPASS")

# What text DOES the parser see in the paren case? show the anchor context
h = paren.find(HANDLER)
print("anchor context:", repr(paren[h:h+70]))

# site-count for paren form (test 2): "generate_handler!" still counted once
print("sites in paren-form mutant:", paren.count(HANDLER), "(test 2 still PASS on count)")

# second-site-in-another-file bypass is structural: registration_source()
# reads ONLY ../src/app.rs — no other crate file is ever scanned.
import os
for f in sorted(os.listdir("/home/z/my-project/audit-workspace/apps/desktop-tauri/src")):
    print("crate src file:", f)
