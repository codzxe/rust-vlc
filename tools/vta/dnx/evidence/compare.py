import sys, hashlib
def stats(a, b, w, h, depth, name):
    ch_w = w//2
    if depth == 8:
        fsz = w*h + 2*ch_w*h
        planes = [("Y", 0, w*h), ("Cb", w*h, w*h+ch_w*h), ("Cr", w*h+ch_w*h, fsz)]
        def get(buf, o): return buf[o]
    else:
        fsz = 2*(w*h + 2*ch_w*h)
        planes = [("Y", 0, w*h), ("Cb", w*h, w*h+ch_w*h), ("Cr", w*h+ch_w*h, w*h+2*ch_w*h)]
        def get(buf, o): return buf[2*o] | (buf[2*o+1] << 8)
    out = []
    am, bm = hashlib.md5(a).hexdigest(), hashlib.md5(b).hexdigest()
    out.append(f"  md5 {'EQUAL' if am==bm else 'differs'} ({am[:12]})")
    for pn, s, e in planes:
        diffs = 0; mx = 0; n = e - s
        for i in range(s, e):
            d = abs(get(a,i) - get(b,i))
            if d: diffs += 1
            if d > mx: mx = d
        out.append(f"  {pn}: maxΔ={mx} differing={diffs}/{n} ({100.0*diffs/n:.3f}%)")
    return "\n".join(out)
lanes = [("L1_1080p_175M",1920,1080,8),("L2_1080p_36M",1920,1080,8),("L3_1080p_145M",1920,1080,8),("L4_720p_90M",1280,720,8),("L5_1080p10_185M",1920,1080,10)]
for name,w,h,d in lanes:
    a = open(f"{name}.ref.raw","rb").read(); b = open(f"{name}.mine.raw","rb").read()
    fsz = (w*h + 2*(w//2)*h) * (1 if d==8 else 2)
    nf = len(a)//fsz
    print(f"{name}: {nf} frames")
    for i in range(nf):
        print(f" frame {i}:")
        print(stats(a[i*fsz:(i+1)*fsz], b[i*fsz:(i+1)*fsz], w, h, d, name))
