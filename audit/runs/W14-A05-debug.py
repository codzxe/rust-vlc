# debug: why does GOOD not match app.rs line 293?
import io
src = io.open("/home/z/my-project/audit-workspace/apps/desktop-tauri/src/app.rs", encoding="utf-8").read()
LB, RB = chr(91), chr(93)
GOOD = ".invoke_handler(tauri::generate_handler" + LB + "lumen_dispatch" + RB + ")"
print("GOOD      :", repr(GOOD))
# extract the real registration line
for i, line in enumerate(src.splitlines(), 1):
    if "invoke_handler" in line and "generate_handler" in line and not line.lstrip().startswith("//"):
        print("real line :", repr(line))
        print("line no   :", i)
print("GOOD in src:", GOOD in src)
# char-diff the tail of the real line vs GOOD
for line in src.splitlines():
    if "generate_handler" in line and not line.lstrip().startswith("//"):
        frag = line.strip()
        print("frag == GOOD:", frag == GOOD, "| frag repr tail:", repr(frag[-40:]))
        break
