#!/usr/bin/env bash
# RED-FIRST for as_macro_body_label.rs. Five mutations, each applied to the
# COMMITTED baseline by a patcher that asserts its anchor matched exactly once,
# each shown landed with `git diff --stat` BEFORE the run, each restored with
# `git checkout HEAD --` and the restore verified empty.
#
# A mutation that fails to apply runs the ORIGINAL file and prints ok, which is
# indistinguishable from a clean restore — so the anchor assertion is not
# decoration, and neither is the diff --stat printed for every one.
#
#   ./mutations.sh <repo-root>
set -uo pipefail
ROOT="$1"
EVAL="$ROOT/crates/sigil-frontend-as/src/eval.rs"
export CARGO_TARGET_DIR="$ROOT/target-parcel"

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
    cargo test --release -p sigil-frontend-as --test as_macro_body_label 2>&1 \
        | grep -E '^test |^test result|error\[|^error:' | grep -v ' ok$'
    echo "--- restoring:"
    git -C "$ROOT" checkout HEAD -- crates/sigil-frontend-as/src/eval.rs
    echo "restore diff (must be empty): [$(git -C "$ROOT" diff --stat -- crates/sigil-frontend-as/src/eval.rs)]"
}

echo "################ BASELINE (must be all green)"
cargo test --release -p sigil-frontend-as --test as_macro_body_label 2>&1 \
    | grep -E '^test result|^test .* FAILED'

patch 'a = """        self.expansion_labels
            .iter()
            .rev()
            .find(|e| e.labels.contains(name))
            .map(|e| e.key.as_str())"""
assert s.count(a) == 1, s.count(a)
s = s.replace(a, "        let _ = name;\n        None")'
run_one M1 "no localization at all — the pre-parcel behaviour"

patch 'a = """        self.expansion_labels
            .iter()
            .rev()
            .find(|e| e.labels.contains(name))
            .map(|e| e.key.as_str())"""
assert s.count(a) == 1, s.count(a)
s = s.replace(a, """        let _ = name;
        self.expansion_labels.last().map(|e| e.key.as_str())""")'
run_one M4 "every plain name keys under the innermost instance, declared or not"

patch 'a = """            .rev()
            .find(|e| e.labels.contains(name))"""
assert s.count(a) == 1, s.count(a)
s = s.replace(a, """            .rev()
            .take(1)
            .find(|e| e.labels.contains(name))""")'
run_one M2 "the chain does not walk outward — innermost instance only"

patch 'a = """            self.push_expansion_labels(plain_labels.clone());
            self.exec(body);
            self.pop_expansion_labels();
            if self.take_exit_expansion() {
                break;
            }
        }
        self.expansion_depth -= 1;
        self.release_loop_body(captured.is_some());"""
assert s.count(a) == 1, s.count(a)
s = s.replace(a, """            self.exec(body);
            if self.take_exit_expansion() {
                break;
            }
        }
        self.expansion_depth -= 1;
        self.release_loop_body(captured.is_some());""")'
run_one M3 "rept gets no namespace at all — its body labels stay global"

patch 'a = """                    | \"label\"
                    | \"macro\""""
assert s.count(a) == 1, s.count(a)
s = s.replace(a, """                    | \"macro\"""")'
run_one M5 "the `label` DIRECTIVE is treated as a PC label"
