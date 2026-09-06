#!/usr/bin/env bash
# Standing backstop for sigil's AEON-SOURCE-DERIVED gates.
#
# WHY THE TRIGGER IS A CLOCK AND NOT A REFREEZE. The warn-tier corpus and its
# neighbours read aeon SOURCE through AEON_DIR, but nobody ran them against a
# fresh aeon tip except at refreeze time — and a refreeze happens only when ROM
# bytes move. Six consecutive zero-byte aeon parcels therefore hid a real
# `layout.odd-field` finding for a day. A ritual keyed to byte movement is
# structurally blind to a source-derived lint set moving, so this lane runs on a
# clock instead and covers every source-derived check without aeon's ritual
# having to enumerate them. Diagnosis:
# docs/superpowers/notes/2026-08-22-warn-tier-drift-open.md
#
# Both ends are DETACHED checkouts of committed master tips, outside their repo
# roots. Outside, for the same reason aeon's nightly gives: a worktree under the
# repo root double-counts every module in aeon's tools/emp_helper_closure.py tree
# scan. Detached and committed, so the lane's verdict is about master-vs-master —
# a session's uncommitted work in either checkout cannot colour it, and the aeon
# main tree carries the owner's live content edits at all times.
#
# Exit-code contract, mirroring aeon/tools/nightly_effects_gates.sh:
#   0  every gate passed
#   1  a gate FAILED
#   2  the lane COULD NOT RUN
# Both nonzero cases notify. A backstop that silently cannot run is the
# vacuous-gate pattern this exists to prevent.
#
# --selftest-fail exercises the notification path without running anything.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STATE=${XDG_STATE_HOME:-$HOME/.local/state}/sigil-source-gates
LOG="$STATE/nightly.log"
mkdir -p "$STATE"

# ── WHERE THE TREES ARE, read at RUN TIME ────────────────────────────────────────
# Five home literals stood here. contract/SUITE_PATHS.md rules one precedence for every
# resolver in the suite — explicit <TOOL>_DIR, then EMPYREAN_SUITE_ROOT joined with the
# repo name, then the sibling derived from this checkout's own `git --git-common-dir`
# (never `--show-toplevel`, which answers wrongly from a worktree), then a refusal that
# names all of them — and scripts/lib/suite_paths.sh is the one implementation of it.
# This lane and the drift lane are the only sites a timer runs with NO override at all,
# which is exactly why they must not carry a path one person's machine happens to have.
#
# THE TWO ARGUMENT-ONLY PATHS ARE EXEMPT, and that is not a softening. `--selftest-fail`
# exists to prove the notification path in an environment where the rest does not work,
# and `--audit` is read-only, derives its own tree from this file's location, and is run
# by `crates/sigil-harness/tests/source_gate_classification.rs` on every workspace test
# run — including in a checkout with no engine sibling, where requiring one would turn a
# correct absence into a red suite. Neither reaches the variables below; the sentinels
# make that structural rather than assumed, and read as a name if one ever does.
SIGIL_MAIN='<unresolved: an argument-only path took no reference tree>'
AEON_MAIN="$SIGIL_MAIN"
SIGIL_GATES="$SIGIL_MAIN"
AEON_GATES="$SIGIL_MAIN"
if [[ ${1:-} != --selftest-fail && ${1:-} != --audit ]]; then
    # shellcheck source=lib/suite_paths.sh
    source "$HERE/lib/suite_paths.sh" || {
        echo "COULD NOT RUN: cannot source $HERE/lib/suite_paths.sh, so no tree can be named" >&2
        exit 2
    }
    SUITE_ROOT=$(suite_resolve_root) || exit 2
    SIGIL_MAIN=$(suite_resolve_checkout sigil SIGIL_DIR) || exit 2
    AEON_MAIN=$(suite_resolve_checkout aeon AEON_DIR) || exit 2
    # This lane's own detached checkouts, outside both repo roots (see the header for
    # why outside). Not checkouts of anything until the lane creates them, so they are
    # joined onto the resolved root rather than resolved as repos.
    SIGIL_GATES="$SUITE_ROOT/.sigil-source-gates"
    AEON_GATES="$SUITE_ROOT/.aeon-sigil-gates"
    # On disk, never under /tmp: /tmp is tmpfs here and a cargo build there wedges
    # the shell.
    export CARGO_TARGET_DIR="$SUITE_ROOT/.sigil-source-gates-target"
fi

# THE GATES THIS LANE RUNS: every sigil test binary whose inputs are aeon SOURCE
# plus sigil's own compilation of it. Nothing here reads a built aeon ROM, a
# listing, or a golden CRC — those need `./build.sh` to have run in the aeon tree
# and belong to the artifact lane, deliberately not this one (see EXCLUDED below).
SOURCE_GATES=(
    # THE BRICK WITNESS: every shipped shape still BUILDS from aeon source, judged by
    # the build entry `sigil build` reaches, with no byte compared to any committed
    # artifact. A brick — the compiler refusing the corpus — is a source-only fact
    # that no refreeze clears, and this is the one gate whose failure text names it
    # as such (the verdict line below reads that back out).
    corpus_builds
    # the layout walk under upstream code growth: a doctored copy of the tree must
    # still build (the 2026-08-26 measure-at-packed-base reproduction) — source only,
    # judged by the same build entry as the brick witness
    measure_at_packed_base
    # the warn tier over the real corpus — the gate the ruling is about
    warn_tier_corpus
    # the suite's own account of how much of itself it did not measure. Source-only in
    # the strictest sense: it reads sigil's own test sources to DERIVE the
    # reference-dependent population and never opens a ROM, a listing or a golden.
    # It is listed here rather than left to fall through because a file naming the
    # reference tree that is in neither SOURCE_GATES nor the artifact bucket is
    # UNCLASSIFIED, and an unclassified file makes this whole lane refuse to run —
    # so adding the gate without adding this line would have darkened the lane
    # nightly, over a file whose entire purpose is to stop things being silent.
    reference_dependence_is_named
    # the d-18 refusal: a run nobody gave a reference tree STOPS, and a declared partial
    # run says how much it left alone. Here for the same reason as the row above, and it
    # is the same shape: it reads sigil's own test sources to derive the not-measured
    # size and never opens a file in any aeon tree — every child it spawns is aimed at a
    # scrubbed environment or at this repo's own directory. It calls `reference_tree`,
    # though, and the classifier below is a static one: it cannot tell a call aimed at a
    # fixture from a call aimed at a real tree, so the file would otherwise be
    # UNCLASSIFIED and darken this whole lane.
    bare_run_refuses
    # whole-corpus source analyses
    contract_closure_corpus
    dead_save_corpus
    movem_restore_guard_corpus
    out_verify_corpus
    preserves_corpus
    slot_type_corpus
    cfg_blind_spots
    # the L1 contract env the isolated port oracles bind: aeon's own
    # engine/system/game_contract.emp against each shipped game's manifest, with the
    # member list read out of the interface. Source only — nothing built, nothing
    # compared to a committed artifact.
    game_contract_env_coverage
    # per-game [defines]: synthetic maps, plus a reference-gated walk of the shipped
    # games' own map.toml — source only, no ROM
    game_config_defines
    # negative probes: doctor an aeon source file, require the compiler to object
    core_negative_probes
    dplc_negative_probes
    hblank_negative_probes
    mt_negative_probes
    sfx_negative_probes
    tranche2_negative_probes
    tranche5_negative_probes
    tranche6_negative_probes
    tranche7_negative_probes
    tranche24_spelling_probes
    z80_clobbers_incomplete
    # source-derived drift and derivation gates
    act_fixture_drift
    banked_carrier_drift
    # the derived-layout invariants, read off the same source resolve the ROM comes from
    derived_layout
    # the placement contract's `[[hole]]` half over each shipped shape's own map.toml and
    # the real resolve, in both directions: every live hole holds nothing but its filler,
    # and a hole whose right edge is widened past the post-hole data (in memory, never in
    # the aeon tree) is refused by name. Source only — the oracle is the map's own
    # declarations, so no refreeze clears or colours it.
    hole_interior_reserved
    # every shipped shape's every section satisfies the alignment `section_align.rs`
    # DECLARES for it, judged before the packing walk and again against the base the
    # section actually lands on. Source only — the requirement is declared in sigil
    # source, not measured off a frozen base.
    section_alignment_declared
    # each `[[region]]`'s declared END CONTRACT against the live layout: a region whose
    # `end` is a label it does not own must say so, and the strict reading is the default.
    # Source only — what is compared is the KIND of contract, which is stable across
    # refreezes; no byte count is declared and no built ROM or assembler listing is read.
    region_end_contracts
    # the MD Debugger island's per-shape MEMBERSHIP: the shapes declaring the island are
    # exactly the shapes whose builds define its blob label, set-diffed in both
    # directions against an expectation taken from each profile's registry rather than
    # from the build. Source only — it compiles the corpus and reads symbol NAMES; no
    # byte is compared to anything committed.
    error_handler_island_membership
    # every shipped shape's listing declares which of its addresses are phased (VMA
    # != LMA) and which are not, judged against the sections the resolve produced.
    # Source only, and the same shape as the row above: it compiles the corpus and
    # reads the assembler's own answer about its sections. It opens no built ROM, no
    # listing FILE and no golden, so no refreeze clears or colours it.
    #
    # A near-twin, `listing_defines`, sits in the ARTIFACT bucket instead, and the
    # difference is an accident of prose rather than of inputs: that file mentions a
    # listing's file extension and this one does not, and the classifier below matches
    # that spelling on the whole file. Named here so a later reader does not conclude
    # the two were judged to have different inputs. They do not.
    listing_phase_marker
    p5_constants_flip
    parcel_8b_stage_gen_touchers
    seam2_layout_derivation
    structs_module
    # source-only compile and round-trip oracles
    dac_port
    diag_assert_vector
    game_debug_port
    m68k_roundtrip_stream
    # same corpus as the row above, judged by capstone instead of by sigil's own
    # encoder — source only, no ROM and no golden
    m68k_capstone_stream
    native_object_bank_budget
    seam2_colink_probe
    seam2_phased_head
    subcommands
    # THIS LANE'S OWN SKIP CHECK, kept from going blind. The grep below is only as
    # wide as the spelling it matches on, and 29 announced early returns spelled it
    # `skipping <gate> …` — invisible to the bar and reading back as coverage. This
    # gate holds every announcement site in the test tree to the one SKIP_MARKER the
    # grep derives, so a new site cannot be written in a spelling this lane cannot
    # see. Source only: it reads sigil's own test sources and builds nothing.
    skip_marker_lint
)

# DELIBERATELY EXCLUDED, and why. These read bytes that only exist after a build:
#   - the ~63 `*_port` region diffs, m1b_gate, m1c_vector_table and
#     repin_pins::pins_rs_is_current read $AEON_DIR/s4.bin / s4.debug.bin / s4.lst;
#   - the ~18 golden-CRC gates (boot_port, native_full_rom, the seam2 co-links,
#     math_port, mt_port, sfx_port, …) compare against
#     crates/sigil-harness/golden/*.bin and provenance.toml.
# Those gates have their own trigger: aeon's byte-identity ritual, which fires
# exactly when bytes move — the correct trigger for a BYTE comparison and the wrong
# one for these. The lane does still BUILD every shipped shape from source
# (`corpus_builds`, a few seconds per shape): whether the compiler accepts the corpus
# is a source fact and is red here on purpose, while whether the bytes match a
# committed image stays the ritual's question.
#
# A THIRD SHAPE, and the one the two buckets above do not name: gates that read
# aeon SOURCE ONLY but are ORACLE'D on a committed sigil artifact — the frozen
# `golden/*.bin` blobs, `golden/provenance.toml`, or `src/pins.rs`. Their inputs
# would run in this lane; their EXPECTATIONS would not. Between an aeon parcel that
# legitimately moves bytes and sigil's refreeze of the artifact those gates compare
# against, they are red by design, so a nightly clock would report a window the
# refreeze ritual already owns. They stay excluded for that reason — which is a
# property of the oracle, not of how they detect the reference tree. The audit below
# classifies them by the artifact they name in their own text, so a file in this
# shape that stops naming one becomes UNCLASSIFIED and the lane refuses to run: loud,
# and the safe direction.
#
# AND A FOURTH, WHICH IS NOT A LANE AT ALL: a file that names the reference tree without
# ever OBTAINING one. The detector below matches an IDENTIFIER, and an identifier appears
# in the file that explains it as readily as in the file that calls it — a gate whose
# whole subject is what happens when `$AEON_DIR` is absent says `$AEON_DIR` on every
# other line while pointing every call at a path that does not exist. Such a file is
# neither a source gate nor artifact-dependent: this lane has nothing to run for it, and
# the ordinary workspace suite already runs it on every invocation, with no reference
# tree needed. It is bucketed as `no-reference`, counted in the verdict, and NOT a
# defect. The membership is DERIVED per file by `classify` below — never a roster, whose
# failure mode is a file that reads the tree being waved through because someone typed
# its name.

SUPPORT_RS=crates/sigil-harness/src/test_support.rs

# The environment variable that names the reference tree, read out of the harness that
# reads it rather than retyped here. A second spelling of a constant is a second thing to
# keep in step, and this lane has already been bitten once by exactly that (SKIP_MARKER).
reference_env_var() {
    sed 's@^[[:space:]]*//.*@@' "$1" \
        | sed -n 's/.*env::var("\(AEON[A-Z0-9_]*\)").*/\1/p' | sort -u | head -1
}

# THE ACCESSORS THAT YIELD THE REFERENCE TREE, derived from the harness by closure and
# never listed. Seed: the public function of test_support.rs that reads the environment
# variable above. Step: any public function of that file whose body calls one already in
# the set. A new accessor spelled there joins this set with no edit to this script.
#
# Comment-only lines are stripped first. A doc comment showing a caller how to open a
# gate contains a call to the accessor, and attributing it to the function it happens to
# sit above manufactures accessors that nothing calls.
#
# The iteration bound is generous (the live chain is three deep) and NOT a silent cap:
# reaching it without a fixed point returns nonzero. A truncated closure is short some
# accessors, and every accessor it is short makes some file look like it reads nothing —
# the one direction in which being wrong is quiet.
#
# THE DOMAIN IS THIS ONE FILE, and that is a known edge rather than an oversight. A test
# reaching the tree through a shared helper module — `crates/*/tests/<dir>/mod.rs`, which
# the selector's own glob does not scan either — would call nothing this closure knows and
# would bucket as no-reference. Not live: no such module names the tree today, checkable
# with `grep -lE '<the selector pattern>' crates/*/tests/*/mod.rs`. If it ever fires, widen
# this closure to those modules — the same fixed point over one more file set.
#
# THE CLOSURE IS WIDER THAN THE ANSWER, and the last step narrows it. Since the SUITE_PATHS
# resolver landed, the function that READS the variable is not the function that YIELDS a
# reference tree: `aeon_checkout` answers "which checkout" and hands back a step and a path
# for a caller to judge, while `aeon_dir` is the one that commits to a tree a gate will
# measure against. The seed has to be the reader (that is the only anchor derivable from
# source), so the closure necessarily passes through the resolver — and emitting the
# resolver as an accessor makes every file that merely ASKS which checkout would be
# resolved, such as the resolver's own precedence gate, look like a file that reads aeon.
#
# So the final set keeps only members whose signature returns a PATH. That is read off the
# source like everything else here, and it is exactly the distinction above: a function
# handing back a `PathBuf` is handing back a tree to read, and one handing back a verdict
# is not.
#
# THE HOLE THIS LEAVES, stated: a caller that takes `aeon_checkout()`'s answer apart and
# joins onto its `.path` reaches the tree through a member this filter drops. One file does
# that today and it is a `SOURCE_GATES` member, so it never reaches this question. What
# keeps the hole from widening quietly is not this script:
# `crates/sigil-harness/tests/source_gate_classification.rs` holds the published set to the
# guard names `sigil_harness::reference_dependence::GUARDS` declares, so a new
# path-yielding accessor — or an existing one dropping out — is a red test rather than a
# silently smaller answer.
accessor_closure() {
    local src=$1 stripped acc more pat i
    stripped=$(sed 's@^[[:space:]]*//.*@@' "$src")
    local scan='
        /^[[:space:]]*(pub )?fn / {
            name = $0; pub = ($0 ~ /pub fn /); yields = ($0 ~ /PathBuf/)
            sub(/^[[:space:]]*(pub )?fn /, "", name); sub(/[(<].*/, "", name)
            next
        }
    '
    acc=$(awk "$scan"'
        /env::var\("AEON[A-Z0-9_]*"\)/ { if (name != "" && pub) print name }
    ' <<< "$stripped" | sort -u)
    [[ -n $acc ]] || { printf '%s\n' "$acc"; return 0; }
    for i in $(seq 1 12); do
        pat=$(paste -sd'|' <<< "$acc")
        more=$(awk -v pat="$pat" "$scan"'
            {
                if (name != "" && pub && $0 ~ ("(^|[^A-Za-z0-9_])(" pat ")[ \t]*\\("))
                    print name
            }
        ' <<< "$stripped" | sort -u)
        more=$(printf '%s\n%s\n' "$acc" "$more" | sort -u)
        if [[ $more == "$acc" ]]; then
            # The narrowing, and it is the LAST step on purpose: the closure has to walk
            # through the non-yielding members to reach the yielding ones.
            awk -v pat="$(paste -sd'|' <<< "$acc")" "$scan"'
                END { for (n in out) print n }
                { if (name != "" && pub && yields && name ~ ("^(" pat ")$")) out[name] = 1 }
            ' <<< "$stripped" | sort -u
            return 0
        fi
        acc=$more
    done
    return 1
}

# THE CLASSIFIER, one definition with two callers: `--audit` (read-only, and what the
# workspace suite runs) and the nightly run below. Sets CLS_* and returns 0; on anything
# it cannot MEASURE it sets CLS_REFUSAL and returns 2, so neither caller can render an
# unanswerable question as an empty bucket. That direction is the whole point: an empty
# accessor set would make every file look like it reads nothing, and the lane would go
# green over a population it never classified.
classify() {
    local tree=$1 f n src var accessors obtains scanned=0
    CLS_SOURCE=(); CLS_ARTIFACT=(); CLS_NOREF=(); CLS_UNCLASSIFIED=()
    CLS_SCANNED=0; CLS_REFUSAL=""; CLS_ACCESSORS=""
    src="$tree/$SUPPORT_RS"
    [[ -r $src ]] || {
        CLS_REFUSAL="$SUPPORT_RS is unreadable in $tree, the reference-tree rule is \
derived from it, so this run cannot tell a file that READS the tree from one that only \
names it"
        return 2
    }
    var=$(reference_env_var "$src")
    [[ -n $var ]] || {
        CLS_REFUSAL="no reference-tree environment variable is extractable from \
$SUPPORT_RS in $tree, half the read rule has no pattern"
        return 2
    }
    accessors=$(accessor_closure "$src") || {
        CLS_REFUSAL="the reference-tree accessor set over $SUPPORT_RS in $tree did not \
reach a fixed point, a truncated closure is short accessors, and each one it is short \
makes some file look like it reads nothing"
        return 2
    }
    # PUBLISHED, not just consumed. An EMPTY closure refuses below and is loud; a
    # closure that is merely SHORT is not — every accessor it misses makes some file
    # look like it reads nothing, and that file then buckets as `no-reference` and is
    # waved through. Nothing in this script can see that from the inside, because a
    # short closure is self-consistently short. So the set is published and
    # `crates/sigil-harness/tests/source_gate_classification.rs` holds it to the guard
    # names `sigil_harness::reference_dependence::GUARDS` declares.
    CLS_ACCESSORS=$(tr '\n' ' ' <<< "$accessors")
    [[ -n $accessors ]] || {
        CLS_REFUSAL="no reference-tree accessor is derivable from $SUPPORT_RS in $tree, \
the read rule has no pattern, so every file would falsely look like it reads nothing"
        return 2
    }
    obtains="(^|[^A-Za-z0-9_])($(paste -sd'|' <<< "$accessors"))[[:space:]]*\("
    while IFS= read -r f; do
        scanned=$((scanned + 1))
        n=$(basename "$f" .rs)
        # PROSE IS NOT A CALL. The read test below runs against the file with comment-only
        # lines removed, exactly as `reference_env_var` and `accessor_closure` already do
        # when deriving the rule from the harness — and for the reason stated there: a doc
        # comment showing a caller how to open a gate CONTAINS a call to the accessor.
        #
        # Without this, a pure unit gate whose header says `let Some(aeon) = aeon_dir()` as
        # an illustration — or one whose prose merely mentions `env::var("AEON_DIR")` — is
        # classified as reference-reading, lands in neither bucket, and MAKES THE WHOLE LANE
        # REFUSE TO RUN. Both reproduced against a controlled tree before this was written.
        #
        # The artifact test above is deliberately NOT decommented, and that asymmetry is
        # measured rather than assumed: SIX files in this tree (`dac_port`,
        # `diag_assert_vector`, `game_debug_port`, `native_object_bank_budget`,
        # `banked_carrier_drift`, `subcommands`) match the artifact pattern ONLY in prose.
        # Decommenting there would push all six out of the artifact bucket and into exactly
        # the refusal this change exists to prevent. Blast radius of the change that WAS
        # made: zero — no file in this tree currently reads the tree by comment alone.
        decommented=$(sed 's@^[[:space:]]*//.*@@' "$f")
        # Asked in this order on purpose. The two established buckets answer first and
        # unchanged, so the new question can only ever speak for a file that used to fall
        # through to UNCLASSIFIED — a rule that re-bucketed a file already bucketed
        # correctly would be a regression, and this ordering makes that impossible rather
        # than merely unobserved.
        # MEMBERSHIP IS ASKED IN THE SHELL, WITH NO PIPE, and that is load-bearing.
        # This was `printf '%s\n' "${SOURCE_GATES[@]}" | grep -qx "$n"`, which is
        # wrong under this file's own `set -o pipefail`: `grep -q` exits the moment
        # it MATCHES, `printf` is then killed by SIGPIPE, and `pipefail` hands the
        # pipeline back 141 — so a match reads as a NON-match. Whether printf's
        # write completes before grep exits is a scheduling race, so the fault is
        # load-dependent, silent, and only ever fires in the one direction that
        # drops a real SOURCE_GATES member out of its bucket and into
        # `unclassified` (or `no-reference`) — which makes the whole nightly lane
        # refuse, and reds any landing run unlucky enough to be running it.
        #
        # Measured on this tree: 192 concurrent audits produced 8 wrong
        # classifications (6 `unclassified=1`, 2 `no-reference=7`), and 9,600
        # concurrent runs of the isolated pipeline disagreed 25 times, EVERY ONE
        # with status 141. The same isolated loop run serially disagreed 0/4,000
        # times, which is why this survived: it cannot be reproduced by running
        # the audit on its own, only by running it inside a busy suite. The
        # equivalent pure-bash loop disagreed 0/9,600.
        in_source_gates=0
        for gate in "${SOURCE_GATES[@]}"; do
            [[ $gate == "$n" ]] && { in_source_gates=1; break; }
        done
        if (( in_source_gates )); then
            CLS_SOURCE+=("$n")
        elif grep -qE 's4\.bin|s4\.debug\.bin|demo\.bin|demo\.debug\.bin|\.lst|golden' "$f"; then
            CLS_ARTIFACT+=("$n")
        elif ! grep -qE "$obtains" <<< "$decommented" \
             && ! grep -qF "env::var(\"$var\")" <<< "$decommented"; then
            CLS_NOREF+=("$n")
        else
            CLS_UNCLASSIFIED+=("$n")
        fi
    done < <(grep -rlE 'AEON_DIR|aeon_dir|reference_tree|--aeon' "$tree"/crates/*/tests/*.rs)
    (( scanned )) || {
        CLS_REFUSAL="the detector matched no test file under $tree/crates/*/tests, a \
classification over an empty population is not a classification"
        return 2
    }
    CLS_SCANNED=$scanned
    return 0
}

note() {
    echo "$(date -Is) $1" >> "$LOG"
    notify-send -u critical "sigil source gates" "$1" 2>/dev/null || true
}

if [[ ${1:-} == --selftest-fail ]]; then
    note "SELFTEST: the failure-notification path works"
    exit 1
fi

# --audit runs ONLY the classification, against the checkout this script lives in (or a
# tree named as $2), and is READ-ONLY: no worktree is created or moved, nothing is built,
# and `note` is not reached, so it sends no desktop notification. Exit 0 when every
# aeon-reading test file is classified, 2 when one is not or when the rule itself cannot
# be derived.
#
# It exists because the only other way to ask this question is to run the whole lane,
# which creates worktrees off master in two shared checkouts, builds, and notifies the
# owner with nothing on stdout to say it did — so the question went unasked, and the files
# that darkened this lane sat unclassified until a 05:17 popup reported it.
# `crates/sigil-harness/tests/source_gate_classification.rs` runs this flag on every
# `cargo test --workspace`, which is what puts the question in front of a landing run
# instead of in front of the owner's lock screen.
if [[ ${1:-} == --audit ]]; then
    AUDIT_TREE=${2:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}
    if ! classify "$AUDIT_TREE"; then
        echo "UNMEASURABLE: $CLS_REFUSAL"
        exit 2
    fi
    echo "tree=$AUDIT_TREE"
    echo "accessors: $CLS_ACCESSORS"
    echo "SOURCE_GATES=${#SOURCE_GATES[@]} scanned=$CLS_SCANNED source=${#CLS_SOURCE[@]} \
artifact=${#CLS_ARTIFACT[@]} no-reference=${#CLS_NOREF[@]} unclassified=${#CLS_UNCLASSIFIED[@]}"
    (( ${#CLS_NOREF[@]} )) && echo "no-reference: ${CLS_NOREF[*]}"
    if (( ${#CLS_UNCLASSIFIED[@]} )); then
        echo "unclassified: ${CLS_UNCLASSIFIED[*]}"
        exit 2
    fi
    exit 0
fi

for d in "$SIGIL_MAIN" "$AEON_MAIN"; do
    [[ -d "$d/.git" || -f "$d/.git" ]] \
        || { note "COULD NOT RUN: no repo at $d"; exit 2; }
done

# Detached checkouts at each repo's master tip.
if [[ ! -d "$SIGIL_GATES" ]]; then
    git -C "$SIGIL_MAIN" worktree add --detach "$SIGIL_GATES" master >> "$LOG" 2>&1 \
        || { note "COULD NOT RUN: sigil gates worktree creation failed"; exit 2; }
fi
if [[ ! -d "$AEON_GATES" ]]; then
    git -C "$AEON_MAIN" worktree add --detach "$AEON_GATES" master >> "$LOG" 2>&1 \
        || { note "COULD NOT RUN: aeon gates worktree creation failed"; exit 2; }
fi

# Both default to master and the timer never sets them. They exist so a branch can
# be put through the real lane before it lands, and so the lane's own red-first
# proof can point at a committed throwaway state.
SIGIL_REF=${SIGIL_SOURCE_GATES_REF:-master}
AEON_REF=${AEON_SOURCE_GATES_REF:-master}

SIGIL_SHA=$(git -C "$SIGIL_MAIN" rev-parse "$SIGIL_REF") \
    || { note "COULD NOT RUN: cannot resolve sigil $SIGIL_REF"; exit 2; }
AEON_SHA=$(git -C "$AEON_MAIN" rev-parse "$AEON_REF") \
    || { note "COULD NOT RUN: cannot resolve aeon $AEON_REF"; exit 2; }
AT="sigil ${SIGIL_SHA:0:8} / aeon ${AEON_SHA:0:8}"

git -C "$SIGIL_GATES" checkout --force --detach "$SIGIL_SHA" >> "$LOG" 2>&1 \
    || { note "COULD NOT RUN: sigil checkout of $SIGIL_SHA failed"; exit 2; }
git -C "$AEON_GATES" checkout --force --detach "$AEON_SHA" >> "$LOG" 2>&1 \
    || { note "COULD NOT RUN: aeon checkout of $AEON_SHA failed"; exit 2; }
# The seam-emitted sound blobs are regenerated by the harness itself; the
# compression self-test vectors are not, and the corpus embeds them, so a
# checkout that has never been built cannot lower the sonic4 debug shape without
# this. Both the packer and the generator are source-only — no ROM is built here.
#
# Deleted first, not merely regenerated. These directories are gitignored, so
# `checkout --force` leaves whatever a previous night left behind, and a stale
# generated file is byte-indistinguishable from a correct one.
rm -rf "$AEON_GATES/engine/sound/generated" "$AEON_GATES/engine/debug/generated"
# And no built ROM or listing survives in this checkout. Nothing the lane runs reads
# one, and keeping the tree provably source-only is what makes that claim checkable
# rather than asserted: a leftover s4.bin from a hand-run build is byte-identical in
# appearance to a current one, and several port tests treat its mere PRESENCE as
# "there is an aeon tree here".
#
# CONSEQUENCE, and it bites: $AEON_GATES is SOURCE-ONLY BY CONSTRUCTION and must never
# be pointed at by an artifact-dependent run. This scrub will delete that run's ROMs
# out from under it mid-suite, and the failures read as 127 golden/region mismatches
# rather than as a race. Build a separate checkout for artifact gates.
rm -f "$AEON_GATES"/*.bin "$AEON_GATES"/*.lst "$AEON_GATES"/*.p "$AEON_GATES"/*.h

if [[ ! -x "$AEON_GATES/tools/bin/salvador" ]]; then
    mkdir -p "$AEON_GATES/tools/bin"
    make -C "$AEON_GATES/tools/salvador" -s > "$STATE/prepare.log" 2>&1 \
        || { note "COULD NOT RUN: salvador build failed at $AT, see $STATE/prepare.log"; exit 2; }
    cp "$AEON_GATES/tools/salvador/salvador" "$AEON_GATES/tools/bin/salvador" \
        || { note "COULD NOT RUN: salvador install failed at $AT"; exit 2; }
fi
( cd "$AEON_GATES" && python3 tools/gen_compression_vectors.py ) >> "$STATE/prepare.log" 2>&1 \
    || { note "COULD NOT RUN: compression vectors at $AT, see $STATE/prepare.log"; exit 2; }
# NOT redundant with the `||` above: gen_compression_vectors.py prints `FAIL: …` and
# exits 0 when the packer is missing, so its exit code cannot be trusted to mean it
# wrote anything. The output is checked instead of the status.
[[ -f "$AEON_GATES/engine/debug/generated/compression_vectors.emp" ]] \
    || { note "COULD NOT RUN: gen_compression_vectors.py produced nothing at $AT"; exit 2; }

# THE LANE AUDITS ITS OWN LIST. SOURCE_GATES is hand-maintained, and a hand-maintained
# list of what to check is the same object as the baseline nobody re-read — a new
# source-only gate would simply never join the lane, silently. So every test file the
# detector matches must land in one of three buckets, each decided from the file's own
# content: IN SOURCE_GATES (this lane runs it), ARTIFACT-DEPENDENT (it names a built ROM,
# a listing or the goldens), or NO-REFERENCE (it obtains no tree at all, so this lane has
# nothing to run for it). Anything else is unclassified and the lane refuses to run
# rather than quietly under-covering. Zero unclassified today.
#
# The artifact-lane files are COUNTED, not just skipped, and the count is printed in the
# verdict line. "Skipped" and "green" are different words for a reason: an artifact gate
# this lane does not run can be red for two reasons that need two different readers —
# CRC DRIFT (bytes moved legitimately; the refreeze ritual owns it) or a BUILD BRICK (the
# compiler refuses the corpus; nobody's ritual clears it). The brick half is what
# `corpus_builds` measures here, so a verdict naming both numbers cannot be read as
# "the artifact gates passed". The no-reference files are counted for the same reason: a
# bucket whose size is never printed is a bucket nobody can notice growing.
if ! classify "$SIGIL_GATES"; then
    note "COULD NOT RUN: $CLS_REFUSAL at $AT"
    exit 2
fi
if (( ${#CLS_UNCLASSIFIED[@]} )); then
    note "COULD NOT RUN: ${#CLS_UNCLASSIFIED[@]} aeon-reading gate(s) are in none of the \
three buckets, each reads the reference tree, is not in SOURCE_GATES, and names no \
built artifact. Classify each at $AT: ${CLS_UNCLASSIFIED[*]}"
    exit 2
fi
artifact=("${CLS_ARTIFACT[@]}")

export AEON_DIR="$AEON_GATES"
# Strict: a reference path this lane cannot find HARD-FAILS instead of skipping
# green. A gate that skips reports nothing and reads as coverage.
export SIGIL_STRICT_GATE=1

ARGS=()
for g in "${SOURCE_GATES[@]}"; do ARGS+=(--test "$g"); done

OUT="$STATE/gates.log"
( cd "$SIGIL_GATES" && cargo test --release --workspace --no-fail-fast "${ARGS[@]}" \
    -- --nocapture ) > "$OUT" 2>&1
rc=$?

# Every named gate must have produced a result line. A cargo invocation that
# silently selected nothing exits 0, and that is indistinguishable from a green
# run by exit code alone.
binaries=$(grep -c '^test result:' "$OUT")
if (( binaries != ${#SOURCE_GATES[@]} )); then
    note "COULD NOT RUN: ${#SOURCE_GATES[@]} gates named but $binaries ran at $AT, see $OUT"
    exit 2
fi
passed=$(awk '/^test result:/ {p += $4} END {print p+0}' "$OUT")
failed=$(awk '/^test result:/ {f += $6} END {print f+0}' "$OUT")
if (( passed == 0 )); then
    note "COULD NOT RUN: no test executed at $AT, see $OUT"
    exit 2
fi
# A skip line is a reference gate that measured nothing while reporting green.
# SIGIL_STRICT_GATE should make these impossible; if one appears, the strict path
# has a hole and the run's green means less than it reads.
#
# THE MARKER IS DERIVED, NOT RETYPED. This grep is only as wide as the spelling
# it was written with, and a retyped copy drifts away from what the tests emit:
# 29 announced early returns said `skipping <gate> …`, which this line matched
# none of, so those gates could no-op and clear the bar. The one definition lives
# in test_support.rs as SKIP_MARKER; skip_marker_lint holds every announcement
# site in the test tree to it and refuses a script that goes back to a literal.
SUPPORT_RS=crates/sigil-harness/src/test_support.rs
SKIP_MARKER=$(sed -n 's/^pub const SKIP_MARKER: &str = "\(.*\)";$/\1/p' \
    "$SIGIL_GATES/$SUPPORT_RS")
# Loud on unmeasurable: an empty marker makes `grep -F` match every line and a
# wrong one makes it match none. Neither may be rendered as a green run.
if [[ -z "$SKIP_MARKER" ]]; then
    note "COULD NOT RUN: SKIP_MARKER not extractable from $SUPPORT_RS at $AT, the \
skip check has no pattern, so this run cannot say whether a gate skipped"
    exit 2
fi
if grep -qF "$SKIP_MARKER" "$OUT"; then
    note "COULD NOT RUN: $(grep -cF "$SKIP_MARKER" "$OUT") gate(s) SKIPPED under strict at $AT, \
see $OUT"
    exit 2
fi

# The open-findings register, named in ordinary output so its rows are read
# nightly rather than resting in a file nobody is obliged to open.
REGISTER=$(sed -n '/^open warn-tier findings:/,/^test /p' "$OUT" | grep -v '^test ' | head -20)

# The verdict names what was NOT measured alongside what was. `corpus_builds` is the
# brick witness and it ran in this very invocation (the gate-count check above holds
# that), so "skipped as artifact-lane" cannot be read as "those gates are green": the one
# failure class no refreeze clears was measured here; the rest is the refreeze ritual's.
SKIPPED="${#artifact[@]} aeon-reading gates skipped as artifact-lane (CRC/region oracles \
against committed artifacts, not measured here; build bricks witnessed by corpus_builds); \
${#CLS_NOREF[@]} no-reference (name the tree, obtain none, the workspace suite runs them)"

if (( rc == 0 && failed == 0 )); then
    {
        echo "$(date -Is) OK at $AT ($passed passed, $binaries gates; $SKIPPED)"
        [[ -n "$REGISTER" ]] && echo "$REGISTER" | sed 's/^/    /'
    } >> "$LOG"
    exit 0
fi

names=$(awk '/^failures:$/ {f=1; next} /^test result:/ {f=0} f && /^    [a-z]/ {print $1}' "$OUT" \
    | sort -u | tr '\n' ' ')
# A brick is named as a brick. The gate's own failure text carries the phrase, and a red
# that includes it means "the corpus does not build" — a different owner and a different
# clock from "a lint moved", so the verdict says which.
kind="SOURCE GATES FAILED"
grep -q 'shipped shapes do NOT build from aeon source' "$OUT" \
    && kind="BUILD BRICK (the corpus does not build from source); SOURCE GATES FAILED"
note "$kind at $AT, $failed failed / $passed passed: $names ($SKIPPED; see $OUT)"
exit 1
