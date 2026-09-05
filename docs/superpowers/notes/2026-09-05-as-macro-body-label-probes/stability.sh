#!/usr/bin/env bash
# Oracle stability: asl 1.42 is known to answer differently across runs for at
# least one shape, so every probe this parcel relies on is run THREE times and
# the whole stream hashed. Three identical hashes, or the shape is excluded from
# the note and said so.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
for f in "$HERE"/p*.asm; do
    b="$(basename "$f" .asm)"
    printf '%s ' "$b"
    for i in 1 2 3; do
        # asl stamps THREE clock readings into a listing, and all three have to
        # go or this hash measures the machine instead of the assembler:
        #
        #   * the page banner's date and time;
        #   * the `DATE` and `TIME` builtins in the symbol table, which share
        #     their lines with real symbols and so cannot be dropped whole;
        #   * `N.NN seconds assembly time` in the trailer, which is the one that
        #     actually bites — it reads 0.00 on most runs and 0.01 on some, at no
        #     fixed rate, so a batch of three can come out identical by luck and
        #     be reported as stability.
        #
        # MEASURED, not assumed: six runs of `p1.asm` diffed byte for byte are
        # identical, and eight runs through this pipeline WITHOUT the seconds
        # rule differ on exactly one line, `0.00` vs `0.01 seconds assembly
        # time`. Everything asl says about the program is stable; the stopwatch
        # is not, and reporting the stopwatch as an oracle divergence is a
        # false red — which is worse than a missing check, because it teaches
        # the next reader to weaken it.
        printf '%s ' "$("$HERE/run.sh" "$b.asm" 2>&1 \
            | sed -E 's#[0-9]{2}/[0-9]{2}/[0-9]{4}#DATE#g
                      s#[0-9]{2}:[0-9]{2}:[0-9]{2}#TIME#g
                      s#^[0-9]+\.[0-9]+ seconds assembly time#SECONDS seconds assembly time#' \
            | md5sum | cut -c1-12)"
    done
    echo
done
