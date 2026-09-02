"""suite_paths.py — contract/SUITE_PATHS.md for the A/B scripts.

Eighteen scripts under `ab/*/` reached empyrean's Aether client by writing one
person's home directory into a `sys.path.insert`. This is the same precedence the
shell half implements in `scripts/lib/suite_paths.sh`, and the two are meant to be
read together: same steps, same announce format, same refusal shape, same meaning
of "set but wrong".

  1. the explicit checkout variable  — EMPYREAN_DIR, AEON_DIR;
  2. EMPYREAN_SUITE_ROOT joined with the repo's directory name;
  3. derivation from THIS repo's own location, via `git rev-parse --git-common-dir`;
  4. refuse, naming every variable consulted and every path tried.

NEVER `--show-toplevel`. It answers with the LINKED WORKTREE's root, and these
scripts are read and edited from worktrees like everything else here, so a sibling
derived from it lands under `.claude/worktrees/` and does not exist.

SET BUT WRONG IS A HARD ERROR at its own step, never a None that lets the next step
run. A variable naming a directory that is not the named checkout is evidence of a
wrong environment; falling through would find the right tree and leave the wrong
value in place for everything downstream.

These scripts drive the emulator by hand and are not run by any suite, so the
failure mode this replaces was silent in the worst way: the import simply fails on
any machine but one, with a traceback that names a path and no reason.
"""

import os
import subprocess
import sys

__all__ = [
    "SuitePathError",
    "resolve_root",
    "resolve_checkout",
    "empyrean_dir",
    "aeon_dir",
    "add_empyrean_clients",
    "debug_listing",
]

SUITE_ROOT_VAR = "EMPYREAN_SUITE_ROOT"

# `.git` plus a marker the repo cannot plausibly be without. The marker is the half
# that makes set-but-wrong detectable: a variable pointing at a sibling checkout, a
# stale copy, or the suite root itself has a `.git` as readily as the right answer.
_MARKERS = {
    "aeon": ("build.sh", "engine"),
    "sigil": ("Cargo.toml", "crates/sigil-harness"),
    "empyrean": ("contract", "clients"),
}

_HERE = os.path.dirname(os.path.abspath(__file__))
_cache = {}


class SuitePathError(RuntimeError):
    """No checkout could be named, or one was named wrongly."""


def _announce(var, path, step, reason):
    """One line on stderr, before the caller does any work with the path.

    The format is fixed because it is a cross-language contract: the shell include
    prints the same shape, and a log grep that reads one must read the other. It
    carries neither spelling sigil's landing bar counts as an unmeasured gate.
    """
    sys.stderr.write("# %s=%s (step %s: %s)\n" % (var, path, step, reason))


def _why_not(name, path):
    """None when `path` is a `name` checkout, else the reason, phrased to be read at
    the end of "...is not the aeon checkout (<reason>)"."""
    if not path:
        return "the empty string"
    if not os.path.isdir(path):
        return "no such directory"
    # A linked worktree's `.git` is a FILE, not a directory. Testing for a directory
    # would refuse every worktree, which is the shape most of this suite runs in.
    if not os.path.exists(os.path.join(path, ".git")):
        return "no .git — that is not a checkout"
    for marker in _MARKERS.get(name, ()):
        if not os.path.exists(os.path.join(path, marker)):
            return "no %s — that is not the %s checkout" % (marker, name)
    return None


def _derived_root():
    """The directory the suite's checkouts hang off, or None.

    Anchored at THIS FILE's directory, not at the caller's cwd: these scripts are run
    from several directories and a cwd-dependent derivation would answer differently
    for each. `--git-common-dir` can answer relatively, so the result is resolved
    against the directory it was asked from.
    """
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--git-common-dir"],
            cwd=_HERE,
            capture_output=True,
            text=True,
        )
    except OSError:
        return None
    if out.returncode != 0 or not out.stdout.strip():
        return None
    common = os.path.abspath(os.path.join(_HERE, out.stdout.strip()))
    # <suite-root>/<repo>/.git  ->  <suite-root>
    return os.path.dirname(os.path.dirname(common))


def resolve_root():
    """The suite root. Steps 1 and 3 — step 2 IS the root."""
    named = os.environ.get(SUITE_ROOT_VAR)
    if named:
        if not os.path.isdir(named):
            raise SuitePathError(
                "%s=%s is not a directory.\n"
                "       A variable that is set but wrong is a wrong environment, not a\n"
                "       missing one: the next step would resolve correctly and leave the\n"
                "       wrong value in place." % (SUITE_ROOT_VAR, named)
            )
        root = os.path.abspath(named)
        _announce(SUITE_ROOT_VAR, root, 1, "explicit " + SUITE_ROOT_VAR)
        return root
    root = _derived_root()
    if root:
        _announce(SUITE_ROOT_VAR, root, 3, "derived from this checkout via git --git-common-dir")
        return root
    raise SuitePathError(
        "cannot locate the suite root.\n"
        "       consulted  %s  (unset)\n"
        "       derived    git --git-common-dir answered nothing from %s\n"
        "       Export %s to the directory the suite's checkouts hang off."
        % (SUITE_ROOT_VAR, _HERE, SUITE_ROOT_VAR)
    )


def resolve_checkout(name, var):
    """The `name` checkout, by the four-step precedence. Announces the step that
    answered; raises SuitePathError on step 4 or on a set-but-wrong variable.

    Resolved once per process: a second announce for the same answer would be noise
    in a transcript a person reads.
    """
    if name in _cache:
        return _cache[name]

    # STEP 1 — the explicit checkout variable.
    named = os.environ.get(var)
    if named:
        why = _why_not(name, named)
        if why:
            raise SuitePathError(
                "%s=%s is not the %s checkout (%s).\n"
                "       A variable that is set but wrong is a wrong environment, not a\n"
                "       missing one, so this stops here rather than resolving %s some\n"
                "       other way and leaving %s pointing somewhere else for everything\n"
                "       downstream." % (var, named, name, why, name, var)
            )
        path = os.path.abspath(named)
        _announce(var, path, 1, "explicit " + var)
        _cache[name] = path
        return path

    # STEP 2 — the suite root, joined with the repo's directory name.
    root_var = os.environ.get(SUITE_ROOT_VAR)
    tried_root = "(unset)"
    if root_var:
        if not os.path.isdir(root_var):
            raise SuitePathError(
                "%s=%s is not a directory, so the %s checkout cannot be joined onto it.\n"
                "       A variable that is set but wrong is a wrong environment, not a\n"
                "       missing one." % (SUITE_ROOT_VAR, root_var, name)
            )
        cand = os.path.join(os.path.abspath(root_var), name)
        why = _why_not(name, cand)
        if why:
            raise SuitePathError(
                "%s=%s names a suite root whose %s entry (%s) is not the %s checkout\n"
                "       (%s). A variable that is set but wrong is a wrong environment,\n"
                "       not a missing one."
                % (SUITE_ROOT_VAR, root_var, name, cand, name, why)
            )
        _announce(var, cand, 2, "%s/%s" % (SUITE_ROOT_VAR, name))
        _cache[name] = cand
        return cand

    # STEP 3 — derive from this repo's own location.
    root = _derived_root()
    if root:
        cand = os.path.join(root, name)
        why = _why_not(name, cand)
        if not why:
            _announce(var, cand, 3, "sibling of this checkout via git --git-common-dir")
            _cache[name] = cand
            return cand
        tried_derived = "%s (%s)" % (cand, why)
    else:
        tried_derived = "git --git-common-dir answered nothing from %s" % _HERE

    # STEP 4 — refuse, by name.
    raise SuitePathError(
        "cannot locate the %s checkout.\n"
        "       consulted  %s   (unset)\n"
        "       consulted  %s   %s\n"
        "       tried      %s\n"
        "       Export %s to the %s checkout, or %s to the directory the suite's\n"
        "       checkouts hang off. This does NOT fall back to a live working tree: a\n"
        "       run against a tree nobody named is a run nobody can reproduce."
        % (name, var, SUITE_ROOT_VAR, tried_root, tried_derived, var, name, SUITE_ROOT_VAR)
    )


def empyrean_dir():
    return resolve_checkout("empyrean", "EMPYREAN_DIR")


def aeon_dir():
    return resolve_checkout("aeon", "AEON_DIR")


def add_empyrean_clients():
    """Put empyrean's Python client package on `sys.path` and return its directory.

    Idempotent: a second call from the same process does not stack a second copy.
    """
    clients = os.path.join(empyrean_dir(), "clients", "python")
    if clients not in sys.path:
        sys.path.insert(0, clients)
    return clients


def debug_listing():
    """The debug-shape listing these profilers read symbols out of.

    PROFILE_LST names it outright and wins, exactly as an explicit checkout variable
    does one level up; otherwise it is the resolved engine checkout's own listing.
    """
    named = os.environ.get("PROFILE_LST")
    if named:
        _announce("PROFILE_LST", named, 1, "explicit PROFILE_LST")
        return named
    return os.path.join(aeon_dir(), "s4.debug.lst")
