#!/usr/bin/env bash
# Run chosen probes through the reference toolchain: asl via asl_run (md5-pinned),
# then the p2bin beside it with the probe's own -z args. Records exits, image,
# listing. Usage: ref.sh
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ab5e0036e93441809
S=/home/volence/sonic_hacks/.scratch/second-space-pin
. "$W/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?
P2BIN="$ASLDIR/p2bin"
echo "asl md5 $(md5sum < "$ASL" | cut -c1-32)  p2bin md5 $(md5sum < "$P2BIN" | cut -c1-32)"
rm -rf "$S/ref-out"
for name in p01_empty1 p02_empty1_z p03_empty2 p04_phase_in_driver p09_z80_host_68k_space p11_two_spaces_z p12_empty2_z p16_org_no_section_open; do
    d="$S/ref-out/$name"
    mkdir -p "$d"
    cp "$S/probes/$name/root.asm" "$d/root.asm"
    mapfile -t extra < "$S/probes/$name/args"
    (
        cd "$d" || exit 2
        asl_run -xx -n -q -A -L -U -i . root.asm > asl.out 2>&1
        echo "ASL_EXIT=$?" >> asl.out
        "$P2BIN" root.p root.bin "${extra[@]}" > p2bin.out 2>&1
        echo "P2BIN_EXIT=$?" >> p2bin.out
    )
    echo "== $name: $(/usr/bin/grep -h -E 'ASL_EXIT|P2BIN_EXIT' "$d/asl.out" "$d/p2bin.out" | tr '\n' ' ')"
    if [[ -f $d/root.bin ]]; then
        xxd "$d/root.bin" | /usr/bin/grep -v "0000 0000 0000 0000 0000 0000 0000 0000"
        echo "size $(stat -c %s "$d/root.bin")"
    fi
done
