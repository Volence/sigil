#!/usr/bin/env python3
"""gen_probes.py: write the register-spelled-label probe set to probes/<name>.asm.
Each probe is a whole file; the header is cpu 68000 (or z80) and org $1200."""
import os
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "probes")
H68 = "\tcpu 68000\n\torg $1200\n"
HZ80 = "\tcpu z80\n\torg 1200h\n"
P = {}
def p(name, body, hdr=H68, desc=""):
    P[name] = (hdr + body, desc)

# --- definitions alone, no use -------------------------------------------
for r in ["A1", "a1", "D0", "d0", "SP", "sp", "A7", "USP", "SR", "CCR", "PC"]:
    p(f"def_{r}", f"{r}:\tnop\n\tnop\n", desc=f"`{r}:` defined, never read")
p("def_A1_equ", "A1\tequ\t5\n\tnop\n", desc="`A1 equ 5`, never read")
p("def_A1_set", "A1\tset\t5\n\tnop\n", desc="`A1 set 5`, never read")

# --- `A1:` label read in each position ------------------------------------
L = "A1:\tnop\nLab:\tnop\n"
uses = {
    "imm":        ("\tmove.w\t#A1,d0\n", "`#A1`"),
    "imm_plus":   ("\tmove.w\t#A1+2,d0\n", "`#A1+2`"),
    "imm_sub":    ("\tmove.w\t#Lab-A1,d0\n", "`#Lab-A1`"),
    "imm_paren":  ("\tmove.w\t#(A1),d0\n", "`#(A1)`"),
    "imm_lab":    ("\tmove.w\t#A1+Lab,d1\n", "`#A1+Lab` (z01's shape)"),
    "dcw":        ("\tdc.w\tA1\n", "`dc.w A1`"),
    "dcl":        ("\tdc.l\tA1\n", "`dc.l A1`"),
    "dcw_plus":   ("\tdc.w\tA1+2\n", "`dc.w A1+2`"),
    "dcl_sub":    ("\tdc.l\tLab-A1\n", "`dc.l Lab-A1`"),
    "dcb_plus":   ("\tdc.b\tA1-Lab+2\n", "`dc.b A1-Lab+2`"),
    "pcrel":      ("\tlea\tA1(pc),a0\n", "`lea A1(pc),a0`"),
    "absw":       ("\tmove.w\t(A1).w,d0\n", "`move.w (A1).w,d0`"),
    "absl":       ("\tmove.w\t(A1).l,d0\n", "`move.w (A1).l,d0`"),
    "bare_move":  ("\tmove.w\tA1,d0\n", "`move.w A1,d0` (bare: register direct?)"),
    "bare_lea":   ("\tlea\tA1,a0\n", "`lea A1,a0`"),
    "bare_jsr":   ("\tjsr\tA1\n", "`jsr A1`"),
    "bra":        ("\tbra.w\tA1\n", "`bra.w A1`"),
    "disp":       ("\tmove.w\tA1(a0),d0\n", "`move.w A1(a0),d0` (displacement)"),
    "disp_plus":  ("\tmove.w\tA1+2(a0),d0\n", "`move.w A1+2(a0),d0`"),
    "equ_rhs":    ("X\tequ\tA1\n\tdc.w\tX\n", "`X equ A1` then `dc.w X`"),
    "set_rhs":    ("X\tset\tA1+2\n\tdc.w\tX\n", "`X set A1+2` then `dc.w X`"),
    "if_cmp":     ("\tif A1=$1200\n\tnop\n\tendif\n", "`if A1=$1200`"),
    "ifdef":      ("\tifdef A1\n\tnop\n\tendif\n", "`ifdef A1`"),
    "defined":    ("\tif defined(A1)\n\tnop\n\tendif\n", "`if defined(A1)`"),
    "org":        ("\torg\tA1+8\n\tnop\n", "`org A1+8`"),
}
for k, (u, d) in uses.items():
    p(f"lbl_{k}", L + u, desc=f"`A1:` label, {d}")

# --- other spellings as labels, read in `#X+2` ---------------------------
for r in ["a1", "D0", "d0", "SP", "sp", "A7", "USP", "SR", "CCR", "PC"]:
    p(f"lbl_{r}_imm_plus", f"{r}:\tnop\n\tmove.w\t#{r}+2,d0\n", desc=f"`{r}:` label, `#{r}+2`")
    p(f"lbl_{r}_dcw_plus", f"{r}:\tnop\n\tdc.w\t{r}+2\n", desc=f"`{r}:` label, `dc.w {r}+2`")
# case: label A1 read as a1 and vice versa
p("case_A1_read_a1", "A1:\tnop\n\tmove.w\t#a1+2,d0\n", desc="`A1:` label, `#a1+2` (other case)")
p("case_a1_read_A1", "a1:\tnop\n\tmove.w\t#A1+2,d0\n", desc="`a1:` label, `#A1+2` (other case)")

# --- equ / set defined register spelling -----------------------------------
p("equ_A1_imm", "A1\tequ\t5\n\tmove.w\t#A1,d0\n", desc="`A1 equ 5`, `#A1`")
p("equ_A1_imm_plus", "A1\tequ\t5\n\tmove.w\t#A1+2,d0\n", desc="`A1 equ 5`, `#A1+2`")
p("equ_A1_dcw", "A1\tequ\t5\n\tdc.w\tA1\n", desc="`A1 equ 5`, `dc.w A1`")
p("equ_A1_dcw_plus", "A1\tequ\t5\n\tdc.w\tA1+2\n", desc="`A1 equ 5`, `dc.w A1+2`")
p("set_a0_dcl", "a0\tset\t5\n\tdc.l\ta0+1\n", desc="`a0 set 5`, `dc.l a0+1`")
p("equ_A1_if", "A1\tequ\t5\n\tif A1=5\n\tnop\n\tendif\n", desc="`A1 equ 5`, `if A1=5`")

# --- controls: no definition ---------------------------------------------
p("ctl_nodef_imm_plus", "Lab:\tnop\n\tmove.w\t#A1+2,d0\n", desc="control: no `A1` defined, `#A1+2`")
p("ctl_nodef_dcw_plus", "Lab:\tnop\n\tdc.w\tA1+2\n", desc="control: no `A1` defined, `dc.w A1+2`")
p("ctl_nodef_dcl", "Lab:\tnop\n\tdc.l\tA1\n", desc="control: no `A1` defined, `dc.l A1`")

# --- names that CONTAIN a register spelling --------------------------------
for nm in ["A1x", "XA1", "A1_2", "A10", "A8", "D8", "SP2", "A1A1"]:
    p(f"cont_{nm}", f"{nm}:\tnop\n\tmove.w\t#{nm}+2,d0\n\tdc.w\t{nm}\n", desc=f"`{nm}:` label, `#{nm}+2` and `dc.w {nm}`")
p("cont_A1_local", "A1:\tnop\n.l:\tnop\n\tmove.w\t#A1.l+2,d0\n\tdc.w\tA1.l\n", desc="`A1:` then `.l:`, read `A1.l`")
p("cont_dollar_A1", "Lab:\tnop\n$$A1:\tnop\n\tmove.w\t#$$A1+2,d0\n\tdc.w\t$$A1\n", desc="`$$A1:` temp label, read")
p("cont_A1_dotw", "A1:\tnop\n\tmove.w\t#A1.w,d0\n", desc="`A1:` label, `#A1.w`")

# --- the booked shapes, and a $$-free twin ---------------------------------
p("i09", "A1:\tnop\n$$x:\tnop\n\tmove.w\t#$$x-A1,d0\n", desc="booked i09 shape: `#$$x-A1`")
p("i09_nodollar", "A1:\tnop\nB:\tnop\n\tmove.w\t#B-A1,d0\n", desc="i09 without `$$`: `#B-A1`")

# --- Z80 -------------------------------------------------------------------
for r in ["hl", "a", "ix", "sp", "bc", "af"]:
    p(f"z80_def_{r}", f"{r}:\tnop\n\tnop\n", hdr=HZ80, desc=f"z80 `{r}:` defined, never read")
    p(f"z80_dw_{r}", f"{r}:\tnop\n\tdw\t{r}\n", hdr=HZ80, desc=f"z80 `{r}:` label, `dw {r}`")
    p(f"z80_dwplus_{r}", f"{r}:\tnop\n\tdw\t{r}+2\n", hdr=HZ80, desc=f"z80 `{r}:` label, `dw {r}+2`")
    p(f"z80_ldimm_{r}", f"{r}:\tnop\n\tld\tde,{r}+2\n", hdr=HZ80, desc=f"z80 `{r}:` label, `ld de,{r}+2`")
p("z80_ctl_dw_hl", "\tnop\n\tdw\thl+2\n", hdr=HZ80, desc="z80 control: no `hl` defined, `dw hl+2`")
p("z80_ld_hl_bare", "hl:\tnop\n\tld\thl,hl\n", hdr=HZ80, desc="z80 `hl:` label, `ld hl,hl`")

# --- dc with a register operand among others --------------------------------
p("dc_multi", "A1:\tnop\nLab:\tnop\n\tdc.w\tLab,A1,Lab\n", desc="`A1:` label, `dc.w Lab,A1,Lab`")
p("dc_b", "A1:\tnop\n\tdc.b\tA1,0\n", desc="`A1:` label, `dc.b A1`")
p("ctl_nodef_dcw_multi", "Lab:\tnop\n\tdc.w\tLab,d3,Lab\n", desc="control: no label, `dc.w Lab,d3,Lab`")
p("ctl_nodef_imm", "\tmove.w\t#A1,d0\n", desc="control: no label, `#A1`")
p("macro_body_label", "M\tmacro\nA1:\tnop\n\tmove.w\t#A1+2,d0\n\tendm\n\tM\n", desc="`A1:` defined inside a macro body, `#A1+2` in the same body")
p("ds_count", "A1:\tnop\n\tds.b\tA1-$11F0\n", desc="`A1:` label, `ds.b A1-$11F0`")

# --- more consumers of a value: absolute EAs, jumps, dbcc, pc-indexed, movem, forward -
L2 = "A1:\tnop\nLab:\tnop\n"
more = {
    "abs_plus":   ("\tmove.w\tA1+2,d0\n", "`move.w A1+2,d0`"),
    "lea_plus":   ("\tlea\tA1+2,a0\n", "`lea A1+2,a0`"),
    "jsr_plus":   ("\tjsr\tA1+2\n", "`jsr A1+2`"),
    "jmp_absl":   ("\tjmp\t(A1+2).l\n", "`jmp (A1+2).l`"),
    "dbf":        ("\tdbf\td0,A1\n", "`dbf d0,A1`"),
    "pcidx":      ("\tlea\tA1(pc,d0.w),a0\n", "`lea A1(pc,d0.w),a0`"),
    "movem_pc":   ("\tmovem.l\tA1(pc),d0-d1\n", "`movem.l A1(pc),d0-d1`"),
    "dcl_fwd":    ("\tdc.l\tA1+Fwd\nFwd:\tnop\n", "`dc.l A1+Fwd`, `Fwd` defined after"),
    "rept":       ("\trept\tA1-$11FF\n\tnop\n\tendm\n", "`rept A1-$11FF`"),
}
for k, (u, d) in more.items():
    p(f"lbl_{k}", L2 + u, desc=f"`A1:` label, {d}")
p("lbl_imm_before_def", "\tmove.w\t#A1+2,d0\nA1:\tnop\n", desc="`#A1+2` read ABOVE the `A1:` definition")

os.makedirs(OUT, exist_ok=True)
with open(os.path.join(OUT, "INDEX.tsv"), "w") as idx:
    for n, (src, d) in P.items():
        with open(os.path.join(OUT, n + ".asm"), "w") as f:
            f.write(src)
        idx.write(f"{n}\t{d}\n")
print(len(P), "probes")
