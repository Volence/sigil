# shellcheck shell=bash
# suite_paths.sh — ONE implementation of contract/SUITE_PATHS.md for sigil's shell.
#
# Sourced, never executed. `set -u` safe; it never assumes a variable is set and it
# never exports anything the caller did not ask for.
#
# WHY SHELL AND NOT A RUST BINARY. Three of the four callers must be able to REFUSE
# before a cargo build exists. `landing-run.sh` builds the assembler it is about to
# judge, so a resolver that needed that build could not refuse ahead of it; the two
# nightly units run from bare detached checkouts on a timer, where the first thing
# that happens is the resolution and the second is a compile that must not start when
# the resolution failed. A resolver reachable only after the compile is a resolver
# that runs after the mistake it exists to catch.
#
# THE PRECEDENCE, identical in every resolver in the suite (contract/SUITE_PATHS.md,
# empyrean origin/main f222f371):
#
#   1. the explicit checkout variable — AEON_DIR, EMPYREAN_DIR, SIGIL_DIR;
#   2. EMPYREAN_SUITE_ROOT joined with the repo's directory name;
#   3. derivation from THIS repo's own location, via `git rev-parse --git-common-dir`;
#   4. refuse, naming every variable consulted and every path tried.
#
# NEVER `--show-toplevel`. It answers with the LINKED WORKTREE's root, and every sigil
# agent runs in a linked worktree, so a sibling derived from it lands under
# `.claude/worktrees/` and does not exist. `--git-common-dir` answers with the MAIN
# checkout's `.git` in both shapes, which is the fact this derivation needs.
#
# SET BUT WRONG IS A HARD ERROR AT ITS OWN STEP, never a null that lets the next step
# run. A variable naming a directory that is not the named checkout is evidence of a
# wrong environment; falling through would resolve to the RIGHT tree while the caller's
# environment stays wrong, and the next thing to consult that variable — a child
# process, a later script, a human reading the log — gets the wrong answer with nothing
# having said so.
#
# NO HOME LITERAL ANYWHERE, and no silent fallback to a live working tree. A live tree
# is a peer's working directory: mid-edit, behind its own remote, and carrying the
# owner's content edits. Reaching it by default is how a run measures something nobody
# named.
#
# EXIT / RETURN CODES, which callers and the Rust side both depend on:
#   0  resolved; the path is on stdout and the announce on stderr
#   3  step 4 — nothing named the checkout
#   4  a variable was set but does not name that checkout (hard error)

# ── The announce ─────────────────────────────────────────────────────────────────
# One line, on stderr, before the caller does any work with the path. The format is
# fixed because it is a cross-language contract: the Rust resolver prints the same
# shape, and a log grep that reads one must read the other.
#
# It deliberately carries neither of the two spellings the landing bar counts as an
# unmeasured gate (`scripts/landing-run.sh` greps both). An announce that a bar reads
# as a skipped gate would inflate that count on every successful run, which is the
# same defect in the other direction: a witness nobody can trust.
suite_paths_announce() {
    printf '# %s=%s (step %s: %s)\n' "$1" "$2" "$3" "$4" >&2
}

suite_paths_error() {
    printf 'suite-paths: REFUSING — %s\n' "$1" >&2
}

# ── What makes a directory a checkout of a named repo ─────────────────────────────
# `.git` plus a marker the repo cannot plausibly be without. The marker is what turns
# "a directory exists there" into "that is the repo", and it is the half that makes
# set-but-wrong detectable at all: a variable pointing at a sibling checkout, at a
# stale copy, or at the suite root itself has a `.git` as readily as the right answer.
#
# A name with no marker row is checked for `.git` only, and says so in its refusal,
# rather than being refused for a rule nobody has written yet.
suite_paths_markers() {
    case "$1" in
        aeon)     printf '%s\n' build.sh engine ;;
        sigil)    printf '%s\n' Cargo.toml crates/sigil-harness ;;
        empyrean) printf '%s\n' contract clients ;;
        *)        : ;;
    esac
}

# suite_paths_why_not <name> <dir> — empty when <dir> is a <name> checkout, else the
# reason, phrased to be read at the end of "…is not an aeon checkout (<reason>)".
suite_paths_why_not() {
    local name=$1 dir=$2 m
    [[ -n $dir ]] || { printf 'the empty string'; return 0; }
    [[ -d $dir ]] || { printf 'no such directory'; return 0; }
    # A linked worktree's `.git` is a FILE, not a directory. Testing for a directory
    # would refuse every worktree, which is the shape most of this suite runs in.
    [[ -e $dir/.git ]] || { printf 'no .git — that is not a checkout'; return 0; }
    while IFS= read -r m; do
        [[ -n $m ]] || continue
        [[ -e $dir/$m ]] || { printf 'no %s — that is not the %s checkout' "$m" "$name"; return 0; }
    done < <(suite_paths_markers "$name")
    printf ''
}

# ── The git derivation, step 3's engine ───────────────────────────────────────────
# Anchored at THIS FILE's own directory, not at $PWD. Two callers run from a directory
# that is not the repo root (`golden/capture_goldens.sh` runs from `golden/`, the
# nightly units run from wherever systemd starts them), and a derivation that depended
# on the caller's cwd would answer differently for each of them.
#
# `--git-common-dir` can answer relatively (`.git`) when asked from a repo root, so the
# result is resolved by `cd` rather than string-joined.
suite_paths_common_dir() {
    local here common
    here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || return 1
    common="$(cd "$here" && git rev-parse --git-common-dir 2>/dev/null)" || return 1
    [[ -n $common ]] || return 1
    (cd "$here" && cd "$common" && pwd) 2>/dev/null
}

# The directory the suite's checkouts hang off, derived: dirname of the main checkout,
# which is dirname of the common git dir.
suite_paths_derived_root() {
    local common root
    common="$(suite_paths_common_dir)" || return 1
    root="$(cd "$common/../.." && pwd)" 2>/dev/null || return 1
    printf '%s' "$root"
}

# ── suite_resolve_root [<consulted-by>] ───────────────────────────────────────────
# The suite root itself, for the things that hang off it and are NOT checkouts: the
# nightly lanes' detached trees and their dedicated target directories. Steps 1 and 3
# of the same precedence — there is no step 2 for the root, because step 2 IS the root.
suite_resolve_root() {
    local root why
    if [[ -n ${EMPYREAN_SUITE_ROOT:-} ]]; then
        root=${EMPYREAN_SUITE_ROOT}
        if [[ ! -d $root ]]; then
            suite_paths_error "EMPYREAN_SUITE_ROOT=$root is not a directory.
       A variable that is set but wrong is a wrong environment, not a missing one: the
       next step would resolve correctly and leave the wrong value in place."
            return 4
        fi
        root="$(cd "$root" && pwd)"
        suite_paths_announce EMPYREAN_SUITE_ROOT "$root" 1 "explicit EMPYREAN_SUITE_ROOT"
        printf '%s' "$root"
        return 0
    fi
    if root="$(suite_paths_derived_root)" && [[ -n $root ]]; then
        suite_paths_announce EMPYREAN_SUITE_ROOT "$root" 3 \
            "derived from this checkout via git --git-common-dir"
        printf '%s' "$root"
        return 0
    fi
    why="git --git-common-dir answered nothing from $(dirname "${BASH_SOURCE[0]}")"
    suite_paths_error "cannot locate the suite root.
       consulted  EMPYREAN_SUITE_ROOT  (unset)
       derived    $why
       Export EMPYREAN_SUITE_ROOT to the directory the suite's checkouts hang off."
    return 3
}

# ── suite_resolve_checkout <name> <VAR> ───────────────────────────────────────────
# The whole contract, in order. Echoes the resolved path on stdout; announces the step
# on stderr; returns 3 (nobody named it) or 4 (somebody named it wrongly).
#
#   AEON=$(suite_resolve_checkout aeon AEON_DIR) || exit $?
#
# The caller keeps its OWN explicit flag ahead of this call when it has one — a
# `--aeon <path>` on the command line is step 1 by another spelling, and the flag is
# the more explicit of the two.
suite_resolve_checkout() {
    local name=$1 var=$2
    local val cand why root root_rc tried_root='' tried_derived=''

    # STEP 1 — the explicit checkout variable.
    eval "val=\${$var:-}"
    if [[ -n $val ]]; then
        why="$(suite_paths_why_not "$name" "$val")"
        if [[ -n $why ]]; then
            suite_paths_error "$var=$val is not the $name checkout ($why).
       A variable that is set but wrong is a wrong environment, not a missing one, so
       this stops here rather than resolving $name some other way and leaving $var
       pointing somewhere else for everything downstream."
            return 4
        fi
        val="$(cd "$val" && pwd)"
        suite_paths_announce "$var" "$val" 1 "explicit $var"
        printf '%s' "$val"
        return 0
    fi

    # STEP 2 — the suite root, joined with the repo's directory name.
    if [[ -n ${EMPYREAN_SUITE_ROOT:-} ]]; then
        if [[ ! -d ${EMPYREAN_SUITE_ROOT} ]]; then
            suite_paths_error "EMPYREAN_SUITE_ROOT=${EMPYREAN_SUITE_ROOT} is not a directory,
       so the $name checkout cannot be joined onto it. A variable that is set but wrong
       is a wrong environment, not a missing one."
            return 4
        fi
        root="$(cd "${EMPYREAN_SUITE_ROOT}" && pwd)"
        cand="$root/$name"
        why="$(suite_paths_why_not "$name" "$cand")"
        if [[ -n $why ]]; then
            suite_paths_error "EMPYREAN_SUITE_ROOT=$root names a suite root whose $name
       entry ($cand) is not the $name checkout ($why). A variable that is set but wrong is
       a wrong environment, not a missing one."
            return 4
        fi
        cand="$(cd "$cand" && pwd)"
        suite_paths_announce "$var" "$cand" 2 "EMPYREAN_SUITE_ROOT/$name"
        printf '%s' "$cand"
        return 0
    fi
    tried_root='(unset)'

    # STEP 3 — derive from this repo's own location.
    if root="$(suite_paths_derived_root)" && [[ -n $root ]]; then
        cand="$root/$name"
        tried_derived="$cand"
        why="$(suite_paths_why_not "$name" "$cand")"
        if [[ -z $why ]]; then
            cand="$(cd "$cand" && pwd)"
            suite_paths_announce "$var" "$cand" 3 \
                "sibling of this checkout via git --git-common-dir"
            printf '%s' "$cand"
            return 0
        fi
        tried_derived="$cand ($why)"
    else
        root_rc=$?
        tried_derived="git --git-common-dir answered nothing from this file's directory (rc ${root_rc})"
    fi

    # STEP 4 — refuse, by name, naming everything consulted and everything tried.
    suite_paths_error "cannot locate the $name checkout.
       consulted  $var                  (unset)
       consulted  EMPYREAN_SUITE_ROOT   $tried_root
       tried      $tried_derived
       Export $var to the $name checkout, or EMPYREAN_SUITE_ROOT to the directory the
       suite's checkouts hang off. This does NOT fall back to a live working tree: a
       run against a tree nobody named is a run nobody can reproduce."
    return 3
}
