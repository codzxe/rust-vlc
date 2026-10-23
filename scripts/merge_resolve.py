import subprocess, sys
def resolve_union(path):
    lines = open(path).readlines()
    out, mode, head, theirs = [], None, [], []
    for l in lines:
        if l.startswith('<<<<<<<'): mode='head'; head=[]; theirs=[]; continue
        if l.startswith('=======') and mode=='head': mode='theirs'; continue
        if l.startswith('>>>>>>>'):
            out.extend(head); out.extend(theirs); mode=None; continue
        if mode=='head': head.append(l)
        elif mode=='theirs': theirs.append(l)
        else: out.append(l)
    open(path,'w').writelines(out)
for f in sys.argv[1:]:
    resolve_union(f)
