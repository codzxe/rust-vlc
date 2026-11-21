#!/usr/bin/env python3
"""Read-only ASF structure walker for W04-A05 fixture audits.

Usage: python3 asf_parse.py <fixture-path>
Parses top-level objects + header children + FPO/SP facts from raw bytes.
"""
import struct
import sys

GUIDS = {
    bytes.fromhex("3026b2758e66cf11a6d900aa0062ce6c"): "HEADER",
    bytes.fromhex("3626b2758e66cf11a6d900aa0062ce6c"): "DATA",
    bytes.fromhex("90080033b1e5cf1189f400a0c90349cb"): "SIMPLE_INDEX",
    bytes.fromhex("a1dcab8c47a9cf118ee400c00c205365"): "FILE_PROPERTIES",
    bytes.fromhex("b503bf5f2ea9cf118ee300c00c205365"): "HEADER_EXTENSION",
    bytes.fromhex("40a4d0d207e3d21197f000a0c95ea850"): "EXTENDED_CONTENT_DESCRIPTION",
    bytes.fromhex("f02e92758e66cf11a6d900aa0062ce6c"): "CONTENT_DESCRIPTION",
    bytes.fromhex("9107dcb7b7a9cf118ee600c00c205365"): "STREAM_PROPERTIES",
    bytes.fromhex("4052d1861d31d011a3a400a0c90348f6"): "CODEC_LIST",
    bytes.fromhex("409e69f84d5bcf11a8fd00805f5c442b"): "STREAM_TYPE_AUDIO",
    bytes.fromhex("c0ef19bc4d5bcf11a8fd00805f5c442b"): "STREAM_TYPE_VIDEO",
    bytes.fromhex("c0cfda59e659d011a3ac00a0c90348f6"): "STREAM_TYPE_COMMAND",
    bytes.fromhex("eacbf8c5af5b77488467aa8c44fa4cca"): "EXTENDED_STREAM_PROPERTIES",
}
HEADER = bytes.fromhex("3026b2758e66cf11a6d900aa0062ce6c")


def guid(b):
    return GUIDS.get(b, "unknown:" + b.hex())


def main(path):
    d = open(path, "rb").read()
    print(f"file: {path}  size={len(d)}")
    off = 0
    if d[:16] != HEADER:
        print(f"  first16={d[:16].hex()} (not bare HEADER guid)")
        if d[12:28] == HEADER:
            print("  -> 12-byte MMS broadcast prefix, HEADER at offset 12")
            off = 12
        else:
            print("  -> HEADER guid not at 0 or 12; aborting walk")
            return
    hsize = struct.unpack_from("<Q", d, off + 16)[0]
    nchildren = struct.unpack_from("<I", d, off + 24)[0]
    reserved2 = d[off + 28:off + 30].hex()
    print(f"  HEADER: header_size={hsize} declared_children={nchildren} "
          f"reserved2={reserved2}")
    p = off + 30
    end = off + hsize
    count = 0
    while p + 24 <= end and count < 40:
        g = d[p:p + 16]
        sz = struct.unpack_from("<Q", d, p + 16)[0]
        name = guid(g)
        print(f"    child[{count}] off={p} {name} size={sz}")
        if name == "FILE_PROPERTIES":
            # layout (ASF spec, matches demuxer header.rs:708-716 with
            # payload = p+24): fileid16 file_size8 creation8 packets8
            # play8 send8 preroll8 flags4 min4 max4 maxbitrate4
            file_size = struct.unpack_from("<Q", d, p + 40)[0]
            pkt_count = struct.unpack_from("<Q", d, p + 56)[0]
            preroll = struct.unpack_from("<Q", d, p + 80)[0]
            fflags = struct.unpack_from("<I", d, p + 88)[0]
            minpkt = struct.unpack_from("<I", d, p + 92)[0]
            maxpkt = struct.unpack_from("<I", d, p + 96)[0]
            print(f"      FPO: file_size={file_size} data_packets={pkt_count} "
                  f"preroll={preroll} flags=0x{fflags:08x} "
                  f"broadcast={fflags & 1} seekable={(fflags >> 1) & 1} "
                  f"min_pkt={minpkt} max_pkt={maxpkt}")
        if name == "STREAM_PROPERTIES":
            stype = guid(d[p + 24:p + 40])
            ec = guid(d[p + 40:p + 56])
            tsd_len = struct.unpack_from("<I", d, p + 56 + 8)[0]
            sp_flags = struct.unpack_from("<H", d, p + 56 + 8 + 4 + 4)[0]
            tsd_off = p + 56 + 8 + 4 + 4 + 2 + 4
            print(f"      SP: type={stype} ec={ec} tsd_len={tsd_len} "
                  f"stream_number={sp_flags & 0x7f}")
            tsd = d[tsd_off:tsd_off + tsd_len]
            if stype == "STREAM_TYPE_AUDIO" and tsd_len >= 18:
                tag, ch, rate = struct.unpack_from("<HHI", tsd, 0)
                avg, align, bits, cb = struct.unpack_from("<IHHH", tsd, 8)
                print(f"      WFX: tag=0x{tag:04x} ch={ch} rate={rate} align={align} "
                      f"bits={bits} cb_size={cb}")
            if stype == "STREAM_TYPE_VIDEO" and tsd_len >= 11:
                # demuxer form (header.rs:808-848): width(4) height(4)
                # 0x02 byte WORD data_size BITMAPINFOHEADER extradata
                w, h = struct.unpack_from("<II", tsd, 0)
                obs = tsd[8]
                data_size = struct.unpack_from("<H", tsd, 9)[0]
                bmih = tsd[11:11 + data_size]
                if len(bmih) >= 24:
                    bisize = struct.unpack_from("<I", bmih, 0)[0]
                    bitc = struct.unpack_from("<H", bmih, 14)[0]
                    fourcc = bmih[16:20].decode("latin1")
                    size_image = struct.unpack_from("<I", bmih, 20)[0]
                    print(f"      VIH: {w}x{h} obs=0x{obs:02x} data_size={data_size} "
                          f"biSize={bisize} bit_count={bitc} fourcc={fourcc!r} "
                          f"size_image={size_image} extradata={data_size - 40}")
        p += sz
        count += 1
    if p != end:
        print(f"    (child walk ended at {p}, header end {end})")
    while p + 24 <= len(d):
        g = d[p:p + 16]
        sz = struct.unpack_from("<Q", d, p + 16)[0]
        name = guid(g)
        extra = ""
        if name == "DATA":
            # GUID16 size8 fileid16 packet_count8 reserved2
            pkts = struct.unpack_from("<Q", d, p + 40)[0]
            reserved = d[p + 48:p + 50].hex()
            extra = f" packet_count={pkts} reserved={reserved}"
        if sz < 24 or p + sz > len(d):
            print(f"  top off={p} {name} size={sz} (INVALID/overrun)")
            break
        print(f"  top off={p} {name} size={sz}{extra}")
        p += sz
    if p != len(d):
        print(f"  (top walk ended at {p} vs file size {len(d)})")


if __name__ == "__main__":
    main(sys.argv[1])
