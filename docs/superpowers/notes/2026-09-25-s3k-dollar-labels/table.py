#!/usr/bin/env python3
"""table.py <probe log>: the note's probe table, one markdown row per probe."""
import re, subprocess, sys
D = {
 "d01": "two scopes, same names, forward and backward", "d02": "`.loc` labels between def and use",
 "d03": "nameless `+`/`-` between def and use", "d04": "`$$x` before the first label, read after `A1:`",
 "d05": "`A2 equ` between def and use", "d06": "`A2 set` between def and use", "d07": "`$$X` read for `$$x`",
 "d08": "`$$v equ $$x+$21` in two scopes", "d09": "`bra.s`/`beq.w` to `$$` labels", "d10": "`B1: dc.w $$x` (label on the using line)",
 "e01": "`V set 1`/`$$x`/`V set 2`/`$$x`", "e02": "body defines and reads `$$l`, called twice", "e03": "body writes `Lb:`, caller reads `$$x` after",
 "e04": "`section` between def and use", "e05": "`phase` block, `$$y` inside it", "e06": "`B equ $$x` (binder reads old scope)",
 "e07": "`$$x:` then `.l:`, read `A1.l`", "e08": "`cpu 68000` between def and use", "e09": "`$$v set` redefined",
 "e10": "`$$x` and `$$X` both defined", "e11": "`$$x` passed as a macro argument", "e12": "macro DEFINITION between def and use",
 "e13": "`rept` between def and use", "e14": "`$$:` and `dc.w $$`", "e15": "`$$1:`", "e16": "z80 `org $1200` (probe design)",
 "e17": "z80 `dw $$` (probe design)", "e18": "68k `dc.w $$`", "e19": "`charset` between def and use", "e20": "undefined `$$z`",
 "f01": "`V set`/`W set`/`V set` returns to V's names", "f02": "body reads caller's `$$x`", "f03": "body owns `$$x`, caller keeps its own",
 "f04": "`$$y` defined inside `section`, read after", "f05": "z80: two scopes", "f06": "z80 `dw $$`", "f07": "`padding off` between",
 "f08": "`supmode on` between", "f09": "`listing on` between", "f10": "body `$$q`, caller reads after", "f11": "body reads caller's LATER `$$q`",
 "f12": "`save`/`restore` between", "f13": "`if 1` block between", "f14": "body `Lb:`+`$$q`, called twice", "f15": "`$$x.y` is its own name",
 "f16": "`B1 label *` between", "f17": "`$$a_b`, `$$c.d`", "f18": "`-$$x+$$x*2`", "f19": "labels in a `phase`, read after `dephase`",
 "f20": "`org` between", "g01": "body `Lb:` twice, caller `$$z:` after each", "g02": "`$$x:` either side of a second `cpu 68000`",
 "g03": "`.l` either side of `cpu 68000` (no `$$`)", "g04": "`$$x:` after `cpu`, again after `padding off`", "g05": "`page 0` between",
 "g06": "`enum E1,E2` between", "g07": "`save` between", "g08": "`restore` without `save`", "g09": "`pushv` (probe design)",
 "g10": "`V set 1` between", "g11": "`padding on` between", "g12": "`supmode off` between", "g13": "`listing off/on` between",
 "g14": "`struct`...`endstruct` between", "g15": "`shared` between", "g16": "`nextenum` between", "g17": "body reads caller `$$x` defined between two calls",
 "g18": "`A1.$$x`", "g19": "`.pre:` then `$$x`", "g20": "`cpu z80`/`cpu 68000` between",
 "h02": "scope after `save`/`cpu z80undoc`/`restore` (listing only)",
 "i01": "`cpu 68000` then `padding off` share `PADDING`", "i02": "`rept 2` body owns `$$r`", "i05": "`$$x label *`",
 "i07": "`defined()`/`ifdef`/`ifndef` of `$$`", "i09": "`#$$x-A1` (`A1` is a register to asl)", "i10": "z80 `jr`/`ld`/`dw` with `$$x` and `$`",
 "i16": "forward read across `B1:`", "i25": "nested body reads the enclosing instance", "i26": "`$$x:` after `cpu z80`, read after `cpu 68000`",
 "i27": "body reads caller `$$q` in two scopes", "i28": "body `Lb:` then caller `A1b set`", "i29": "`$$y` read after `W set`",
 "i30": "three expansions each own `$$x`", "i31": "`cmpi.w #$$x`", "j01": "`defined()` before/after, `if defined($$v)`",
 "j02": "`#`, `(pc)`, `.w`, `dc.l` operands", "j03": "`$$x-Lab1+$10`", "j04": "`#$$x-Lab1`, `#($$x-Lab1)*3`", "j05": "`#$$x+$10`",
 "j06": "`set`-scoped names in V and W", "j07": "`#A1+$$x` (`A1` register)", "j08": "`#($$x-A1)` (`A1` register)",
 "k01": "`$$_x`, `$$.y`", "k02": "body reads `$$q` before its own `Lb:`/`$$q:`", "k03": "body `$$v equ`, `$$w set`",
 "k04": "body `$$x label *`", "k05": "`pushv` (probe design)", "k06": "k02 with no caller `$$q`", "k07": "second expansion reads its own forward `$$q`",
 "x01": "`$$v set` read by `dc.w`", "x02": "`$$v equ` read by `dc.w`", "x03": "control: plain `set`", "x04": "control: `dc.l Lab1+$10000`",
 "x05": "`dc.l $$x`", "x06": "control: `#A1+Lab2`, `A1` not a label", "y01": "`$$v set`", "y02": "`$$v set` in `#`", "y03": "`$$v :=`",
 "y04": "`$$v =`", "z01": "control: `#A1+Lab2` with `A1:` a label (no `$$`)", "z02": "control: `section` (no `$$`)",
}
rows = subprocess.run(["python3", "/home/volence/sonic_hacks/.scratch/s3k-dollar-labels/summ.py", sys.argv[1]], capture_output=True, text=True).stdout.splitlines()
print("| probe | input | asl | sigil | verdict |")
print("|---|---|---|---|---|")
for r in rows:
    n, a, s, v, ab, sb = (r.split("\t") + ["", ""])[:6]
    a = a.split("=")[1]; s = s.split("=")[1]
    ac = f"exit {a}: `` {ab} ``" if ab else f"exit {a}"
    sc = f"exit {s}: `` {sb} ``" if sb else f"exit {s}"
    print(f"| {n} | {D.get(n, '?')} | {ac} | {sc} | {v} |")
