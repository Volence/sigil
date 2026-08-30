#!/bin/bash
# capture_goldens.sh — the ONE-STEP fresh-build golden capture (Flip Stage 1 · S1.6).
#
# Captures ALL SEVEN Stage-2 comparand ROMs (six canonical/config + the crash-report-OFF
# `lean` shape) asl-derived WHILE ASL IS LIVE — the only
# moment the off-canonical Config-A/B are reproducible (they have no shipped file and
# their NATIVE reproduction is Stage-2-coupled; see PROVENANCE.md + the
# 2026-07-30-flip-stage1-demo-config-native-blocked.md fork note). For each target it
# records BOTH split-golden layers: the FULL FILE (assembled + convsym deb2 appendix)
# and the assembled-ROM ANCHOR (0..EndOfRom, header-neutral — the PRIMARY-class CRC,
# the drift-stable Stage-2 bar).
#
# It is STRUCTURALLY IMPOSSIBLE to capture a stale artifact: for each ROM the script
# (1) deletes the target, (2) rebuilds via the POSITIONAL game arg (`./build.sh <game>`
# / env-prefixed — NEVER `GAME=<game> ./build.sh`, which build.sh IGNORES:
# `GAME="${1:-sonic4}"` is positional, build.sh:4), and (3) ASSERTS the artifact
# reappeared newer than a pre-build marker (a real rebuild) before CRC-ing it. This
# guard catches the demo stale-baseline class: the false `2b71b37d/88738` plain-demo
# "golden" was a pre-existing demo.bin CRC'd WITHOUT a rebuild (compounded by the
# ignored `GAME=` env var); the TRUE plain demo bar is `18c64002/90776`.
#
# Config-A (DEBUG+hotkeys+mirror) writes to s4.debug.bin, and Config-B (sound-off) and
# lean (crash-report off) both write to s4.bin — so they CLOBBER the canonical
# references. This script captures the four canonical/demo goldens FIRST, then
# Config-A/B and lean into distinct golden blobs (config_a.bin / config_b.bin /
# lean.bin), then REBUILDS canonical s4.bin + s4.debug.bin so the aeon tree is left in
# its canonical state for every other gate that reads them. ORDER IS LOAD-BEARING:
# every s4.bin-clobbering capture must precede that restore.
#
# Usage:
#   SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
#   AEON_DIR=/path/to/aeon ./capture_goldens.sh [--write]
#     --write  freeze each fresh full file into this golden dir as a committed blob.
#              REFUSED unless SIGIL_GOLDEN_WRITE says which kind of write this is; see
#              THE WRITE GATE below. Without --write nothing is replaced: the script
#              captures and reports CRCs, which is what most hand runs actually want.
#
# Under --write the committed blobs are replaced only once ALL SEVEN have been captured,
# through the staged commit in atomic_freeze.sh — read its header for exactly what a kill
# at each point leaves behind, which is the reason the write path is not a bare `cp`.
#
# THE WRITE GATE. The committed goldens are the measuring instrument every byte gate and
# every paired freeze reads. `refreeze --freeze` moves them through a ritual that leaves a
# completion journal and a provenance entry naming the parcel and the aeon revision they
# were built from; THIS SCRIPT run by hand moves the same bytes and leaves neither. The
# two are one flag apart, so the hand form is refused unless the operator says in the
# environment which one it is:
#
#   SIGIL_GOLDEN_WRITE=refreeze      set by `refreeze` on the child it spawns.
#   SIGIL_GOLDEN_WRITE=unjournalled  the operator's own acknowledgement. Recorded, before
#                                    anything is built, in THE HAND-WRITE TRACE below.
#
# Nothing here stops an operator from spelling `refreeze` by hand: no shell gate can tell
# a forged caller from a real one. What it does is make the silent path a deliberate act
# instead of a slip of one flag, and give the deliberate act somewhere to leave a mark.
#
# THE HAND-WRITE TRACE is `.unjournalled-write` beside the blobs. Every acknowledged hand
# write appends to it, and if it cannot be appended to, the run is REFUSED — the whole
# point of the acknowledgement is the record, so a write that cannot be recorded does not
# happen. Every --write run prints it when it is present, so a later freeze reports the
# hand writes this checkout's goldens carry. It is untracked and it is never removed by a
# later journalled freeze: the record of a hand write outlives the blobs it wrote, and
# only a person can say it has been accounted for.
#
# ITS ABSENCE PROVES NOTHING, and must not be read as "no hand write happened". It is
# checkout-local and untracked, so a fresh clone starts without one; a hand write in
# another checkout leaves nothing here; and a forged `refreeze` leaves nothing anywhere.
# It is evidence when present, and silence otherwise.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=atomic_freeze.sh
source "$HERE/atomic_freeze.sh"
SIGIL_ROOT="$(cd "$HERE/../../.." && pwd)"
AEON="${AEON_DIR:-/home/volence/sonic_hacks/aeon}"
WRITE=0
[[ "${1:-}" == "--write" ]] && WRITE=1

# The hand-write trace: one file beside the blobs, appended to by every acknowledged hand
# write. See THE HAND-WRITE TRACE in the header for what its presence and its absence each
# mean.
UNJOURNALLED_TRACE="$HERE/.unjournalled-write"

# trace_line <text> — append one stamped line. Returns non-zero if it cannot be written,
# which the caller turns into a refusal rather than an unrecorded write.
trace_line() {
    printf '%s  %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1" >> "$UNJOURNALLED_TRACE"
}

# who <field> <command...> — a record field, or the field named as unresolved. A field
# that renders empty when its command is missing reads as "nobody", which is a stronger
# claim than the record is entitled to make.
who() {
    local label="$1"; shift
    local v; v="$("$@" 2>/dev/null)" || v=""
    printf '%s=%s' "$label" "${v:-<unresolved>}"
}

# THE WRITE GATE, consulted before AEON_DIR and SIGIL_EMIT. It is about intent rather than
# environment, so it costs nothing to ask first — and asking first is what makes it
# provable without a provisioned aeon tree and without a single ROM being built.
if [[ "$WRITE" == "1" ]]; then
    case "${SIGIL_GOLDEN_WRITE:-}" in
    refreeze)
        ;;
    unjournalled)
        # Recorded BEFORE anything is built. A run killed later then leaves a line for a
        # write that never landed, which overstates; the alternative understates, and
        # under-reporting a golden write is the defect this gate exists to close.
        if ! trace_line "started    $(who user id -un)  $(who host uname -n)  $(who sigil git -C "$HERE" rev-parse --short HEAD)  cwd=$PWD  AEON_DIR=${AEON_DIR:-<unset>}"; then
            echo "ERROR: cannot record this hand write in $UNJOURNALLED_TRACE." >&2
            echo "       An acknowledged hand write is allowed BECAUSE it is recorded, so a" >&2
            echo "       write that cannot be recorded does not happen. Fix the path (a" >&2
            echo "       read-only golden directory is the usual cause) and re-run." >&2
            exit 1
        fi
        echo ">> UNJOURNALLED WRITE acknowledged — recorded in $UNJOURNALLED_TRACE"
        ;;
    *)
        cat >&2 <<GATE
ERROR: refusing an unjournalled golden write.

  --write replaces the committed golden blobs in
    $HERE
  and those blobs are the measuring instrument every byte gate and every paired freeze
  reads. Moved from here, they move with no journal and no provenance entry: nothing in
  the tree records that the bar moved, or which aeon revision it was built from.

  THE JOURNALLED PATH runs this script for you, then the size tables, then the pins, then
  the provenance append — and records each step as it completes:

    SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \\
    SIGIL_BUILD=<sigil>/target/release/sigil \\
    AEON_DIR=<clean checkout of a committed aeon SHA> \\
      cargo run -p sigil-harness --bin refreeze -- \\
        --freeze <parcel-name> --ab <A/B-evidence-ref>

  TO CAPTURE AND COMPARE WITHOUT MOVING ANYTHING, drop --write. The CRCs are printed
  either way; only --write replaces a blob.

  TO WRITE THE GOLDENS BY HAND ANYWAY, say so. The run is then recorded in
  $UNJOURNALLED_TRACE:

    SIGIL_GOLDEN_WRITE=unjournalled ./capture_goldens.sh --write
GATE
        exit 1
        ;;
    esac
    # Printed on EVERY write run, journalled or not: a freeze should say what hand writes
    # the goldens it is about to replace were carrying.
    if [[ -e "$UNJOURNALLED_TRACE" ]]; then
        echo ">> NOTE: this checkout's goldens carry hand writes recorded in $UNJOURNALLED_TRACE:"
        sed 's/^/     /' "$UNJOURNALLED_TRACE" || true
        echo "   A journalled freeze does not remove that file. Remove it by hand once the"
        echo "   writes above are accounted for."
    fi
fi

[[ -d "$AEON" ]] || { echo "ERROR: AEON_DIR not a dir: $AEON"; exit 1; }
if [[ -z "${SIGIL_EMIT:-}" || ! -x "${SIGIL_EMIT:-}" ]]; then
    echo "ERROR: set SIGIL_EMIT to <sigil>/target/release/emit_sound_blob (sound-on builds need it)."
    exit 1
fi
# The off-canonical Config-A/B shapes have no build.sh env path (post-flip build.sh
# forwards only --game/--debug to sigil); they build through `sigil build --config-*`
# directly. The canonical four still go through build.sh.
# Honor CARGO_TARGET_DIR (a shared-target worktree build) — else the crate's own target.
TARGET_DIR="${CARGO_TARGET_DIR:-$SIGIL_ROOT/target}"
SIGIL_BUILD="${SIGIL_BUILD:-$TARGET_DIR/release/sigil}"
[[ -x "$SIGIL_BUILD" ]] || { echo "ERROR: sigil build binary not at $SIGIL_BUILD (set SIGIL_BUILD)"; exit 1; }

# report <lst> <bin> — print both split-golden layers (full file + header-neutral
# assembled anchor). EndOfRom (the appendix boundary) is read from the .lst END line.
report() {
    python3 - "$1" "$2" <<'PY'
import zlib, re, sys
lst, binf = sys.argv[1], sys.argv[2]
end = None
for line in open(lst, errors='replace'):
    # asl marks the assembled length with a `… : END` line; sigil's own listing
    # (post-flip build.sh = sigil build) marks it with the `EndOfRom` label instead.
    m = re.search(r'/\s*([0-9A-Fa-f]+)\s*:\s+END\s*$', line) \
        or re.search(r'/\s*([0-9A-Fa-f]+)\s*:\s+EndOfRom:\s*$', line)
    if m:
        end = int(m.group(1), 16)  # last match wins
raw = open(binf, 'rb').read()
if end is None or end > len(raw):
    print(f"FAIL: bad EndOfRom {end} for {binf} (len {len(raw)})"); sys.exit(1)
# header-neutral: zero the checksum ($18E..$190) and ROM-end pointer ($1A4..$1A8)
d = bytearray(raw)
for i in range(0x18E, 0x190): d[i] = 0
for i in range(0x1A4, 0x1A8): d[i] = 0
full_crc = zlib.crc32(raw) & 0xffffffff
anc_crc  = zlib.crc32(bytes(d[:end])) & 0xffffffff
print(f"full {full_crc:08x} / {len(raw)}    anchor {anc_crc:08x} / {end}  (EndOfRom {end:#x})")
PY
}

# capture <golden_name> <game> <out_rom> <out_lst> <env...>
capture() {
    local golden="$1" game="$2" rom="$3" lst="$4"; shift 4
    local path="$AEON/$rom" lstp="$AEON/$lst"
    local marker; marker="$(mktemp)"; sleep 0.01
    rm -f "$path"
    echo ">> $golden  ($game: ${*:-canonical})"
    ( cd "$AEON" && env "$@" SIGIL_EMIT="$SIGIL_EMIT" ./build.sh "$game" >/dev/null )
    [[ -f "$path" ]] || { echo "FAIL: $rom not produced"; rm -f "$marker"; exit 1; }
    [[ "$path" -nt "$marker" ]] || { echo "FAIL: $rom not newer than the pre-build marker (stale-capture guard)"; rm -f "$marker"; exit 1; }
    rm -f "$marker"
    printf "   "; report "$lstp" "$path"
    if [[ "$WRITE" == "1" ]]; then freeze_stage "$path" "$golden"; echo "   staged -> $golden"; fi
}

# capture_config <golden_name> <--config-a|--config-b> <out_rom> <out_lst>
# The off-canonical shapes: `sigil build --config-*` directly (the whole shape is
# fixed by the flag). Same stale-artifact-trap guard as `capture`.
capture_config() {
    local golden="$1" flag="$2" rom="$3" lst="$4"
    local path="$AEON/$rom" lstp="$AEON/$lst"
    local marker; marker="$(mktemp)"; sleep 0.01
    rm -f "$path"
    echo ">> $golden  (sigil build $flag)"
    ( cd "$AEON" && SIGIL_EMIT="$SIGIL_EMIT" "$SIGIL_BUILD" build --aeon . --native "$flag" \
        -o "$rom" --emit-lst "$lst" >/dev/null )
    [[ -f "$path" ]] || { echo "FAIL: $rom not produced"; rm -f "$marker"; exit 1; }
    [[ "$path" -nt "$marker" ]] || { echo "FAIL: $rom not newer than the pre-build marker (stale-capture guard)"; rm -f "$marker"; exit 1; }
    rm -f "$marker"
    printf "   "; report "$lstp" "$path"
    if [[ "$WRITE" == "1" ]]; then freeze_stage "$path" "$golden"; echo "   staged -> $golden"; fi
}

# The config/lean captures CLOBBER the canonical s4.bin / s4.debug.bin and are
# restored at the end, so an abort between the two leaves a config ROM sitting
# where every other gate reads the canonical one. Under `set -e` the contract
# closure gate makes that reachable for the first time: a baseline drift on any
# one shape aborts mid-sequence. The trap restores unconditionally.
restore_canonical() {
    ( cd "$AEON" && SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 >/dev/null 2>&1 \
        && DEBUG=1 SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 >/dev/null 2>&1 ) || true
}

# The staging area opens BEFORE the first build AND before the restore trap: a leftover
# from a killed run is a refusal that must cost nothing, neither a capture nor the two
# restore builds the trap would otherwise spend on a run that never touched the aeon tree.
[[ "$WRITE" == "1" ]] && freeze_open "$HERE"

# An abort before the commit leaves captures in the staging area that no longer describe
# a completed set; dropping them keeps the next run from refusing over output nothing
# wants. `freeze_abandon` declines once the commit loop has begun, which is the one case
# where the leftover is evidence rather than litter.
on_exit() {
    freeze_abandon || true
    restore_canonical
}
trap on_exit EXIT

echo "== Flip Stage 1 golden capture — all seven, fresh-build =="
echo "   aeon: $AEON  ($(cd "$AEON" && git rev-parse --short HEAD 2>/dev/null || echo '?'))"

# 1-4: the canonical + demo goldens (each writes its own filename; no clobber).
capture s4.bin        sonic4 s4.bin        s4.lst
capture s4.debug.bin  sonic4 s4.debug.bin  s4.debug.lst  DEBUG=1
capture demo.bin      demo   demo.bin      demo.lst
capture demo.debug.bin demo  demo.debug.bin demo.debug.lst DEBUG=1

# 5-6: the off-canonical configs (CLOBBER s4.debug.bin / s4.bin — captured into
# distinct golden blobs; canonical is rebuilt below). Built via `sigil build
# --config-*` directly (the stale SOUND_* env path through build.sh is gone post-flip).
capture_config config_a.bin  --config-a  s4.debug.bin  s4.debug.lst
capture_config config_b.bin  --config-b  s4.bin        s4.lst

# 7: the LEAN shape (crash-report OFF: no MD Debugger island, no deb2 appendix, faults
# route at ReleaseFault). Also CLOBBERS s4.bin — so it MUST come before the restore
# below, not after it.
capture_config lean.bin      --lean      s4.bin        s4.lst

# The set is complete, so the committed blobs are replaced now — before the restore
# below, which is about the aeon tree and cannot invalidate a capture that already
# happened. Sequencing it after would discard a good seven-target capture whenever a
# restore build failed.
if [[ "$WRITE" == "1" ]]; then
    freeze_commit
    echo ">> frozen: the committed goldens are this capture's set"
    # The second half of a hand write's record, written where the blobs actually moved.
    # The `started` line above says one was attempted; this one says one landed.
    if [[ "${SIGIL_GOLDEN_WRITE:-}" == "unjournalled" ]]; then
        trace_line "committed  the seven committed goldens are this hand run's capture" || true
    fi
fi

# RESTORE the canonical aeon references clobbered by Config-A/B/lean.
#
# The off-canonical captures above leave THEIR listings behind under the canonical
# names: config_a writes s4.debug.lst, config_b and lean write s4.lst. aeon's
# build-fatal checks derive over every shape whose listing is PRESENT, so a leftover
# off-canonical listing is read as the canonical shape's and judged against the
# canonical shape's budget. Clearing both listings first is what makes the two restore
# builds order-independent; sequencing them so the survivor happens to be correct would
# leave the same hazard armed behind a working build.
#
# The ROMs go too: a restore build that stops at a fatal check leaves the previous
# shape's binary in place, and a stale ROM whose CRC matches its pin perfectly is
# indistinguishable from a fresh one. Absent fails loudly; stale does not fail at all.
rm -f "$AEON/s4.bin" "$AEON/s4.debug.bin" "$AEON/s4.lst" "$AEON/s4.debug.lst"
echo ">> restoring canonical aeon s4.bin + s4.debug.bin ..."
( cd "$AEON" && SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 >/dev/null && DEBUG=1 SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 >/dev/null )
echo "   restored: $(cd "$AEON" && python3 -c "import zlib;print('s4.bin',f'{zlib.crc32(open(\"s4.bin\",\"rb\").read())&0xffffffff:08x}','/','s4.debug.bin',f'{zlib.crc32(open(\"s4.debug.bin\",\"rb\").read())&0xffffffff:08x}')")"

# The expected full-file bars live in golden/provenance.toml (the chain tip) —
# `refreeze --check` is the gate; no CRC literals are maintained here.
echo "== done — expected bars = the provenance chain tip (golden/provenance.toml; refreeze --check) =="