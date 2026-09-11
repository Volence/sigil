#!/usr/bin/env bash
# run_all_diag.sh <binary-name>... : the exact-line diagnostic diffs over the
# three corpora, each configuration run with every named binary.
S=/home/volence/sonic_hacks/.scratch/z80-half-registers
C=$S/corpora
bash "$S/run_diag.sh" "$C/s2disasm" s2.asm s2-plain "" "$@"
bash "$S/run_diag.sh" "$C/s2disasm" s2.asm s2-instr "-p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after" "$@"
bash "$S/run_diag.sh" "$C/s1disasm" sonic.asm s1-plain "" "$@"
bash "$S/run_diag.sh" "$C/s1disasm" sonic.asm s1-instr "-p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after" "$@"
bash "$S/run_diag.sh" "$C/skdisasm" sonic3k.asm s3k-plain "" "$@"
echo ALL_DIAG_END
