#!/usr/bin/env python3
"""Apply ONE half-fix mutation to the committed eval.rs, asserting its anchor
matches exactly once.   mutate.py <M1..M8>

M0 (the whole file at base 07edc95f) is written by mutations.sh with git show.
The anchors are the committed text of the AS-MACRO-DOT-SCOPE-AFTER-CALL fix."""
import os, sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../.."))
PATH = os.path.join(ROOT, "crates/sigil-frontend-as/src/eval.rs")

MUT = {
    # piece 1: a plain body label no longer writes the real scope
    "M1": ("            if !self.macro_frames.iter().all(|f| f.transparent) {\n"
           "                self.outer_scope = Some(name.to_string());\n"
           "            }\n"
           "            self.scope_epoch += 1;\n",
           "            self.scope_epoch += 1;\n"),
    # piece 2: a nested exit ignores whether its body opened a scope
    "M2": ("        } else if self.scope_epoch != epoch {\n",
           "        } else if false && self.scope_epoch != epoch {\n"),
    # piece 3, writer: the enclosing body's own `.y:` is never filed in its instance
    "M3": ("    fn file_in_body_instance(&mut self, name: &str, q: &str) -> Option<String> {\n",
           "    fn file_in_body_instance(&mut self, name: &str, q: &str) -> Option<String> {\n"
           "        if true {\n            return None;\n        }\n"),
    # piece 3, reader: owned_by_head asks only about the head
    "M4": (".or_else(|| self.plain_label_scope(&q))", ""),
    # piece 4: no expansion-then-outside fallback for a privatised key
    "M5": ("        if k != q && self.env.resolve(&k, Some(\"\")).is_none()",
           "        if false && k != q && self.env.resolve(&k, Some(\"\")).is_none()"),
    # piece 2's input: open_scope does not count as a scope write
    "M6": ("    fn open_scope(&mut self, name: &str) {\n        self.scope_epoch += 1;\n",
           "    fn open_scope(&mut self, name: &str) {\n"),
    # piece 2's input: a plain label does not count as a scope write
    "M7": ("                self.outer_scope = Some(name.to_string());\n"
           "            }\n"
           "            self.scope_epoch += 1;\n",
           "                self.outer_scope = Some(name.to_string());\n"
           "            }\n"),
    # piece 3's input: the frame never learns its instance key
    "M8": ("                f.instance = key;\n",
           "                f.instance = None;\n                let _ = key;\n"),
    # WRONG-FIX, for the control test: the outermost exit hands back whatever
    # scope the body left (its private ` macro#N` when it wrote no label)
    # instead of the real scope
    "M9": ("        } else if outermost {\n            self.scope = self.outer_scope.clone();\n",
           "        } else if outermost {\n            self.scope = self.scope.clone();\n"),
    # WRONG-FIX, for the control test: the body SCAN's claimed plain label moves
    # the real scope at entry, whether or not the line that writes it runs
    "M10": ("        let epoch = self.scope_epoch;\n",
            "        let epoch = self.scope_epoch;\n"
            "        if !global_symbols && outermost {\n"
            "            if let Some(l) = self.expansion_labels.last().and_then(|e| e.labels.iter().next_back().cloned()) {\n"
            "                self.outer_scope = Some(l);\n"
            "            }\n"
            "        }\n"),
}

mid = sys.argv[1]
old, new = MUT[mid]
src = open(PATH).read()
n = src.count(old)
if n != 1:
    print(f"ANCHOR {mid} matched {n} times, refusing", file=sys.stderr)
    sys.exit(3)
open(PATH, "w").write(src.replace(old, new))
print(f"APPLIED {mid}")
