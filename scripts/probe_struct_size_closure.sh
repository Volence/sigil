#!/usr/bin/env bash
# Reproduction: when is a struct's `(size: N)` declaration actually checked?
#
# `check_struct_size` (crates/sigil-frontend-emp/src/layout.rs) runs inside
# `layout_of_struct`, which is DEMAND-DRIVEN: it fires only when something
# forces the struct's layout. So the declaration is a guarantee about the
# structs a build happens to reach, not about the structs a root declares.
#
# Each arm below is one shape an author might be in. The script prints, per
# arm, the exit status and the full diagnostic text, so the negative arms are
# read beside a positive control rather than on their own (an absent
# diagnostic proves nothing until the same probe is shown to produce one).
#
# Usage:  scripts/probe_struct_size_closure.sh [path-to-sigil-binary]
set -u

SIGIL=${1:-}
if [ -z "$SIGIL" ]; then
    echo "usage: $0 <path-to-sigil-binary>" >&2
    exit 2
fi
if [ ! -x "$SIGIL" ]; then
    echo "not executable: $SIGIL" >&2
    exit 2
fi

S=$(mktemp -d)
trap 'rm -rf "$S"' EXIT

# The one struct under test, byte-identical in every arm: 12 bytes of fields,
# a deliberately wrong `(size: 99)`.
BAD_STRUCT='pub struct Sst (size: 99) { id: u16, x_pos: u16, sst_custom: [u8; 8] }'

run_arm() {
    # run_arm <name> <target.emp relative to root> <expectation>
    local name=$1 target=$2 expect=$3
    echo "===== ARM ${name}  (expected: ${expect}) ====="
    "$SIGIL" emp "$S/$name/$target" --root "$S/$name" -o "$S/$name/out.bin" \
        >"$S/$name/stdout" 2>"$S/$name/stderr"
    local rc=$?
    echo "exit=$rc"
    if grep -q "declared size 99" "$S/$name/stderr"; then
        echo "size check: FIRED"
    else
        echo "size check: SILENT"
    fi
    echo "--- stdout:"; cat "$S/$name/stdout"
    echo "--- stderr:"; cat "$S/$name/stderr"
    echo
}

# ---------------------------------------------------------------------------
# A: the module declaring the bad struct is NOT in the target's use closure.
# ---------------------------------------------------------------------------
mkdir -p "$S/A/lib"
echo "module lib.types
$BAD_STRUCT" > "$S/A/lib/types.emp"
cat > "$S/A/main.emp" <<'EOF'
module main
proc tick (a0: *u8) {
    move.w #1, d0
    rts
}
EOF
run_arm A main.emp "SILENT if the caveat holds"

# ---------------------------------------------------------------------------
# B: POSITIVE CONTROL. Same struct bytes, module used, field dereferenced.
# ---------------------------------------------------------------------------
mkdir -p "$S/B/lib"
cp "$S/A/lib/types.emp" "$S/B/lib/types.emp"
cat > "$S/B/main.emp" <<'EOF'
module main
use lib.types.{Sst}
proc tick (a0: *Sst) {
    move.w x_pos(a0), d0
    rts
}
EOF
run_arm B main.emp "FIRED"
