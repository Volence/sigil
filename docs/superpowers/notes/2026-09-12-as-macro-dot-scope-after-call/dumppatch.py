#!/usr/bin/env python3
"""Instrument an extracted source tree (never the worktree): each pass writes
its whole symbol environment to $SIGIL_DUMP_ENV.pass<N>, one `key<TAB>value`
row per symbol, key-sorted.   dumppatch.py <tree>"""
import sys

path = sys.argv[1] + "/crates/sigil-frontend-as/src/eval.rs"
anchor = ('    if std::env::var_os("SIGIL_CENSUS_EXPLABEL").is_some() {\n'
          '        eprintln!("CENSUS-EXPLABEL\\tinstances-with-labels={}", asm.expansion_label_used);\n'
          '    }\n')
add = ('    if let Some(p) = std::env::var_os("SIGIL_DUMP_ENV") {\n'
       '        let mut s = String::new();\n'
       '        for (k, v) in asm.env.iter() {\n'
       '            s.push_str(&format!("{k:?}\\t{v:?}\\n"));\n'
       '        }\n'
       '        std::fs::write(format!("{}.pass{mompass}", p.to_string_lossy()), s).expect("dump");\n'
       '    }\n')
src = open(path).read()
n = src.count(anchor)
if n != 1:
    sys.exit(f"anchor matched {n} times in {path}")
open(path, "w").write(src.replace(anchor, anchor + add))
print("PATCHED", path)
