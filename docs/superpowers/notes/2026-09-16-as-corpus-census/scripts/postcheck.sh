#!/usr/bin/env bash
# postcheck.sh: do build.lua's post-p2bin steps change any byte at these revisions?
# Hand-runs the exact asl and p2bin lines common.lua's assemble_file() runs, keeps
# p2bin's RAW output, and compares it against the ROM build.lua wrote (which has had
# amend_sound_driver_size, where it exists, and fix_header applied on top).
set -u
S=/home/volence/sonic_hacks/.scratch/as-corpus-census
run() { # run <tag> <tree> <root> <hdrflag: -c|""> <p2bin arg string>
  TAG="$1"; T="$2"; ROOT="$3"; HDR="$4"; P2ARGS="$5"
  AS="$T/build_tools/Linux-x86_64/asl"; P2="$T/build_tools/Linux-x86_64/p2bin"
  echo "== $TAG"
  echo "   asl md5 $(md5sum "$AS" | cut -d' ' -f1)   p2bin md5 $(md5sum "$P2" | cut -d' ' -f1)"
  ( cd "$T" && rm -f "$ROOT.p" "$ROOT.h" "$ROOT.log" hand.bin
    "$AS" -xx -n -q -A -L -U -E -i . $HDR "$ROOT.asm"; echo "   ASL_EXIT=$?"
    if [ -f "$ROOT.log" ]; then echo "   ASL LOG PRESENT (diagnostics):"; sed 's/^/     /' "$ROOT.log"; else echo "   no $ROOT.log: asl emitted no diagnostics"; fi
    if [ "$HDR" = "-c" ]; then
      "$P2" $P2ARGS "$ROOT.p" hand.bin "$ROOT.h"; echo "   P2BIN_EXIT=$?"
      echo "   share file after p2bin: $(head -c 80 "$ROOT.h" | tr -d '\n')"
    else
      "$P2" $P2ARGS "$ROOT.p" hand.bin; echo "   P2BIN_EXIT=$?"
    fi
    echo "   p2bin RAW output: md5 $(md5sum hand.bin | cut -d' ' -f1) size $(stat -c%s hand.bin)"
  )
}
run "Sonic 1 (no header file, no amend step; fix_header only)" "$S/trees/s1disasm-post" sonic "" "-p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after"
run "Sonic 2 (-c header file, amend_sound_driver_size + fix_header)" "$S/trees/s2disasm-post" s2 "-c" "-p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after"
echo POSTCHECK_END
