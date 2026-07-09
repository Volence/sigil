# S4LZ vector provenance

Every `*.s4lz` file here was produced by the REAL `aeon/tools/s4lz.py`
encoder (Python, ground truth), never by this crate's own port. The Rust
byte-exact gate (`crates/sigil-frontend-emp/tests/s4lz_vectors.rs`) loads the
`.bin` input next to each `.s4lz` output and asserts
`s4lz_from_data(input, opts) == committed .s4lz bytes` for every pair.

- `s4lz.py` path: `aeon/tools/s4lz.py`
- aeon git rev at capture time: `a103e46da0cb7bb5dbc237cca57f6904c540c89d`
- Every vector was also round-tripped (`s4lz.decompress(compressed, ...) ==
  original`) against the same `s4lz.py` at generation time, so each `.s4lz`
  file is a real, valid, self-consistent stream — not just "whatever the
  encoder happened to emit."

## Regeneration

```
python3 - <<'PY'
import sys
sys.path.insert(0, "/home/volence/sonic_hacks/aeon/tools")
import s4lz
# ... see the full generator, reproduced below for exact provenance ...
PY
```

The exact generator script used (kept in the sigil-side agent's scratchpad,
not committed to either repo — reproduced here verbatim for provenance):

```python
#!/usr/bin/env python3
import os
import struct
import sys

AEON = "/home/volence/sonic_hacks/aeon"
SIGIL = "/home/volence/sonic_hacks/sigil/.worktrees/compression-builtins"
OUT = os.path.join(SIGIL, "crates/sigil-frontend-emp/tests/vectors/s4lz")

sys.path.insert(0, os.path.join(AEON, "tools"))
import s4lz

os.makedirs(OUT, exist_ok=True)


def write(name, data):
    path = os.path.join(OUT, name)
    with open(path, "wb") as f:
        f.write(data)
    print(f"wrote {name}: {len(data)} bytes")


def build_payload() -> bytes:
    """Faithful copy of aeon/tools/gen_compression_vectors.py::build_payload()."""
    out = bytearray()
    out += bytes([0xAB, 0xCD]) * 24
    for i in range(40):
        out += struct.pack(">H", 0x1000 + 17 * i)
    out += out[48:48 + 60]
    for i in range(260):
        out += struct.pack(">H", 0x4000 + 23 * i)
    out += out[108:108 + 16]
    for i in range(3):
        out += struct.pack(">H", 0x7001 + 2 * i)
    out += out[-6:]
    for i in range(4):
        out += struct.pack(">H", 0x7101 + 2 * i)
    assert len(out) % 2 == 0
    assert len(out) <= 1024
    return bytes(out)


payload = build_payload()
assert len(payload) == 744
write("payload_744.bin", payload)
plain = s4lz.compress(payload)
write("payload_744_plain.s4lz", plain)
assert s4lz.decompress(plain) == payload

dict256 = payload[:256]
write("payload_744_dict.bin", dict256)
dict_stream = s4lz.compress(payload, dictionary=dict256)
write("payload_744_dict.s4lz", dict_stream)
assert s4lz.decompress(dict_stream, dictionary=dict256) == payload

# 768-byte block from art/uncompressed/shields/lightning_shield.bin (4320B,
# real committed uncompressed tile art; 135 tiles of 32B each).
ASSET = os.path.join(AEON, "art/uncompressed/shields/lightning_shield.bin")
with open(ASSET, "rb") as f:
    asset = f.read()
assert len(asset) == 4320

BLOCK_OFF = 2304
BLOCK_LEN = 768
block = asset[BLOCK_OFF:BLOCK_OFF + BLOCK_LEN]
write("shield_block_768.bin", block)
plain_block = s4lz.compress(block)
write("shield_block_768_plain.s4lz", plain_block)
assert s4lz.decompress(plain_block) == block

for dict_len in (768, 1536, 2304):
    dict_start = BLOCK_OFF - dict_len
    d = asset[dict_start:BLOCK_OFF]
    write(f"shield_dict_{dict_len}.bin", d)
    stream = s4lz.compress(block, dictionary=d)
    write(f"shield_block_768_dict{dict_len}.s4lz", stream)
    assert s4lz.decompress(stream, dictionary=d) == block

# Edge vectors
write("edge_empty.bin", b"")
c = s4lz.compress(b"")
write("edge_empty.s4lz", c)
assert s4lz.decompress(c) == b""

odd1 = bytes([0x42])
write("edge_odd1.bin", odd1)
c = s4lz.compress(odd1)
write("edge_odd1.s4lz", c)
assert s4lz.decompress(c) == odd1


def boundary_data(gap_bytes: int) -> bytes:
    pattern = bytes([0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE])
    filler = bytearray()
    i = 0
    while len(filler) < gap_bytes - len(pattern):
        filler.extend(struct.pack(">H", 0x8000 + i))
        i += 1
    return pattern + bytes(filler[:gap_bytes - len(pattern)]) + pattern


b510 = boundary_data(510)
write("edge_boundary_offset_510.bin", b510)
c = s4lz.compress(b510)
write("edge_boundary_offset_510.s4lz", c)
assert s4lz.decompress(c) == b510

b512 = boundary_data(512)
write("edge_boundary_offset_512.bin", b512)
c = s4lz.compress(b512)
write("edge_boundary_offset_512.s4lz", c)
assert s4lz.decompress(c) == b512

lits_16 = bytes((0x40 + i) & 0xFF for i in range(32))
both_ext = bytes(lits_16) + lits_16[-2:] * 20
write("edge_both_extended.bin", both_ext)
c = s4lz.compress(both_ext)
write("edge_both_extended.s4lz", c)
assert s4lz.decompress(c) == both_ext

# tile_delta vector: 5 tiles (160B) of the same real shield asset.
TILE_SIZE = 32
td_bytes = asset[:TILE_SIZE * 5]
write("tile_delta_5tiles.bin", td_bytes)
c = s4lz.compress(td_bytes, tile_delta=True)
write("tile_delta_5tiles.s4lz", c)
assert s4lz.decompress(c) == td_bytes
```

## Vector inventory

| files | shape | exercises |
|---|---|---|
| `payload_744.bin` / `payload_744_plain.s4lz` | 744B synthetic payload (`gen_compression_vectors.py::build_payload()`, reproduced verbatim above), plain v3 | every v3 decoder branch per that generator's token map: short + long offsets, both-extended tokens, overlap-2 copy, literal-only tail |
| `payload_744.bin` (input) / `payload_744_dict.bin` (dict = payload[:256]) / `payload_744_dict.s4lz` | same 744B payload, dictionary-seeded | dict rebase path (match source below output start), short offset from dict |
| `shield_block_768.bin` / `shield_block_768_plain.s4lz` | 768B block sliced from `aeon/art/uncompressed/shields/lightning_shield.bin` at byte offset 2304 (real committed tile art, no dictionary) | plain compression on real non-synthetic data (`BLOCK_RAW_SIZE=768` shape) |
| `shield_dict_768.bin` + `shield_block_768_dict768.s4lz` | 768B dict (asset bytes [1536:2304)) + same 768B block | 1-block dictionary window, real data |
| `shield_dict_1536.bin` + `shield_block_768_dict1536.s4lz` | 1536B dict (asset bytes [768:2304)) + same 768B block | 2-block dictionary window |
| `shield_dict_2304.bin` + `shield_block_768_dict2304.s4lz` | 2304B dict (asset bytes [0:2304)) + same 768B block | 3-block dictionary window (`MAX_DICT_BLOCKS=3` shape) |
| `edge_empty.bin` / `edge_empty.s4lz` | 0-byte input | empty-input header-only stream (`TOKEN_END` immediately) |
| `edge_odd1.bin` / `edge_odd1.s4lz` | 1-byte (odd) input | odd-length zero-padding to word alignment |
| `edge_boundary_offset_510.bin` / `.s4lz` | 516B, engineered repeat at exactly byte offset 510 | short-form offset encoding at its maximum (`offmark=255`) |
| `edge_boundary_offset_512.bin` / `.s4lz` | 518B, engineered repeat at exactly byte offset 512 | long-form offset word (smallest offset that no longer fits offmark) |
| `edge_both_extended.bin` / `.s4lz` | 72B: 16 unique literal words + a 20-word match at offset 2 | literal run ≥15 words AND match ≥15 words in the SAME token — both nibble extension words present, short-form offmark |
| `tile_delta_5tiles.bin` / `.s4lz` | 160B = 5 tiles (32B each) sliced from the same real shield asset, `tile_delta=True` | tile-delta XOR preprocessing on real tile-shaped data |

All `.s4lz` outputs were independently round-tripped through
`s4lz.decompress(...)` (with the matching dictionary where applicable) back
to their `.bin` input at generation time — see the `assert` calls in the
generator above.
