#!/usr/bin/env bash
# run_sigil.sh BINARY probe...  : run each probe through a sigil binary, print its diagnostics.
BIN="$1"; shift
cd /home/volence/sonic_hacks/.scratch/as-macro-diag/probes || exit 1
echo "sigil md5: $(md5sum "$BIN" | cut -d' ' -f1)  path: $BIN"
for f in "$@"; do
    echo "=== $f.asm"
    "$BIN" "$f.asm" -o /dev/null 2>&1 | grep -E 'error|warning|note'
    echo "SIGIL_EXIT=${PIPESTATUS[0]}"
done
