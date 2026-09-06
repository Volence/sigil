#!/usr/bin/env bash
# matrix.sh -- for every value-binding form, ask BOTH assemblers which parent a
# following `.local` attached to, in BOTH directions.
#
#   ./matrix.sh [<sigil binary>]
#
# Two probes per form, generated into ./gen/:
#   <form>_prev.asm   references `Anchor.zz`  -- resolves iff NO scope opened
#   <form>_bind.asm   references `<Name>.zz`  -- resolves iff a scope DID open
#
# The local is spelled `zz` and occurs exactly once per file, and `Anchor` and
# the binder name are distinct, so the spelling that resolves names the parent
# outright.  A local name that existed under both candidates could not.
#
# Every asl run reports its exit status AND its ASL_DIAG completeness (via
# asl_run), and this prints both: a run whose pass loop stopped early never
# looked for `symbol undefined`, which is the whole class under measurement.
set -u
cd "$(dirname "$0")" || exit 2
SIGIL="${1:-../../../../.target-land/release/sigil}"
if [ ! -x "$SIGIL" ]; then
    echo "FATAL: no executable sigil at $SIGIL" >&2
    exit 2
fi
. ../asl-reference/asl_ref.sh || exit $?

mkdir -p gen
echo "SIGIL   $SIGIL  md5=$(md5sum "$SIGIL" | cut -d' ' -f1)"
echo "ASL     $ASL  md5=$(md5sum "$ASL" | cut -d' ' -f1)"
echo

emit() { # $1=file $2=binder-line
    cat > "$1" <<EOF
	cpu	68000
	padding	off
	org	\$1000
Anchor:
	nop
$2
.zz:
	nop
	dc.l	$3
	end
EOF
}

printf '%-14s %-9s | %-28s | %-28s\n' form direction asl sigil
printf '%s\n' "---------------------------------------------------------------------------------------"

run_one() {
    local form="$1" dir="$2" file="$3"
    local a_out a_rc a_diag s_out s_rc a_val s_val
    ( cd gen && rm -f "$(basename "${file%.asm}").lst" )
    a_out="$(cd gen && asl_run -xx -n -q -A -L -U -i . "$(basename "$file")" 2>&1)"
    a_rc=$?
    a_diag="$(printf '%s' "$a_out" | /usr/bin/grep -o 'ASL_DIAG=[A-Za-z]*' | head -1)"
    if [ "$a_rc" -eq 0 ]; then
        # The dc.l's byte column, from the listing line that carries it.
        a_val="$(/usr/bin/grep -E '^ +[0-9]+/ +[0-9A-F]+ : [0-9A-F]{4} [0-9A-F]{4} +	dc\.l' \
                 "gen/$(basename "${file%.asm}").lst" \
                 | /usr/bin/sed -E 's/^.*: ([0-9A-F]{4}) ([0-9A-F]{4}).*/$\1\2/')"
        a_val="ok $a_val"
    else
        a_val="rc=$a_rc $(printf '%s' "$a_out" | /usr/bin/grep -o 'error #[0-9]*: [a-z ]*' | head -1)"
    fi
    s_out="$("$SIGIL" "$file" --hex 2>&1)"
    s_rc=$?
    if [ "$s_rc" -eq 0 ]; then
        s_val="ok \$$(printf '%s' "$s_out" | /usr/bin/tr -d ' \n' | /usr/bin/tail -c 8)"
    else
        s_val="rc=$s_rc $(printf '%s' "$s_out" | /usr/bin/grep -o 'unresolved symbol `[^`]*`' | head -1)"
    fi
    printf '%-14s %-9s | %-28s | %-28s  %s\n' "$form" "$dir" "$a_val" "$s_val" "$a_diag"
}

# form-name  binder line (label field holds the name `Bn`)
while IFS='|' read -r form line; do
    [ -n "$form" ] || continue
    emit "gen/${form}_prev.asm" "$line" "Anchor.zz"
    emit "gen/${form}_bind.asm" "$line" "Bn.zz"
    run_one "$form" "PREV" "gen/${form}_prev.asm"
    run_one "$form" "BINDER" "gen/${form}_bind.asm"
done <<'FORMS'
set|Bn	set	5
set_colon|Bn:	set	5
equ|Bn	equ	5
equ_colon|Bn:	equ	5
assign|Bn	=	5
assign_colon|Bn:	=	5
colon_eq|Bn	:=	5
eval|Bn	eval	5
set_comma|	set	Bn,5
eval_comma|	eval	Bn,5
str_set|Bn	set	"ab"
str_equ|Bn	equ	"ab"
label_dir|Bn	label	*
plain_label|Bn:
FORMS
