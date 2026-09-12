#!/usr/bin/env bash
# Run every probe through each named binary and record exit, stdout, stderr and the
# image. Usage: probes.sh <outdir> <label>=<binary> [<label>=<binary> ...]
set -u
S=/home/volence/sonic_hacks/.scratch/second-space-pin
OUT=$1; shift
mkdir -p "$OUT"
for pair in "$@"; do
    label=${pair%%=*}; bin=${pair#*=}
    echo "$label $(md5sum < "$bin" | cut -c1-32) $bin" >> "$OUT/BINARIES"
    for d in "$S"/probes/p*/; do
        name=$(basename "$d")
        dest="$OUT/$label/$name"
        mkdir -p "$dest"
        mapfile -t extra < "$d/args"
        rm -f "$d/out.bin"
        ( cd "$d" && "$bin" root.asm -o out.bin "${extra[@]}" > "$dest/stdout" 2> "$dest/stderr"; echo $? > "$dest/exit" )
        if [[ -f $d/out.bin ]]; then
            mv "$d/out.bin" "$dest/out.bin"
            xxd "$dest/out.bin" > "$dest/out.hex"
        else
            echo "no image" > "$dest/out.hex"
        fi
    done
done
