#!/usr/bin/env bash
# RED-FIRST for the `label`-DIRECTIVE cells: the two new tests in
# `as_macro_body_label.rs` and the three new `snippets_golden.txt` vectors.
# Each mutation is applied to the COMMITTED baseline by a patcher that asserts
# its anchor matched exactly once, is shown landed with `git diff` BEFORE the
# run, and is restored with `git checkout HEAD --` with the restore verified
# empty.
#
# A mutation that fails to apply runs the ORIGINAL file and prints ok, which is
# indistinguishable from a clean restore — so the anchor assertion and the
# printed diff are the proof the subject was reached, not decoration.
#
# BOTH gates run under every mutation, deliberately. The snippet gate reads
# asl-minted bytes and the rust file reads written-out expectations; a mutation
# that reddens only one of them says which instrument is carrying that cell.
#
#   ./mutations-labeldir.sh <repo-root>
set -uo pipefail
ROOT="$1"
EVAL="$ROOT/crates/sigil-frontend-as/src/eval.rs"
export CARGO_TARGET_DIR="$ROOT/target"

patch() {  # $1 = python replacement program
    python3 - "$EVAL" <<PY
import sys
p = sys.argv[1]
s = open(p).read()
$1
open(p, "w").write(s)
PY
}

run_one() {
    local tag="$1" desc="$2"
    echo "================ $tag — $desc"
    echo "--- the mutation, landed:"
    git -C "$ROOT" diff --stat -- crates/sigil-frontend-as/src/eval.rs
    git -C "$ROOT" diff -- crates/sigil-frontend-as/src/eval.rs | grep -E '^[-+][^-+]' | head -20
    echo "--- as_macro_body_label:"
    cargo test --release -p sigil-frontend-as --test as_macro_body_label 2>&1 \
        | grep -E '^test |^test result|error\[|^error:' | grep -v ' ok$'
    echo "--- asl_snippets (the asl-minted vectors):"
    cargo test --release -p sigil-frontend-as --test asl_snippets 2>&1 \
        | grep -E '^test |^test result|error\[|^error:|diverged' | grep -v ' ok$' | head -8
    echo "--- restoring:"
    git -C "$ROOT" checkout HEAD -- crates/sigil-frontend-as/src/eval.rs
    echo "restore diff (must be empty): [$(git -C "$ROOT" diff --stat -- crates/sigil-frontend-as/src/eval.rs)]"
}

echo "################ BASELINE (must be all green in BOTH gates)"
cargo test --release -p sigil-frontend-as --test as_macro_body_label 2>&1 \
    | grep -E '^test result|^test .* FAILED'
cargo test --release -p sigil-frontend-as --test asl_snippets 2>&1 \
    | grep -E '^test result|^test .* FAILED'

# N1 — the PC-valued `label` stops being a PLACED label. This is the only line
# in `directive_label` that a constant-valued `label` never reaches, so it is
# the line `p12` exists for and the one `p11`/m18 cannot see.
patch 'a = "        if v == self.here_i64() {\n            self.builder.define_label(&qualified);\n        }"
assert s.count(a) == 1, s.count(a)
s = s.replace(a, "        if false {\n            self.builder.define_label(&qualified);\n        }")'
run_one N1 "a PC-valued \`label\` is never placed with the builder"

# N2 — the opposite error: EVERY `label` claims a position at the current PC,
# including the constant-valued ones that are not addresses at all.
patch 'a = "        if v == self.here_i64() {\n            self.builder.define_label(&qualified);\n        }"
assert s.count(a) == 1, s.count(a)
s = s.replace(a, "        if true {\n            self.builder.define_label(&qualified);\n        }")'
run_one N2 "every \`label\`, PC-valued or not, claims a position"

# N3 — localize the directive AT THE WRITER: bind it under the innermost live
# expansion instead of bare. This is the shape the whole cell is drawn against.
patch 'a = "        self.env.define(&qualified, SymbolValue::Int(v));\n        self.known_labels.insert(qualified.clone());"
assert s.count(a) == 1, s.count(a)
s = s.replace(a, """        let qualified = match self.expansion_labels.last() {
            Some(e) => format!("{}.{}", e.key, qualified),
            None => qualified,
        };
        self.env.define(&qualified, SymbolValue::Int(v));
        self.known_labels.insert(qualified.clone());""")'
run_one N3 "the \`label\` directive binds under the expansion that wrote it"

# N4 — the duplicate report is skipped inside an expansion, which is what an
# expansion-local reading of the directive would imply. Only the REFUSAL test
# can see this one; no byte vector can.
patch 'a = "        self.declare_class(&qualified, SymClass::Const, span);"
assert s.count(a) == 1, s.count(a)
s = s.replace(a, """        if self.macro_frames.is_empty() {
            self.declare_class(&qualified, SymClass::Const, span);
        }""")'
run_one N4 "no duplicate-definition report for a \`label\` written in an expansion"

# N5 — `scan_plain_labels` treats the `label` directive as a PC label. Recorded
# because the m18/m19 outside read is INERT under it (the parcel that added p11
# found this the hard way); run here to see which of the new cells, if any,
# reaches it.
patch 'a = """                    | \"eval\"
                    | \"label\""""
assert s.count(a) == 1, s.count(a)
s = s.replace(a, """                    | \"eval\"""")'
run_one N5 "\`label\` is scanned as a PC label (the known-inert-on-outside-reads one)"
