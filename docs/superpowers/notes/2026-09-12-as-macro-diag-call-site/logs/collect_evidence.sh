#!/usr/bin/env bash
# collect_evidence.sh : one unfiltered asl transcript of every probe, then copy the
# probes, transcripts and scripts beside the note.
S=/home/volence/sonic_hacks/.scratch/as-macro-diag
D=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a46ef34b369e42312/docs/superpowers/notes/2026-09-12-as-macro-diag-call-site
cd "$S/probes" || exit 1
names=()
for f in *.asm; do names+=("${f%.asm}"); done
bash "$S/run_asl.sh" "${names[@]}" > "$S/logs/asl-all.txt" 2>&1
echo "asl transcript: $(grep -c '^===' "$S/logs/asl-all.txt") probes"
mkdir -p "$D/probes" "$D/scripts" "$D/logs"
cp "$S"/probes/*.asm "$S"/probes/*.inc "$D/probes/"
cp "$S/logs/asl-all.txt" "$D/logs/asl-transcript.txt"
cp "$S/logs/sigil-base-probes.txt" "$D/logs/sigil-before-probes.txt"
cp "$S/logs/sigil-c2-probes.txt" "$D/logs/sigil-after-probes.txt"
cp "$S/logs/s2-base.stderr" "$D/logs/s2-before.stderr"
cp "$S/logs/s2-c2.stderr" "$D/logs/s2-after.stderr"
cp "$S/logs/s2-base.meta" "$D/logs/s2-before.meta"
cp "$S/logs/s2-c2.meta" "$D/logs/s2-after.meta"
cp "$S/logs/s2-setcmp-base-c2.txt" "$D/logs/s2-setcmp.txt"
cp "$S/logs/s2gen-setcmp.txt" "$D/logs/s2gen-setcmp.txt"
cp "$S/logs/emp-compare.txt" "$D/logs/emp-compare.txt"
cp "$S/logs/byte-corpus.txt" "$D/logs/byte-corpus.txt"
cp "$S/bytecorpus/report.txt" "$D/logs/byte-corpus-report.txt"
for s in run_asl.sh run_sigil.sh probe_matrix.py setcmp.py run_s2.sh s2gen.lua emp_compare.sh byte_corpus.sh mutate.py run_mutations.sh census_scaffold.py run_census.sh; do
  cp "$S/$s" "$D/scripts/"
done
echo "copied: $(find "$D" -type f | wc -l) files"
