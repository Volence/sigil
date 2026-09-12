#!/usr/bin/env python3
"""Probes for asl's nameless-label counter under `+` runs, `-` and `/`.

One shape per file, at `org $100` behind a `$1111` filler word so a bound
label reads as a non-zero address. Each definition carries a distinct data
word, so a reference's value names the line it landed on. At most one
suspect line per file, marked `; REF` (a `rept` may repeat it).

    gen.py <out-dir>
"""
import os
import sys

OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
H = "\tcpu\t68000\n\tpadding\toff\n\torg\t$100\n\tdc.w\t$1111\n"
T = "\tdc.w\t$4444\n"


def d(lbl, word):
    return f"{lbl}\tdc.w\t${word}\n"


def ref(expr):
    return f"\tdc.w\t{expr}\t; REF\n"


def mac(*lines):
    return "mac\tmacro\n" + "".join(lines) + "\tendm\n"


CALL = "\tmac\n"

P = {
    # A: numbering of file-level definitions, read off asl's symbol table.
    "a01_p_pp_p_p": ("defs + ++ + +", d("+", 2222) + d("++", 3333) + d("+", 5555) + d("+", 6666)),
    "a02_pp_pp": ("defs ++ ++", d("++", 2222) + d("++", 3333)),
    "a03_ppp_p_p_p": ("defs +++ + + +", d("+++", 2222) + d("+", 3333) + d("+", 5555) + d("+", 6666)),
    "a04_pp_p_p": ("defs ++ + +", d("++", 2222) + d("+", 3333) + d("+", 5555)),
    "a05_ppp_pp": ("defs +++ ++", d("+++", 2222) + d("++", 3333)),
    "a06_pp_ppp": ("defs ++ +++", d("++", 2222) + d("+++", 3333)),
    "a07_pppp": ("defs ++++", d("++++", 2222)),
    "a08_p_p_pp_p": ("defs + + ++ +", d("+", 2222) + d("+", 3333) + d("++", 5555) + d("+", 6666)),
    "a09_slash_minus": ("defs / -", d("/", 2222) + d("-", 3333)),
    "a10_minus_slash_minus": ("defs - / -", d("-", 2222) + d("/", 3333) + d("-", 5555)),
    "a11_slash_pp_p": ("defs / ++ +", d("/", 2222) + d("++", 3333) + d("+", 5555)),
    "a12_p_pp_p_p_pp": ("defs + ++ + + ++", d("+", 2222) + d("++", 3333) + d("+", 5555) + d("+", 6666) + d("++", 7777)),
    # B: file-level references spanning a run.
    "b01_ref_p_over_p_pp_p": ("ref + before defs + ++ +", ref("+") + d("+", 2222) + d("++", 3333) + d("+", 5555)),
    "b02_ref_pp_over_p_pp_p": ("ref ++ before defs + ++ +", ref("++") + d("+", 2222) + d("++", 3333) + d("+", 5555)),
    "b03_ref_ppp_over_p_pp_p": ("ref +++ before defs + ++ +", ref("+++") + d("+", 2222) + d("++", 3333) + d("+", 5555)),
    "b04_ref_pppp_over_p_pp_p": ("ref ++++ before defs + ++ +", ref("++++") + d("+", 2222) + d("++", 3333) + d("+", 5555)),
    "b05_pp_ref_p_p": ("def ++, ref +, def +", d("++", 2222) + ref("+") + d("+", 3333)),
    "b06_pp_ref_pp_p": ("def ++, ref ++, def +", d("++", 2222) + ref("++") + d("+", 3333)),
    "b07_pp_sameline_ref_p": ("`++` line carries ref +; def + after", "++\tdc.w\t+\t; REF\n" + d("+", 3333)),
    "b08_pp_sameline_ref_pp": ("`++` line carries ref ++; def + after", "++\tdc.w\t++\t; REF\n" + d("+", 3333)),
    "b09_bra_pp_over_run": ("bra.s ++ before defs + ++ +",
                            "\tbra.s\t++\t; REF\n+\tdc.w\t$2222\n++\tdc.w\t$3333\n+\tdc.w\t$5555\n"),
    "b10_bra_ppp_over_run": ("bra.s +++ before defs + ++ +",
                             "\tbra.s\t+++\t; REF\n+\tdc.w\t$2222\n++\tdc.w\t$3333\n+\tdc.w\t$5555\n"),
    "b11_p_pp_ref_p_p": ("defs + ++, ref +, def +", d("+", 2222) + d("++", 3333) + ref("+") + d("+", 5555)),
    "b12_p_pp_ref_pp_p": ("defs + ++, ref ++, def +", d("+", 2222) + d("++", 3333) + ref("++") + d("+", 5555)),
    "b13_ref_p_over_ppp_p_p": ("ref + before defs +++ + +", ref("+") + d("+++", 2222) + d("+", 3333) + d("+", 5555)),
    "b14_ref_ppp_over_ppp_p_p": ("ref +++ before defs +++ + +", ref("+++") + d("+++", 2222) + d("+", 3333) + d("+", 5555)),
    "b15_minus_run_ref": ("defs - - -, ref ---", d("-", 2222) + d("-", 3333) + d("-", 5555) + ref("---")),
    "b16_bra_p_after_pp": ("def ++, bra.s +, def +",
                           "++\tdc.w\t$2222\n\tbra.s\t+\t; REF\n+\tdc.w\t$3333\n"),
    "b17_bra_pp_after_pp": ("def ++, bra.s ++, def +",
                            "++\tdc.w\t$2222\n\tbra.s\t++\t; REF\n+\tdc.w\t$3333\n"),
    "b18_slash_pp_p_ref_pp": ("ref ++ before defs / ++ +", ref("++") + d("/", 2222) + d("++", 3333) + d("+", 5555)),
    # C: runs in macro bodies.
    "c01_body_pp_ref_pp": ("body: def ++, ref ++", mac(d("++", 2222), ref("++")) + CALL),
    "c02_body_ref_pp_over_run": ("body: ref ++ before defs + ++ +",
                                 mac(ref("++"), d("+", 2222), d("++", 3333), d("+", 5555)) + CALL),
    "c03_ref_p_body_pp_file_p": ("ref + before call; body ++; file + after", ref("+") + mac(d("++", 2222)) + CALL + d("+", 3333)),
    "c04_ref_p_body_ppp_twice": ("ref + before two calls of body +++; file + after",
                                 ref("+") + mac(d("+++", 2222)) + CALL + CALL + d("+", 3333)),
    "c05_body_pp_then_file_ref": ("body ++; after the call: ref ++, def +",
                                  mac(d("++", 2222)) + CALL + ref("++") + d("+", 3333)),
    "c06_body_p_pp_then_file": ("body: def + ++; file after the call: ref +, def + +",
                                mac(d("+", 2222), d("++", 3333)) + CALL + ref("+") + d("+", 5555) + d("+", 6666)),
    # D: runs in rept iterations.
    "d01_ref_p_rept2_pp_file_p": ("ref +; rept 2 { ++ }; file +", ref("+") + "\trept\t2\n" + d("++", 2222) + "\tendm\n" + d("+", 3333)),
    "d02_rept2_pp_ref_pp": ("rept 2 { ++, ref ++ }", "\trept\t2\n" + d("++", 2222) + ref("++") + "\tendm\n"),
    "d03_ref_ppp_rept2_p_file_pp_p": ("ref +++; rept 2 { + }; file ++ +",
                                      ref("+++") + "\trept\t2\n" + d("+", 2222) + "\tendm\n" + d("++", 3333) + d("+", 5555)),
    "d04_rept2_ref_p_pp_p": ("rept 2 { ref +, ++, + }", "\trept\t2\n" + ref("+") + d("++", 2222) + d("+", 3333) + "\tendm\n"),
    # E: backward definitions, and `/`.
    "e01_mm_def": ("def --", d("--", 2222)),
    "e02_ss_def": ("def //", d("//", 2222)),
    "e03_body_mm_def": ("body: def --", mac(d("--", 2222)) + CALL),
    "e04_body_slash_minus_ref_mm": ("body: / - then ref --", mac(d("/", 2222), d("-", 3333), ref("--")) + CALL),
    "e05_body_minus_slash_ref_mm": ("body: - / then ref --", mac(d("-", 2222), d("/", 3333), ref("--")) + CALL),
    "e06_file_minus_body_slash_file_minus_ref_mm": ("file -; body /; file -; ref --",
                                                    d("-", 2222) + mac(d("/", 3333)) + CALL + d("-", 5555) + ref("--")),
    "e07_body_slash_twice_file_minus": ("body /, called twice; file - (symbols)",
                                        mac(d("/", 2222)) + CALL + CALL + d("-", 3333)),
    "e08_rept_slash_file_minus": ("rept 1 { / }; file - (symbols)", "\trept\t1\n" + d("/", 2222) + "\tendm\n" + d("-", 3333)),
    "e09_body_slash_file_plus_minus": ("body /; file + -; file ref - (x7 with a ref)",
                                       mac(d("/", 2222)) + CALL + d("+", 3333) + d("-", 5555) + ref("-")),
    "e10_file_minus_rept_slash_file_minus_ref_mm": ("file -; rept 1 { / }; file -; ref --",
                                                    d("-", 2222) + "\trept\t1\n" + d("/", 3333) + "\tendm\n" + d("-", 5555) + ref("--")),
    "e11_body_minus_file_minus_ref_mm": ("file -; body -; file -; ref -- (control for e06)",
                                         d("-", 2222) + mac(d("-", 3333)) + CALL + d("-", 5555) + ref("--")),
    "e12_body_slash_ref_p_file_p": ("body: / then ref + ; file + after the call",
                                    mac(d("/", 2222), ref("+")) + CALL + d("+", 3333)),
    # F: how long a run may be, definition and reference.
    "f01_ref_pppp_over_four_p": ("ref ++++ before four single + defs",
                                 ref("++++") + d("+", 2222) + d("+", 3333) + d("+", 5555) + d("+", 6666)),
    "f02_four_m_ref_mmmm": ("four - defs, ref ----",
                            d("-", 2222) + d("-", 3333) + d("-", 5555) + d("-", 6666) + ref("----")),
    "f03_ppppp_def": ("def +++++", d("+++++", 2222)),
    "f04_bra_pppp_over_four_p": ("bra.s ++++ before four single + defs",
                                 "\tbra.s\t++++\t; REF\n+\tdc.w\t$2222\n+\tdc.w\t$3333\n+\tdc.w\t$5555\n+\tdc.w\t$6666\n"),
    "f05_three_m_ref_mmm_minus_1": ("three - defs, ref ----1 read as (---) - 1",
                                    d("-", 2222) + d("-", 3333) + d("-", 5555) + ref("----1")),
    "f06_four_m_ref_mmmmm_minus_1": ("four - defs, ref -----1",
                                     d("-", 2222) + d("-", 3333) + d("-", 5555) + d("-", 6666) + ref("-----1")),
    "f07_ref_pppp_plus_1_over_three_p": ("ref ++++1 read as (+++)+1, three single + defs",
                                         ref("++++1") + d("+", 2222) + d("+", 3333) + d("+", 5555)),
    "f08_dbf_mmmm": ("four - defs, dbf d0,----",
                     d("-", 2222) + d("-", 3333) + d("-", 5555) + d("-", 6666) + "\tdbf\td0,----\t; REF\n"),
    # G: a run landing on a slot another definition holds.
    "g01_body_p_pp_p_p": ("body: defs + ++ + +", mac(d("+", 2222), d("++", 3333), d("+", 5555), d("+", 6666)) + CALL),
    "g02_pp_slash_p": ("defs ++ / +", d("++", 2222) + d("/", 3333) + d("+", 5555)),
    "g03_rept2_p_pp_p_p": ("rept 2 { defs + ++ + + }", "\trept\t2\n" + d("+", 2222) + d("++", 3333) + d("+", 5555) + d("+", 6666) + "\tendm\n"),
    "g04_pp_body_p_file_p": ("file ++; body +; file + (lands on the file ++'s slot)",
                             d("++", 2222) + mac(d("+", 3333)) + CALL + d("+", 5555)),
    # b16 without the zero branch distance.
    "b19_bra_p_after_pp_nop": ("def ++, bra.s +, nop, def +",
                               "++\tdc.w\t$2222\n\tbra.s\t+\t; REF\n\tnop\n+\tdc.w\t$3333\n"),
}

for name, (comment, body) in P.items():
    with open(os.path.join(OUT, name + ".asm"), "w") as f:
        f.write(f"; {comment}\n" + H + body + T)
print(len(P))
