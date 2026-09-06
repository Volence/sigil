#!/usr/bin/env python3
"""Enumerate the AS-SET-OPENS-SCOPE population in a source tree.

    ./scan.py <dir> [<dir> ...]           counts, plus every site of shape S3
    ./scan.py --sites <dir> [...]         the S3 sites, one per line

Three numbers, because no one of them is the population:

  S1  every GLOBAL-named value binder (`set` `equ` `=` `:=` `eval`, and the
      `set NAME,v` / `eval NAME,v` operand forms).  These are the lines that
      open a local-label scope in asl and did not here.  A DOTTED name is not
      one: probe s03 shows a dotted binder opens nothing.
  S2  of those, the ones with at least one `.local` before the next global
      label -- the lines where the two assemblers file a following local
      under different parents.
  S3  of those, the ones where a `.local` NAME is written both inside such a
      window and outside it in the same file.  A local whose definition and
      every reference sit in ONE window resolves consistently under either
      assembler and cannot move a byte; a name that STRADDLES can.
  S4  explicit `Global.local` references whose `local` is DEFINED inside a
      binder window in the same file.  S3 catches the shape written with a
      bare `.local`; this catches the one written out in full, which is the
      shape the two direction probes use and which S3 cannot see, because a
      qualified name carries no bare `.local` for it to match.

A count of zero is printed as a result.  A directory that cannot be read is
reported UNMEASURABLE and never counted as zero.

Windows are cut at a global label (`Name:` at column 0, colon-less `Name` at
column 0 with no directive, or `Name label ...`) and at a global binder, which
is what asl's scope rule does.  Lines inside a macro/rept body are still
scanned: a `set` there opens the CALLER's scope (probe s09), so the shape is
the same one.
"""
import os
import re
import sys

# A name in asl's LABEL FIELD: column 0.  `\t x set 5` is an instruction to
# asl, not an assignment (the colon-less form is column-gated), so the anchor
# is deliberate.
NAME = r"[A-Za-z_@?][A-Za-z0-9_@?.]*"
BINDER_LABELFIELD = re.compile(
    rf"^({NAME}):?[ \t]+(?:set|equ|eval|=|:=)(?:[ \t]|$)", re.IGNORECASE
)
BINDER_OPERAND = re.compile(
    rf"^[ \t]+(?:set|eval)[ \t]+({NAME})[ \t]*,", re.IGNORECASE
)
GLOBAL_LABEL = re.compile(rf"^({NAME}):")
GLOBAL_LABEL_BARE = re.compile(rf"^({NAME})[ \t]*(?:;.*)?$")
LABEL_DIRECTIVE = re.compile(rf"^({NAME}):?[ \t]+label(?:[ \t]|$)", re.IGNORECASE)
# A `.`-local anywhere on the line, as a definition or as a reference.  Not
# preceded by an identifier character or a dot, so `Foo.bar` (already
# qualified) and `1.5` are not locals.
DOTLOCAL = re.compile(r"(?<![A-Za-z0-9_@?.])\.([A-Za-z_@?][A-Za-z0-9_@?]*)")
# An explicitly qualified reference `Global.local`, the spelling S3 cannot see.
# The local half is TWO characters or more, because a one-letter suffix is an
# operand size in this syntax (`move.l`, `dc.l`) and matching it reports every
# sized instruction in the tree as a qualified reference.  The cost is stated
# rather than hidden: a real one-letter local written out in full (`Base.l`)
# is outside S4.  s2.macrosetup.asm does define a local named `.l`, so that
# blind spot is not hypothetical, and S3 covers the bare-`.local` spelling of
# it.
QUALIFIED = re.compile(
    r"(?<![A-Za-z0-9_@?.])([A-Za-z_@?][A-Za-z0-9_@?]*)"
    r"\.([A-Za-z_@?][A-Za-z0-9_@?]+)"
)


def strip_comment(line):
    """Drop a `;` comment, respecting quotes and the `'` char literal."""
    out, q = [], None
    for ch in line:
        if q:
            out.append(ch)
            if ch == q:
                q = None
        elif ch in "\"'":
            q = ch
            out.append(ch)
        elif ch == ";":
            break
        else:
            out.append(ch)
    return "".join(out)


def scan_file(path):
    """Return (n_binders, n_binders_with_local, straddle_sites, qualified_sites)."""
    try:
        text = open(path, "r", encoding="utf-8", errors="replace").read()
    except OSError:
        return None
    lines = text.split("\n")
    binders = []          # (lineno, name)
    in_window = None      # index into `binders`, or None
    window_locals = {}    # binder index -> set of local names
    outside_locals = set()
    quals = []            # (lineno, Global, local)
    in_window_defs = set()  # local names DEFINED inside some binder window
    for i, raw in enumerate(lines, 1):
        line = strip_comment(raw).rstrip()
        if not line.strip():
            continue
        m = BINDER_LABELFIELD.match(line) or BINDER_OPERAND.match(line)
        if m and not m.group(1).startswith("."):
            binders.append((i, m.group(1)))
            in_window = len(binders) - 1
            window_locals[in_window] = set()
            continue
        if m:                      # a DOTTED binder: opens nothing, window stands
            pass
        elif (
            LABEL_DIRECTIVE.match(line)
            or (GLOBAL_LABEL.match(line) and not GLOBAL_LABEL.match(line).group(1).startswith("."))
            or (
                GLOBAL_LABEL_BARE.match(line)
                and not GLOBAL_LABEL_BARE.match(line).group(1).startswith(".")
            )
        ):
            in_window = None
        for g, loc in QUALIFIED.findall(line):
            quals.append((i, g, loc))
        for loc in DOTLOCAL.findall(line):
            if in_window is None:
                outside_locals.add(loc)
            else:
                window_locals[in_window].add(loc)
                if line.startswith("."):
                    in_window_defs.add(loc)
    with_local = [
        (path, binders[k][0], binders[k][1], sorted(v))
        for k, v in window_locals.items()
        if v
    ]
    straddle = []
    for k, v in window_locals.items():
        shared = v & outside_locals
        if shared:
            ln, nm = binders[k]
            straddle.append((path, ln, nm, sorted(shared)))
    qsites = [
        (path, ln, g, loc) for (ln, g, loc) in quals if loc in in_window_defs
    ]
    return len(binders), with_local, straddle, qsites


def main():
    sites_only = "--sites" in sys.argv
    dirs = [a for a in sys.argv[1:] if a != "--sites"]
    grand = [0, 0, 0, 0]
    for d in dirs:
        if not os.path.isdir(d):
            print(f"# {d}  UNMEASURABLE: not a directory")
            continue
        n_files = s1 = 0
        s2 = []
        s3 = []
        s4 = []
        for root, _dirs, files in os.walk(d):
            if "/.git" in root:
                continue
            for f in files:
                if not f.lower().endswith((".asm", ".inc", ".s", ".z80")):
                    continue
                p = os.path.join(root, f)
                r = scan_file(p)
                if r is None:
                    print(f"# {p}  UNMEASURABLE: unreadable")
                    continue
                n_files += 1
                s1 += r[0]
                s2.extend(r[1])
                s3.extend(r[2])
                s4.extend(r[3])
        if not sites_only:
            print(f"# {d}")
            print(f"#   source files walked : {n_files}")
            print(f"#   S1 global binders   : {s1}")
            print(f"#   S2 with a .local    : {len(s2)}")
            print(f"#   S3 straddling names : {len(s3)}")
            print(f"#   S4 qualified refs   : {len(s4)}")
        for path, ln, nm, locs in sorted(s2):
            print(f"S2\t{path}:{ln}\t{nm}\t{','.join(locs)}")
        for path, ln, nm, shared in sorted(s3):
            print(f"S3\t{path}:{ln}\t{nm}\t{','.join(shared)}")
        for path, ln, g, loc in sorted(s4):
            print(f"S4\t{path}:{ln}\t{g}.{loc}")
        grand[0] += s1
        grand[1] += len(s2)
        grand[2] += len(s3)
        grand[3] += len(s4)
    if not sites_only:
        print(
            f"# TOTAL  S1={grand[0]}  S2={grand[1]}  "
            f"S3={grand[2]}  S4={grand[3]}"
        )


if __name__ == "__main__":
    main()
