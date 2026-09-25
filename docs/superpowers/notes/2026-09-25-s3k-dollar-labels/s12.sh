#!/usr/bin/env bash
# s12.sh <tag> <sigil>: Sonic 1 and Sonic 2 whole-ROM runs with the census's (Q3)
# roots and arguments, on this parcel's gen trees; image md5/CRC32/size per run.
S=/home/volence/sonic_hacks/.scratch/s3k-dollar-labels
TAG="$1"; SB="$2"
LOG=$S/logs/s12-$TAG.log
{
  echo "pwd=$(pwd) sigil: $SB md5 $(md5sum < "$SB" | cut -c1-32)"
  "$SB" --version | sed -n 1,2p
  for spec in "s1:s1disasm-gen:sonic.asm:-p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after" \
              "s2:s2disasm-gen:s2.asm:-p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after"; do
    IFS=: read -r n tree root args <<< "$spec"
    out=$S/runs/$n-$TAG.bin; rm -f "$out"
    ( cd "$S/trees/$tree" && "$SB" "$root" -o "$out" $args ) > "$S/runs/$n-$TAG.stdout" 2> "$S/runs/$n-$TAG.stderr"
    echo "$n rc=$? stderr lines $(wc -l < "$S/runs/$n-$TAG.stderr")"
    [ -f "$out" ] && python3 -c 'import zlib,sys,hashlib;d=open(sys.argv[1],"rb").read();print("  %s md5 %s crc32 %08x size %d"%(sys.argv[1].split("/")[-1],hashlib.md5(d).hexdigest(),zlib.crc32(d)&0xffffffff,len(d)))' "$out"
  done
} > "$LOG" 2>&1
echo S12_END_$TAG >> "$LOG"
cat "$LOG"
