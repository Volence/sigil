#!/usr/bin/env bash
# nodefine.sh <tip> <base>: without -D nothing moves. S1 and S2 whole ROMs (census Q3
# commands) and the S&K wrapper root, each by the base and the tip binary, compared
# with each other and with the recorded identities.
set -u
TIP="$1"; BASE="$2"
S=/home/volence/sonic_hacks/.scratch/as-cli-define
ID="python3 $S/s3k/scripts/ident.py"
echo "tip:  $($TIP --version | sed -n 1p) md5 $(md5sum "$TIP" | cut -d' ' -f1)"
echo "base: $($BASE --version | sed -n 1p) md5 $(md5sum "$BASE" | cut -d' ' -f1)"
printf 'Sonic3_Complete = 0\n\tinclude "sonic3k.asm"\n' > "$S/s3k/trees/gen/wrapper.asm"
mkdir -p "$S/s12/out"
one() { # one <tag> <tree> <root> <args...>
  local tag="$1" tree="$2" root="$3"; shift 3
  for side in tip base; do
    local bin="$TIP"; [ $side = base ] && bin="$BASE"
    rm -f "$S/s12/out/$tag-$side.bin"
    ( cd "$tree" && "$bin" "$root" -o "$S/s12/out/$tag-$side.bin" "$@" ) > "$S/s12/out/$tag-$side.log" 2>&1
    echo "$tag $side: exit $? stderr+stdout lines $(wc -l < "$S/s12/out/$tag-$side.log")"
    $ID "$S/s12/out/$tag-$side.bin"
  done
  cmp "$S/s12/out/$tag-tip.bin" "$S/s12/out/$tag-base.bin" && echo "$tag: tip == base"
}
one s1 "$S/s12/s1disasm-gen" sonic.asm -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
one s2 "$S/s12/s2disasm-gen" s2.asm -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
one sk-wrapper "$S/s3k/trees/gen" wrapper.asm -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before
rm -f "$S/s3k/trees/gen/wrapper.asm"
echo "recorded: s1 09dadb5071eb35050067a32462e39c5f afe05eee; s2 9feeb724052c39982d432a7851c98d3e 7b905383; sk 4ea493ea4e9f6c9ebfccbdb15110367e 0658f691"
echo NODEFINE_END
