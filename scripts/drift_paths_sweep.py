#!/usr/bin/env python3
"""Every reference-tree path sigil's sources name, checked against the live tip.

THE LATENCY THIS CLOSES. A test that names a file in the engine lane's tree reads that
name only when something runs the test, and the gates that read most of these run when
a freeze moves bytes. A zero-byte engine parcel can therefore rename a fixture out from
under a fixed string and nothing notices for two parcels, at which point it surfaces
inside an unrelated byte-mover's attestation and reads as that parcel's fault.
`extra_entry.rs` closed that for the paths ONE file names, by hand and in both
directions; this is the same check over every file, and it costs no build because it is
a `stat` sweep.

IT REPORTS, IT DOES NOT BLOCK. A rename in the other lane's tree is not sigil's failure
and must not redden sigil's landing bar. The nightly drift job carries this because
that job already runs against their live tip and blocks nothing.

WHAT IT CANNOT TELL YOU: whether a fixture still fires the guard it used to. Only the
cases that build it can. This turns a rename from "red inside someone else's landing,
attributed wrongly" into "named here, the morning after".

WHICH FILES ARE SWEPT IS DERIVED, NOT LISTED. Only files that actually READ the
reference tree are swept, and the pattern deciding that is EXTRACTED from
`nightly_source_gates.sh`, which already classifies files by exactly that question. A
retyped second copy drifts from the first and the drift is invisible — the same lesson
that script records about its own skip marker. Sweeping everything instead costs
precision that matters: across all 445 source files the sweep reports 8 misses of which
7 are synthetic fixtures written into a test's own temp tree, and a report that is
mostly false is a report nobody reads.

BOTH DIRECTIONS. A path a test asserts ABSENT is drift when it starts existing — the
case that quietly stops testing what it claims while continuing to pass. Those live in
the expected-absent file, each row naming the test file that asserts the absence, and a
row whose naming file no longer names the path is itself reported: an allowlist nothing
checks is the next thing to rot.
"""

import argparse
import os
import re
import sys
import tempfile

# A literal is a candidate path when it has a separator, no whitespace, no format
# placeholder, and a file extension. The first segment must be a directory that EXISTS
# at the reference root — derived from the tree rather than a hand-kept list of
# top-level names, so a new top-level directory over there joins the sweep by itself.
LITERAL = re.compile(r'"([^"\\\n]{3,200})"')
LOOKS_LIKE_PATH = re.compile(r'^[A-Za-z0-9_][A-Za-z0-9_.\-/]*/[A-Za-z0-9_.\-/]+\.[A-Za-z0-9]{1,6}$')

EXIT_OK = 0
EXIT_DRIFT = 1
EXIT_CANNOT_RUN = 2


def source_files(root):
    out = []
    crates = os.path.join(root, "crates")
    for crate in sorted(os.listdir(crates)):
        for sub in ("tests", "src"):
            for dirpath, _, names in os.walk(os.path.join(crates, crate, sub)):
                for n in sorted(names):
                    if n.endswith(".rs"):
                        out.append(os.path.join(dirpath, n))
    return out


def reads_reference_tree(files, pattern):
    rx = re.compile(pattern)
    out = []
    for f in files:
        with open(f, errors="replace") as fh:
            if rx.search(fh.read()):
                out.append(f)
    return out


def named_paths(root, files, tops):
    """{path: [(file, line), ...]} for every candidate literal in the given files."""
    found = {}
    for f in files:
        with open(f, errors="replace") as fh:
            for lineno, line in enumerate(fh, 1):
                for lit in LITERAL.findall(line):
                    if not LOOKS_LIKE_PATH.match(lit):
                        continue
                    if lit.split("/", 1)[0] not in tops:
                        continue
                    found.setdefault(lit, []).append((os.path.relpath(f, root), lineno))
    return found


def read_expected_absent(path):
    """Rows of `<path> <naming file>`; comments and blanks ignored."""
    rows = []
    with open(path) as f:
        for line in f:
            line = line.split("#", 1)[0].strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) != 2:
                raise ValueError(f"malformed row (want `<path> <naming file>`): {line!r}")
            rows.append((parts[0], parts[1]))
    return rows


def sweep(sigil_root, ref_tree, expected_absent, reads_pattern, out=print):
    if not os.path.isdir(ref_tree):
        out(f"CANNOT RUN: no reference tree at {ref_tree}, so NOTHING was swept. "
            "This is not a clean sweep.")
        return EXIT_CANNOT_RUN
    tops = {d for d in os.listdir(ref_tree)
            if os.path.isdir(os.path.join(ref_tree, d)) and not d.startswith(".")}
    if not tops:
        out(f"CANNOT RUN: {ref_tree} has no top-level directories, so every literal would "
            "be filtered out and the sweep would report a clean zero")
        return EXIT_CANNOT_RUN
    if not reads_pattern:
        out("CANNOT RUN: no reference-reading pattern was given, so the sweep cannot say "
            "which files it should cover")
        return EXIT_CANNOT_RUN
    try:
        absent_rows = read_expected_absent(expected_absent)
    except (OSError, ValueError) as e:
        out(f"CANNOT RUN: the expected-absent list is unusable ({e}), so a missing path "
            "could not be told from one asserted missing")
        return EXIT_CANNOT_RUN
    absent = {p for p, _ in absent_rows}

    try:
        files = reads_reference_tree(source_files(sigil_root), reads_pattern)
    except (OSError, re.error) as e:
        out(f"CANNOT RUN: the source scan failed ({e}), so nothing was swept")
        return EXIT_CANNOT_RUN
    if not files:
        out(f"CANNOT RUN: no source file under {sigil_root} matches the reference-reading "
            f"pattern /{reads_pattern}/, so a sweep of zero paths would report clean")
        return EXIT_CANNOT_RUN

    named = named_paths(sigil_root, files, tops)
    sites = sum(len(v) for v in named.values())
    missing, stale_rows, resurrected = [], [], []

    for path, site_list in sorted(named.items()):
        if path in absent:
            continue
        if not os.path.exists(os.path.join(ref_tree, path)):
            missing.append((path, site_list))

    for path, naming_file in absent_rows:
        full = os.path.join(sigil_root, naming_file)
        if not os.path.exists(full):
            stale_rows.append((path, naming_file, "the naming file does not exist"))
        else:
            with open(full, errors="replace") as fh:
                if path not in fh.read():
                    stale_rows.append((path, naming_file,
                                       "the naming file no longer names this path"))
        if os.path.exists(os.path.join(ref_tree, path)):
            resurrected.append((path, naming_file))

    out(f"reference-path sweep: {len(named)} distinct path(s) at {sites} site(s) in "
        f"{len(files)} reference-reading file(s), checked against {ref_tree}")
    out(f"  asserted absent: {len(absent_rows)}")
    if missing:
        out(f"  MISSING — {len(missing)} named path(s) no longer exist over there:")
        for path, site_list in missing:
            out(f"    {path}")
            out(f"      named by {', '.join(f'{f}:{n}' for f, n in site_list[:4])}")
        out("    Do NOT simply re-point at the new name: a renamed fixture is often also a "
            "re-aimed one, so read the replacement's own header first.")
    if resurrected:
        out(f"  RESURRECTED — {len(resurrected)} path(s) asserted ABSENT now exist:")
        for path, naming_file in resurrected:
            out(f"    {path} (asserted absent by {naming_file})")
        out("    The case using it has stopped testing what it claims while still passing.")
    if stale_rows:
        out(f"  STALE ALLOWLIST — {len(stale_rows)} expected-absent row(s) no longer apply:")
        for path, naming_file, why in stale_rows:
            out(f"    {path}: {why} ({naming_file})")
    if not (missing or resurrected or stale_rows):
        out("  no drift: every named path resolves and every asserted absence holds")
        return EXIT_OK
    return EXIT_DRIFT


# ── selftest ─────────────────────────────────────────────────────────────────────

def _write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(text)


def selftest():
    failures = []

    def check(name, cond, detail=""):
        print(f"  {'ok  ' if cond else 'FAIL'}  {name}{'' if cond else ': ' + detail}")
        if not cond:
            failures.append(name)

    pattern = "AEON_DIR"
    with tempfile.TemporaryDirectory() as d:
        sig, ref = os.path.join(d, "sigil"), os.path.join(d, "ref")
        absent_file = os.path.join(d, "absent.txt")
        _write(os.path.join(ref, "engine", "present.emp"), "")
        _write(os.path.join(ref, "games", "keep.emp"), "")
        _write(os.path.join(sig, "crates", "c", "tests", "reads.rs"),
               'const A: &str = "engine/present.emp";\n'
               'const B: &str = "engine/renamed_away.emp";\n'
               'fn f() { std::env::var("AEON_DIR"); }\n')
        _write(os.path.join(sig, "crates", "c", "tests", "synthetic.rs"),
               'const S: &str = "engine/only_in_a_temp_tree.emp";\n')
        _write(absent_file, "games/gone.emp crates/c/tests/reads.rs\n")

        lines = []
        rc = sweep(sig, ref, absent_file, pattern, out=lines.append)
        text = "\n".join(lines)
        check("a renamed path is reported", "engine/renamed_away.emp" in text)
        check("the report names the site", "crates/c/tests/reads.rs:2" in text)
        check("a file that does not read the reference tree is not swept",
              "only_in_a_temp_tree" not in text)
        check("a resolving path is not reported", "engine/present.emp" not in text)
        check("drift exits non-zero", rc == EXIT_DRIFT)
        # The allowlist row names a path its file does not name -> stale, and reported.
        check("a stale allowlist row is reported", "STALE ALLOWLIST" in text)

        # The absence assertion holds: name it in the file and keep it off disk.
        _write(os.path.join(sig, "crates", "c", "tests", "reads.rs"),
               'const A: &str = "engine/present.emp";\n'
               'const G: &str = "games/gone.emp";\n'
               'fn f() { std::env::var("AEON_DIR"); }\n')
        lines = []
        rc = sweep(sig, ref, absent_file, pattern, out=lines.append)
        text = "\n".join(lines)
        check("a held absence is clean", rc == EXIT_OK, text)
        check("a held absence is not reported as missing", "games/gone.emp" not in text)

        # ... and now it exists: the case using it has stopped testing what it claims.
        _write(os.path.join(ref, "games", "gone.emp"), "")
        lines = []
        rc = sweep(sig, ref, absent_file, pattern, out=lines.append)
        text = "\n".join(lines)
        check("a resurrected absence is reported", "RESURRECTED" in text)
        check("a resurrected absence exits non-zero", rc == EXIT_DRIFT)

        # Unmeasurable, three ways, and none of them may read as clean.
        lines = []
        rc = sweep(sig, os.path.join(d, "no-such-tree"), absent_file, pattern, out=lines.append)
        check("a missing reference tree cannot run", rc == EXIT_CANNOT_RUN)
        check("a missing reference tree is not a clean sweep",
              "not a clean sweep" in "\n".join(lines))
        lines = []
        rc = sweep(sig, ref, absent_file, "NoFileMatchesThis", out=lines.append)
        check("a pattern matching no file cannot run", rc == EXIT_CANNOT_RUN)
        check("a zero-file sweep says it would report clean",
              "would report clean" in "\n".join(lines))
        lines = []
        rc = sweep(sig, ref, os.path.join(d, "no-such-list"), pattern, out=lines.append)
        check("an unreadable allowlist cannot run", rc == EXIT_CANNOT_RUN)

    print("")
    if failures:
        print(f"SELFTEST FAILED: {len(failures)} check(s): {', '.join(failures)}")
        return 1
    print("SELFTEST PASSED")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--sigil-root")
    ap.add_argument("--ref-tree", help="the engine lane's checkout to check against")
    ap.add_argument("--expected-absent")
    # NO DEFAULT: the job extracts this from nightly_source_gates.sh, which already
    # classifies files by the same question. A default here would be the second copy.
    ap.add_argument("--reads-pattern")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        return selftest()
    missing = [n for n in ("sigil_root", "ref_tree", "expected_absent", "reads_pattern")
               if not getattr(a, n)]
    if missing:
        ap.error("missing required argument(s): " + ", ".join("--" + m.replace("_", "-")
                                                              for m in missing))
    return sweep(a.sigil_root, a.ref_tree, a.expected_absent, a.reads_pattern)


if __name__ == "__main__":
    sys.exit(main())
