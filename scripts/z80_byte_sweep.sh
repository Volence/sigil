#!/usr/bin/env bash
# z80_byte_sweep.sh — assemble every self-contained Z80 instruction line the
# Sonic 1 and Sonic 2 corpora actually use with BOTH `asl` and `sigil`, and
# compare the emitted bytes.
#
# Why bytes and not a diagnostic count: under `CPU Z80` an unrecognized head
# used to be bound silently as a label, so an unimplemented mnemonic emitted
# nothing, exited 0, and made the blob short with no diagnostic at all. A
# diagnostic count cannot see that class by construction; a byte comparison can.
#
# Usage:  scripts/z80_byte_sweep.sh <s1-corpus-dir> <s2-corpus-dir> [sigil-binary]
#
# The two corpus directories are checkouts of `s1disasm` and `s2disasm`. `asl`
# and `p2bin` are taken from the S1 corpus's own `build_tools/Linux-x86_64/`
# (upstream AS, md5 61e672562465725a8c102288a7da9098) — S2 ships the flamewing
# fork under the same version string, so the binary is named, not assumed.
# `-U` is passed on every invocation.
#
# Exit status: 0 when every line's bytes agree; 1 when any line DIFFERS or when
# sigil fails a line asl assembles. A line neither tool assembles is SKIPPED and
# does not affect the exit status.
set -u

S1_DIR="${1:-}"
S2_DIR="${2:-}"
SIGIL="${3:-}"

if [ -z "$S1_DIR" ] || [ -z "$S2_DIR" ]; then
    echo "usage: $0 <s1-corpus-dir> <s2-corpus-dir> [sigil-binary]" >&2
    exit 2
fi

ASL="$S1_DIR/build_tools/Linux-x86_64/asl"
P2BIN="$S1_DIR/build_tools/Linux-x86_64/p2bin"
for t in "$ASL" "$P2BIN"; do
    [ -x "$t" ] || { echo "FATAL: missing tool $t" >&2; exit 2; }
done

if [ -z "$SIGIL" ]; then
    SIGIL="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/target/release/sigil"
fi
[ -x "$SIGIL" ] || { echo "FATAL: missing sigil binary $SIGIL" >&2; exit 2; }

echo "== tools =="
echo "asl    $ASL  md5 $(md5sum "$ASL" | cut -d' ' -f1)"
echo "p2bin  $P2BIN"
echo "sigil  $SIGIL  md5 $(md5sum "$SIGIL" | cut -d' ' -f1)"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ---- Collect the Z80 instruction lines the corpora use -----------------------
# The Z80 regions are the `CPU Z80` / `CPU Z80UNDOC` spans; their bounds are
# found by locating the cpu switch and its matching `restore`/`dephase`, so a
# corpus edit that moves them does not silently narrow the sweep.
z80_region() {  # file, start-pattern
    local f="$1" startpat="$2"
    local start
    start="$(grep -niE "$startpat" "$f" | head -1 | cut -d: -f1)"
    [ -n "$start" ] || return 0
    local end
    end="$(awk -v s="$start" 'NR>s && /^[[:blank:]]*(restore|dephase)([[:blank:]]|$)/ {print NR; exit}' "$f")"
    [ -n "$end" ] || end="$(wc -l < "$f")"
    sed -n "${start},${end}p" "$f"
}

{
    z80_region "$S2_DIR/s2.sounddriver.asm" '^[[:blank:]]*CPU[[:blank:]]+Z80'
    z80_region "$S1_DIR/sound/z80.asm"      '^[[:blank:]]*CPU[[:blank:]]+Z80'
    z80_region "$S1_DIR/sonic.asm"          '^[[:blank:]]*CPU[[:blank:]]+Z80'
} | sed -E 's/;.*//' \
  | grep -E '^[[:blank:]]+[A-Za-z_]' \
  | sed -E 's/^[[:blank:]]+//; s/[[:blank:]]+$//; s/[[:blank:]]+/ /g' \
  | sort -u > "$WORK/all.txt"

# Keep the lines whose operands are self-contained — registers, condition codes,
# numeric literals, and the punctuation that joins them. A line referencing a
# corpus symbol cannot be assembled standalone by either tool, so including it
# would only produce noise in both columns.
grep -iE "^(nop|ld|add|adc|sub|sbc|and|or|xor|cp|inc|dec|push|pop|ex|exx|ret|jr|jp|call|djnz|rrca|rlca|rla|rra|daa|cpl|ccf|halt|rst|scf|ei|di|bit|res|set|srl|rr|sla|rlc|rrc|rl|sra|neg|im|ldi|ldir|ldd|lddr|cpi|cpir|cpd|cpdr|in|ini|inir|ind|indr|out|outi|otir|outd|otdr|reti|retn|rld|rrd|sll)( |$)" "$WORK/all.txt" \
  | grep -viE "[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z_]" \
  | awk '{
      line=$0; sub(/^[^ ]+ ?/,"",line);
      if (line=="") { print $0; next }
      t=line; gsub(/[ \t]/,"",t);
      if (t ~ /^([aAbBcCdDeEhHlL]|af|AF|af\x27|AF\x27|bc|BC|de|DE|hl|HL|ix|IX|iy|IY|sp|SP|nz|NZ|z|Z|nc|NC|c|C|po|PO|pe|PE|p|P|m|M|[0-9][0-9A-Fa-f]*[hH]?|\$|[(),+\x27-]|0[xX][0-9A-Fa-f]+)+$/) print $0;
    }' | sort -u > "$WORK/lines.txt"

TOTAL="$(wc -l < "$WORK/lines.txt")"
echo "== $TOTAL self-contained Z80 instruction lines, from $(wc -l < "$WORK/all.txt") distinct corpus lines =="

# ---- Compare, line by line ---------------------------------------------------
same=0; differ=0; skipped=0; sigil_failed=0
: > "$WORK/report.txt"

while IFS= read -r line; do
    [ -n "$line" ] || continue
    printf '\tCPU Z80UNDOC\n\torg 0\n\t%s\n' "$line" > "$WORK/t.asm"

    rm -f "$WORK/t.p" "$WORK/t.bin"
    ( cd "$WORK" && "$ASL" -xx -n -q -A -U -i . t.asm ) >"$WORK/asl.log" 2>&1
    asl_rc=$?
    if [ $asl_rc -ne 0 ] || [ ! -f "$WORK/t.p" ]; then
        skipped=$((skipped+1))
        echo "SKIPPED  $line (asl does not assemble it)" >> "$WORK/report.txt"
        continue
    fi
    ( cd "$WORK" && "$P2BIN" t.p t.bin -r '$0-$FFFF' ) >/dev/null 2>&1
    asl_bytes="$(xxd -p "$WORK/t.bin" 2>/dev/null | tr -d '\n' | tr 'a-f' 'A-F')"

    sig_out="$("$SIGIL" "$WORK/t.asm" --hex 2>&1)"
    sig_rc=$?
    if [ $sig_rc -ne 0 ]; then
        sigil_failed=$((sigil_failed+1))
        echo "SIGIL-ERR $line  | asl=$asl_bytes | ${sig_out//$'\n'/ }" >> "$WORK/report.txt"
        continue
    fi
    sig_bytes="$(echo "$sig_out" | tr -d ' \n' | tr 'a-f' 'A-F')"

    if [ "$asl_bytes" = "$sig_bytes" ]; then
        same=$((same+1))
    else
        differ=$((differ+1))
        echo "DIFFERS  $line  | asl=$asl_bytes | sigil=$sig_bytes" >> "$WORK/report.txt"
    fi
done < "$WORK/lines.txt"

echo
echo "== findings =="
if [ -s "$WORK/report.txt" ]; then
    grep -v '^SKIPPED' "$WORK/report.txt" || true
    echo "-- skipped --"
    grep '^SKIPPED' "$WORK/report.txt" || true
else
    echo "(none)"
fi

echo
echo "== totals =="
echo "identical    $same"
echo "DIFFERS      $differ"
echo "SIGIL-ERR    $sigil_failed"
echo "skipped      $skipped"
echo "total        $TOTAL"

if [ $differ -gt 0 ] || [ $sigil_failed -gt 0 ]; then
    echo "RESULT: FAIL"
    exit 1
fi
echo "RESULT: PASS"
exit 0
