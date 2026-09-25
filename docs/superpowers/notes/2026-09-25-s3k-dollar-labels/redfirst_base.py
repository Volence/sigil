#!/usr/bin/env python3
"""redfirst_base.py: put this parcel's tests (integration file, vectors, and the
lexer unit test) into the base-src archive tree (120be609), leaving the base
IMPLEMENTATION alone, so a run there shows the tests red before the change."""
import pathlib, re, shutil
W = pathlib.Path("/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a620ff60a900c195e/crates/sigil-frontend-as")
B = pathlib.Path("/home/volence/sonic_hacks/.scratch/s3k-dollar-labels/base-src/crates/sigil-frontend-as")
shutil.copy(W / "tests/as_dollar_labels.rs", B / "tests/as_dollar_labels.rs")
shutil.copytree(W / "tests/vectors/s3k_dollar_labels", B / "tests/vectors/s3k_dollar_labels", dirs_exist_ok=True)
w = (W / "src/lexer.rs").read_text()
start = w.index("    /// `$$name` is ONE identifier under both CPUs")
end = w.index("    }\n", w.index("fn a_temporary_symbol_is_one_identifier_under_both_cpus")) + 6
end = w.index("\n    }\n", w.index('Tok::Ident("$$x".into()),')) + 7
block = w[start:end]
b = (B / "src/lexer.rs").read_text()
anchor = "            .map(|t| t.tok)\n            .collect()\n    }\n"
assert b.count(anchor) == 1 and "a_temporary_symbol_is_one" not in b
b = b.replace(anchor, anchor + "\n" + block)
(B / "src/lexer.rs").write_text(b)
print("inserted", block.count("\n"), "lines; base implementation untouched otherwise")
