#!/usr/bin/env python3
"""ident.py <file>...: md5, IEEE/zlib CRC32 (8 hex) and size of each file."""
import sys, zlib, hashlib
for p in sys.argv[1:]:
    d = open(p, "rb").read()
    print(f"{hashlib.md5(d).hexdigest()}  crc32={zlib.crc32(d) & 0xffffffff:08x}  size={len(d)}  {p}")
