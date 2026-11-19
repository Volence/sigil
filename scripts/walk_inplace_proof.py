#!/usr/bin/env python3
"""walk_inplace_proof.py: is the old walk's measure-in-place change explained byte for byte?

THE PARCEL (sigil `parcel/walk-measure-in-place`). The old driver's packing walk
(`sigil-harness/src/native.rs`, `packed_true_bases`) re-measured a ROM section at its
packed base only when a FROZEN size table (`golden/offcanonical_sizes/<shape>.txt`) named
one of its labels. Every other section was measured at a far-scratch slot (0x700000+),
where references into it, or into any other never-pinned section, cannot take the short
forms, so it was measured LONGER than it is emitted at its real base and the difference
stayed in the ROM as zero padding after its last row. The parcel measures every non-bank
ROM section in place. This script compares one shape's OLD build (master) with its NEW
build (the parcel) and fails unless every changed byte is explained.

INPUTS per side: the ROM, its `.lst` (symbol table) and its `.build.json` (the G0a build
export: the per-section line table `lines.sections`, rows `[vma, lma, len, file, line,
kind]`). Plus the aeon tree both were built from (map.toml anchors, source lines for the
site names, `tools/deb2_probe.py` for the appendix decoder) and the sigil tree whose
frozen tables say which sections the OLD walk pinned.

WHAT IT CHECKS (each a hard failure, and a check that cannot measure is a failure too):
  1. Same sections, same rows: every section carries the same (file, line, kind, len)
     row sequence in both builds. Emitted widths are unchanged (each build emits at the
     real base); a changed row length would be an emitted width flip and is counted.
  2. Islands unmoved: every section at a declared `[[anchor]]` (for this shape's sound
     polarity) and every phase bank (vma != lma) keeps its base.
  3. The walk deltas: each section's allotment (base to the next section's base) changes
     by `new_gap - old_gap <= 0` when the next section is packed; the dropped bytes are
     0x00 in the old ROM and sit directly after the section's last row.
  4. Named sites: for every section the old walk measured at scratch (no frozen-table
     label), the bytes that section's position-dependent sites would have cost at a far
     scratch slot are derived from the emitted instructions (a model independent of
     sigil) and named `file:line`. For each section whose next section packs, the bytes
     dropped after it must EQUAL the site total, and a pinned section must drop nothing.
  5. Symbols: identical name sets; each symbol's shift equals its section's base shift;
     shifts are <= 0, zero at every island, and non-increasing along each run.
  6. Bytes: every positional difference in the image is classified (shift, padding,
     width flip, pointer re-aimed at a shifted symbol, header checksum / ROM end), with
     ZERO unexplained; the content of every row is compared at its mapped address, and
     a differing byte must lie in a pointer field whose new value is the relocation of
     its old one.
  7. deb2 appendix: starts at EndOfRom in both; every decoded (address, name) in the
     new appendix is a symbol at that address in the new listing; the decoded name sets
     differ only by names whose shared-address group changed.

Exit 0 explained · 1 an unexplained byte or failed check · 2 could not measure.
"""
from __future__ import annotations

import argparse
import bisect
import importlib.util
import json
import re
import sys
import tomllib
from collections import Counter, defaultdict
from pathlib import Path

CHECKSUM = range(0x18E, 0x190)
ROM_END = range(0x1A4, 0x1A8)

# The OLD walk's far-scratch slots (`native.rs` SCRATCH_FIRST / far_scratch_slot) only ever
# encode abs.l: any reference whose target sits in a scratch-measured section is long.
SCRATCH_IS_LONG = True


class CouldNotMeasure(Exception):
    pass


class Fail(Exception):
    pass


def abs_w_ok(a: int) -> bool:
    """`sigil_ir::asl_width_rule`: abs.w reaches [0, 0x7FFF] and [0xFF8000, 0xFFFFFF]."""
    a &= 0xFFFFFF
    return a <= 0x7FFF or a >= 0xFF8000


def be16(b, i):
    return (b[i] << 8) | b[i + 1]


def be32(b, i):
    return (b[i] << 24) | (b[i + 1] << 16) | (b[i + 2] << 8) | b[i + 3]


def sext16(v):
    return v - 0x10000 if v & 0x8000 else v


def sext8(v):
    return v - 0x100 if v & 0x80 else v


# ---------------------------------------------------------------------------- inputs
class Build:
    def __init__(self, stem: Path):
        self.stem = stem
        for suf in (".bin", ".lst", ".build.json"):
            if not Path(str(stem) + suf).is_file():
                raise CouldNotMeasure(f"missing {stem}{suf}")
        self.rom = Path(str(stem) + ".bin").read_bytes()
        self.export = json.loads(Path(str(stem) + ".build.json").read_text())
        self.syms = read_symtab(Path(str(stem) + ".lst"))
        if "EndOfRom" not in self.syms:
            raise CouldNotMeasure(f"{stem}.lst has no EndOfRom")
        self.eor = self.syms["EndOfRom"]
        secs = self.export["lines"]["sections"]
        self.image_rows = secs.get("(image)", {}).get("rows", [])
        self.sections = {}
        for name, s in secs.items():
            if name == "(image)":
                continue
            rows = s["rows"]
            if not rows:
                continue
            lo = min(r[1] for r in rows)
            hi = max(r[1] + r[2] for r in rows)
            vlo = min(r[0] for r in rows)
            self.sections[name] = dict(name=name, space=s["space"], rows=rows, lo=lo, hi=hi,
                                       bank=(vlo != lo))
        if not self.sections:
            raise CouldNotMeasure(f"{stem}.build.json has no sections")
        self.order = sorted(self.sections.values(), key=lambda s: (s["lo"], s["hi"]))


def read_symtab(p: Path) -> dict[str, int]:
    out, on = {}, False
    row = re.compile(r"^[ *](\S+) : ([0-9A-F]+) C \|$")
    for line in p.read_text(errors="replace").splitlines():
        if line.startswith("  Symbol Table"):
            on = True
            continue
        if on:
            if line.strip().endswith("symbols"):
                break
            m = row.match(line)
            if m:
                out[m.group(1)] = int(m.group(2), 16)
    if not out:
        raise CouldNotMeasure(f"{p}: empty symbol table")
    return out


def frozen_labels(sigil: Path, table: str) -> set[str]:
    p = sigil / "crates/sigil-harness/golden/offcanonical_sizes" / table
    if not p.is_file():
        raise CouldNotMeasure(f"frozen table {p} missing")
    out = set()
    for line in p.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        out.add(line.split()[0])
    if not out:
        raise CouldNotMeasure(f"frozen table {p} is empty")
    return out


# ------------------------------------------------------------- 68000 EA decoding
def ea_ext(mode, reg, size):
    """Bytes of extension an EA takes (size in bytes for #imm); None = invalid."""
    if mode in (0, 1, 2, 3, 4):
        return 0
    if mode in (5, 6):
        return 2
    return {0: 2, 1: 4, 2: 2, 3: 2, 4: (4 if size == 4 else 2)}.get(reg, None)


def decode_one(b: bytes, at: int = 0):
    """Decode the instruction at `b[at:]`: (length, [(mode, reg, ext_offset)], branch).

    branch is None, ("bcc", width) or ("db",). Offsets are relative to `at`. Raises
    ValueError on anything it does not know, so a row it cannot walk is reported, never
    skipped: the decoder only has to cover what the emitted code uses."""
    op = be16(b, at)
    hi = op >> 12
    eas = []
    off = 2
    sz2 = {0: 1, 1: 2, 2: 4}

    def add(mode, reg, size):
        nonlocal off
        n = ea_ext(mode, reg, size)
        if n is None:
            raise ValueError(f"bad EA in {op:04x}")
        eas.append((mode, reg, off))
        off += n

    def ea(size):
        add((op >> 3) & 7, op & 7, size)

    if hi in (1, 2, 3):  # MOVE / MOVEA
        size = {1: 1, 3: 2, 2: 4}[hi]
        ea(size)
        add((op >> 6) & 7, (op >> 9) & 7, size)
        return off, eas, None
    if hi == 0:
        if op & 0x0100:
            if (op >> 3) & 7 == 1:  # movep d16(An)
                return 4, [], None
            ea(1)  # dynamic bit op
            return off, eas, None
        sub = (op >> 9) & 7
        if sub == 4:  # static bit op: #imm word then EA
            off = 4
            ea(1)
            return off, eas, None
        s = (op >> 6) & 3
        if s == 3:
            raise ValueError(f"unknown {op:04x}")
        if (op & 0x3F) == 0x3C:  # ori/andi/eori to ccr/sr
            return 4, [], None
        off = 2 + (4 if s == 2 else 2)
        ea(sz2[s])
        return off, eas, None
    if hi == 4:
        if op in (0x4E70, 0x4E71, 0x4E73, 0x4E75, 0x4E76, 0x4E77) or (op & 0xFFF0) in (0x4E40, 0x4E60) \
                or (op & 0xFFF8) in (0x4E58, 0x4840, 0x4880, 0x48C0):
            return 2, [], None
        if op == 0x4E72 or (op & 0xFFF8) == 0x4E50:  # stop / link
            return 4, [], None
        if (op & 0xFB80) == 0x4880:  # movem
            off = 4
            ea(2)
            return off, eas, None
        if (op & 0xF1C0) == 0x41C0:  # lea
            ea(4)
            return off, eas, None
        if (op & 0xF1C0) == 0x4180:  # chk.w
            ea(2)
            return off, eas, None
        if (op & 0xFFC0) in (0x4E80, 0x4EC0, 0x4840, 0x4AC0, 0x4800):  # jsr jmp pea tas nbcd
            ea(4)
            return off, eas, None
        if (op & 0xFFC0) in (0x40C0, 0x44C0, 0x46C0):  # move sr / ccr
            ea(2)
            return off, eas, None
        if (op & 0xFF00) in (0x4000, 0x4200, 0x4400, 0x4600, 0x4A00):  # negx clr neg not tst
            s = (op >> 6) & 3
            if s == 3:
                raise ValueError(f"unknown {op:04x}")
            ea(sz2[s])
            return off, eas, None
        raise ValueError(f"unknown {op:04x}")
    if hi == 5:
        if (op & 0xC0) == 0xC0:
            if (op >> 3) & 7 == 1:  # DBcc
                return 4, [], ("db",)
            ea(1)  # Scc
            return off, eas, None
        ea(sz2[(op >> 6) & 3])
        return off, eas, None
    if hi == 6:
        d = op & 0xFF
        if d == 0:
            return 4, [], ("bcc", 2)
        if d == 0xFF:
            raise ValueError("68020 branch")
        return 2, [], ("bcc", 1)
    if hi == 7:
        return 2, [], None
    if hi in (8, 9, 0xB, 0xC, 0xD):
        opm = (op >> 6) & 7
        if hi in (8, 0xC) and opm in (3, 7):  # div / mul
            ea(2)
            return off, eas, None
        if hi in (9, 0xB, 0xD) and opm in (3, 7):  # suba / cmpa / adda
            ea(2 if opm == 3 else 4)
            return off, eas, None
        if hi in (8, 9, 0xC, 0xD) and opm >= 4 and ((op >> 3) & 7) in (0, 1):
            return 2, [], None  # sbcd/abcd/subx/addx/exg
        if hi == 0xB and opm >= 4 and ((op >> 3) & 7) == 1:
            return 2, [], None  # cmpm
        ea(sz2[opm & 3])
        return off, eas, None
    if hi == 0xE:
        if (op & 0xC0) == 0xC0:
            ea(2)
            return off, eas, None
        return 2, [], None
    raise ValueError(f"unknown {op:04x}")


def decode_row(b: bytes):
    """Walk every instruction of one code row: [(at, length, eas, branch)]. A row whose
    instructions do not tile it exactly raises ValueError."""
    out, at = [], 0
    while at < len(b):
        ln, eas, br = decode_one(b, at)
        out.append((at, ln, eas, br))
        at += ln
    if at != len(b):
        raise ValueError(f"instructions overrun the row ({at} != {len(b)})")
    return out


# --------------------------------------------------------------------- the proof
class Proof:
    def __init__(self, shape, old: Build, new: Build, tree: Path, sigil: Path, table: str):
        self.shape = shape
        self.old, self.new = old, new
        self.tree, self.sigil = tree, sigil
        self.table = table
        self.fails: list[str] = []
        self.notes: list[str] = []
        self.classes = Counter()
        self.sites: list[str] = []
        self.src_cache = {}

    def fail(self, msg):
        self.fails.append(msg)

    # -- helpers
    def src(self, b: Build, fi, ln):
        f = b.export["files"][fi] if 0 <= fi < len(b.export["files"]) else None
        if not f:
            return f"file#{fi}:{ln}"
        return f"{f['root']}/{f['path']}:{ln}"

    def sec_of(self, b: Build, addr, inclusive_end=False):
        los = [s["lo"] for s in b.order if not s["bank"]]
        secs = [s for s in b.order if not s["bank"]]
        i = bisect.bisect_right(los, addr) - 1
        while i >= 0:
            s = secs[i]
            if s["lo"] <= addr < s["hi"] or (inclusive_end and addr == s["hi"]):
                return s
            if s["hi"] <= addr:
                return None
            i -= 1
        return None

    def run(self):
        o, n = self.old, self.new
        # 1. same sections, same rows
        if set(o.sections) != set(n.sections):
            self.fail(f"section sets differ: {sorted(set(o.sections) ^ set(n.sections))}")
            return
        if any([r[3:] for r in s["rows"]] != [r[3:] for r in n.sections[nm]["rows"]]
               or s["space"] != n.sections[nm]["space"] for nm, s in o.sections.items()):
            bad = [nm for nm, s in o.sections.items()
                   if [r[3:] for r in s["rows"]] != [r[3:] for r in n.sections[nm]["rows"]]]
            self.fail(f"row sequences differ in section(s) {bad[:5]}")
            return
        # THE ROW MAP. Each row is emitted at its real base in both builds, so a row's
        # length differs only where a reference's target moved across a width boundary
        # (an EMITTED width flip, e.g. a `lea` whose target crossed below $8000). Inside a
        # section every row's offset must then move by exactly the bytes its predecessors
        # flipped: the section is contiguous code/data, nothing else may open or close.
        self.rows = []  # (old lma, old len, new lma, new len, section, row)
        self.flips = []
        for name, s in o.sections.items():
            t = n.sections[name]
            acc = 0
            for a, b in zip(s["rows"], t["rows"]):
                if (b[1] - t["lo"]) - (a[1] - s["lo"]) != acc:
                    self.fail(f"`{name}`: row {self.src(o, a[3], a[4])} moved {(b[1]-t['lo'])-(a[1]-s['lo'])} "
                              f"inside its section, but the rows before it flipped {acc}")
                    break
                if a[2] != b[2]:
                    if b[2] > a[2]:
                        self.fail(f"`{name}` row {self.src(o, a[3], a[4])} GREW {a[2]} -> {b[2]}")
                    self.flips.append((name, a, b))
                    acc += b[2] - a[2]
                self.rows.append((a[1], a[2], b[1], b[2], name, a))
        self.rows.sort()
        self.row_starts = [r[0] for r in self.rows]
        self.flips_emitted = len(self.flips)
        if self.fails:
            return
        self.shift = {name: n.sections[name]["lo"] - s["lo"] for name, s in o.sections.items()}
        # order preserved
        on = [s["name"] for s in o.order]
        nn = [s["name"] for s in n.order]
        if on != nn:
            self.fail("section order differs between old and new")
            return

        # 2. islands
        mp = self.tree / "games" / self.game() / "map.toml"
        try:
            anchors = tomllib.loads(mp.read_text()).get("anchor", [])
        except Exception as e:  # noqa: BLE001
            raise CouldNotMeasure(f"map.toml {mp}: {e}")
        sound_on = bool(o.export["shape"]["defines"].get("SOUND_DRIVER_ENABLED", 0))
        active = [a for a in anchors if a.get("when") in (None, "sound_on" if sound_on else "sound_off")]
        if not active:
            raise CouldNotMeasure("no active anchors")
        self.islands = set()
        for a in active:
            hit = [s for s in o.order if s["lo"] == a["at"] and s["hi"] > s["lo"]]
            if not hit:
                self.fail(f"anchor `{a['name']}` at {a['at']:#x}: no old section starts there")
                continue
            for s in hit:
                self.islands.add(s["name"])
                if self.shift[s["name"]] != 0:
                    self.fail(f"anchor `{a['name']}` section `{s['name']}` moved by {self.shift[s['name']]}")
        for s in o.order:
            if s["bank"]:
                self.islands.add(s["name"])
                if self.shift[s["name"]] != 0:
                    self.fail(f"phase bank `{s['name']}` moved by {self.shift[s['name']]}")
        self.notes.append(f"islands unmoved: {len(self.islands)} ({len(active)} anchors + phase banks)")

        # 3 + 4. walk deltas and named sites
        frozen = frozen_labels(self.sigil, self.table)
        labeled = set()
        rom_secs = [s for s in o.order if not s["bank"] and s["space"] == "m68k"]
        # A frozen label's address from the export's m68k records: a phased Z80 label's
        # listing value is its $8000-window VMA, which names no ROM section (and a phase
        # bank is never remeasured either way).
        m68k = {x["name"]: x["vma"] for x in o.export["symbols"]
                if x.get("space") == "m68k" and x["kind"] in ("label", "proc")}
        for lab in frozen:
            v = m68k.get(lab)
            if v is None:
                continue
            s = self.sec_of(o, v)
            if s is not None:
                labeled.add(s["name"])
        self.labeled = labeled
        self.dropped_total = 0
        self.sites_total = 0
        drop_by_sec = {}
        for i, s in enumerate(rom_secs[:-1]):
            nx = rom_secs[i + 1]
            t, tn = n.sections[s["name"]], n.sections[nx["name"]]
            og, ng = nx["lo"] - s["hi"], tn["lo"] - t["hi"]
            if og < 0 or ng < 0:
                self.fail(f"`{s['name']}` overlaps `{nx['name']}`")
                continue
            if nx["name"] in self.islands:
                continue  # the island absorbs the run's accumulated shift
            d = og - ng
            if d < 0:
                self.fail(f"gap after `{s['name']}` GREW {og} -> {ng}")
                continue
            if d:
                z = o.rom[s["hi"]:s["hi"] + d]
                if any(z):
                    self.fail(f"dropped bytes after `{s['name']}` at {s['hi']:#x} are not all zero: {z.hex()}")
                drop_by_sec[s["name"]] = (d, s["hi"])
                self.dropped_total += d
        # the site model
        sec_sites = {}
        for s in rom_secs:
            sites = self.scratch_sites(s, labeled)
            sec_sites[s["name"]] = sites
        for i, s in enumerate(rom_secs[:-1]):
            nx = rom_secs[i + 1]
            sites = sec_sites[s["name"]]
            total = sum(c for _, c in sites)
            d = drop_by_sec.get(s["name"], (0, 0))[0]
            if nx["name"] in self.islands:
                if total:
                    self.notes.append(f"`{s['name']}` (before island `{nx['name']}`) would have measured "
                                      f"+{total} at scratch; the island absorbs it")
                continue
            if total != d:
                self.fail(f"`{s['name']}` dropped {d} byte(s) but its scratch sites total {total}: "
                          + "; ".join(f"{w} (+{c})" for w, c in sites))
            elif d:
                self.sites.append(f"`{s['name']}` -{d} at {drop_by_sec[s['name']][1]:#x}: "
                                  + "; ".join(f"{w} (+{c})" for w, c in sites))
                self.sites_total += total
        if self.dropped_total != self.sites_total and not self.fails:
            self.fail(f"dropped {self.dropped_total} != named sites {self.sites_total}")

        # 5. symbols
        self.check_symbols()
        # 6. bytes
        self.check_bytes()
        # 7. appendix
        self.check_appendix()

    def game(self):
        return self.old.export["shape"]["game"]

    # -- the scratch-width model -------------------------------------------------
    def scratch_sites(self, s, labeled):
        """[(`file:line` description, extra bytes the OLD walk measured)] for one section.

        The OLD walk's remeasure round placed every frozen-labeled ("pinned") section at
        its packed base, which is its old-ROM base, and every other ROM section at its own
        far-scratch slot (0x700000+, abs.l-only by `far_scratch_slot`'s construction). So:
          * a reference to a label in a never-pinned section (this one included, when this
            one is never-pinned) is abs.l-only;
          * a branch or PC-relative form whose target is in another section reaches it
            exactly as emitted when both sections sit at their old bases (both pinned);
            otherwise the distance is far and it takes the absolute rung (`jsr`/`jmp`
            abs.w when the target address can be abs.w, else abs.l).
        The emitted ROM is the other side of the comparison: every row is emitted at its
        real base, so the difference per site is (measured form) - (emitted row length)."""
        o = self.old
        sites = []
        shorts = []
        pinned = s["name"] in labeled

        def far(target):
            """Is `target` (old address) abs.l-only while `s` is measured?"""
            ts = self.sec_of(o, target, inclusive_end=True)
            if ts is None:
                return not abs_w_ok(target)
            if ts["name"] not in labeled:
                return True
            return not abs_w_ok(target)

        def reach_changes(target):
            """Does a PC-relative reach to `target` differ from the emitted one?"""
            ts = self.sec_of(o, target, inclusive_end=True)
            if ts is not None and ts["name"] == s["name"]:
                return False
            if pinned and ts is not None and ts["name"] in labeled:
                return False
            return True

        for r in s["rows"]:
            if r[5] != "code":
                continue
            a, ln = r[1], r[2]
            b = o.rom[a:a + ln]
            where = self.src(o, r[3], r[4])
            try:
                insns = decode_row(b)
            except (ValueError, IndexError) as e:
                self.fail(f"cannot decode the code row at {a:#x} ({where}): {e}; {b.hex()}")
                continue
            text = self.src_text(o, r[3], r[4]) if len(insns) == 1 else ""
            explicit_pc = "(pc)" in text.lower()
            explicit_size = re.search(r"\b(j?b\w+|bra|bsr)\.[swl]\b", text.lower()) is not None
            extra = 0
            what = []
            for at, ilen, eas, br in insns:
                for mode, reg, eo in eas:
                    eo += at
                    if mode != 7:
                        continue
                    if reg == 0:  # abs.w
                        v = be16(b, eo)
                        if v < 0x8000 and far(v):
                            extra += 2
                            what.append(f"abs.w ${v:X} -> abs.l")
                    elif reg == 2 and not explicit_pc:  # (d16,PC) the relaxer chose
                        t = a + eo + sext16(be16(b, eo))
                        if reach_changes(t) and far(t):
                            extra += 2
                            what.append(f"(d16,pc) ${t:X} -> abs.l")
                if br and br[0] == "bcc" and not explicit_size:
                    cond = b[at] & 0x0F
                    t = a + at + 2 + (sext16(be16(b, at + 2)) if br[1] == 2 else sext8(b[at + 1]))
                    if reach_changes(t):
                        if cond in (0, 1):  # bra / bsr: the jmp / jsr rung
                            need = 6 if far(t) else 4
                            if need > ilen:
                                extra += need - ilen
                                what.append(f"{'bsr' if cond else 'bra'}.{'w' if br[1] == 2 else 's'} "
                                            f"${t:X} -> {'jsr' if cond else 'jmp'} "
                                            f"abs.{'l' if need == 6 else 'w'}")
                        else:
                            what.append(f"Bcc ${t:X} leaves the section (unmodelled)")
                            extra += 10**6  # poison: forces a loud site-sum mismatch
            if extra:
                sites.append([a + insns[0][0], extra, f"{where} {', '.join(what)}"])
            for at, ilen, eas, br in insns:
                if br and br[0] == "bcc" and br[1] == 1 and not explicit_size:
                    t = a + at + 2 + sext8(b[at + 1])
                    if not reach_changes(t):
                        shorts.append((a + at, t, where))
        # THE CASCADE: a short (8-bit) branch inside the section whose span grew past
        # [-128, 127] while measured takes its word form (+2), which can push another.
        grown = set()
        while True:
            changed = False
            for (pa, t, where) in shorts:
                if pa in grown:
                    continue
                lo, hi = (pa + 2, t) if t >= pa + 2 else (t, pa + 2)
                ins = sum(e for (sa, e, _w) in sites if lo <= sa < hi and sa != pa)
                d = (t - (pa + 2)) + (ins if t >= pa + 2 else -ins)
                if not -128 <= d <= 127:
                    grown.add(pa)
                    sites.append([pa, 2, f"{where} short branch ${t:X} spans grown sites -> .w"])
                    changed = True
            if not changed:
                break
        return [(w, e) for (_a, e, w) in sorted(sites)]

    def src_text(self, b: Build, fi, ln):
        f = b.export["files"][fi] if 0 <= fi < len(b.export["files"]) else None
        if not f:
            return ""
        root = {"engine": "engine", "game": "games/" + self.game(), "generated": None}.get(f["root"])
        if root is None:
            return ""
        p = self.tree / root / f["path"]
        if p not in self.src_cache:
            try:
                self.src_cache[p] = p.read_text(errors="replace").splitlines()
            except OSError:
                self.src_cache[p] = []
        lines = self.src_cache[p]
        return lines[ln - 1].split("//")[0] if 0 < ln <= len(lines) else ""

    # -- symbols -----------------------------------------------------------------
    def check_symbols(self):
        o, n = self.old, self.new
        if set(o.syms) != set(n.syms):
            self.fail(f"listing symbol sets differ: {sorted(set(o.syms) ^ set(n.syms))[:10]}")
            return
        # Placement comes from the export's symbol records, which carry their address
        # space (a phased Z80 label's listing value is a $8000-window address, not ROM).
        def placed(b):
            return {x["name"]: x["vma"] for x in b.export["symbols"]
                    if x.get("space") == "m68k" and x["kind"] in ("label", "proc")}
        op, np_ = placed(o), placed(n)
        if set(op) != set(np_):
            self.fail(f"export m68k label sets differ: {sorted(set(op) ^ set(np_))[:10]}")
            return
        zo = {x["name"]: x["vma"] for x in o.export["symbols"] if x.get("space") == "z80"}
        zn = {x["name"]: x["vma"] for x in n.export["symbols"] if x.get("space") == "z80"}
        if zo != zn:
            self.fail("a Z80-space symbol moved")
        rom = []
        for name, v in op.items():
            if v >= o.eor or name == "EndOfRom":
                continue
            s = self.sec_of(o, v, inclusive_end=True)
            if s is None:
                if np_[name] != v:
                    self.fail(f"symbol `{name}` {v:#x} outside any section moved to {np_[name]:#x}")
                continue
            want = self.R(v)
            if np_[name] != want:
                self.fail(f"symbol `{name}` {v:#x} -> {np_[name]:#x}, the row map says "
                          f"{want if want is None else hex(want)}")
                continue
            rom.append((v, np_[name] - v, name, s["name"]))
        rom.sort()
        prev = 0
        neg = 0
        for v, d, name, sec in rom:
            if d > 0:
                self.fail(f"symbol `{name}` moved UP by {d}")
            if sec in self.islands and self.sec_of(o, v) is not None and self.sec_of(o, v)["name"] == sec:
                prev = d
            elif d > prev:
                self.fail(f"symbol `{name}` at {v:#x}: shift {d} after {prev} (not monotone within its run)")
            prev = d
            neg += d < 0
        self.notes.append(f"symbols: {len(o.syms)} listing names identical; {len(rom)} m68k ROM labels, "
                          f"{neg} shifted (each exactly as the row map says, all <= 0, monotone per run); "
                          f"{len(zo)} Z80-space symbols unmoved")
        self.sym_count = len(o.syms)

    # -- bytes -------------------------------------------------------------------
    def R(self, x):
        """The relocation: old ROM address -> new, through the row map; None where the
        address names nothing (inside a flipped row other than its start)."""
        i = bisect.bisect_right(self.row_starts, x) - 1
        if i >= 0:
            ol, olen, nl, nlen, sec, _r = self.rows[i]
            if x < ol + olen:
                if olen == nlen or x == ol:
                    return nl + (x - ol)
                return None
        # one past a section's end (an `_End` marker / EndOfRom)
        for nm, s in self.old.sections.items():
            if not s["bank"] and x == s["hi"]:
                return self.new.sections[nm]["hi"]
        if x == self.old.eor:
            return self.new.eor
        return None

    def explain_row(self, ob, nb, old_at, new_at, z80):
        """Mark the differing bytes of one row explained by pointer fields. Returns the set
        of explained offsets."""
        diff = {i for i in range(len(nb)) if ob[i] != nb[i]}
        got = set()
        n = len(nb)
        for i in range(n):
            # 32-bit absolute (68k), BE
            if i + 4 <= n and not z80:
                ov, nv = be32(ob, i), be32(nb, i)
                if ov != nv:
                    r = self.R(ov & 0xFFFFFF)
                    if r is not None and nv == ((ov & 0xFF000000) | r):
                        got |= set(range(i, i + 4))
            if i + 2 <= n and not z80:
                ov, nv = be16(ob, i), be16(nb, i)
                if ov != nv:
                    # a 16-bit ROM address: abs.w (base 0), an offset from an island
                    # base (`#Label - ObjCodeBase`), or the low word of a symbol in a
                    # 64 KB bank (`Map_TestObj & $FFFF`); the base never moves, so
                    # the field moves by its target's shift, and the target must be an
                    # exact old symbol address unless the base is 0.
                    for base in self.bases16:
                        t = ov + base
                        if base and t not in self.old_sym_addrs:
                            continue
                        if base == 0 and ov >= 0x8000:
                            continue
                        r = self.R(t)
                        if r is not None and (r - base) & 0xFFFF == nv:
                            got |= {i, i + 1}
                            break
                    # pc-relative word: base = the field's own address
                    t = old_at + i + sext16(ov)
                    r = self.R(t)
                    if r is not None and (r - (new_at + i)) & 0xFFFF == nv:
                        got |= {i, i + 1}
            # short branch / brief-extension byte: base = field address + 1 (Bcc.s: opcode+2)
            if i >= 1 and not z80:
                ov, nv = ob[i], nb[i]
                if ov != nv:
                    for base_off in (1, -1):
                        t = old_at + i + base_off + sext8(ov)
                        r = self.R(t)
                        if r is not None and (r - (new_at + i + base_off)) & 0xFF == nv:
                            got.add(i)
        return diff, got

    def explain_flip(self, ob, nb, old_at, new_at, z80):
        """An emitted width flip, checked rather than assumed: the rows decode to the same
        instruction sequence, and each differing instruction is the same operation with
        one absolute operand re-encoded abs.l -> abs.w (opcode EA register 1 -> 0) whose
        new value is the relocation of its old target, which now lies in abs.w reach.
        Everything else must match byte for byte. None = not explained."""
        if z80:
            return None
        try:
            oi, ni = decode_row(ob), decode_row(nb)
        except (ValueError, IndexError):
            return None
        if len(oi) != len(ni):
            return None
        why = []
        for (oa, ol, oeas, obr), (na, nl, neas, nbr) in zip(oi, ni):
            x, y = ob[oa:oa + ol], nb[na:na + nl]
            if x == y:
                continue
            if len(oeas) != len(neas) or obr or nbr:
                return None
            xo = be16(x, 0)
            yo = be16(y, 0)
            for (m1, r1, e1), (m2, r2, e2) in zip(oeas, neas):
                if (m1, r1) == (m2, r2):
                    continue
                if not (m1 == 7 and r1 == 1 and m2 == 7 and r2 == 0):
                    return None
                tgt = be32(x, e1) & 0xFFFFFF
                r = self.R(tgt)
                if r is None or not abs_w_ok(r) or (sext16(be16(y, e2)) & 0xFFFFFF) != r:
                    return None
                why.append(f"abs.l ${tgt:X} -> abs.w ${r:X}")
            # the opcode words differ only in the flipped EA register field(s)
            diffbits = xo ^ yo
            if diffbits & ~0x0E07:
                return None
            # every extension word outside the flipped operand(s) is identical: compare
            # the instruction with the flipped operands cut out
            def strip(b_, eas_, flipped_reg):
                out, last = bytearray(), 2
                for (m, r_, e) in eas_:
                    if m == 7 and r_ == flipped_reg:
                        out += b_[last:e]
                        last = e + (4 if flipped_reg == 1 else 2)
                return bytes(out + b_[last:])
            if strip(x, oeas, 1) != strip(y, neas, 0):
                return None
        return "; ".join(why) if why else None

    def check_bytes(self):
        o, n = self.old, self.new
        self.flip_sites = []
        self.old_sym_addrs = {x["vma"] for x in o.export["symbols"]
                              if x.get("space") == "m68k" and x["kind"] in ("label", "proc")}
        isl = {o.sections[nm]["lo"] for nm in self.islands if not o.sections[nm]["bank"]}
        self.bases16 = [0] + sorted(isl - {0}) + [k << 16 for k in range(1, (o.eor >> 16) + 1)
                                                   if (k << 16) not in isl]
        cls = self.classes
        unexplained = []
        # content at mapped addresses, per row
        pos_owner = {}  # new address -> class for differing positions
        for name, s in o.sections.items():
            t = n.sections[name]
            z80 = s["space"] != "m68k"
            for a, b in zip(s["rows"], t["rows"]):
                ob = o.rom[a[1]:a[1] + a[2]]
                nb = n.rom[b[1]:b[1] + b[2]]
                if len(ob) != len(nb):
                    why = self.explain_flip(ob, nb, a[1], b[1], z80)
                    if why is None:
                        for k in range(len(nb)):
                            unexplained.append((b[1] + k, a[1] + k, name, self.src(o, a[3], a[4]),
                                                ob[k] if k < len(ob) else 0, nb[k]))
                    else:
                        self.flip_sites.append(f"`{name}` {self.src(o, a[3], a[4])}: {why}")
                        for k in range(len(nb)):
                            pos_owner[b[1] + k] = "width flip"
                    continue
                if ob == nb:
                    continue
                diff, got = self.explain_row(ob, nb, a[1], b[1], z80)
                for k in diff:
                    pos = b[1] + k
                    if pos in CHECKSUM or pos in ROM_END:
                        pos_owner[pos] = "header"
                    elif k in got:
                        pos_owner[pos] = "pointer"
                    else:
                        unexplained.append((pos, a[1] + k, name, self.src(o, a[3], a[4]), ob[k], nb[k]))
        # image fills of the new build must be zero
        for r in n.image_rows:
            if r[5] == "fill" and any(n.rom[r[1]:r[1] + r[2]]):
                self.fail(f"new fill at {r[1]:#x} is not zero")
        # header fields verified, not assumed
        self.check_header(n)
        # classify every positional difference in the image
        lim = min(o.eor, n.eor)
        newpos_cls = {}
        for ol, olen, nl, nlen, _sec, _r in self.rows:
            for k in range(nlen):
                newpos_cls[nl + k] = "shift" if nl != ol else "same"
        for r in n.image_rows:
            if r[5] == "fill":
                for k in range(r[2]):
                    newpos_cls[r[1] + k] = "padding"
        positional = 0
        for i in range(lim):
            if o.rom[i] == n.rom[i]:
                continue
            positional += 1
            if i in CHECKSUM or i in ROM_END:
                cls["header"] += 1
                continue
            c = pos_owner.get(i)
            if c is None:
                c = newpos_cls.get(i)
                if c == "same":
                    c = None  # an in-place row byte differing must have been explained above
                if c == "padding" and n.rom[i] != 0:
                    c = None
            if c is None:
                unexplained.append((i, None, "?", "positional", o.rom[i], n.rom[i]))
                continue
            cls[c] += 1
        # beyond the shorter image: the longer one's tail
        if o.eor > n.eor:
            # old image bytes past the new EndOfRom: the new build's appendix starts here
            cls["rom end (old image tail vs new appendix)"] += sum(
                1 for i in range(n.eor, min(o.eor, len(n.rom))) if o.rom[i] != n.rom[i])
        tail = min(len(o.rom), len(n.rom))
        start = max(o.eor, n.eor)
        cls["deb2 appendix"] += sum(1 for i in range(start, tail) if o.rom[i] != n.rom[i])
        self.file_delta = len(n.rom) - len(o.rom)
        for k in [k for k, v in cls.items() if v == 0]:
            del cls[k]
        self.positional = positional
        for u in unexplained[:20]:
            pos, opos, name, where, ob, nb = u
            self.fail(f"UNEXPLAINED byte at new {pos:#x} (old {opos if opos is None else hex(opos)}) "
                      f"in `{name}` {where}: {ob:02x} -> {nb:02x}")
        self.unexplained = len(unexplained)
        # mapped-content totals
        cls_pointer_rows = sum(1 for v in pos_owner.values() if v == "pointer")
        self.notes.append(f"mapped content: {cls_pointer_rows} byte(s) re-aimed pointers, "
                          f"{self.flips_emitted} emitted width flip(s), "
                          f"{sum(a[2] - b[2] for _n, a, b in self.flips)} byte(s) saved by them")
        for f in self.flip_sites:
            self.notes.append(f"emitted width flip {f}")

    def check_header(self, b: Build):
        rom = b.rom
        s = 0
        for i in range(0x200, len(rom) - 1, 2):
            s = (s + be16(rom, i)) & 0xFFFF
        if len(rom) % 2:
            s = (s + (rom[-1] << 8)) & 0xFFFF
        if be16(rom, 0x18E) != s:
            self.notes.append(f"header checksum {be16(rom, 0x18E):#06x} vs fold {s:#06x} (fold model differs; "
                              "checked against old instead)")
            # the same fold model on the old ROM tells whether the model or the ROM is off
            os_ = 0
            for i in range(0x200, len(self.old.rom) - 1, 2):
                os_ = (os_ + be16(self.old.rom, i)) & 0xFFFF
            if be16(self.old.rom, 0x18E) == os_:
                self.fail("new header checksum does not fold, but old does")
        if be32(rom, 0x1A4) != len(rom) - 1 and be32(rom, 0x1A4) != b.eor - 1:
            self.fail(f"ROM-end field {be32(rom, 0x1A4):#x} names neither the file end nor EndOfRom")

    # -- deb2 appendix -----------------------------------------------------------
    def check_appendix(self):
        o, n = self.old, self.new
        has_o = len(o.rom) > o.eor
        has_n = len(n.rom) > n.eor
        if has_o != has_n:
            self.fail(f"appendix presence differs: old {has_o} new {has_n}")
            return
        if not has_o:
            self.notes.append("no deb2 appendix in this shape (lean)")
            self.appendix = "none"
            return
        probe = self.tree / "tools" / "deb2_probe.py"
        if not probe.is_file():
            raise CouldNotMeasure(f"{probe} missing")
        spec = importlib.util.spec_from_file_location("deb2_probe", probe)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        oa, na = o.rom[o.eor:], n.rom[n.eor:]
        if oa[:2] != b"\xde\xb2" or na[:2] != b"\xde\xb2":
            self.fail("deb2 magic not at EndOfRom")
            return
        od, nd = mod.decode_names(oa), mod.decode_names(na)
        # every decoded new record is a listing symbol at that address
        nby = defaultdict(set)
        for name, v, _u in mod.demangle([(k, v, False) for k, v in n.syms.items()]):
            nby[v & 0xFFFFFF].add(name)
        bad = [(a, nm) for a, nm in nd.items() if nm not in nby.get(a, set())]
        if bad:
            self.fail(f"new appendix records with no matching listing symbol: {bad[:5]}")
        oby = defaultdict(set)
        for name, v, _u in mod.demangle([(k, v, False) for k, v in o.syms.items()]):
            oby[v & 0xFFFFFF].add(name)
        # names stored: differences must come from shared-address groups changing
        os_, ns_ = set(od.values()), set(nd.values())
        only = os_ ^ ns_
        grp_o = {nm: len(oby[v]) for v, nms in oby.items() for nm in nms}
        grp_n = {nm: len(nby[v]) for v, nms in nby.items() for nm in nms}
        unexplained = [nm for nm in only if grp_o.get(nm, 1) == 1 and grp_n.get(nm, 1) == 1]
        if unexplained:
            self.fail(f"appendix names differ without a shared-address change: {sorted(unexplained)[:10]}")
        self.appendix = (f"old {len(oa)} B / {len(od)} records, new {len(na)} B / {len(nd)} records; "
                         f"distinct listing addresses old {len(oby)} new {len(nby)}; "
                         f"{len(only)} name(s) differ, all in changed shared-address groups; "
                         f"every new record matches the new listing")


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--shape", required=True)
    ap.add_argument("--old", required=True, help="stem: <stem>.bin/.lst/.build.json")
    ap.add_argument("--new", required=True)
    ap.add_argument("--tree", required=True, help="the aeon tree both were built from")
    ap.add_argument("--sigil", required=True, help="the sigil tree (frozen tables)")
    ap.add_argument("--table", required=True, help="the frozen size table the OLD walk read")
    ap.add_argument("--json", help="write the result record here")
    a = ap.parse_args(argv)
    try:
        p = Proof(a.shape, Build(Path(a.old)), Build(Path(a.new)), Path(a.tree), Path(a.sigil), a.table)
        p.run()
    except CouldNotMeasure as e:
        print(f"PROOF {a.shape} COULD-NOT-MEASURE {e}")
        return 2
    o, n = p.old, p.new
    print(f"== {a.shape}: old EndOfRom {o.eor:#x} file {len(o.rom)} | new EndOfRom {n.eor:#x} file {len(n.rom)}")
    for s in p.sites:
        print(f"  site {s}")
    for s in p.notes:
        print(f"  note {s}")
    print(f"  appendix {getattr(p, 'appendix', '?')}")
    print(f"  classes {dict(p.classes)}")
    for f in p.fails:
        print(f"  FAIL {f}")
    unexp = getattr(p, "unexplained", None)
    verdict = "EXPLAINED" if not p.fails else "UNEXPLAINED"
    rec = dict(shape=a.shape, old_eor=o.eor, new_eor=n.eor, old_len=len(o.rom), new_len=len(n.rom),
               dropped=getattr(p, "dropped_total", None), sites=getattr(p, "sites_total", None),
               positional=getattr(p, "positional", None), classes=dict(p.classes),
               file_delta=getattr(p, "file_delta", None), emitted_flips=getattr(p, "flips_emitted", None),
               unexplained=unexp, fails=len(p.fails), verdict=verdict,
               unlabeled_sections=sorted(set(s for s in o.sections) - getattr(p, "labeled", set())))
    if a.json:
        Path(a.json).write_text(json.dumps(rec, indent=1))
    print(f"PROOF {a.shape} {verdict} dropped={rec['dropped']} sites={rec['sites']} "
          f"positional={rec['positional']} unexplained={unexp} fails={len(p.fails)}")
    return 0 if verdict == "EXPLAINED" else 1


if __name__ == "__main__":
    sys.exit(main())
