#!/usr/bin/env python3
"""Stub the S1 corpus past every frontend class sigil refuses, so the run REACHES LINK.

The ROM this produces is deliberately WRONG in the places stubbed (charset text,
FM/PSG frequency tables, Z80 bank arithmetic). Its only purpose is to answer
"what does sigil's linker say about a whole Sonic 1 program", which no
frontend-only run can reach.

Every edit is recorded so the diff is auditable.
"""
import re
import sys
import os

ROOT = "/home/volence/sonic_hacks/.s1recon-corpus"
LOG = []


def read(p):
    with open(os.path.join(ROOT, p), encoding="utf-8", errors="surrogateescape") as f:
        return f.read()


def write(p, s):
    with open(os.path.join(ROOT, p), "w", encoding="utf-8", errors="surrogateescape") as f:
        f.write(s)


def sub(path, old, new, count=0, tag=""):
    s = read(path)
    n = s.count(old)
    if n == 0:
        LOG.append(f"MISS  {path}: {tag or old[:40]!r}")
        return
    s = s.replace(old, new) if count == 0 else s.replace(old, new, count)
    write(path, s)
    LOG.append(f"ok    {path}: {tag or old[:40]!r} x{n}")


# --- C12: page / listing directives -----------------------------------------
sub("MacroSetup.asm", "\tlisting purecode", ";STUB\tlisting purecode", tag="listing purecode")
sub("MacroSetup.asm", "\tpage\t0", ";STUB\tpage\t0", tag="page 0")

# --- C3: dc.ATTRIBUTE [count]value ------------------------------------------
sub("MacroSetup.asm",
    "dcb macro count,value\n\tdc.ATTRIBUTE\t[count]value\n",
    "dcb macro count,value\n\trept count\n\tdc.ATTRIBUTE\tvalue\n\tendr\n",
    tag="dcb [count]value")

# --- C2: macro default argument value ---------------------------------------
sub("Macros.asm",
    "locVRAM:\tmacro loc,controlport=(vdp_control_port).l\n"
    "\t\tmove.l\t#($40000000+(((loc)&$3FFF)<<16)+(((loc)&$C000)>>14)),controlport\n",
    "locVRAM:\tmacro loc,controlport\n"
    "\tif \"controlport\"<>\"\"\n"
    "\t\tmove.l\t#($40000000+(((loc)&$3FFF)<<16)+(((loc)&$C000)>>14)),controlport\n"
    "\telse\n"
    "\t\tmove.l\t#($40000000+(((loc)&$3FFF)<<16)+(((loc)&$C000)>>14)),(vdp_control_port).l\n"
    "\tendif\n",
    tag="locVRAM default arg")

# --- C4: abs() in a rept count ----------------------------------------------
sub("Macros.asm",
    "\trept 1+(abs(first-last)/abs(step))",
    "\trept 1+(zqabs(first-last)/zqabs(step))",
    tag="range abs()")
sub("Macros.asm",
    "range: macro first,last,step,repeat",
    "zqabs function zqx,((zqx)<0)*(0-(zqx))+((zqx)>=0)*(zqx)\nrange: macro first,last,step,repeat",
    tag="zqabs function")

# --- C9: zonewarning (warning directive + macro-local arithmetic) -----------
s = read("Macros.asm")
s = s.replace(
    "zonewarning:\tmacro loc,elementsize\n"
    "    if (MOMPASS=1)\n"
    "._end:\n"
    "\tif (._end-loc)-(ZoneCount*elementsize)<>0\n"
    "\t\twarning \"Size of loc (\\{(._end-loc)/elementsize}) does not match ZoneCount (\\{ZoneCount}).\"\n"
    "\tendif\n"
    "    endif\n"
    "\t\tendm\n",
    "zonewarning:\tmacro loc,elementsize\n\t\tendm\n")
write("Macros.asm", s)
LOG.append("ok    Macros.asm: zonewarning body emptied")

# --- C1: float literals / INT() in the frequency tables ---------------------
sub("s1.sounddriver.asm",
    "\t\t\tdc.w MakeFMFrequency(op)+octave*$800",
    "\t\t\tdc.w 0\t;STUB float",
    tag="MakeFMFrequency")
sub("s1.sounddriver.asm",
    "\t\t\tdc.w MakePSGFrequency(op)",
    "\t\t\tdc.w 0\t;STUB float",
    tag="MakePSGFrequency")

# --- C6: charset ------------------------------------------------------------
s = read("sonic.asm")
s2 = re.sub(r"(?m)^(\s*charset\b.*)$", r";STUB\1", s)
write("sonic.asm", s2)
LOG.append(f"ok    sonic.asm: charset lines commented x{len(re.findall(r'(?m)^\s*charset\b', s))}")

# --- C11: multi-character literals as immediates ----------------------------
SBZ = "_incObj/82, 83 SBZ Eggman Cutscene and Crumbling Floor.asm"
s = read(SBZ)
mc = set(re.findall(r'#"(..)"', s))
for lit in mc:
    val = (ord(lit[0]) << 8) | ord(lit[1])
    s = s.replace(f'#"{lit}"', f"#${val:04X}")
write(SBZ, s)
LOG.append(f"ok    {SBZ}: multichar immediates {sorted(mc)}")

# --- C8: z80.asm ------------------------------------------------------------
s = read("sound/z80.asm")
s = s.replace("\tlisting purecode", ";STUB\tlisting purecode")
s = s.replace("zmake68kBank(SegaPCM)&1", "0")
s = s.replace("zmake68kBank(SegaPCM)>>1", "0")
s = s.replace("zmake68kPtr(SegaPCM)", "0")
s = s.replace("pcmLoopCounter(16000)", "0")
write("sound/z80.asm", s)
LOG.append("ok    sound/z80.asm: listing purecode + 4 function-call operands stubbed")

# --- C5/C7: enum / nextenum / enumconf, and switch/case on an integer -------
SMPS = "sound/_smps2asm_inc.asm"
lines = read(SMPS).split("\n")
out = []
step = 1
counter = None          # symbolic expression string for the NEXT value
in_switch = 0


def expand_enum(items, indent):
    """AS ENUM/NEXTENUM: each item is NAME or NAME=EXPR; the counter advances by
    ENUMCONF's step after each item, and an explicit value re-seeds it."""
    global counter
    res = []
    for it in items:
        it = it.strip()
        if not it:
            continue
        if "=" in it:
            name, expr = it.split("=", 1)
            name, expr = name.strip(), expr.strip()
            res.append(f"{indent}{name} = {expr}")
        else:
            name = it
            res.append(f"{indent}{name} = {counter}")
        counter = f"({name})+{step}"
    return res


for ln in lines:
    body = ln.split(";")[0]
    m = re.match(r"^(\s*)enumconf\s+(.*)$", body)
    if m:
        step = m.group(2).strip()
        out.append(";STUB " + ln)
        continue
    m = re.match(r"^(\s*)enum\s+(.*)$", body)
    if m:
        counter = None
        out.extend(expand_enum(m.group(2).split(","), m.group(1)))
        continue
    m = re.match(r"^(\s*)nextenum\s+(.*)$", body)
    if m:
        out.extend(expand_enum(m.group(2).split(","), m.group(1)))
        continue
    m = re.match(r"^(\s*)switch\s+(.*)$", body)
    if m:
        in_switch += 1
        out.append(f"{m.group(1)};STUB switch")
        out.append(f"{m.group(1)}__zqsw{in_switch} = {m.group(2).strip()}")
        out.append(f"{m.group(1)}__zqfirst{in_switch} = 1")
        continue
    m = re.match(r"^(\s*)case\s+(.*)$", body)
    if m:
        kw = "if" if True else "elseif"
        out.append(f"{m.group(1)}{'if' if False else ''}")
        out[-1] = (f"{m.group(1)}if __zqsw{in_switch}==({m.group(2).strip()})"
                   if False else f"{m.group(1)}__ZQCASE{in_switch}:{m.group(2).strip()}")
        continue
    m = re.match(r"^(\s*)elsecase\b", body)
    if m:
        out.append(f"{m.group(1)}__ZQELSECASE{in_switch}:")
        continue
    m = re.match(r"^(\s*)endcase\b", body)
    if m:
        out.append(f"{m.group(1)}__ZQENDCASE{in_switch}:")
        in_switch -= 1
        continue
    out.append(ln)

# Second pass over the marker lines: turn the case chain into if/elseif/endif.
final = []
for ln in out:
    m = re.match(r"^(\s*)__ZQCASE(\d+):(.*)$", ln)
    if m:
        final.append(f"{m.group(1)}ZQMARK_CASE {m.group(2)} {m.group(3)}")
        continue
    final.append(ln)

# Now rewrite: the FIRST case in a switch becomes `if`, the rest `elseif`.
seen = {}
final2 = []
for ln in final:
    m = re.match(r"^(\s*)ZQMARK_CASE (\d+) (.*)$", ln)
    if m:
        ind, sw, val = m.group(1), m.group(2), m.group(3).strip()
        kw = "if" if sw not in seen else "elseif"
        seen[sw] = True
        final2.append(f"{ind}{kw} __zqsw{sw}==({val})")
        continue
    m = re.match(r"^(\s*)__ZQELSECASE(\d+):$", ln)
    if m:
        final2.append(f"{m.group(1)}else")
        continue
    m = re.match(r"^(\s*)__ZQENDCASE(\d+):$", ln)
    if m:
        final2.append(f"{m.group(1)}endif")
        seen.pop(m.group(2), None)
        continue
    final2.append(ln)

write(SMPS, "\n".join(final2))
LOG.append(f"ok    {SMPS}: enum/nextenum expanded, switch/case -> if/elseif")

print("\n".join(LOG))
