#!/usr/bin/env python3
"""Instrument a SCRATCH COPY of the AS front end for the exposure census.

    instrument.py <scratch-source-root>

The copy is a `git archive` of the measured revision, never the worktree's
own `crates/`. Refuses if <root> is inside a git checkout's tracked tree
(it must hold no `.git`). Each substitution must match exactly once or the
script refuses and writes nothing. It adds three Warning diagnostics and
changes no byte, no placement and no other diagnostic:

  SEEKFEED            at every in-section `org` seek: the instrument sees seeks
  SEEKSPLIT           at the latest seek, when a section is CLOSED with its
                      cursor behind its extent (the defect's precondition)
  SEEKSPLIT-BY-ORG    the same, when the closer is an `org` that leaves the
                      section, which pins the next section (no split there)

A SEEKSPLIT that has no SEEKSPLIT-BY-ORG at the same line is a close whose
next section opens Chained: the shape that splits labels from bytes.
"""
import os
import sys

root = sys.argv[1]
if os.path.exists(os.path.join(root, ".git")):
    sys.exit("REFUSED: %s has a .git; instrument a git-archive copy only" % root)
path = os.path.join(root, "crates/sigil-frontend-as/src/eval.rs")
src = open(path).read()

SUBS = [
    (
        "            self.builder.seek(target_abs - base, 0, span);\n"
        "            self.last_seek = Some(span);\n",
        "            self.builder.seek(target_abs - base, 0, span);\n"
        "            self.last_seek = Some(span);\n"
        "            self.diags.push(Diagnostic { level: Level::Warning, message: format!(\"SEEKFEED seek to {:#x}, extent {:#x}\", target_abs, self.builder.extent()), primary: span }); // INSTRUMENT\n",
    ),
    (
        "            if self.builder.current_offset() < self.builder.extent() {\n"
        "                if let Some(seek) = self.last_seek {\n"
        "                    self.rebased_at = Some(seek);\n"
        "                }\n",
        "            if self.builder.current_offset() < self.builder.extent() {\n"
        "                if let Some(seek) = self.last_seek {\n"
        "                    self.rebased_at = Some(seek);\n"
        "                    self.diags.push(Diagnostic { level: Level::Warning, message: format!(\"SEEKSPLIT close with cursor {:#x} behind extent {:#x}, phys base {:#x}\", self.builder.current_offset(), self.builder.extent(), self.phys_base), primary: seek }); // INSTRUMENT\n"
        "                } else {\n"
        "                    eprintln!(\"SEEKSPLIT-NOSEEK cursor {:#x} extent {:#x}\", self.builder.current_offset(), self.builder.extent()); // INSTRUMENT\n"
        "                }\n",
    ),
    (
        "        } else {\n"
        "            self.close_section();\n"
        "            self.phys_base = phys_target;\n"
        "            self.rebased_at = Some(span);\n",
        "        } else {\n"
        "            if self.builder.current_offset() < self.builder.extent() { if let Some(seek) = self.last_seek { self.diags.push(Diagnostic { level: Level::Warning, message: \"SEEKSPLIT-BY-ORG\".into(), primary: seek }); } } // INSTRUMENT\n"
        "            self.close_section();\n"
        "            self.phys_base = phys_target;\n"
        "            self.rebased_at = Some(span);\n",
    ),
]

for i, (old, new) in enumerate(SUBS):
    n = src.count(old)
    if n != 1:
        sys.exit("REFUSED: substitution %d matched %d sites, want 1" % (i, n))
for old, new in SUBS:
    src = src.replace(old, new)
open(path, "w").write(src)
for ln, line in enumerate(src.splitlines(), 1):
    if "// INSTRUMENT" in line:
        print("%s:%d: %s" % (path, ln, line.strip()[:110]))
print("INSTRUMENT_APPLIED=%d" % len(SUBS))
