# shellcheck shell=bash
# sigil_tool.sh — choose the assembler a provisioning run will judge, and prove it
# corresponds to the tree it is being provisioned from.
#
# ── WHY THIS FILE EXISTS ─────────────────────────────────────────────────────────
# provision-aeon-ref.sh's own header argues that "no errors" is not a witness, and
# names the positive witness it wants instead. It then chose its single most
# important build input — the assembler — with
#
#     SIGIL_BIN="${SIGIL_BIN:-$SIGIL_ROOT/target/release/sigil}"
#
# which is whatever a SHARED checkout last relinked: not the revision being
# provisioned for, and not necessarily related to the change under test.
#
# On 2026-09-02 that default handed a three-parcel byte-neutrality proof a PRE-MERGE
# compiler, and the run's four `REBUILD CONTROL ... MATCHES THE GOLDEN` lines
# certified a build containing NONE of the three parcels. A second run of the same
# lane reached a binary four days old. FOUR MATCHING CRCs ARE EQUALLY CONSISTENT
# WITH "byte-neutral parcel" AND "the fix was never in the build" — and on a
# byte-neutral parcel those are the only two worlds there are, so the control cannot
# separate them. What separated them was a count that came out RIGHT when it had
# been predicted to come out WRONG: 51/51 where the merged compiler must give 50/51.
#
# The three-line discriminator, if you ever need to see the class again:
#     module d
#     ensure(1 != "x", "cross-type compare answered instead of refusing")
# A pre-merge binary prints `built: 0 bytes` and exits 0; a merged one refuses with
# `[eq.cross-type]`. Nothing in the provisioning log distinguished them.
#
# ── THE PIN SHAPE, AND WHY IT IS "BUILD IT" RATHER THAN "REQUIRE IT" ─────────────
# Three shapes were available: require SIGIL_BIN, default-but-verify, or build one.
#
#   * REQUIRE would make the ordinary correct invocation start refusing. The aeon
#     lane runs this script with no environment at all; a refusal-by-default lands
#     on them at whatever hour they next provision, for a variable they have never
#     had to set. This lane has already retracted one queued "make it refuse unless
#     told" fix for exactly that reason.
#   * DEFAULT-BUT-VERIFY keeps a default whose only defence is a check. The check is
#     the thing under budget pressure, and the default it guards is a shared
#     artifact other lanes relink on purpose.
#   * BUILD removes the class instead of detecting it. The binary is compiled from
#     THIS tree, including its uncommitted edits, into the reference target dir this
#     script already builds `emit_sound_blob` into — so it cannot be stale, cannot be
#     a peer's, and cannot be silently substituted. The shared
#     `target/release/sigil` is never read and never written.
#
# So: SIGIL_BIN unset builds one; SIGIL_BIN set is honoured, because a caller that
# names a binary is stating an intent this script has no standing to override — the
# nightly drift job builds its own at a published revision and hands it here. Both
# paths then go through the SAME correspondence check, because a self-built artifact
# is not self-evidently the one we just built: sibling worktrees share this target
# dir by default, so the build and the use are two events with a gap between them.
#
# ── THE CORRESPONDENCE CHECK, AND THE TRAP IN IT ─────────────────────────────────
# NOT `revision:` against `git rev-parse HEAD`. The binary's own `--version` says why
# in its `drift:` block: `revision` moves on EVERY commit here, including ones no
# compilation can see, so comparing it alone warns PERMANENTLY and therefore says
# nothing. As of this writing HEAD is four docs commits ahead of the last commit that
# could touch a compiler, so that comparison would fire right now, on a correct
# binary. A check that fires on correct code is not the safe direction: it trains
# people to weaken it, and the weakening gets written down as advice.
#
# The derived form the binary itself prescribes is used instead:
#     closure-revision   ==   git log -1 --format=%H HEAD -- <closure-paths>
# where `closure-paths` is what cargo actually compiles the binary from, walked from
# cargo's own dependency graph rather than listed by hand and reported BY THE BINARY.
#
# WHAT IT PROVES AND WHAT IT DOES NOT. It over-reports and never under-reports: a
# pass means later commits in this tree CANNOT have affected this binary. It is not
# a claim that the output would be identical — only a rebuild and a byte compare
# supports that. The refusal message says so rather than overclaiming.
#
# WHAT IS DELIBERATELY NOT GATED:
#   * `source:` — the directory the binary was built from. A perfectly correct
#     binary is routinely built somewhere else (a landing worktree, the freeze bin);
#     gating on it would refuse correct runs. Printed, never asserted.
#   * `tree:` dirty/clean. A parcel under measurement is often dirty on purpose, and
#     a self-built binary inherits that dirt by design. Printed, never asserted.
#
# ── THE DECLARED-MISMATCH HATCH, AND WHY IT CANNOT BE USED AS AN OFF SWITCH ──────
# One legitimate run pairs an OLD binary with a CURRENT tree: the base-compiler arm
# of an A/B, where the old compiler is the control. Refusing it would be the
# always-red failure again. `SIGIL_BIN_CLOSURE` is the hatch, and it is not a
# silencer: its value must be the 40-hex closure revision the binary ACTUALLY
# reports. Set it wrong and the run still refuses; set it right and the log carries
# the sha you deliberately measured with. It converts a silent mismatch into a
# declared one, and the declaration is itself checked.
#
# ── SOURCED OR EXECUTED ──────────────────────────────────────────────────────────
# suite_paths.sh is sourced-never-executed; this one is both, on purpose. A gate
# whose only demonstration is a full provisioning run is a gate nobody re-proves,
# and the two halves that matter — a wrong binary refuses, a correct binary proceeds
# silently — must both be runnable in a second:
#     scripts/lib/sigil_tool.sh <sigil-root> <ref-target>
# and the two halves it derives on its own are runnable alone:
#     scripts/lib/sigil_tool.sh --ref-target <sigil-root>
#     scripts/lib/sigil_tool.sh --anchor     <sigil-root>

# ── the refusal ──────────────────────────────────────────────────────────────────
# Terminal by design: every later step of provisioning is worthless once the tool is
# wrong, and a warning here is a line somebody reads after the fact.
sigil_tool_refuse() {
    printf '\nERROR: %s\n' "$1" >&2
    shift
    local line
    for line in "$@"; do printf '       %s\n' "$line" >&2; done
    exit 1
}

# ── the build directory, and why it has no shared default ────────────────────────
# sigil_tool_ref_target <sigil-root>
#
# Echo the directory this run compiles into. One implementation, because every caller
# that derives it separately derives it differently.
#
# THE PRECEDENCE, and there is no shared step in it:
#   1. REF_TARGET — the explicit override, a caller stating an intent;
#   2. CARGO_TARGET_DIR — a caller who has ALREADY chosen a build directory has
#      chosen this one too; reaching past it to a fixed path is how two lanes end up
#      writing one directory while each believes it owns it;
#   3. <sigil-root>/.target-ref — derived from the invoking tree, so it is unique per
#      worktree by construction and cannot be shared without being named.
#
# A SHARED DIRECTORY IS A CORRECTNESS FAULT, NOT AN UNTIDY ONE. Cargo's unit hash is
# checkout-independent, so a second worktree writes `deps/<name>-<hash>` with its own
# absolute CARGO_MANIFEST_DIR baked in; the first worktree then sees a matching
# fingerprint, does not rebuild, and runs a binary compiled against another checkout's
# paths. That surfaces as missing fixtures on files that are present and reads exactly
# like golden divergence. `cargo clean` does not fix it — only a per-worktree directory
# does. The predecessor default here was `<sigil-root>/../.sigil-ref-target`, one path
# for every worktree of this repo, and it was measured holding a `sigil` reporting a
# branch that had been deleted hours earlier beside rlibs from a different lane's tree.
#
# The two spellings of a checkout's own `target/` are refused rather than merely not
# defaulted to, because they are the same artifact reached by a different route: that
# directory is relinked deliberately by other lanes and pinned by hash by some of them.
sigil_tool_ref_target() {
    local root="$1" dir origin common main

    if [ -n "${REF_TARGET:-}" ]; then
        dir="$REF_TARGET"
        origin="1: explicit REF_TARGET"
    elif [ -n "${CARGO_TARGET_DIR:-}" ]; then
        dir="$CARGO_TARGET_DIR"
        origin="2: the caller's CARGO_TARGET_DIR"
    else
        dir="$root/.target-ref"
        origin="3: this checkout's own .target-ref (neither REF_TARGET nor CARGO_TARGET_DIR is set)"
    fi

    dir="$(sigil_tool_abspath "$dir")"

    # The main checkout, whose `target/` is the shared one. From a linked worktree
    # `--git-common-dir` answers with the MAIN checkout's git dir, which is the fact a
    # sibling derivation needs; from the main checkout it answers the same way.
    common="$(cd "$root" 2>/dev/null && git rev-parse --git-common-dir 2>/dev/null)" || common=""
    main=""
    if [ -n "$common" ]; then
        case "$common" in /*) ;; *) common="$root/$common" ;; esac
        main="$(cd "$common/.." 2>/dev/null && pwd)" || main=""
    fi

    local forbidden
    for forbidden in "$root/target" ${main:+"$main/target"}; do
        [ "$dir" = "$(sigil_tool_abspath "$forbidden")" ] || continue
        sigil_tool_refuse \
            "the build directory resolves to $dir, which is a checkout's DEFAULT target/." \
            "" \
            "  resolved by  step $origin" \
            "" \
            "That directory is shared: other lanes relink it on purpose and some pin its" \
            "binary by hash. Cargo's unit hash is checkout-independent, so a build there" \
            "from a second worktree hands a later run artifacts compiled against a" \
            "DIFFERENT checkout's absolute paths, missing fixtures on files that are" \
            "present, which reads exactly like golden divergence, and which \`cargo clean\`" \
            "does not fix." \
            "" \
            "Use a directory of this run's own: unset the variable above to get" \
            "$root/.target-ref, or point REF_TARGET somewhere this tree alone writes."
    done

    printf 'ref-target: %s (step %s)\n' "$dir" "$origin" >&2
    printf '%s\n' "$dir"
}

# Absolute form of a path that need not exist yet: its parent is resolved when it does,
# so two spellings of one directory compare equal, and a not-yet-created leaf is kept.
sigil_tool_abspath() {
    local p="${1%/}" parent base
    [ -n "$p" ] || { printf '%s\n' "$1"; return; }
    parent="$(dirname "$p")"
    base="$(basename "$p")"
    if [ -d "$parent" ]; then
        printf '%s/%s\n' "$(cd "$parent" && pwd)" "$base"
    else
        printf '%s\n' "$p"
    fi
}

# ── what the comparison is anchored to, said out loud ────────────────────────────
# sigil_tool_anchor <sigil-root>
#
# Echo one line naming the revision this run compares against AND where that revision
# stands relative to what anyone else can see.
#
# WHY THE SECOND HALF. The comparison below is against local HEAD, and that is the
# correct anchor for the question it asks: does this binary correspond to the tree being
# provisioned FROM. What it does not settle is what a reader should do about a mismatch,
# because on this machine every sibling checkout is a peer's live working tree, so a
# local HEAD can be ahead of, behind, or divergent from anything another lane holds.
# `behind` is not a fact until something names what it is behind — the aeon lane read
# exactly that word and had to run a scoped diff against origin/master by hand to turn it
# into a measurement.
#
# SO THE ANCHOR IS NAMED RATHER THAN MOVED. Anchoring the refusal at the remote instead
# would refuse every lane holding unpushed commits, which is the ordinary state of work
# in progress and would be an always-red check: it fires on correct work, and the remedy
# a reasonable person reaches for is deleting the guard.
#
# The remote-tracking ref, not `git ls-remote`. The remote here is an SSH URL, so asking
# the server blocks, needs an agent, and fails offline — inside a script that runs before
# every provisioning. The ref is named as the LOCAL CACHE it is, with what refreshes it,
# so a reader who needs the real answer knows the one command that gets it.
sigil_tool_anchor() {
    local root="$1" head branch upstream tip standing
    head="$(git -C "$root" rev-parse HEAD 2>/dev/null)" || head=""
    branch="$(git -C "$root" symbolic-ref --quiet --short HEAD 2>/dev/null)" || branch="detached"

    # `@{upstream}` when the branch tracks one; otherwise the remote's own default branch
    # as recorded here. A parcel branch and a detached checkout track nothing, and both
    # are the ordinary shapes in this repo.
    upstream="$(git -C "$root" rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null)" \
        || upstream="$(git -C "$root" symbolic-ref --quiet --short refs/remotes/origin/HEAD 2>/dev/null)" \
        || upstream=""

    if [ -z "$upstream" ]; then
        standing="nothing on this machine names a published tip (no upstream branch, no refs/remotes/origin/HEAD), so how this HEAD stands against the remote is UNKNOWN"
    else
        tip="$(git -C "$root" rev-parse "$upstream" 2>/dev/null)" || tip=""
        if [ -z "$tip" ]; then
            standing="$upstream does not resolve here, so how this HEAD stands against the remote is UNKNOWN"
        elif git -C "$root" merge-base --is-ancestor "$head" "$tip" 2>/dev/null; then
            standing="that HEAD is contained in $upstream ($tip)"
        else
            standing="that HEAD is NOT contained in $upstream ($tip), which is ordinary for unpushed lane work and is not a fault"
        fi
        [ -n "$tip" ] && standing="$standing; $upstream is a LOCAL remote-tracking ref, refreshed only by \`git fetch\`"
    fi

    printf 'HEAD %s on %s in %s, %s\n' "${head:-unknown}" "$branch" "$root" "$standing"
}

# sigil_tool_resolve <sigil-root> <ref-target>
#
# Sets SIGIL_BIN to a binary that has been proved to correspond to <sigil-root>, and
# heads the log with that binary's own --version self-report. Refuses, loudly and
# before any side effect, if it cannot.
sigil_tool_resolve() {
    local root="$1" ref_target="$2" origin

    if [ -n "${SIGIL_BIN:-}" ]; then
        origin="pinned by the caller (SIGIL_BIN)"
        [ -f "$SIGIL_BIN" ] || sigil_tool_refuse \
            "SIGIL_BIN names a file that does not exist: $SIGIL_BIN" \
            "Nothing is guessed in its place. The default this script used to fall back to" \
            "is the defect it now refuses to repeat."
        [ -x "$SIGIL_BIN" ] || sigil_tool_refuse \
            "SIGIL_BIN is not executable: $SIGIL_BIN"
    else
        # NEVER $root/target/release/sigil. That path is a shared artifact other
        # lanes relink deliberately, and from a linked worktree it does not even
        # name the tree the binary would have come from.
        origin="built by this run from $root"
        echo "==> no SIGIL_BIN given; building the assembler from this tree"
        ( cd "$root" && CARGO_TARGET_DIR="$ref_target" cargo build --release --bin sigil ) \
            || sigil_tool_refuse \
                "the assembler did not build from $root" \
                "Provisioning cannot continue: every later step runs this binary."
        SIGIL_BIN="$ref_target/release/sigil"
        [ -x "$SIGIL_BIN" ] || sigil_tool_refuse \
            "cargo reported success but $SIGIL_BIN is not there"
    fi

    # ── the self-report, verbatim, at the head of the log ────────────────────────
    # Provenance belongs in the artifact, not in the operator's memory. A run whose
    # header does not name the revision under test did not measure it, and nothing
    # else in the log can say so afterwards.
    local version_out
    version_out="$("$SIGIL_BIN" --version 2>&1)" || sigil_tool_refuse \
        "$SIGIL_BIN cannot report its version, so this run has no honest provenance" \
        "$(printf '%s' "$version_out" | head -3 | tr '\n' ' ')"

    echo
    echo "==> THE ASSEMBLER THIS RUN WILL JUDGE, $origin"
    echo "    $SIGIL_BIN"
    printf '%s\n' "$version_out" | sed 's/^/    | /'
    echo

    # ── the correspondence check ─────────────────────────────────────────────────
    local closure_rev closure_paths tree_rev
    closure_rev="$(printf '%s\n' "$version_out" \
        | sed -n 's/^  closure-revision: *\([0-9a-f]\{40\}\).*$/\1/p')"
    closure_paths="$(printf '%s\n' "$version_out" | sed -n 's/^  closure-paths: //p')"

    # A binary too old to state what it was compiled from cannot be checked at all,
    # and "cannot be checked" is not "is fine". This is a refusal on an
    # unverifiable binary, not on a correct one.
    [ -n "$closure_rev" ] && [ -n "$closure_paths" ] || sigil_tool_refuse \
        "$SIGIL_BIN does not self-report a closure revision and closure paths" \
        "It predates the provenance banner, so nothing can say which tree it came from." \
        "Rebuild it, or run with SIGIL_BIN unset and let this script build one."

    local -a paths
    read -r -a paths <<< "$closure_paths"
    tree_rev="$(git -C "$root" log -1 --format=%H HEAD -- "${paths[@]}" 2>/dev/null)" \
        || tree_rev=""
    [ -n "$tree_rev" ] || sigil_tool_refuse \
        "cannot derive this tree's closure revision at $root" \
        "Asked: git log -1 --format=%H HEAD -- <the ${#paths[@]} paths the binary names>" \
        "Either $root is not a git checkout, or the binary names paths this tree" \
        "has never had, both mean the two cannot be compared, so neither is assumed."

    if [ "$closure_rev" = "$tree_rev" ]; then
        # Silent-by-design on the correct case beyond this one line: the check must
        # cost a correct run nothing, or it becomes the thing people delete.
        echo "==> tool closure ${closure_rev:0:8} == this tree's closure at HEAD, no commit here can have affected it"
        echo "    compared against $(sigil_tool_anchor "$root")"
        echo "    (that is 'cannot have affected', not 'the output is identical'; only a rebuild and a byte compare says the second)"
    elif [ -n "${SIGIL_BIN_CLOSURE:-}" ] && [ "$SIGIL_BIN_CLOSURE" = "$closure_rev" ]; then
        echo "==> tool closure ${closure_rev:0:8} DIFFERS from this tree's ${tree_rev:0:8}, and SIGIL_BIN_CLOSURE declares exactly that"
        echo "    Accepted as a deliberate off-tree measurement. The sha is in this log because you had to type it."
    else
        # A WRONG declaration is its own diagnosis, and a distinct one: it means the
        # caller believed something specific about this binary that is not true.
        local declared="(SIGIL_BIN_CLOSURE is unset, no off-tree measurement was declared.)"
        if [ -n "${SIGIL_BIN_CLOSURE:-}" ]; then
            declared="SIGIL_BIN_CLOSURE says $SIGIL_BIN_CLOSURE, which is NOT what this binary reports. A declaration is checked, not trusted."
        fi
        sigil_tool_refuse \
            "the assembler does not correspond to the tree being provisioned. REFUSING." \
            "" \
            "  binary      $SIGIL_BIN" \
            "  its closure $closure_rev" \
            "  this tree   $tree_rev   (git log -1 HEAD -- <closure-paths> at $root)" \
            "  anchored at $(sigil_tool_anchor "$root")" \
            "" \
            "A commit that this binary could not have seen has touched the sources it is" \
            "compiled from. This over-reports and never under-reports: it proves 'cannot" \
            "have been affected', so a mismatch is not proof the output differs, but a" \
            "control built by an unknown compiler is not a control, and four matching CRCs" \
            "cannot tell a byte-neutral parcel from a compiler that never had the parcel." \
            "" \
            "  * to measure THIS tree: unset SIGIL_BIN and let this script build the tool," \
            "    or point SIGIL_BIN at a binary built from it;" \
            "  * to measure with an OFF-TREE compiler on purpose (the base arm of an A/B):" \
            "        SIGIL_BIN_CLOSURE=$closure_rev" \
            "    which is checked against the binary, not trusted, a wrong sha still refuses." \
            "" \
            "  $declared"
    fi
}

# ── executed directly: the gate on its own, so both halves stay cheap to prove ───
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    set -euo pipefail
    # `--ref-target <root>` answers the build-directory half alone. It is the half a
    # gate can run in milliseconds, and a precedence nobody can exercise without a
    # cargo build is a precedence nobody re-proves.
    if [ "${1:-}" = "--ref-target" ]; then
        shift
        [ $# -ge 1 ] || { echo "usage: sigil_tool.sh --ref-target <sigil-root>" >&2; exit 2; }
        sigil_tool_ref_target "$1"
        exit 0
    fi
    # `--anchor <root>` answers the "compared against what" half alone, for the same
    # reason: a line nobody can print without a cargo build is a line nobody re-proves.
    if [ "${1:-}" = "--anchor" ]; then
        shift
        [ $# -ge 1 ] || { echo "usage: sigil_tool.sh --anchor <sigil-root>" >&2; exit 2; }
        sigil_tool_anchor "$1"
        exit 0
    fi
    if [ $# -lt 1 ]; then
        echo "usage: sigil_tool.sh <sigil-root> [<ref-target>] | --ref-target <sigil-root>" >&2
        exit 2
    fi
    # Assigned, not inlined into the argument: `sigil_tool_ref_target` refuses by
    # exiting, and inside a command substitution that exits the SUBSHELL — as an
    # argument default the refusal would print and the caller would carry on with an
    # empty build directory. An assignment under `set -e` propagates it.
    ref_target_arg="${2:-}"
    if [ -z "$ref_target_arg" ]; then
        ref_target_arg="$(sigil_tool_ref_target "$1")"
    fi
    sigil_tool_resolve "$1" "$ref_target_arg"
    echo "==> resolved: $SIGIL_BIN"
fi
