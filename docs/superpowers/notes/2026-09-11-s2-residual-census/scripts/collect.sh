#!/usr/bin/env bash
# collect.sh: copy the small text evidence into the committed directory.
S=/home/volence/sonic_hacks/.scratch/s2-residual-census
E=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a106286d7f094d71e/docs/superpowers/notes/2026-09-11-s2-residual-census
mkdir -p "$E/rows" "$E/logs" "$E/probes" "$E/scripts"
for r in raw gen stubA stubB stubD; do
  [ -f "$S/runs/$r/stderr" ] && cp "$S/runs/$r/stderr" "$E/rows/sigil-$r.rows"
  [ -f "$S/runs/$r/stdout" ] && cp "$S/runs/$r/stdout" "$E/rows/sigil-$r.stdout"
  [ -f "$S/runs/$r/exit" ] && cp "$S/runs/$r/exit" "$E/rows/sigil-$r.exit"
done
cp "$S/ref/s2.log" "$E/rows/asl-s1build-on-s2.rows"
for f in ref_build.log ref2_build.log luaref-build.log stub_run.log stub_run2.log; do
  [ -f "$S/$f" ] && cp "$S/$f" "$E/logs/$f"
done
for m in A B C D; do [ -f "$S/stub-$m.log" ] && cp "$S/stub-$m.log" "$E/logs/stub-$m.edits"; done
for c in compare-stubB.txt compare-stubD.txt; do [ -f "$S/runs/$c" ] && cp "$S/runs/$c" "$E/logs/$c"; done
cp "$S/gen-md5.luaref.txt" "$E/logs/generated-inputs.md5"
for f in rows-by-class.txt crossgrep.txt; do [ -f "$S/$f" ] && cp "$S/$f" "$E/$f"; done
for f in classify_rows.py crossgrep.py; do cp "$S/$f" "$E/scripts/$f"; done
cp "$S/refout/tool-md5s.txt" "$E/logs/tool-md5s.ref-s1asl.txt"
cp "$S/ref2out/tool-md5s.txt" "$E/logs/tool-md5s.s2toolchain.txt"
cp "$S"/probes/*.asm "$E/probes/"
cp "$S"/probes/run*.log "$E/probes/"
for f in run_sigil.sh ref_build.sh ref2_build.sh post_steps.lua mk_gen_corpus.sh stub.py stub_run.sh stub_run2.sh compare.py saxdec.py pfile.py trunc.py count.py lines.py bslines.py probe.sh collect.sh; do
  [ -f "$S/$f" ] && cp "$S/$f" "$E/scripts/$f"
done
du -sh "$E"
find "$E" -type f | wc -l
echo COLLECT_END
