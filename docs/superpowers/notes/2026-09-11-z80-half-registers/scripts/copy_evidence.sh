#!/usr/bin/env bash
# copy_evidence.sh: the parcel's evidence into the note's directory. Probe
# sources, asl's result tables and listing extract, the sigil comparison
# reports, the scripts, and the logs. Listings themselves are summarised in
# asl-listings.txt (code lines and pass footer of each), not copied whole.
set -u
S=/home/volence/sonic_hacks/.scratch/z80-half-registers
E=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a735fcd77afe45035/docs/superpowers/notes/2026-09-11-z80-half-registers
rm -rf "$E"
mkdir -p "$E/probes" "$E/scripts" "$E/logs/mutations" "$E/tables"
for r in m1 m2 m3 m4; do
    mkdir -p "$E/probes/$r"
    cp "$S/probes/$r"/*.asm "$E/probes/$r/"
    cp "$S/probes/$r/_run.log" "$E/probes/$r/asl-run.log"
    cp "$S/table-$r.tsv" "$E/tables/"
    for b in base c1; do cp "$S/probes/check-$r-$b.txt" "$E/tables/"; done
done
cp "$S/asl-listings.txt" "$E/tables/"
for f in run_asl.sh gen_probes.py check_sigil.py check_all.sh gen_test.py test_template.rs mutproof.py \
         run_luaref.sh run_nopost.sh setup_corpora.sh prestep_s2.lua build_nopost.lua run_diag.sh \
         run_all_diag.sh run_s2_whole.sh copy_evidence.sh; do
    if [ -f "$S/$f" ]; then cp "$S/$f" "$E/scripts/"; elif [ -f "$S/s2/$f" ]; then cp "$S/s2/$f" "$E/scripts/"; else echo "MISSING $f"; fi
done
for f in luaref.log nopost.log setup.log diag-base-v1.log diag-base-c1.log s2whole-v1.log s2whole-c1.log \
         mutproof.log binary-md5s.txt suite-frontend-as.summary.txt suite-cli.summary.txt \
         suite-harness-gate.summary.txt clippy.log; do
    [ -f "$S/$f" ] && cp "$S/$f" "$E/logs/" || echo "not yet: $f"
done
cp "$S/logs/mutations"/*.log "$E/logs/mutations/" 2>/dev/null
cp "$S/s2/luaref-build.log" "$E/logs/"
du -sh "$E"; find "$E" -type f | wc -l
echo COPY_END
