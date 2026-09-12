#!/usr/bin/env python3
"""Write the probe sources. Each probe is <dir>/root.asm plus <dir>/args (extra CLI
arguments, one per line). Lines starting with a tab are statements; column-0 lines
are labels or equates."""
import os

P = "/home/volence/sonic_hacks/.scratch/second-space-pin/probes"

HEAD = "\tcpu 68000\nSize1 equ $10\nSize2 equ $10\n\tdc.l 0, 0\n\torg $100\n\tdc.w $4E71\n"
TAIL1 = "\trestore\n\tpadding off\n\t!org $110\n\tdc.w $4E71\n"

probes = {
    # One empty section opens first after the org: the label between `!org 0` and
    # `cpu z80` opens an empty 68000 section; the Z80 code after it is the content.
    "p01_empty1": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tdi\n\tld a,1\n" + TAIL1, []),
    # The same, placed into the ROM by -z: the image is observable.
    "p02_empty1_z": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tdi\n\tld a,1\n" + TAIL1,
                     ["-z=0,uncompressed,Size1,after"]),
    # Two empty sections first: the 68000 label, then a Z80 label between two cpu lines.
    "p03_empty2": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\nInner:\n\tcpu z80\n\tdi\n\tld a,1\n" + TAIL1, []),
    # Rule 2 with a phase open: inside the driver an org leaves the section, a label
    # opens an empty Z80 section, a phase breaks it, and the phased byte is content.
    "p04_phase_in_driver": (HEAD + "\tsave\n\t!org 0\n\tcpu z80\n\tdi\n\torg 40h\nL40:\n\tphase 1000h\n\tdb 5\n\tdephase\n" + TAIL1, []),
    # A reservation leading the content section (content = Reserve + Data).
    "p05_reserve_then_code": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tds.b 4\n\tdi\n" + TAIL1, []),
    # Control: a reservation-only section decides the space and takes no pin; the code
    # after it is a later section with no org before it.
    "p06_reserve_only_first": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tds.b 4\n\tcpu z80\n\tdi\n" + TAIL1, []),
    # A later seek: the content section seeks back into its own bytes and a cpu line
    # closes it behind, so the next bytes land on its tail in the second space.
    "p07_later_seek": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tdb 1,2,3,4\n\torg 2\n\tdb 9\n\tcpu z80\nNext:\n\tdb 7\n" + TAIL1, []),
    # A collision inside the second space, empty section first.
    "p08_overlap_in_space": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tdb 1,2,3,4\n\torg 10h\n\tdb 5\n\torg 2\n\tdb 6\n" + TAIL1, []),
    # A Z80 program that enters a 68000 second space, empty Z80 section first.
    "p09_z80_host_68k_space": ("\tcpu z80\n\torg 0\n\tdb 1\n\torg 100h\nMark:\n\tcpu 68000\n\tdc.w $1234\n", []),
    # Control: the same empty-first shape inside the image. The pin never applies here;
    # the content section stays Chained behind the pinned empty one on both binaries.
    "p10_image_control": ("\tcpu 68000\n\tdc.l 0\n\torg $200\nMark:\n\tcpu 68000\n\tdc.w 1\n", []),
    # Two second spaces, each entered with an empty section first, both placed by -z.
    "p11_two_spaces_z": (HEAD + "\tsave\n\t!org 0\nD1:\n\tcpu z80\n\tdi\n\tld a,1\n\trestore\n\tpadding off\n"
                         "\t!org $110\n\tdc.w $4E71\n\tsave\n\t!org $1300\nD2:\n\tcpu z80\n\tdb 7,8\n"
                         "\trestore\n\tpadding off\n\t!org $120\n\tdc.w $4E71\n",
                         ["-z=0,uncompressed,Size1,after", "-z=1300h,uncompressed,Size2,after"]),
    # A reservation leading the content section, placed by -z at the first byte after it.
    "p13_reserve_then_code_z": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tds.b 4\n\tdi\n" + TAIL1,
                                ["-z=4,uncompressed,Size1,after"]),
    # The same, with -z naming the section's origin instead of its first byte.
    "p15_reserve_then_code_z0": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tds.b 4\n\tdi\n" + TAIL1,
                                 ["-z=0,uncompressed,Size1,after"]),
    # Control: a phase open at the org makes it an image placement (rule 3), so the
    # second-space pin cannot apply; an empty section still opens first.
    "p14_phase_rule3_control": (HEAD + "\tsave\n\t!org $104\nMark:\n\tcpu z80\n\tphase 0\n\tdb 1\n\tdephase\n" + TAIL1, []),
    # The org finds no section open (a cpu line closed it), so directive_org takes
    # its no-section arm; an empty section still opens first after it.
    "p16_org_no_section_open": ("\tcpu 68000\nSize1 equ $10\n\tdc.l 0, 0\n\torg $100\n\tdc.w $4E71\n\tcpu 68000\n"
                                "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tdi\n\tld a,1\n" + TAIL1,
                                ["-z=0,uncompressed,Size1,after"]),
    # Two empty sections first, placed by -z.
    "p12_empty2_z": (HEAD + "\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\nInner:\n\tcpu z80\n\tdi\n\tld a,1\n" + TAIL1,
                     ["-z=0,uncompressed,Size1,after"]),
}

for name, (src, args) in probes.items():
    d = os.path.join(P, name)
    os.makedirs(d, exist_ok=True)
    open(os.path.join(d, "root.asm"), "w").write(src)
    open(os.path.join(d, "args"), "w").write("".join(a + "\n" for a in args))
print("\n".join(sorted(probes)))
