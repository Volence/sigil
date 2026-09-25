#!/usr/bin/env python3
"""gen_tests.py: emit the accepted-probe tests of as_bcd_pcindex.rs from the probe
sources and ASL'S OWN IMAGES (probes/<name>.asl.bin, written by probe.sh only from
a run that exited 0 with its pass loop complete). No expected byte comes from sigil."""
import os
import sys

S = "/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex"
P = S + "/probes"

ACCEPTED = [
    ("x01", "addx_and_subx_every_size_both_forms_and_the_bare_word_default"),
    ("x02", "abcd_and_sbcd_both_forms_bare_and_byte_suffixed"),
    ("x03", "negx_and_nbcd_over_the_data_alterable_modes"),
    ("pcx01", "pc_indexed_source_over_every_instruction_that_admits_it"),
    ("pcx02", "pc_indexed_backward_targets"),
    ("pcx03", "pc_indexed_movem_the_s3k_shape"),
    ("pcb01", "pc_indexed_displacement_boundaries_127_and_minus_128_accepted"),
    ("pcd01", "movem_through_d16_pc"),
    ("sk01", "s3k_extract_both_movem_sites_and_their_tables"),
    ("sk02", "s3k_extract_the_three_abcd_sites"),
    ("sk03", "s3k_extract_subx_at_4d252"),
    ("sk04", "s3k_extract_subx_at_5a036"),
    ("sk05", "s3k_extract_subx_at_5a05a"),
]

out = []
for name, fn in ACCEPTED:
    src = open(os.path.join(P, name + ".asm"), encoding="latin-1").read()
    asl = open(os.path.join(P, name + ".asl.bin"), "rb").read()
    assert "\"#" not in src
    out.append("/// Probe `%s`." % name)
    out.append("#[test]")
    out.append("fn %s() {" % fn)
    out.append("    assert_asl(")
    out.append("        r#\"%s\"#," % src)
    out.append("        \"%s\"," % asl.hex())
    out.append("    );")
    out.append("}")
    out.append("")
sys.stdout.write("\n".join(out))
