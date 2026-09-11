#!/usr/bin/env python3
"""Generate one probe per Z80 half-register form, run the pinned asl on each
(through run_asl.sh, i.e. asl_ref.sh's asl_run), and tabulate accept/refuse and
the listed bytes of the probe line.

usage: gen_probes.py <outdir> <table-name>
Every probe is

    <cpu line>
    <pre lines>
    <form>          <- the line whose bytes are read
    nop
    end

Values are read only from ASL_EXIT=0 runs with a complete footer.
"""
import os
import re
import subprocess
import sys

S = "/home/volence/sonic_hacks/.scratch/z80-half-registers"
HALVES = ["ixl", "ixu", "iyl", "iyu"]
R8 = ["a", "b", "c", "d", "e", "h", "l"]
ALU = ["add", "adc", "sub", "sbc", "and", "xor", "or", "cp"]
CB = ["rlc", "rrc", "rl", "rr", "sla", "sra", "srl", "sll"]


def probes():
    """(name, cpu, pre, form)"""
    out = []
    und = "z80undoc"
    # spellings, as a source of ld a,<X>
    for sp in ["ixl", "ixu", "ixh", "iyl", "iyu", "iyh",
               "IXL", "IXU", "IXH", "IYL", "IYU", "IYH", "IxL", "iXu", "Iyh",
               "lx", "hx", "xl", "xh", "ly", "hy", "yl", "yh", "ixlo", "ixhi",
               "ix.l", "ix.h"]:
        tag = sp.replace(".", "dot")
        out.append((f"sp_{tag}", und, [], f"ld a,{sp}"))
    # the high-half alias as a destination too
    for sp in ["ixh", "iyh", "IXH"]:
        out.append((f"spd_{sp}", und, [], f"ld {sp},a"))
    # the documented cpu: are the halves registers, symbols or refused?
    for sp in ["ixl", "ixu", "iyl", "iyu", "ixh"]:
        out.append((f"doc_{sp}", "z80", [], f"ld a,{sp}"))
        out.append((f"docsym_{sp}", "z80", [f"{sp} equ 5"], f"ld a,{sp}"))
        out.append((f"undsym_{sp}", und, [f"{sp} equ 5"], f"ld a,{sp}"))
    # ld r,H and ld H,r
    for h in HALVES:
        for r in R8:
            out.append((f"ld_{r}_{h}", und, [], f"ld {r},{h}"))
            out.append((f"ld_{h}_{r}", und, [], f"ld {h},{r}"))
        for h2 in HALVES:
            out.append((f"ld_{h}_{h2}", und, [], f"ld {h},{h2}"))
        out.append((f"ldi_{h}", und, [], f"ld {h},5"))
        out.append((f"ldineg_{h}", und, [], f"ld {h},-1"))
        out.append((f"ldibig_{h}", und, [], f"ld {h},256"))
        out.append((f"ldisym_{h}", und, ["val equ 7Fh"], f"ld {h},val"))
        out.append((f"ldifwd_{h}", und, [], f"ld {h},fwd\nfwd equ 12h"))
        out.append((f"ld_{h}_indhl", und, [], f"ld {h},(hl)"))
        out.append((f"ld_indhl_{h}", und, [], f"ld (hl),{h}"))
        out.append((f"ld_{h}_indix", und, [], f"ld {h},(ix+1)"))
        out.append((f"ld_indix_{h}", und, [], f"ld (ix+1),{h}"))
        out.append((f"ld_{h}_indiy", und, [], f"ld {h},(iy+1)"))
        out.append((f"ld_indiy_{h}", und, [], f"ld (iy+1),{h}"))
        out.append((f"ld_{h}_mem", und, [], f"ld {h},(1234h)"))
        out.append((f"ld_mem_{h}", und, [], f"ld (1234h),{h}"))
        out.append((f"ld_{h}_i", und, [], f"ld {h},i"))
        out.append((f"ld_i_{h}", und, [], f"ld i,{h}"))
        for op in ALU:
            out.append((f"alu2_{op}_{h}", und, [], f"{op} a,{h}"))
            out.append((f"alu1_{op}_{h}", und, [], f"{op} {h}"))
        out.append((f"alud_add_{h}_a", und, [], f"add {h},a"))
        out.append((f"alud_add_b_{h}", und, [], f"add b,{h}"))
        out.append((f"alu_add_hl_{h}", und, [], f"add hl,{h}"))
        out.append((f"inc_{h}", und, [], f"inc {h}"))
        out.append((f"dec_{h}", und, [], f"dec {h}"))
        for op in CB:
            out.append((f"cb_{op}_{h}", und, [], f"{op} {h}"))
        for op in ["bit", "res", "set"]:
            out.append((f"cbb_{op}_{h}", und, [], f"{op} 0,{h}"))
        out.append((f"push_{h}", und, [], f"push {h}"))
        out.append((f"pop_{h}", und, [], f"pop {h}"))
        out.append((f"in_{h}", und, [], f"in {h},(c)"))
        out.append((f"out_{h}", und, [], f"out (c),{h}"))
        out.append((f"jp_{h}", und, [], f"jp ({h})"))
        out.append((f"ex_{h}", und, [], f"ex de,{h}"))
    # controls: documented forms the halves shadow, same harness
    for f in ["ld a,l", "ld a,h", "ld l,a", "ld h,l", "ld l,5", "add a,l",
              "inc l", "dec h", "ld a,(ix+1)", "sll a", "sll b"]:
        out.append(("ctl_" + re.sub(r"[^a-z0-9]+", "_", f), und, [], f))
    return out


def probes_m2():
    """Second round: the half name in expressions and parentheses, the mode
    across save/restore and cpu switches, one-operand ALU controls, immediate
    range, and 16-bit mixes."""
    und = "z80undoc"
    out = []
    # the name inside an expression or parentheses, with a symbol of that name
    for h in ["ixl", "iyu"]:
        out.append((f"expr_{h}", und, [f"{h} equ 5"], f"ld a,{h}+1"))
        out.append((f"exprl_{h}", und, [f"{h} equ 5"], f"ld a,1+{h}"))
        out.append((f"paren_{h}", und, [f"{h} equ 1234h"], f"ld a,({h})"))
        out.append((f"pareni_{h}", und, [f"{h} equ 5"], f"ld a,({h})+1"))
        out.append((f"db_{h}", und, [f"{h} equ 5"], f"db {h}"))
        out.append((f"dbnosym_{h}", und, [], f"db {h}"))
        out.append((f"jp_{h}", und, [f"{h} equ 1234h"], f"jp {h}"))
        out.append((f"ldimm_{h}", und, [f"{h} equ 5"], f"ld b,{h}"))
        out.append((f"label_{h}", und, [f"{h}: nop"], f"ld a,{h}"))
        out.append((f"plus_{h}", und, [], f"ld a,+{h}"))
        out.append((f"neg_{h}", und, [], f"ld a,-{h}"))
    # the mode across save/restore and switches
    out.append(("mode_save_z80_restore", und, ["save", "cpu z80", "restore"], "ld a,ixl"))
    out.append(("mode_save_68k_restore", und, ["save", "cpu 68000", "restore"], "ld a,ixl"))
    out.append(("mode_z80_then_undoc", "z80", ["cpu z80undoc"], "ld a,ixl"))
    out.append(("mode_undoc_then_z80", und, ["cpu z80"], "ld a,ixl"))
    out.append(("mode_undoc_then_z80_sym", und, ["ixl equ 5", "cpu z80"], "ld a,ixl"))
    out.append(("mode_68k_save_undoc", "68000", ["save", "cpu z80undoc"], "ld a,ixl"))
    out.append(("mode_z80_save_undoc_restore", "z80", ["ixl equ 5", "save", "cpu z80undoc", "restore"], "ld a,ixl"))
    out.append(("mode_undoc_upper", "Z80UNDOC", [], "ld a,ixl"))
    out.append(("mode_undoc_mixed", "Z80UnDoc", [], "ld a,ixl"))
    # MOMCPU / MOMCPUNAME under each spelling
    for c in ["z80", "z80undoc", "Z80UNDOC"]:
        t = c.lower() if c.islower() else c + "u"
        out.append((f"momcpu_{t}", c, [], "dw MOMCPU"))
        out.append((f"momcpuname_{t}", c, ['if MOMCPUNAME="Z80UNDOC"', "db 1", "else", "db 2", "endif"], "nop"))
        out.append((f"momcpunamez_{t}", c, ['if MOMCPUNAME="Z80"', "db 1", "else", "db 2", "endif"], "nop"))
    # one-operand ALU with a plain register and an immediate (controls for the
    # halves' one-operand rule)
    for op in ALU:
        out.append((f"alu1r_{op}", und, [], f"{op} b"))
        out.append((f"alu1r_{op}_z80", "z80", [], f"{op} b"))
        out.append((f"alu1i_{op}", und, [], f"{op} 5"))
        out.append((f"alu2b_{op}_ixl", und, [], f"{op} b,ixl"))
    # immediate range, beside the plain register
    for h in ["ixl", "l"]:
        for v in ["-128", "-129", "255", "0FFh", "'A'"]:
            t = re.sub(r"[^A-Za-z0-9]+", "_", v)
            out.append((f"rng_{h}_{t}", und, [], f"ld {h},{v}"))
    # 16-bit mixes and odd operands
    for f in ["ld hl,ixl", "ld ixl,hl", "ld ix,ixl", "ld ixl,ix", "ld bc,ixl",
              "ld sp,ixl", "ld ixl,r", "ld r,ixl", "ld ixl,af", "ld ixl,(c)",
              "ld ixl,nz", "ld (ixl),a", "ld a,(ixl)", "ld ixl", "ld ixl,a,b",
              "inc ixl,a", "cp ixl,a", "ld iyh,iyl", "ld ixh,ixu", "ld ixl,iyh",
              "LD A,IXL", "ld A,ixl", "ld ixl,A", "ld IXU,B", "ld ixl,H", "ld L,ixu"]:
        out.append(("mix_" + re.sub(r"[^A-Za-z0-9]+", "_", f), und, [], f))
    return out


def probes_m3():
    """Third round: with a SYMBOL of the half's name defined, in which operand
    positions is the bare name still the register (register decode), and in
    which is it the symbol (expression decode)?"""
    und = "z80undoc"
    out = []
    for h, v in [("ixl", "5"), ("iyu", "5")]:
        s = [f"{h} equ {v}"]
        forms = {
            "ldhl": f"ld hl,{h}", "ldbc": f"ld bc,{h}", "ldix": f"ld ix,{h}",
            "ldsp": f"ld sp,{h}", "ldindhl": f"ld (hl),{h}",
            "ldindix": f"ld (ix+1),{h}", "ldmem": f"ld (1234h),{h}",
            "ldmemd": f"ld {h},(1234h)", "ldhlmem": f"ld hl,({h})",
            "ldixd": f"ld a,(ix+{h})", "ldixd2": f"ld ({h}+ix),a",
            "ldhexpr": f"ld {h},{h}+1", "ldhparen": f"ld {h},({h})",
            "addhl": f"add hl,{h}", "adda": f"add a,{h}", "sub1": f"sub {h}",
            "inc": f"inc {h}", "ldh5": f"ld {h},5", "push": f"push {h}",
            "bit": f"bit {h},b", "res": f"res {h},b", "set": f"set {h},(hl)",
            "im": f"im {h}", "rst": f"rst {h}", "call": f"call {h}",
            "jpnz": f"jp nz,{h}", "callnz": f"call nz,{h}", "retnz": f"ret {h}",
            "ina": f"in a,({h})", "outa": f"out ({h}),a", "ldia": f"ld i,{h}",
            "exsp": f"ex (sp),{h}", "ldab": f"ld a,{h}",
        }
        for k, f in forms.items():
            out.append((f"sym_{k}_{h}", und, s, f))
        # relative targets: a label of that name
        out.append((f"lab_jr_{h}", und, [], f"jr {h}\n{h}:"))
        out.append((f"lab_djnz_{h}", und, [], f"djnz {h}\n{h}:"))
        out.append((f"lab_jp_{h}", und, [], f"jp {h}\n{h}:"))
        out.append((f"lab_jrz_{h}", und, [], f"jr z,{h}\n{h}:"))
    # the uppercase name defined as a symbol
    out.append(("usym_ldaIXL", und, ["IXL equ 5"], "ld a,IXL"))
    out.append(("usym_ldaIYU", und, ["IYU equ 5"], "ld a,IYU"))
    out.append(("usym_dbIXL", und, ["IXL equ 5"], "db IXL"))
    # bit number positions without a symbol: register decode or undefined?
    out.append(("nosym_bit", und, [], "bit ixl,b"))
    out.append(("nosym_im", und, [], "im ixl"))
    out.append(("nosym_rst", und, [], "rst ixl"))
    out.append(("nosym_ldhl", und, [], "ld hl,ixl"))
    out.append(("nosym_ldindhl", und, [], "ld (hl),ixl"))
    return out


def probes_m4():
    """Fourth round: the Sonic 2 shape (a 68000 root switching to z80undoc
    under save/restore), and each of Sonic 2's twelve lines."""
    out = []
    out.append(("root68k_iyl", "68000", ["move.w #$1234,d0", "save", "cpu z80undoc"],
                "ld a,iyl\n\trestore\n\tmove.w #$5678,d1"))
    out.append(("root68k_after", "68000", ["save", "cpu z80undoc", "nop", "restore",
                                           "save", "cpu z80"], "ld a,ixl\n\trestore"))
    out.append(("root68k_after_sym", "68000", ["ixl equ 5", "save", "cpu z80undoc", "nop",
                                               "restore", "save", "cpu z80"],
                "ld a,ixl\n\trestore"))
    for i, f in enumerate(["ld a,iyl", "adc a,iyu", "ld e,ixl", "ld d,ixu",
                           "ld a,ixl", "adc a,ixu", "add a,ixl"]):
        out.append((f"s2_{i}", "z80undoc", [], f))
    return out


def write(outdir, name, cpu, pre, form):
    lines = [f"\tcpu {cpu}", "\torg 0"]
    for p in pre:
        lines.append(p if " equ " in p and not p.startswith("\t") else "\t" + p)
    first, *more = form.split("\n")
    lines.append("\t" + first)
    lines.extend(more)
    lines.append("\tnop")
    lines.append("\tend")
    with open(os.path.join(outdir, name + ".asm"), "w") as fh:
        fh.write("\n".join(lines) + "\n")
    return 3 + len(pre)  # 1-based listing line of the form


def main():
    outdir, table = sys.argv[1], sys.argv[2]
    os.makedirs(outdir, exist_ok=True)
    which = sys.argv[3] if len(sys.argv) > 3 else "m1"
    ps = {"m1": probes, "m2": probes_m2, "m3": probes_m3, "m4": probes_m4}[which]()
    formline = {}
    for name, cpu, pre, form in ps:
        formline[name] = write(outdir, name, cpu, pre, form)
    log = subprocess.run(["bash", f"{S}/run_asl.sh", outdir] + [p[0] + ".asm" for p in ps],
                         capture_output=True, text=True)
    open(os.path.join(outdir, "_run.log"), "w").write(log.stdout + log.stderr)
    md5 = re.search(r"ASL_MD5=(\w+)", log.stdout).group(1)
    exits = dict(re.findall(r"== (\S+)\.asm ASL_EXIT=(\d+)", log.stdout))
    rows = []
    for name, cpu, pre, form in ps:
        rc = int(exits[name])
        lst = os.path.join(outdir, name + ".lst")
        txt = open(lst, errors="replace").read() if os.path.exists(lst) else ""
        complete = bool(re.search(r"^ +\d+ passe?s?$", txt, re.M)) and \
            not re.search(r"^\s+Additional necessary passes", txt, re.M)
        val = ""
        if rc == 0 and complete:
            n = formline[name]
            m = re.search(rf"^\s+{n}/\s*[0-9A-F]+ :((?: [0-9A-F]{{2}})*)", txt, re.M)
            val = m.group(1).strip() if m else "?"
        err = ""
        if rc != 0:
            al = open(os.path.join(outdir, name + ".asllog"), errors="replace").read()
            m = re.search(r"(error|warning) #\d+: [^\n]*", al)
            err = m.group(0) if m else al.strip().splitlines()[0] if al.strip() else "?"
        rows.append((name, cpu, form.replace("\n", " / "), rc, "complete" if complete else "INCOMPLETE/none", val, err))
    with open(os.path.join(S, table), "w") as fh:
        fh.write(f"# asl md5 {md5}\n# name\tcpu\tform\texit\tfooter\tbytes\terror\n")
        for r in rows:
            fh.write("\t".join(str(x) for x in r) + "\n")
    print(f"asl md5 {md5}; {len(rows)} probes; accepted {sum(1 for r in rows if r[3] == 0)}; refused {sum(1 for r in rows if r[3] != 0)}")


main()
