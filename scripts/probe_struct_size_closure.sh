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

# ---------------------------------------------------------------------------
# STEP B shapes: the module IS reached, in one way or another. Which of these
# force the struct's layout, and therefore its `(size: N)` check?
# ---------------------------------------------------------------------------

# C: BLANK IMPORT. `use lib.types._` puts the module in the closure and
#    elaborates its body (crates/sigil-cli/tests/blank_import.rs), but nothing
#    names the struct.
mkdir -p "$S/C/lib"
cp "$S/A/lib/types.emp" "$S/C/lib/types.emp"
cat > "$S/C/main.emp" <<'EOF'
module main
use lib.types._
pub data D: [u8; 1] = [$11]
EOF
run_arm C main.emp "?"

# D: the module is in the closure for a DIFFERENT name. The struct is a
#    passenger in a module the build genuinely compiles.
mkdir -p "$S/D/lib"
echo "module lib.types
pub const Marker: u8 = \$EE
$BAD_STRUCT" > "$S/D/lib/types.emp"
cat > "$S/D/main.emp" <<'EOF'
module main
use lib.types.{Marker}
pub data D: [u8; 1] = [Marker]
EOF
run_arm D main.emp "?"

# E: the struct is IMPORTED BY NAME but never used.
mkdir -p "$S/E/lib"
cp "$S/A/lib/types.emp" "$S/E/lib/types.emp"
cat > "$S/E/main.emp" <<'EOF'
module main
use lib.types.{Sst}
pub data D: [u8; 1] = [$11]
EOF
run_arm E main.emp "?"

# F: the only use is `sizeof(Sst)` -- exactly how the engine's Region_Resolve
#    reaches Region.
mkdir -p "$S/F/lib"
cp "$S/A/lib/types.emp" "$S/F/lib/types.emp"
cat > "$S/F/main.emp" <<'EOF'
module main
use lib.types.{Sst}
pub const Stride: u16 = sizeof(Sst)
pub data D: [u8; 1] = [$11]
proc tick () {
    move.w #Stride, d0
    rts
}
EOF
run_arm F main.emp "?"

# G: the struct is a FIELD TYPE of another struct that is itself used.
#    NOTE both names must be imported: `use lib.types.{Holder}` ALONE fails with
#    `unknown type: Sst` at the nested field, because a name-list import injects a
#    CLONE that re-evaluates in the IMPORTING scope (blank_import.rs's note), where
#    the un-imported field type does not resolve. That is a separate observation,
#    not this probe's subject -- and note it is an error, so the shape is not
#    silently wrong.
mkdir -p "$S/G/lib"
echo "module lib.types
$BAD_STRUCT
pub struct Holder { h_id: u16, h_sst: Sst }" > "$S/G/lib/types.emp"
cat > "$S/G/main.emp" <<'EOF'
module main
use lib.types.{Sst, Holder}
proc tick (a0: *Holder) {
    move.w h_id(a0), d0
    rts
}
EOF
run_arm G main.emp "?"

# H: MITIGATION CANDIDATE. A `pub vars` overlay on the struct, in the struct's
#    own module, with no consumer reference at all (the always-on overlay
#    validation the cross-module CLI test leans on).
mkdir -p "$S/H/lib"
echo "module lib.types
$BAD_STRUCT
pub vars PlantV: sst_custom { timer: u8 }" > "$S/H/lib/types.emp"
cat > "$S/H/main.emp" <<'EOF'
module main
use lib.types._
pub data D: [u8; 1] = [$11]
EOF
run_arm H main.emp "?"

# I: MULTI-TARGET. One root, two entry points; only the debug entry reaches the
#    module. Build the release entry -- the realistic shape where an author's
#    other target does check it.
mkdir -p "$S/I/lib"
cp "$S/A/lib/types.emp" "$S/I/lib/types.emp"
cat > "$S/I/debug.emp" <<'EOF'
module debug
use lib.types.{Sst}
proc dtick (a0: *Sst) {
    move.w x_pos(a0), d0
    rts
}
EOF
cat > "$S/I/release.emp" <<'EOF'
module release
pub data D: [u8; 1] = [$11]
EOF
run_arm I release.emp "?"

# J: RE-EXPORT / INTERMEDIARY. main uses lib.api, which imports the struct;
#    main itself never names it.
mkdir -p "$S/J/lib"
cp "$S/A/lib/types.emp" "$S/J/lib/types.emp"
cat > "$S/J/lib/api.emp" <<'EOF'
module lib.api
use lib.types.{Sst}
pub const AK: u8 = 1
EOF
cat > "$S/J/main.emp" <<'EOF'
module main
use lib.api.{AK}
pub data D: [u8; 1] = [AK]
EOF
run_arm J main.emp "?"

# K: DEAD CODE IN A LIVE MODULE. The struct is dereferenced, but only inside a
#    proc nothing calls, in a module that is in the closure.
mkdir -p "$S/K/lib"
cp "$S/A/lib/types.emp" "$S/K/lib/types.emp"
cat > "$S/K/main.emp" <<'EOF'
module main
use lib.types.{Sst}
pub data D: [u8; 1] = [$11]
proc never_called (a0: *Sst) {
    move.w x_pos(a0), d0
    rts
}
EOF
run_arm K main.emp "?"

# L: THE TARGET MODULE ITSELF declares the bad struct and never references it.
#    No closure question arises at all -- this is the file the author handed the
#    compiler.
mkdir -p "$S/L"
echo "module main
$BAD_STRUCT
pub data D: [u8; 1] = [\$11]" > "$S/L/main.emp"
run_arm L main.emp "?"

# M: the target module declares it AND uses it -- L's positive control.
mkdir -p "$S/M"
echo "module main
$BAD_STRUCT
pub data D: [u8; 1] = [\$11]
proc tick (a0: *Sst) {
    move.w x_pos(a0), d0
    rts
}" > "$S/M/main.emp"
run_arm M main.emp "FIRED"

# N: MITIGATION an author can write by hand today -- a module-level `ensure`
#    over `sizeof`, in the struct's own module, behind a blank import.
mkdir -p "$S/N/lib"
echo "module lib.types
$BAD_STRUCT
ensure(sizeof(Sst) == 99, \"Sst is not 99 bytes\")" > "$S/N/lib/types.emp"
cat > "$S/N/main.emp" <<'EOF'
module main
use lib.types._
pub data D: [u8; 1] = [$11]
EOF
run_arm N main.emp "?"

# O: does the arm-N remedy survive a NAME-LIST import? blank_import.rs says a
#    name list injects a clone and does NOT elaborate the callee module, so a
#    module-level `ensure` beside the declaration may never run.
mkdir -p "$S/O/lib"
echo "module lib.types
$BAD_STRUCT
ensure(sizeof(Sst) == 99, \"Sst is not 99 bytes\")" > "$S/O/lib/types.emp"
cat > "$S/O/main.emp" <<'EOF'
module main
use lib.types.{Sst}
pub data D: [u8; 1] = [$11]
EOF
run_arm O main.emp "?"

# P: same remedy, name-list import, and the struct IS dereferenced.
mkdir -p "$S/P/lib"
cp "$S/O/lib/types.emp" "$S/P/lib/types.emp"
cat > "$S/P/main.emp" <<'EOF'
module main
use lib.types.{Sst}
proc tick (a0: *Sst) {
    move.w x_pos(a0), d0
    rts
}
EOF
run_arm P main.emp "?"

# Q: the remedy in the DECLARING module when that module is the TARGET (arm L's
#    shape). This is the one an author can apply with no import discipline at all.
mkdir -p "$S/Q"
echo "module main
$BAD_STRUCT
ensure(sizeof(Sst) == 99, \"Sst is not 99 bytes\")
pub data D: [u8; 1] = [\$11]" > "$S/Q/main.emp"
run_arm Q main.emp "?"
