#!/usr/bin/env python3
"""Write one probe per section-closing shape into probes/.

Every probe has the same skeleton, so every row reads the same way:

  $00  dc.l L_next    where the assembler BOUND the first label after the close
  $04  dc.l L_after   where it bound a label one more section further on
  then a run of four marker words, a backward in-section seek, one overwrite,
  then THE CLOSER under test, then

  L_next:  dc.w $1234 / dc.l *     ($1234 finds where the bytes LANDED, and
                                    the longword says where that code believes
                                    it is)
  <plain close with no seek pending>
  L_after: dc.w $5678              ($5678 finds where the next section landed)

The label addresses come out of the image itself (the two head longwords), so
no symbol dump is needed from either assembler, and the bytes' location comes
from searching the image for the marker. One shape per file, so a refusal in
one shape cannot contaminate another's values.

Z80 bodies use `0AAh` hex, never `$AA`: under `cpu z80` a `$` is the program
counter in asl, and `*` is multiplication.
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "probes")

HEAD = """\
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
"""

# The 68000 seek body: four words at $08..$0F, seek back to $0A, overwrite one
# word. Cursor ends at $0C, extent stays $10.
SEEK68 = """\
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
"""

NEXT68 = """\
L_next: dc.w $1234
        dc.l *
"""

AFTER68 = """\
L_after: dc.w $5678
"""

SHAPES = {}

# c01: `cpu` naming the CPU already selected. close_section runs unconditionally.
SHAPES["c01_cpu_same"] = HEAD + SEEK68 + "        cpu 68000\n" + NEXT68 + "        cpu 68000\n" + AFTER68

# c02: `phase` closes the section. The next section's labels are phase-relative
# in both assemblers; the question is where its BYTES land.
SHAPES["c02_phase"] = (
    HEAD + SEEK68 + "        phase $8000\n" + NEXT68 + "        dephase\n" + AFTER68
)

# c03: `dephase` closes a PHASED section that had a backward seek inside it.
SHAPES["c03_dephase"] = HEAD + """\
        phase $8000
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org $8002
        dc.w $EEEE
        dephase
""" + NEXT68 + "        cpu 68000\n" + AFTER68

# c04: `restore` that changes the CPU closes a phased Z80 section that had a
# backward seek (Sonic 1's SetupValues_Z80 shape, save/cpu z80/phase, with a
# seek added and the closer moved to the restore).
SHAPES["c04_restore_z80_phased"] = HEAD + """\
        save
        cpu z80
        phase 8000h
        db 0AAh,0BBh,0CCh,0DDh
        org 8001h
        db 0EEh
        restore
        dephase
""" + NEXT68 + "        cpu 68000\n" + AFTER68

# c05: an `org` FORWARD past the extent leaves the section (and pins the next).
SHAPES["c05_org_forward_leaves"] = (
    HEAD + SEEK68 + "        org $40\n" + NEXT68 + "        cpu 68000\n" + AFTER68
)

# c06: an `org` BACKWARD below the section's base leaves the section (and pins).
# The seek section is opened at $20 so there is ground below its base.
SHAPES["c06_org_backward_leaves"] = HEAD + """\
        org $20
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        org $10
""" + NEXT68 + "        cpu 68000\n" + AFTER68

# c07: seek back, then seek forward to EXACTLY the extent (the back-patch idiom
# done right), then close. Cursor == extent at the close: nothing is pending.
SHAPES["c07_seek_back_then_to_extent"] = HEAD + """\
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        dc.w $EEEE
        org Start+8
        cpu 68000
""" + NEXT68 + "        cpu 68000\n" + AFTER68

# c08: two seeks, the second forward but still short of the extent.
SHAPES["c08_two_seeks_short_of_extent"] = HEAD + """\
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start
        dc.w $EEEE
        org Start+4
        cpu 68000
""" + NEXT68 + "        cpu 68000\n" + AFTER68

# c09: seek back with NO bytes after it, then close.
SHAPES["c09_seek_no_bytes_then_cpu"] = HEAD + """\
Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
        org Start+2
        cpu 68000
""" + NEXT68 + "        cpu 68000\n" + AFTER68

# c10: `cpu z80` with no phase after a host seek (stage 1's refused shape).
SHAPES["c10_cpu_z80_unphased"] = HEAD + SEEK68 + """\
        cpu z80
L_next: db 12h,34h
        cpu 68000
""" + AFTER68

# c11: `cpu z80` + `phase` after a host seek: the Z80 block is image content.
SHAPES["c11_cpu_z80_phased"] = HEAD + SEEK68 + """\
        cpu z80
        phase 8000h
L_next: db 12h,34h
        dw $
        dephase
        cpu 68000
""" + AFTER68

# c12: AS `section` / `endsection` after a seek (a symbol-scope construct in AS:
# a label made inside it is local unless declared `public`, which is why the
# head table can only see L_next through the `public` line).
SHAPES["c12_section_endsection"] = HEAD + SEEK68 + """\
        section blk
        public L_next
L_next: dc.w $1234
        dc.l *
        endsection blk
        cpu 68000
""" + AFTER68

# c13: a Z80-host program: the same seek-then-close with the image CPU a Z80.
# The head table is Z80 words, so the head longwords become two words each.
SHAPES["c13_z80_host_cpu_same"] = """\
        cpu z80
        org 0
        dw L_next
        dw L_after
Start:  db 0AAh,0BBh,0CCh,0DDh
        org Start+1
        db 0EEh
        cpu z80
L_next: db 12h,34h
        dw $
        cpu z80
L_after: db 56h,78h
"""

# c14: the same as c01 but with the closer a cpu-changing `restore` out of a
# 68000 section: the saved CPU is a Z80 declared before any content, so the
# host (first section with content) is still the 68000 and the seek section
# is a host section. The Z80 after the restore is phased, so it is image.
SHAPES["c14_restore_out_of_host"] = """\
        cpu z80
        save
        cpu 68000
        org 0
        dc.l L_next
        dc.l L_after
""" + SEEK68 + """\
        restore
        phase 8000h
L_next: db 12h,34h
        dw $
        dephase
        cpu 68000
""" + AFTER68

# c15: control. A `restore` that does NOT change the CPU does not close the
# section, so the bytes after it overwrite in place, exactly as in asl.
SHAPES["c15_restore_same_cpu_control"] = HEAD + """\
        save
""" + SEEK68 + """\
        restore
""" + NEXT68 + "        cpu 68000\n" + AFTER68

# c16: the section's tail behind the cursor is a trailing reservation, not
# bytes. Seek back to where the `ds.b` began, then close. The next section's
# bytes land on ground no byte was written to.
SHAPES["c16_trailing_ds_then_seek_to_it"] = HEAD + """\
Start:  dc.w $AAAA,$BBBB
        ds.b 4
        org Start+4
        cpu 68000
""" + NEXT68 + "        cpu 68000\n" + AFTER68

# c17: the next section writes nothing into the stale tail: its label is at the
# cursor, and an `org` moves it past the old extent before its first byte.
SHAPES["c17_next_orgs_past_the_tail"] = HEAD + SEEK68 + """\
        cpu 68000
L_next:
        org Start+8
        dc.w $1234
        dc.l *
        cpu 68000
""" + AFTER68

# c18: the next section writes nothing into the stale tail either, but steps
# over it with a reservation instead of an `org`.
SHAPES["c18_next_reserves_over_the_tail"] = HEAD + SEEK68 + """\
        cpu 68000
L_next: ds.b 4
        dc.w $1234
        dc.l *
        cpu 68000
""" + AFTER68

# c19: after the close, an `org` of the author's own puts the next code inside
# the stale tail, but not at the cursor the seek left there.
SHAPES["c19_org_into_the_tail_off_the_cursor"] = HEAD + SEEK68 + """\
        cpu 68000
        org Start+6
""" + NEXT68 + "        cpu 68000\n" + AFTER68

# c20: after the close, other code first, then an `org` of the author's own back
# to exactly the cursor the seek left.
SHAPES["c20_org_back_to_the_cursor_after_other_code"] = HEAD + SEEK68 + """\
        cpu 68000
        org $20
        dc.w $9999
        org Start+4
""" + NEXT68 + "        cpu 68000\n" + AFTER68


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, body in sorted(SHAPES.items()):
        with open(os.path.join(OUT, name + ".asm"), "w") as f:
            f.write("; shape " + name + ", written by gen.py\n" + body)
    print("PROBES_WRITTEN=%d" % len(SHAPES))


if __name__ == "__main__":
    main()
