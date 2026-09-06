#!/usr/bin/env python3
"""Print, for one S2 site, the actual lines whose `.local` put it in the count.

    ./sites.py <file> <binder lineno>

scan.py counts a window as populated when a bare `.name` appears in it, and a
bare `.name` is also how this syntax spells an operand SIZE (`(port).l`,
`move.w`) and an attribute (`beq.ATTRIBUTE`).  This prints the evidence so each
site gets a verdict a reader can check, rather than a count nobody looked at.
"""
import re
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from scan import (  # noqa: E402
    BINDER_LABELFIELD,
    BINDER_OPERAND,
    DOTLOCAL,
    GLOBAL_LABEL,
    GLOBAL_LABEL_BARE,
    LABEL_DIRECTIVE,
    strip_comment,
)


def main():
    path, want = sys.argv[1], int(sys.argv[2])
    lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
    active = False
    for i, raw in enumerate(lines, 1):
        line = strip_comment(raw).rstrip()
        if not line.strip():
            continue
        m = BINDER_LABELFIELD.match(line) or BINDER_OPERAND.match(line)
        if m and not m.group(1).startswith("."):
            if i == want:
                active = True
                print(f"{i:6}  BINDER  {raw}")
                continue
            if active:
                print(f"{i:6}  (next binder ends the window) {raw}")
                return
            continue
        if not m and (
            LABEL_DIRECTIVE.match(line)
            or (GLOBAL_LABEL.match(line) and not GLOBAL_LABEL.match(line).group(1).startswith("."))
            or (
                GLOBAL_LABEL_BARE.match(line)
                and not GLOBAL_LABEL_BARE.match(line).group(1).startswith(".")
            )
        ):
            if active:
                print(f"{i:6}  (global label ends the window) {raw}")
                return
            continue
        if active:
            hits = DOTLOCAL.findall(line)
            if hits:
                print(f"{i:6}  .{'  .'.join(hits):22}  {raw}")


if __name__ == "__main__":
    main()
