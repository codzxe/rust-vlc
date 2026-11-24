# W14-A05 — faithful replication of apps/desktop-tauri/tests/single_command.rs
# against the real src/app.rs (the cargo test target cannot build: E0425 + gdk-sys).
# v2: index arithmetic mirrors the Rust test EXACTLY (relative slice finds).
import io, os

ROOT = "/home/z/my-project/audit-workspace/apps/desktop-tauri"
src = io.open(os.path.join(ROOT, "src/app.rs"), encoding="utf-8").read()
source = "\n".join(l for l in src.splitlines() if not l.lstrip().startswith("//"))

LB = chr(91)   # '['
RB = chr(93)   # ']'
HANDLER = "generate_handler!"

def scan(text):
    """Mirror of registration_source + test 1 parsing. Returns (ok, registered, note)."""
    handler = text.find(HANDLER)
    if handler == -1:
        return False, None, "unreachable: no generate_handler! in app.rs code"
    array_open = text[handler:].find(LB)          # relative to handler (Rust: source[handler..].find('['))
    if array_open == -1:
        return False, None, "unreachable: no '[' after generate_handler!"
    array_close = text[handler + array_open:].find(RB)  # relative to the '['
    if array_close == -1:
        return False, None, "unreachable: no ']' closing the array"
    body = text[handler + array_open + 1 : handler + array_open + array_close]
    registered = [e for e in (s.strip() for s in body.split(",")) if e]
    return True, registered, None

# --- test 1
ok, registered, note = scan(source)
if ok:
    print("test1 registered:", registered,
          "-> PASS" if registered == ["lumen_dispatch"] else "-> FAIL (assert)")
else:
    print("test1 -> PANIC via", note)

# --- test 2
sites = source.count(HANDLER)
print("test2 sites:", sites, "-> PASS" if sites == 1 else "-> FAIL (assert)")

# --- mutation probes: does the gate actually gate?
def probe(name, mutated):
    ok, reg, note = scan(mutated)
    if not ok:
        print("mutation %-40s -> parser LOST: %s" % (name, note))
        return
    verdict = "CAUGHT (assert fails)" if reg != ["lumen_dispatch"] else "NOT CAUGHT — BYPASS"
    print("mutation %-40s registered=%s -> %s" % (name, reg, verdict))

BANG = "!"
GOOD = ".invoke_handler(tauri::generate_handler" + BANG + LB + "lumen_dispatch" + RB + ")"
assert GOOD in source, "GOOD must match the real registration line"
probe("add 2nd cmd to the array", source.replace(GOOD,
       ".invoke_handler(tauri::generate_handler" + BANG + LB + "lumen_dispatch, sneaky_cmd" + RB + ")"))
probe("paren-form macro hides 2nd cmd", source.replace(GOOD,
       ".invoke_handler(tauri::generate_handler(lumen_dispatch, sneaky_cmd))"))
probe("manual closure, no macro at all", source.replace(GOOD,
       ".invoke_handler(Box::new(|invoke| hand_roll(invoke)))"))
probe("block-comment decoy site (false red)", source.replace(
       "pub fn run()", "/* generate_handler" + BANG + LB + "decoy" + RB + " */\npub fn run()", 1))
probe("remove registration entirely", source.replace(GOOD, ".invoke_handler(noop)"))
