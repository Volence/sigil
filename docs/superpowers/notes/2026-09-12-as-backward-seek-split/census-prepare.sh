#!/usr/bin/env bash
# Rebuild the exposure census's inputs from committed state. Everything is
# written under <work-dir>, which must be inside a sigil worktree and never
# under /tmp (tmpfs). The corpus repositories are only READ, through
# `git archive`; their checkouts are never written.
#
#   census-prepare.sh <sigil-worktree> <work-dir> <sigil-rev>
#
# 1. `git archive <sigil-rev>` -> <work-dir>/instr-src, instrument.py over it,
#    `cargo build --release --bin sigil` into <sigil-worktree>/target/instr-<sha>,
#    <sha> the short commit <sigil-rev> names. One target directory per commit:
#    two archive copies built into ONE directory share cargo's unit names, and
#    cargo can judge a crate of the second copy fresh from the first copy's
#    build. Measured: a build of `c5a2564d` after one of `8f5e03c3` in a shared
#    `target/instr` recompiled sigil-frontend-as, sigil-harness and sigil-cli but
#    not sigil-link, and the binary ran `8f5e03c3`'s linker.
# 2. `git archive HEAD` of s1disasm, s2disasm, skdisasm -> <work-dir>/{s1,s2,sk},
#    then the repo's scripts/corpus-prepare.sh over each copy.
# 3. The two skdisasm wrapper roots (Sonic3_Complete = 0 and = 1).
#
# Then run:
#   census.sh <sigil-worktree>/target/instr-<sha>/release/sigil <out-dir> \
#     s1=<work-dir>/s1:sonic.asm s2=<work-dir>/s2:s2.asm sk=<work-dir>/sk:sonic3k.asm \
#     sk0=<work-dir>/sk:wrap_sk0.asm sk1=<work-dir>/sk:wrap_sk1.asm
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
WT="$(cd "$1" && pwd)"
mkdir -p "$2"
WORK="$(cd "$2" && pwd)"
REV="$3"
case "$WORK" in /tmp/*) echo "REFUSED: $WORK is under /tmp" >&2; exit 2 ;; esac
CORPORA=/home/volence/sonic_hacks

rm -rf "$WORK/instr-src" "$WORK/s1" "$WORK/s2" "$WORK/sk"
mkdir -p "$WORK/instr-src" "$WORK/s1" "$WORK/s2" "$WORK/sk"

git -C "$WT" archive --format=tar -o "$WORK/instr-src.tar" "$REV"
tar -xf "$WORK/instr-src.tar" -C "$WORK/instr-src"
python3 "$HERE/instrument.py" "$WORK/instr-src"
SHA="$(git -C "$WT" rev-parse --short "$REV^{commit}")"
(cd "$WORK/instr-src" && CARGO_TARGET_DIR="$WT/target/instr-$SHA" cargo build --release --bin sigil)
md5sum "$WT/target/instr-$SHA/release/sigil"

for pair in s1:s1disasm s2:s2disasm sk:skdisasm; do
    short="${pair%%:*}"; repo="${pair#*:}"
    git -C "$CORPORA/$repo" archive --format=tar -o "$WORK/$short.tar" HEAD
    tar -xf "$WORK/$short.tar" -C "$WORK/$short"
    echo "$short $repo $(git -C "$CORPORA/$repo" rev-parse --short HEAD)"
done
"$WT/scripts/corpus-prepare.sh" "$WORK/s1" | tail -n 1
"$WT/scripts/corpus-prepare.sh" "$WORK/s2" | tail -n 1
"$WT/scripts/corpus-prepare.sh" "$WORK/sk" buildSK.lua | tail -n 1
printf 'Sonic3_Complete = 0\n\tinclude "sonic3k.asm"\n' > "$WORK/sk/wrap_sk0.asm"
printf 'Sonic3_Complete = 1\n\tinclude "sonic3k.asm"\n' > "$WORK/sk/wrap_sk1.asm"
echo "CENSUS_PREPARE_END"
