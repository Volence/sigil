#!/usr/bin/env python3
"""Apply one literal substitution to a tracked file, by an anchor that must match
exactly once, and quote the mutated line back from disk by its MUTATION marker.

usage: mutate.py apply <repo> <path> <id>   (id selects an entry of MUTATIONS)
       mutate.py restore <repo> <path>       (git show HEAD:<path> > <path>)
"""
import subprocess
import sys

MUTATIONS = {
    "M4": (
        "                    sec.placement = sigil_ir::SectionPlacement::Pinned;\n",
        "                    let _ = &sec.placement; // MUTATION M4 second-space pin removed\n",
    ),
    "M5": (
        "            // silently compacted back into sequence and the org would vanish.\n"
        "            self.builder.pin_next_section();\n",
        "            // silently compacted back into sequence and the org would vanish.\n"
        "            let _ = 0; // MUTATION M5 org-leave pin removed\n",
    ),
    "M6": (
        "            // link-time placement pass.\n"
        "            self.builder.pin_next_section();\n",
        "            // link-time placement pass.\n"
        "            let _ = 0; // MUTATION M6 no-section org pin removed\n",
    ),
}


def tracked_dirty(repo):
    out = subprocess.run(
        ["git", "-C", repo, "status", "--porcelain", "--untracked-files=no"],
        capture_output=True, text=True, check=True,
    ).stdout
    return out.strip()


def main():
    op = sys.argv[1]
    repo = sys.argv[2]
    path = sys.argv[3]
    full = f"{repo}/{path}"
    if op == "apply":
        mid = sys.argv[4]
        dirty = tracked_dirty(repo)
        if dirty:
            sys.exit(f"REFUSING: tracked tree is dirty:\n{dirty}")
        anchor, repl = MUTATIONS[mid]
        text = open(full).read()
        n = text.count(anchor)
        if n != 1:
            sys.exit(f"REFUSING: anchor matches {n} times, need exactly 1")
        open(full, "w").write(text.replace(anchor, repl))
        # Quote back from disk.
        for i, line in enumerate(open(full).read().splitlines(), 1):
            if f"MUTATION {mid}" in line:
                print(f"MUTATED {path}:{i}: {line.strip()}")
        print("tracked status after apply:")
        print(tracked_dirty(repo))
    elif op == "restore":
        blob = subprocess.run(
            ["git", "-C", repo, "show", f"HEAD:{path}"], capture_output=True, check=True
        ).stdout
        open(full, "wb").write(blob)
        print(f"RESTORED {path} from HEAD; tracked status now: [{tracked_dirty(repo)}]")
    else:
        sys.exit("unknown op")


if __name__ == "__main__":
    main()
