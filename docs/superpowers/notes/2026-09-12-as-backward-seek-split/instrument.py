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

# The fix for this row (`2026-09-12-as-backward-seek-split-fix.md`) adds one
# line to the block substitution 1 anchors on, `self.builder.pin_next_section();`
# after `self.rebased_at = Some(seek);`. The same instrument, with that line kept
# in place, is the alternative anchor, so the census runs on either revision.
# Exactly one of the two must match, exactly once.
PIN = "                    self.builder.pin_next_section();\n"
REBASE = "                    self.rebased_at = Some(seek);\n"
old1, new1 = SUBS[1]
ALTERNATIVES = {1: [(old1, new1), (old1.replace(REBASE, REBASE + PIN), new1.replace(REBASE, REBASE + PIN))]}

chosen = []
for i, (old, new) in enumerate(SUBS):
    options = ALTERNATIVES.get(i, [(old, new)])
    hits = [(o, n) for o, n in options if src.count(o) == 1]
    counts = [src.count(o) for o, _ in options]
    if len(hits) != 1 or sum(counts) != 1:
        sys.exit("REFUSED: substitution %d matched %s sites across its anchors, want exactly one" % (i, counts))
    chosen.append(hits[0])
    if len(options) > 1:
        print("SUBSTITUTION_%d_ANCHOR=%d" % (i, options.index(hits[0])))
for old, new in chosen:
    src = src.replace(old, new)
open(path, "w").write(src)
for ln, line in enumerate(src.splitlines(), 1):
    if "// INSTRUMENT" in line:
        print("%s:%d: %s" % (path, ln, line.strip()[:110]))
print("INSTRUMENT_APPLIED=%d" % len(SUBS))
