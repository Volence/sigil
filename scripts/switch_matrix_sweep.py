#!/usr/bin/env python3
"""switch_matrix_sweep.py: build every arm of every build option a corpus declares,
with both toolchains, and compare the two images.

The census this replaces was a hand-typed list of eleven flips. A list cannot be
shown to cover what the corpus offers, so this program does not carry one: it
DERIVES the switch set from the corpus source, executes one leg per non-current
value of every switch it derived, and refuses to report anything unless the
derived set and the executed set reconcile exactly.

Derivation, in three mechanical steps, none of which reads a list of names:

  1. DOCUMENTED-DECLARATION SCAN, over every `.asm` file in the corpus: a line
     `Name = <rhs>` at column 0 whose NEXT line is a `;...|` prose comment. That
     shape is how both corpora document a build option, and in both it selects
     the whole ASSEMBLY OPTIONS block and nothing else.
  2. BLOCK CROSS-CHECK: the ASSEMBLY OPTIONS block is located independently (by
     its own header comment and the section rule that ends it) and the two sets
     must be equal. A documented option appearing outside the block, or a block
     entry losing its prose, is a loud failure, not a silent drop.
  3. DOMAIN FROM PROSE: the legal values of a switch are read out of the prose
     the corpus wrote next to it, `If <n>,` and `<n> = <text>` enumerations. A
     switch whose prose enumerates no values has an UNREADABLE domain and is a
     loud failure unless it is in ACK_UNREADABLE below, which is asserted equal
     to the unreadable set in BOTH directions so it cannot go stale.

A switch whose right-hand side mentions any identifier is DERIVED (`0|AllOptimizations`):
it is not independently settable and is never flipped. It is still counted, and
the switches it is derived FROM are swept, which is how it gets exercised.

The corpus's build script is a second source of settings, and the `.asm` scan
cannot see it by construction: `build.lua` chooses the compressor it hands to
p2bin (and to sigil) from a Lua local. Those are derived the same way, again
from no list of names:

  L1. SETTINGS BLOCK: `build.lua`'s own `-- Settings --` ... `-- End of
      settings --` block, located by those two comment lines. Every statement in
      it must be a column-0 `local <name> = <literal>`; anything else in the
      block is a loud failure rather than a setting silently skipped.
  L2. CROSS-CHECK: every column-0 `local <name> = true|false` in EVERY `.lua`
      file of the corpus must sit inside that block. A toggle declared anywhere
      else is a loud failure, not a setting this sweep never reaches.
  L3. DOMAIN FROM THE LITERAL: a boolean literal has the domain {false, true},
      swept as 0 and 1. Any other literal (a number, a string) states no set of
      legal values, so it is UNREADABLE and must be acknowledged in
      ACK_UNREADABLE under the key `build.lua:<name>`, exactly like an `.asm`
      switch whose prose enumerates nothing.

A build-script setting is flipped by rewriting its Lua literal, with the same
prove-it-applied gate, and every leg re-derives the p2bin arguments from ITS OWN
edited `build.lua`, so sigil is handed what the stock build was handed.

Every leg proves its own edit applied by printing the line before and after from
disk and failing if they are equal, every image compare carries a planted-byte
control, and the leg count launched is reconciled against the leg count reported.

Usage:
    python3 scripts/switch_matrix_sweep.py --sigil <path to sigil> [--scratch DIR]
                                           [--corpus NAME=PATH[@REV]]...
                                           [--only TAG]... [--cross] [--derive-only]
A corpus is always read out of a COMMIT (`REV`, default `HEAD`, resolved to a
full SHA and printed), extracted afresh by `git archive` on every run. Its
working tree is never read, so uncommitted edits there cannot colour a result.
`--derive-only` stops after the derivation and its acknowledgement
reconciliation and launches no leg; it is a probe, never a sweep result.
`--cross` additionally runs every corner of the switch space, which for these
two corpora is 288 and 96 corners and about a quarter of an hour, and tests
whether a refusal is caused by one setting and composes.

Exit status is 0 only if the reconciliations hold and every leg's outcome is
either agreement or an acknowledged disagreement.
"""

import argparse
import itertools
import os
import re
import shutil
import subprocess
import sys
import zlib

# ---------------------------------------------------------------------------
# Acknowledgements. Each is asserted equal to what the run finds, in BOTH
# directions: an entry the run does not produce is as loud as a finding the run
# produces that is not here. Neither table can be used to hide a result.
# ---------------------------------------------------------------------------

# (corpus, switch): why its legal values cannot be read off the corpus prose.
ACK_UNREADABLE = {
    ("s1disasm", "ZoneCount"):
        "prose enumerates no values; it reads 'Do not change, unless more zones "
        "get added' and then names the six zones, so the corpus states a count, "
        "not a domain. Sweeping it would be sweeping a recompile of the zone "
        "tables, not a build option.",
    ("s2disasm", "build.lua:music_buffer_address"):
        "a number literal, and build.lua says it 'Should always match zMusicData "
        "in s2.sounddriver.asm'. It describes where the sound driver already "
        "put its Saxman buffer, so the only legal value is the one the driver "
        "source fixes; it is a consistency constant, not a choice.",
    ("s2disasm", "build.lua:music_buffer_size"):
        "a number literal, and build.lua says it 'Should always be zStack minus "
        "0x40'. Like music_buffer_address it restates a fact of the driver "
        "source, so there is no second legal value to sweep.",
}

# (corpus, leg tag): an outcome other than byte agreement that is known,
# adjudicated and booked. The value must equal the class the run computes, so an
# acknowledgement cannot quietly cover a result that changed underneath it.
#
# The value is either the class alone or `(class, text)`. With `text`, the leg's
# own diagnostic output must also contain that text: `SIGIL-DECLINED` is one class
# for every reason sigil can refuse, and an acknowledgement of one refusal must not
# go on covering the leg once it refuses for a neighbouring reason.
ACK_DISAGREE = {
    # `("s1disasm", "s1disasm-FixBugs-1"): "SIGIL-DECLINED"` USED TO BE HERE and
    # is gone because the row closed, not because the leg stopped being run.
    # `_incObj/DebugMode.asm:245`, reachable only under FixBugs, reads
    # `move.w (v_limitright2),d0`: an absolute address operand with no width
    # suffix. sigil refused it; `AS-WIDTH-SUFFIX-BARE-EXPR` routes the
    # parenthesised absolute operand through the same width selection the bare
    # one already used, and the leg now AGREES at crc32 `888defef` / 551,288
    # bytes with no source edit. The entry has to go rather than be re-labelled:
    # this table is asserted equal to what the run finds in both directions, so
    # a stale entry is a loud failure and not a comment.

    ("s2disasm", "s2disasm-fixBugs-1"):
        ("SIGIL-DECLINED", "the size the source declares for it, is $F64"),
    # Under fixBugs the Saxman stream grows to $F88 while the source declares
    # `Size_of_Snd_driver_guess = $F64`, so every byte computed from that name
    # is short by $24. build.lua repairs the reference afterwards, reading the
    # real size out of asl's share file and patching the image in
    # `amend_sound_driver_size`; sigil writes no share file and so cannot, and
    # refuses rather than write a ROM whose decompressor is told the wrong
    # length. Chosen and argued in `SWITCH-SETTING-SILENT-ROMS`, fault 2.

    ("s2disasm", "s2disasm-build.lua:improved_sound_driver_compression-1"):
        ("DIFFER", "2 bytes in 2 runs at 0x18F 0xEC051"),
    # NOT the compressor. sigil's saxman-optimised stream is p2bin's own, byte
    # for byte (`sigil-clownlzss-sys`'s `p2bin_optimised_vectors.rs`), and the
    # whole Sonic 1 ROM AGREES on the sibling leg with kosinski-optimised. What
    # differs here is `amend_sound_driver_size`, the same fault as the fixBugs
    # row above and in the other direction.
    #
    # `Size_of_Snd_driver_guess = $F64` and the optimal parser stores $F4A, $1A
    # FEWER. Sonic 2 loads the constant into the `move.w` its Saxman
    # decompressor reads as a byte count, and build.lua patches that immediate
    # afterwards from the real size asl's share file reports. sigil writes no
    # share file, so the patch finds nothing and skips, and sigil's image keeps
    # the source's own $F64 where the reference holds $F4A: one byte at
    # 0xEC051, plus the header checksum at 0x18F that follows from it.
    #
    # sigil does not refuse this direction, and that is measured rather than
    # chosen: skdisasm SHIPS a stream smaller than its own
    # `Size_of_Snd_driver_guess`, so refusing it would fire on a corpus at its
    # shipped settings. See the long comment at the `declared_size` check in
    # `sigil-link/src/blob.rs`. Closing it needs sigil to give a build script
    # the real stored size, which is the share-file question and not this
    # parcel; booked in the campaign gap ledger under `SWEEP-NIGHTLY`.
    #
    # The text pins the difference's size, shape and place, so a difference that
    # grows, moves or spreads stops being covered (self-test C12b).

    # `("s1disasm", "s1disasm-build.lua:improved_dac_driver_compression-1")` and
    # `("s2disasm", "s2disasm-build.lua:improved_sound_driver_compression-1")`
    # USED TO BE HERE, both `SIGIL-DECLINED` with the text "is a p2bin format
    # sigil does not implement", and are gone because the rows closed, not
    # because the legs stopped being run. Each corpus's build.lua picks its
    # sound-driver compressor from its own `improved_*_compression` local, and at
    # `true` that is p2bin's `kosinski-optimised` / `saxman-optimised`, which
    # sigil refused by name. `sigil-link/src/blob.rs`'s `BlobFormat` now
    # implements both (and plain `saxman` with them), so these legs compare
    # bytes like any other. The entries have to go rather than be re-labelled:
    # this table is asserted equal to what the run finds in both directions, so
    # a stale entry is a loud failure and not a comment.
}

# (corpus, leg tag of a phase-1 arm): an arm no single companion switch could
# make visible in the stock image. An entry here is a declaration that the arm
# is UNMEASURED, never that it agreed.
ACK_UNMEASURABLE = {
}

# (corpus, key): a diagnostic one toolchain emits and the other does not, on a
# leg where both toolchains ran. Keys are `asl#<code>` for a coded asl warning,
# `asl-text:<text>` / `sigil-text:<text>` for a source `warning` directive only
# one of them fired, and `sigil-only:<head>` for a warning sigil raises about
# itself. The observed set and this set are asserted equal in both directions,
# so a diagnostic either toolchain starts or stops emitting at any corner is
# loud. This is a MEASUREMENT of parity, not a demand for it: sigil is a
# drop-in for asl plus p2bin, not a reimplementation of asl's lint set.
#
# A coded asl warning with a sigil counterpart (CODED_COUNTERPARTS) is parity
# only when both fire at the same SET of source locations on the leg; a location
# only one of them names is the `asl#<code>` or `sigil-only:<id>` key.
ACK_WARNING_GAP = {
    ("s2disasm", "sigil-only:`shared` is ignored"):
        "sigil writes no share file, so it says so where s2's source uses the "
        "`shared` directive. asl without `-c` says the same thing; this fires "
        "on every Sonic 2 leg and is the standing `-c` residual.",
}

# (corpus, "asl#<code> = <sigil id>"): a coded asl warning the run must observe
# sigil matching location for location on at least one leg. Asserted equal to
# what the run sees in both directions, so a pairing that stops being exercised
# (asl no longer raises the code anywhere) is as loud as a new one.
EXPECT_WARNING_PARITY = {
    ("s2disasm", "asl#180 = [as.odd-address]"):
        "asl warns `address is not properly aligned` on `move.w (1).w,d0` at "
        "s2.asm:30438, inside `if gameRevision=0`, a line the source annotates "
        "as deliberately crashing. It is the only asl warning code either "
        "corpus raises at any corner, and sigil's [as.odd-address] names the "
        "same line.",
}

# (corpus, partial assignment, why): a combination of the corpus's own
# documented build options that the CORPUS'S OWN toolchain cannot build. sigil
# has nothing to agree with at such a corner, and calling it a sigil result
# would be reading a corpus defect as ours. A rule states the partial
# assignment responsible; `--cross` asserts that the corners the stock
# toolchain declined and the corners some rule covers are the same set, and
# that no rule covers zero corners.
# (corpus, corner tag): a corner the STOCK toolchain could not build where
# sigil wrote an image anyway. There is no reference to compare such an image
# against, so it is a finding and never a pass, and the set is asserted equal
# to what the run observes in both directions. It is EMPTY, and that emptiness
# is a measurement: at Sonic 1's 24 `Revision=0 + FixBugs=1 +
# AllOptimizations=0` corners sigil refuses too, at the same source line asl
# names, with `(d16,PC)/bra.w displacement out of range (32790) in section
# sec752` against asl's `error #1370: jump distance too big`. That is the
# answer to the `BRA-W-RANGE-UNCHECKED` row, and it was unmeasurable until
# `AS-WIDTH-SUFFIX-BARE-EXPR` let sigil reach those corners at all.
ACK_STOCK_DECLINE_SIGIL_BUILT = set()

ACK_STOCK_DECLINE = [
    ("s1disasm", {"Revision": 0, "FixBugs": 1, "AllOptimizations": 0},
     "asl reports `87, 88, 89 Ending Sequence Sonic, Emeralds, Logo.asm(270): "
     "error #1370: jump distance too big`. Line 270 is `bra.w DisplaySprite` "
     "inside an `if Revision=0` arm, and a `bra.w` reaches +/-32 KB. FixBugs "
     "adds enough code between the branch and `DisplaySprite` to put it out of "
     "range, and AllOptimizations brings it back only because "
     "PaddingOptimization removes 3,314 bytes. So two of Sonic 1's documented "
     "options cannot be combined unless a third is also set, and this is a "
     "Sonic 1 defect, not a sigil one: the stock toolchain is the thing that "
     "fails. Free across the other four switches, so it covers 2*2*2*3 = 24 "
     "of the 288 corners."),
]


# (corpus, partial assignment, pinned difference, why): a family of CROSS corners
# where both toolchains build and the images differ for one known, adjudicated,
# booked reason.
#
# `ACK_DISAGREE` above cannot reach a cross corner: it is keyed by a leg tag and
# a cross corner's tag names a point in the switch space, not a switch. The shape
# here is `ACK_STOCK_DECLINE`'s instead, because the thing being stated is the
# same kind of thing: a partial assignment is what CAUSES the family, and the
# whole family shares one cause. The partial assignment must be the smallest one
# that names the cause, so that a corner differing for a second reason falls
# outside it.
#
# `pinned` is the leg's own report (see `leg_report`), asserted to be IDENTICAL
# at every covered corner, not merely contained: a family is only a family if its
# members really do differ in the same way, and a rule that swallowed two sizes
# of difference would be hiding the second. As with the other tables, the covered
# set and the observed set are asserted equal in BOTH directions, so a rule that
# covers no disagreeing corner is as loud as a corner no rule covers.
ACK_CROSS_DISAGREE = [
    ("s2disasm", {"build.lua:improved_sound_driver_compression": 1},
     "2 bytes in 2 runs at 0x18F 0xEC051",
     "`amend_sound_driver_size`, and not the compressor. The optimal Saxman "
     "parser stores $F4A where `Size_of_Snd_driver_guess` declares $F64; Sonic "
     "2 loads that constant into the `move.w` its decompressor reads as a byte "
     "count, and build.lua patches the immediate afterwards from the real size "
     "asl's share file reports. sigil writes no share file, so the patch finds "
     "nothing and skips, and sigil's image keeps the source's own $F64: one "
     "byte at 0xEC051, plus the header checksum at 0x18F that follows from it. "
     "Identical at every corner it covers, because no other switch changes the "
     "driver's own source. The same fault as the `fixBugs` entry in "
     "ACK_DISAGREE, in the direction sigil deliberately does not refuse: a "
     "stream SMALLER than its constant is what skdisasm ships, so refusing it "
     "would fire on a corpus at its shipped settings (`sigil-link/src/blob.rs`, "
     "at the `declared_size` check). Booked in the campaign gap ledger under "
     "`SWEEP-NIGHTLY` Open 5. Free across the other five switches except "
     "`fixBugs`, whose arm sigil refuses before any image exists, so it covers "
     "3*2*1*2*2*2 = 48 of the 192 corners."),
]


class Fail(Exception):
    pass


# ---------------------------------------------------------------------------
# Step 1 and 2: derive the switch set
# ---------------------------------------------------------------------------

DECL = re.compile(r"^([A-Za-z_]\w*)[ \t]*=[ \t]*(\S.*?)[ \t]*$")
PROSE = re.compile(r"^;.*\|")
COMMENT = re.compile(r"^;")
# a section rule: a comment made of one punctuation character repeated
RULE = re.compile(r"^;[ \t]*([=>*#-])\1{9,}")
IDENT = re.compile(r"[A-Za-z_]\w*")


def asm_files(tree):
    out = []
    for root, dirs, files in os.walk(tree):
        dirs[:] = [d for d in dirs if d != ".git"]
        for f in files:
            if f.endswith(".asm"):
                out.append(os.path.join(root, f))
    return sorted(out)


def read_lines(path):
    return open(path, encoding="latin-1").read().split("\n")


def documented_declarations(tree):
    """Step 1. Every `Name = rhs` at column 0 whose next line is `;...|` prose."""
    found = []
    for path in asm_files(tree):
        L = read_lines(path)
        for i, line in enumerate(L):
            m = DECL.match(line)
            if not m or i + 1 >= len(L) or not PROSE.match(L[i + 1]):
                continue
            prose = []
            j = i + 1
            while j < len(L) and COMMENT.match(L[j]):
                if "|" in L[j]:
                    prose.append(L[j])
                j += 1
            found.append({
                "file": os.path.relpath(path, tree),
                "line": i + 1,
                "name": m.group(1),
                "rhs": m.group(2),
                "text": line,
                "prose": "\n".join(prose),
            })
    return found


def options_block(tree, root_asm):
    """Step 2. The ASSEMBLY OPTIONS block located on its own terms."""
    path = os.path.join(tree, root_asm)
    L = read_lines(path)
    start = None
    for i, line in enumerate(L):
        if COMMENT.match(line) and "ASSEMBLY OPTIONS" in line.upper():
            start = i + 1
            break
    if start is None:
        raise Fail("no `ASSEMBLY OPTIONS` header comment in %s" % root_asm)
    names = []
    for i in range(start, len(L)):
        if RULE.match(L[i]):
            break
        m = DECL.match(L[i])
        if m:
            names.append(m.group(1))
    else:
        raise Fail("the ASSEMBLY OPTIONS block in %s is never closed by a "
                   "section rule; the scan would run to end of file" % root_asm)
    return names


def domain_from_prose(prose, current):
    """Step 3. The legal values the corpus wrote next to the switch.

    Both corpora document an option in one of two prose forms: a run of
    `If <n>, <what that value does>` clauses, or an `<n> = <what it means>`
    enumeration. Neither form is obliged to mention the value the switch is
    currently set to, so the current value always joins the set.

    A form that mentions only values in {0,1} is a boolean flag documented by
    its active arm alone: `padToPowerOfTwo = 1` is followed by `If 1, pads the
    end of the ROM`, and the unwritten arm is 0. Completing it is what stops a
    real switch from deriving a one-element domain, i.e. from being counted as
    swept while producing no leg at all. The complement is added ONLY when
    every value the prose named is already in {0,1}, so a switch documented
    with a value of 2 or 3 never has a value invented for it.
    """
    vals = set()
    for m in re.finditer(r"\bIf\s+(\d+)\s*,", prose):
        vals.add(int(m.group(1)))
    for m in re.finditer(r"(?<![\w$])(\d+)\s*=\s*[A-Za-z(]", prose):
        vals.add(int(m.group(1)))
    if not vals:
        return None
    vals.add(current)
    if vals <= {0, 1}:
        vals = {0, 1}
    return sorted(vals)


# ---------------------------------------------------------------------------
# Steps L1 to L3: derive the build script's own settings
# ---------------------------------------------------------------------------

LUA_BLOCK_OPEN = re.compile(r"^--[ \t]*settings[ \t]*--[ \t]*$", re.I)
LUA_BLOCK_CLOSE = re.compile(r"^--[ \t]*end of settings[ \t]*--[ \t]*$", re.I)
LUA_LOCAL = re.compile(r"^local[ \t]+([A-Za-z_]\w*)[ \t]*=[ \t]*(.*)$")
LUA_BOOL_LOCAL = re.compile(r"^local[ \t]+([A-Za-z_]\w*)[ \t]*=[ \t]*(true|false)"
                            r"[ \t]*(--.*)?$")


def lua_literal(rest):
    """Split the text after `local <name> =` into (kind, literal, trailing
    comment). kind is `boolean`, `number` or `string`; None if the text is not
    exactly one literal, optionally followed by a `--` comment."""
    m = re.match(r"(true|false|0[xX][0-9A-Fa-f]+|\d+(?:\.\d+)?|\"[^\"\\\n]*\"|"
                 r"'[^'\\\n]*')[ \t]*(--.*)?$", rest)
    if not m:
        return None
    lit = m.group(1)
    kind = ("boolean" if lit in ("true", "false")
            else "string" if lit[0] in "\"'" else "number")
    return kind, lit, m.group(2) or ""


def lua_files(tree):
    out = []
    for root, dirs, files in os.walk(tree):
        dirs[:] = [d for d in dirs if d != ".git"]
        for f in files:
            if f.endswith(".lua"):
                out.append(os.path.join(root, f))
    return sorted(out)


def build_script_settings(tree, corpus):
    """Steps L1 to L3 over the corpus's build.lua. Returns switch rows shaped
    like the `.asm` ones, named `build.lua:<local>`, and the number of `.lua`
    files the L2 cross-check read."""
    path = os.path.join(tree, "build.lua")
    L = read_lines(path)
    opens = [i for i, l in enumerate(L) if LUA_BLOCK_OPEN.match(l)]
    closes = [i for i, l in enumerate(L) if LUA_BLOCK_CLOSE.match(l)]
    if len(opens) != 1 or len(closes) != 1 or closes[0] < opens[0]:
        raise Fail("%s: build.lua must have exactly one `-- Settings --` line "
                   "followed by one `-- End of settings --` line; found openers "
                   "on lines %s and closers on lines %s, so its settings cannot "
                   "be located" % (corpus, [i + 1 for i in opens],
                                   [i + 1 for i in closes]))
    rows = []
    for i in range(opens[0] + 1, closes[0]):
        line = L[i]
        if not line.strip() or line.lstrip().startswith("--"):
            continue
        m = LUA_LOCAL.match(line)
        lit = lua_literal(m.group(2)) if m else None
        if not lit:
            raise Fail("%s: build.lua:%d is inside the Settings block but is not a "
                       "column-0 `local <name> = <literal>`, so its domain cannot be "
                       "derived and skipping it would drop a setting silently: %r"
                       % (corpus, i + 1, line))
        kind, text, _ = lit
        rows.append({"file": "build.lua", "line": i + 1, "lang": "lua",
                     "local": m.group(1), "name": "build.lua:" + m.group(1),
                     "rhs": text, "text": line, "lit_kind": kind})

    # L2: a toggle anywhere else in the corpus's Lua is a setting this sweep
    # would never reach.
    scanned = lua_files(tree)
    stray = []
    for p in scanned:
        for j, line in enumerate(read_lines(p)):
            m = LUA_BOOL_LOCAL.match(line)
            if not m:
                continue
            here = os.path.relpath(p, tree) == "build.lua" \
                and opens[0] < j < closes[0]
            if not here:
                stray.append("%s:%d %s" % (os.path.relpath(p, tree), j + 1,
                                          line.strip()))
    if stray:
        raise Fail("%s: a column-0 boolean local outside build.lua's Settings "
                   "block is a toggle this sweep cannot reach: %s"
                   % (corpus, stray))
    names = [r["local"] for r in rows]
    if len(names) != len(set(names)):
        raise Fail("%s: a build.lua setting is declared twice: %s" % (corpus, names))

    # L3: the domain, read off the literal.
    for r in rows:
        if r["lit_kind"] == "boolean":
            r["kind"] = "swept"
            r["current"] = 1 if r["rhs"] == "true" else 0
            r["domain"] = [0, 1]
            r["arms"] = [1 - r["current"]]
        else:
            r["kind"] = "unreadable"
            r["why"] = ("`%s` is a %s literal; build.lua states no set of legal "
                        "values for it" % (r["rhs"], r["lit_kind"]))
    return rows, len(scanned)


def classify(tree, corpus, root_asm):
    docs = documented_declarations(tree)
    block = options_block(tree, root_asm)

    doc_names = [d["name"] for d in docs]
    if sorted(doc_names) != sorted(block):
        raise Fail(
            "%s: the documented-declaration scan and the ASSEMBLY OPTIONS block "
            "disagree. scan-only=%s block-only=%s"
            % (corpus, sorted(set(doc_names) - set(block)),
               sorted(set(block) - set(doc_names))))
    if len(doc_names) != len(set(doc_names)):
        raise Fail("%s: a switch is declared twice: %s" % (corpus, doc_names))

    for d in docs:
        if IDENT.search(d["rhs"]):
            d["kind"] = "derived"
            d["why"] = ("right-hand side `%s` names another switch, so this is "
                        "not independently settable" % d["rhs"])
            continue
        if not re.fullmatch(r"\d+", d["rhs"]):
            d["kind"] = "unreadable"
            d["why"] = "right-hand side `%s` is not a decimal literal" % d["rhs"]
            continue
        d["current"] = int(d["rhs"])
        dom = domain_from_prose(d["prose"], d["current"])
        if dom is None:
            d["kind"] = "unreadable"
            d["why"] = "prose next to the switch enumerates no values"
            continue
        arms = [v for v in dom if v != d["current"]]
        if not arms:
            # A switch that derives a one-element domain would be reported as
            # swept and would execute nothing. That is the silent drop this
            # program exists to make impossible, so it is a failure of the
            # derivation, never a switch with no work to do.
            raise Fail(
                "%s: `%s` derived the one-element domain %s from its prose, so "
                "it would be counted as swept and execute no leg. Its prose "
                "reads:\n%s" % (corpus, d["name"], dom, d["prose"]))
        d["kind"] = "swept"
        d["domain"] = dom
        d["arms"] = arms
    return docs


# ---------------------------------------------------------------------------
# The corpus's own toolchain arguments, read out of its build.lua
# ---------------------------------------------------------------------------

def derive_tool_args(tree):
    """Read the root stem, output stem and p2bin argument string that the
    corpus's own build.lua passes, so sigil is invoked with the corpus's
    arguments rather than with arguments copied into this file."""
    path = os.path.join(tree, "build.lua")
    src = open(path, encoding="latin-1").read()
    m = re.search(r"build_rom_and_handle_failure\((.*?)\)\n", src, re.S)
    if not m:
        raise Fail("%s: no build_rom_and_handle_failure call found" % path)
    args = split_lua_args(m.group(1))
    if len(args) < 4:
        raise Fail("%s: build_rom_and_handle_failure has %d arguments, expected "
                   "at least 4" % (path, len(args)))
    root_stem = lua_value(args[0], src, path)
    out_stem = lua_value(args[1], src, path)
    as_args = lua_value(args[2], src, path)
    p2bin_args = lua_value(args[3], src, path)
    if as_args.strip():
        raise Fail("%s: build.lua passes assembler arguments %r, which this "
                   "runner does not model" % (path, as_args))
    return root_stem + ".asm", out_stem + ".bin", p2bin_args.split()


def split_lua_args(text):
    out, depth, cur, q = [], 0, "", None
    for ch in text:
        if q:
            cur += ch
            if ch == q:
                q = None
            continue
        if ch in "\"'":
            q = ch
            cur += ch
        elif ch in "([{":
            depth += 1
            cur += ch
        elif ch in ")]}":
            depth -= 1
            cur += ch
        elif ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def lua_value(expr, src, path):
    """Evaluate the small expression language build.lua uses for these four
    arguments: string literals, `..` concatenation, local string variables, and
    `<local boolean> and "a" or "b"`. Anything else is a loud failure."""
    parts = [p.strip() for p in expr.split("..")]
    out = ""
    for p in parts:
        if len(p) >= 2 and p[0] == p[-1] and p[0] in "\"'":
            out += p[1:-1]
            continue
        if re.fullmatch(r"[A-Za-z_]\w*", p):
            m = re.search(r"^local\s+%s\s*=\s*(.+)$" % re.escape(p), src, re.M)
            if not m:
                raise Fail("%s: cannot resolve `%s`" % (path, p))
            out += lua_value(m.group(1).strip(), src, path)
            continue
        m = re.fullmatch(r"([A-Za-z_]\w*)\s+and\s+(\S.*?)\s+or\s+(\S.*)", p)
        if m:
            cond = re.search(r"^local\s+%s\s*=\s*(true|false)\b"
                             % re.escape(m.group(1)), src, re.M)
            if not cond:
                raise Fail("%s: `%s` is not a local boolean" % (path, m.group(1)))
            out += lua_value(m.group(2) if cond.group(1) == "true"
                             else m.group(3), src, path)
            continue
        raise Fail("%s: cannot evaluate build.lua expression %r" % (path, p))
    return out


# ---------------------------------------------------------------------------
# The edit, which must prove it applied
# ---------------------------------------------------------------------------

def apply_edit(path, lineno, name, want, log):
    """Rewrite one declaration. Returns nothing; raises on anything that leaves
    the tree not carrying the mutation, including a rewrite that changes no text."""
    L = read_lines(path)
    if lineno > len(L):
        raise Fail("%s has %d lines, cannot edit line %d" % (path, len(L), lineno))
    before = L[lineno - 1]
    m = DECL.match(before)
    if not m or m.group(1) != name:
        raise Fail("%s:%d does not declare `%s`, it reads %r"
                   % (path, lineno, name, before))
    sites = [i + 1 for i, l in enumerate(L)
             if DECL.match(l) and DECL.match(l).group(1) == name]
    if sites != [lineno]:
        raise Fail("`%s` is declared at column 0 on lines %s, not only %d; a "
                   "second declaration would shadow the edit"
                   % (name, sites, lineno))
    after = "%s = %d" % (name, want)
    L[lineno - 1] = after
    open(path, "w", encoding="latin-1").write("\n".join(L))

    # Prove it applied, from disk, not from the variable just assigned.
    back = read_lines(path)[lineno - 1]
    log("   edit %s:%d" % (os.path.basename(path), lineno))
    log("     before: %r" % before)
    log("     after:  %r" % back)
    if back == before:
        raise Fail("the edit changed no text at %s:%d; an unapplied mutation "
                   "and a real pass are the same artifact" % (path, lineno))
    if back != after:
        raise Fail("%s:%d reads %r after the write, expected %r"
                   % (path, lineno, back, after))
    m2 = DECL.match(back)
    if m2.group(1) != name or m2.group(2) != str(want):
        raise Fail("%s:%d does not parse as `%s = %d`" % (path, lineno, name, want))


def apply_lua_edit(path, lineno, local, want, log):
    """Rewrite one build.lua boolean setting to `want` (0 false, 1 true), keeping
    any trailing comment. The same refusals as `apply_edit`: a wrong line, a
    second assignment that would override the edit, and a rewrite that changes
    no text."""
    L = read_lines(path)
    if lineno > len(L):
        raise Fail("%s has %d lines, cannot edit line %d" % (path, len(L), lineno))
    before = L[lineno - 1]
    m = LUA_BOOL_LOCAL.match(before)
    if not m or m.group(1) != local:
        raise Fail("%s:%d does not declare the boolean setting `%s`, it reads %r"
                   % (path, lineno, local, before))
    # Any statement that assigns the name, at any depth. A line ending in `,` is a
    # table field or an argument (`{ name = name, }`), which reads the setting
    # and cannot assign it, because no Lua statement ends in a comma.
    assign = re.compile(r"^[ \t]*(local[ \t]+)?%s[ \t]*=(?!=)" % re.escape(local))
    sites = [i + 1 for i, l in enumerate(L)
             if assign.match(l) and not re.sub(r"--.*$", "", l).rstrip().endswith(",")]
    if sites != [lineno]:
        raise Fail("`%s` is assigned on lines %s, not only %d; a second assignment "
                   "would override the edit" % (local, sites, lineno))
    word = "true" if want else "false"
    after = "local %s = %s%s" % (local, word,
                                 (" " + m.group(3)) if m.group(3) else "")
    L[lineno - 1] = after
    open(path, "w", encoding="latin-1").write("\n".join(L))

    back = read_lines(path)[lineno - 1]
    log("   edit %s:%d" % (os.path.basename(path), lineno))
    log("     before: %r" % before)
    log("     after:  %r" % back)
    if back == before:
        raise Fail("the edit changed no text at %s:%d; an unapplied mutation "
                   "and a real pass are the same artifact" % (path, lineno))
    m2 = LUA_BOOL_LOCAL.match(back)
    if back != after or not m2 or m2.group(1) != local or m2.group(2) != word:
        raise Fail("%s:%d reads %r after the write, expected %r"
                   % (path, lineno, back, after))


def apply_switch_edit(tree, sw, want, log):
    if sw.get("lang") == "lua":
        apply_lua_edit(os.path.join(tree, sw["file"]), sw["line"], sw["local"],
                       want, log)
    else:
        apply_edit(os.path.join(tree, sw["file"]), sw["line"], sw["name"], want, log)


# ---------------------------------------------------------------------------
# The compare, which carries its own positive control
# ---------------------------------------------------------------------------

# asl's warning code -> the lint id sigil prints for the same finding. Measured
# rule and probe table: docs/superpowers/notes/2026-09-17-asl-warn-180/.
CODED_COUNTERPARTS = {"180": "[as.odd-address]"}

# The location a warning line names, in the spelling both toolchains share:
# `file(line)` plus any macro or `rept` trail (`s2.asm(12) name(3)`), without
# asl's `> > > ` lead or sigil's `:col`. Lazy up to the FIRST `(digits)`, so a
# file name with commas or spaces (Sonic 1's ending-sequence file) survives.
WARN_LOCATION = re.compile(r"^(?:> > > )?(.+?\(\d+\)(?: [^:\n]*?)?)(?::\d+)?: warning")

# The location recorded for a counterpart-coded warning line WARN_LOCATION could
# not read. sigil never names it, so an unreadable asl line is a parity gap
# rather than a location silently dropped from the set.
UNREADABLE_LOCATION = "<unreadable location>"


def asl_warnings(text):
    """asl's coded warnings, the text of `warning` directives the SOURCE wrote,
    and, for each code with a sigil counterpart, the set of locations it fired
    at. Coded warnings and source directives are keyed apart rather than
    compared as one bag of strings."""
    coded = set(re.findall(r"warning #(\d+)", text))
    texts = {t.strip() for t in re.findall(r"warning: (?!#)([^\n]+)", text)}
    locs = {}
    for line in text.split("\n"):
        m = re.search(r"warning #(\d+)", line)
        if not m or m.group(1) not in CODED_COUNTERPARTS:
            continue
        where = WARN_LOCATION.match(line)
        locs.setdefault(m.group(1), set()).add(
            where.group(1) if where else UNREADABLE_LOCATION)
    return coded, texts, locs


def sigil_warnings(text):
    """sigil's rendering of a source `warning` directive, which it prefixes
    `[as.warning]`; the locations of each warning that is the counterpart of an
    asl code, keyed by that code; and every other warning sigil raises about
    itself."""
    texts, own, locs = set(), set(), {}
    counterpart = {v: k for k, v in CODED_COUNTERPARTS.items()}
    for line in text.split("\n"):
        m = re.search(r"warning: ([^\n]+)", line)
        if not m:
            continue
        t = m.group(1).strip()
        cid = next((i for i in counterpart if t.startswith(i + " ")), None)
        if t.startswith("[as.warning] "):
            texts.add(t[len("[as.warning] "):].strip())
        elif cid is not None:
            where = WARN_LOCATION.match(line)
            locs.setdefault(counterpart[cid], set()).add(
                where.group(1) if where else UNREADABLE_LOCATION)
        else:
            own.add(t.split(":")[0].strip())
    return texts, own, locs


def diagnostic_parity(rows):
    """Diagnostic parity over every leg where BOTH toolchains ran on the SAME
    source. Returns (legs compared, {(corpus, key): legs}, {(corpus, pairing):
    [legs, locations]}).

    A leg one toolchain refused proves nothing about what the other would have
    said, so it is excluded rather than counted as silence. The end-to-end
    control leg (`control`) is excluded too: it edits the source AFTER the
    reference build on purpose, so asl and sigil were handed different
    programs and a diagnostic only one of them raises there is the control
    working, not a parity gap. Measured: on the first cross run with the
    alignment warning, that leg flipped `gameRevision` 1 to 0 after asl built,
    and sigil correctly warned at s2.asm(30438) on the revision-0 source asl
    never saw."""
    gaps, pairs, both = {}, {}, 0
    for r in rows:
        if not (r.get("lua_wrote") and r.get("sigil_wrote")) or r.get("control"):
            continue
        both += 1
        c = r["corpus"]
        for key in warning_parity_keys(r):
            gaps[(c, key)] = gaps.get((c, key), 0) + 1
        for code, locs in sorted(r.get("asl_coded_locs", {}).items()):
            if locs == r.get("sig_counterpart_locs", {}).get(code, set()):
                p = pairs.setdefault((c, "asl#%s = %s" % (
                    code, CODED_COUNTERPARTS[code])), [0, set()])
                p[0] += 1
                p[1] |= locs
    return both, gaps, pairs


def warning_parity_keys(r):
    """The warning-parity keys one leg where both toolchains ran contributes.

    `asl#<code>`: asl raised the code at a location sigil's counterpart does not
    name, or the code has no counterpart at all. `sigil-only:<id>`: sigil's
    counterpart names a location asl did not warn at. `asl-text:` /
    `sigil-text:`: a source `warning` directive only one of them printed.
    `sigil-only:<head>`: any other warning sigil raises about itself."""
    keys = set()
    alocs = r.get("asl_coded_locs", {})
    slocs = r.get("sig_counterpart_locs", {})
    for code in r.get("asl_coded", ()):
        if code in CODED_COUNTERPARTS and alocs.get(code, set()) <= slocs.get(code, set()):
            continue
        keys.add("asl#" + code)
    for code, locs in slocs.items():
        if not locs <= alocs.get(code, set()):
            keys.add("sigil-only:" + CODED_COUNTERPARTS[code])
    for t in r.get("asl_text", set()) - r.get("sig_text", set()):
        keys.add("asl-text:" + t[:60])
    for t in r.get("sig_text", set()) - r.get("asl_text", set()):
        keys.add("sigil-text:" + t[:60])
    for t in r.get("sig_own", set()):
        keys.add("sigil-only:" + t)
    return keys


def ident(path):
    d = open(path, "rb").read()
    return d, "%08x" % (zlib.crc32(d) & 0xffffffff), len(d)


def bytediff(a, b):
    n = min(len(a), len(b))
    runs, i, total = [], 0, 0
    while i < n:
        if a[i] != b[i]:
            j = i
            while j < n and a[j] != b[j]:
                j += 1
            runs.append((i, j))
            total += j - i
            i = j
        else:
            i += 1
    total += abs(len(a) - len(b))
    return total, runs


def compare_with_control(ref, cand, log):
    """Whole-image compare, no window. The control plants three bytes in a copy
    of the candidate and requires the comparer to report exactly those three on
    top of the real difference. A comparer that cannot see a planted byte
    reports agreement, so no figure is used unless the control passes."""
    a, ca, la = ident(ref)
    b, cb, lb = ident(cand)
    total, runs = bytediff(a, b)
    log("   reference crc32=%s size=%d" % (ca, la))
    log("   candidate crc32=%s size=%d" % (cb, lb))
    log("   DIFF %d bytes in %d runs%s"
        % (total, len(runs),
           "" if la == lb else "  SIZE DIFFERS by %d" % (lb - la)))
    for (i, j) in runs[:8]:
        log("     [0x%06X,0x%06X) ref %s cand %s"
            % (i, j, a[i:j][:8].hex(" "), b[i:j][:8].hex(" ")))
    planted = sorted({0x200, lb // 2, lb - 2})
    m = bytearray(b)
    for o in planted:
        m[o] ^= 0xFF
    t2, r2 = bytediff(a, bytes(m))
    got = {i for (i, j) in r2}
    ok = all(o in got or any(i <= o < j for (i, j) in runs) for o in planted) \
        and t2 >= total
    log("   CONTROL planted at %s -> %d bytes in %d runs: %s"
        % ([hex(o) for o in planted], t2, len(r2), "PASSED" if ok else "FAILED"))
    if not ok:
        raise Fail("the planted-byte control failed; this compare cannot report "
                   "a difference and its figure is void")
    return total, runs, ca, la, cb, lb


# ---------------------------------------------------------------------------
# One leg
# ---------------------------------------------------------------------------

def run_leg(cfg, tag, edits, log, vacuity_ref=None, post_lua_edits=()):
    """Copy the pristine tree, apply the edits, build with the corpus's own
    build.lua and with sigil, and compare. `edits` is a list of (switch, value);
    an empty list is the shipped-settings baseline.

    `vacuity_ref` is the stock image this leg's own stock image must differ
    from for the leg to have exercised anything. For a one-switch leg that is
    the shipped build; for a two-switch rescue leg it is the stock build of the
    companion switch alone, so that what is being tested is still the one
    switch under study."""
    tree = os.path.join(cfg["scratch"], "trees", "leg-" + tag)
    if os.path.isdir(tree):
        shutil.rmtree(tree)
    shutil.copytree(cfg["pristine"], tree, symlinks=True)

    log("   LEG_START %s" % tag)
    for sw, val in edits:
        apply_switch_edit(tree, sw, val, log)

    r = {"tag": tag, "corpus": cfg["corpus"],
         "edits": [(s["name"], v) for s, v in edits]}

    lua = subprocess.run(["lua", "build.lua"], cwd=tree,
                         capture_output=True, text=True, timeout=1800)
    r["lua_exit"] = lua.returncode
    refbin = os.path.join(tree, cfg["out_bin"])
    r["lua_wrote"] = os.path.isfile(refbin)
    lualog = os.path.join(cfg["scratch"], "logs", tag + ".lua.log")
    open(lualog, "w").write(lua.stdout + lua.stderr)
    # asl distinguishes a coded warning of its own from a `warning` directive
    # the source wrote. The two are keyed apart rather than compared as one bag
    # of strings, and a coded warning with a sigil counterpart keeps its
    # locations so parity can be judged per site.
    r["asl_coded"], r["asl_text"], r["asl_coded_locs"] = asl_warnings(
        lua.stdout + lua.stderr)
    log("   BUILD_LUA exit=%d wrote=%s  (%s)"
        % (lua.returncode, r["lua_wrote"], os.path.basename(lualog)))
    if not r["lua_wrote"]:
        for line in (lua.stdout + lua.stderr).split("\n"):
            if re.search(r"error|Error|ERROR|> >", line):
                log("     lua: " + line.strip())

    # Keep the reference out of the way and remove the stock toolchain's
    # intermediates, so sigil neither reads nor overwrites them. The generated
    # inputs build.lua just produced stay, because they are what the reference
    # consumed and sigil must consume the same ones.
    keep = os.path.join(cfg["scratch"], "images", tag + ".ref.bin")
    if r["lua_wrote"]:
        shutil.move(refbin, keep)
        r["ref"] = keep
        r["ref_crc"] = ident(keep)[1]
    for f in os.listdir(tree):
        if f.endswith((".p", ".h", ".lst")):
            os.remove(os.path.join(tree, f))

    # An end-to-end control edits the source AFTER the reference was built, so
    # the two toolchains are handed different source and the run must report a
    # difference. It is the proof that the reference build, the candidate build
    # and the compare are three independent things rather than one file read
    # twice.
    for sw, val in post_lua_edits:
        apply_switch_edit(tree, sw, val, log)

    # The p2bin arguments are re-derived from THIS leg's build.lua, after every
    # edit, because a build-script setting changes them. Reusing the shipped
    # tree's arguments would hand sigil a compressor the stock build did not use.
    root_asm, out_bin, p2bin_args = derive_tool_args(tree)
    if (root_asm, out_bin) != (cfg["root_asm"], cfg["out_bin"]):
        raise Fail("%s: build.lua names root %s / output %s on this leg, the "
                   "shipped tree named %s / %s" % (tag, root_asm, out_bin,
                                                   cfg["root_asm"], cfg["out_bin"]))
    r["p2bin_args"] = p2bin_args
    if p2bin_args != cfg["p2bin_args"]:
        log("   p2bin/sigil args for this leg: %s  (shipped: %s)"
            % (" ".join(p2bin_args), " ".join(cfg["p2bin_args"])))

    out = os.path.join(tree, "sigil.bin")
    sg = subprocess.run([cfg["sigil"], root_asm, "-o", "sigil.bin"]
                        + p2bin_args, cwd=tree,
                        capture_output=True, text=True, timeout=1800)
    r["sigil_exit"] = sg.returncode
    r["sigil_wrote"] = os.path.isfile(out)
    r["sigil_stderr"] = [l for l in sg.stderr.split("\n")
                         if l.strip() and "`shared` is ignored" not in l]
    siglog = os.path.join(cfg["scratch"], "logs", tag + ".sigil.err")
    open(siglog, "w").write(sg.stderr)
    r["sig_text"], r["sig_own"], r["sig_counterpart_locs"] = sigil_warnings(sg.stderr)
    log("   SIGIL exit=%d wrote=%s  stderr lines=%d (%s)"
        % (sg.returncode, r["sigil_wrote"], len(r["sigil_stderr"]),
           os.path.basename(siglog)))
    for line in r["sigil_stderr"][:4]:
        log("     sigil: " + line.strip())

    if not r["lua_wrote"] and not r["sigil_wrote"]:
        r["klass"] = "BOTH-DECLINED"
    elif not r["lua_wrote"]:
        r["klass"] = "STOCK-DECLINED-SIGIL-BUILT"
    elif not r["sigil_wrote"]:
        r["klass"] = "SIGIL-DECLINED"
    else:
        total, runs, ca, la, cb, lb = compare_with_control(keep, out, log)
        r.update(diff=total, runs=len(runs), ref_crc=ca, ref_size=la,
                 sig_crc=cb, sig_size=lb,
                 offsets=["0x%X" % i for (i, j) in runs[:6]])
        r["klass"] = "AGREE" if total == 0 else "DIFFER"

    # Vacuity: did the flip move the stock toolchain's own image at all? A leg
    # whose reference equals the shipped reference exercised nothing, and
    # reporting it as agreement would be reporting a measurement that could
    # only ever have given one answer.
    if edits and r["lua_wrote"] and vacuity_ref:
        _, base_crc, _ = ident(vacuity_ref)
        _, leg_crc, _ = ident(keep)
        r["vacuous"] = (base_crc == leg_crc)
        r["vacuity_ref"] = os.path.basename(vacuity_ref)
        if r["vacuous"]:
            log("   VACUOUS: the stock build of this leg is byte-identical to %s "
                "(crc32 %s), so the switch under study changed no source the "
                "build reached, and an agreement here would be an answer this "
                "leg could not have failed to give"
                % (os.path.basename(vacuity_ref), base_crc))
    shutil.rmtree(tree)
    log("   LEG_REPORTED %s" % tag)
    return r


# ---------------------------------------------------------------------------
# Controls on this program itself, proven red
# ---------------------------------------------------------------------------

def leg_report(row):
    """What the leg itself reported, as one line, in the terms of its class.

    This is the string the summary table prints in its DETAIL column AND the
    string an acknowledgement's `text` is matched against, so an acknowledgement
    is pinned to what a reader of the table sees rather than to a second,
    privately computed rendering of the same leg.
    """
    if row["klass"] == "AGREE":
        return "crc32 %s size %d" % (row["sig_crc"], row["sig_size"])
    if row["klass"] == "DIFFER":
        return "%d bytes in %d runs at %s" % (row["diff"], row["runs"],
                                              " ".join(row["offsets"]))
    if row["klass"] == "SIGIL-DECLINED":
        said = "\n".join(row.get("sigil_stderr") or [])
        return said if said else "no message"
    if row["klass"] == "LEG-ERROR":
        return row.get("err", "")
    return ""


def ack_mismatch(row, ack):
    """Why an ACK_DISAGREE value does not describe this leg, or None if it does.

    The value is a class, or `(class, text)` where the leg's own report must
    contain `text`. What "its own report" is depends on the class, and
    [`leg_report`] is the one place that decides: for a refusal it is sigil's
    stderr (or the error of a leg that could not run), and for a DIFFER it is the
    byte count, the run count and the offsets.

    A DIFFER is the class that most needs the text. `SIGIL-DECLINED` at least
    names one refusal out of many; `DIFFER` alone says only "the images are not
    the same", which would go on covering the leg however far apart they drifted.
    """
    klass, needle = (ack, None) if isinstance(ack, str) else ack
    if row["klass"] != klass:
        return "acknowledged %s" % klass
    if needle is not None:
        said = leg_report(row)
        if needle not in said:
            return ("acknowledged %s with the text %r, which the leg's own report "
                    "does not contain; it said: %s"
                    % (klass, needle, (said.split("\n") or [""])[0][:120]))
    return None


def reconcile_cross_disagreements(rules, tags, settings, rows):
    """Match each disagreeing CROSS corner against the cross rules.

    A corner is covered when a rule's partial assignment holds at it AND the
    corner reported EXACTLY that rule's pinned difference. Returns
    `(hits_per_rule, uncovered_tags, pin_mismatches)`; the caller asserts that
    the uncovered list is empty, that no rule has zero hits, and that no pin
    mismatched, so the covered set and the observed set are equal in both
    directions.

    Identity, not containment, is deliberate. A rule states that a family of
    corners differs in ONE way for ONE reason; if two of its members differed by
    different amounts, the rule would be covering a second thing nobody
    adjudicated, and containment would let it.
    """
    hits = [0] * len(rules)
    uncovered, wrong = [], []
    for tag in tags:
        setting = settings[tag]
        covered_by = [i for i, (d, pin, why) in enumerate(rules)
                      if all(setting.get(k) == v for k, v in d.items())]
        if not covered_by:
            uncovered.append(tag)
            continue
        for i in covered_by:
            hits[i] += 1
            said = leg_report(rows[tag])
            if said != rules[i][1]:
                wrong.append((tag, rules[i][1], said))
    return hits, uncovered, wrong


def reconcile_stock_declines(rows, ack):
    """Split the corners the STOCK toolchain could not build by what sigil did
    there, and say which of those need acknowledging.

    At such a corner there is no reference image, so the two outcomes mean
    opposite things and must never be counted together:

      sigil ALSO declined  the two toolchains agree that the corner is not
                           buildable. Nothing to check, and this is what the
                           Sonic 1 `Revision=0 + FixBugs=1 + AllOptimizations=0`
                           corners do: both refuse at the same source line, asl
                           with `error #1370: jump distance too big` and sigil
                           with `(d16,PC)/bra.w displacement out of range`.

      sigil BUILT          sigil wrote a ROM nothing can compare against. That
                           is the silent-wrong-ROM direction this whole campaign
                           exists to close, so it is a FINDING: it must be named
                           in `ACK_STOCK_DECLINE_SIGIL_BUILT` or the run fails.

    The acknowledgement set is asserted equal to the observed set in BOTH
    directions, so an entry that stops matching a corner is as loud as a corner
    no entry covers.
    """
    agreed, unverifiable = [], []
    for r in rows:
        if r.get("lua_wrote"):
            continue
        (unverifiable if r.get("sigil_wrote") else agreed).append(r["tag"])
    problems = []
    for t in unverifiable:
        if t not in ack:
            problems.append(
                "corner %s: the stock toolchain declined it and sigil wrote an "
                "image anyway, which nothing can check against a reference" % t)
    for t in sorted(ack):
        if t not in unverifiable:
            problems.append(
                "ACK_STOCK_DECLINE_SIGIL_BUILT names %s, which this run did not "
                "produce; the acknowledgement is stale" % t)
    return agreed, unverifiable, problems


def self_test(scratch, log):
    """Four controls. Each shows the gate it exercises firing on a mutation, on
    disk, because a gate that has never been seen red is a gate nobody has
    shown can be red at all."""
    ok = True
    d = os.path.join(scratch, "selftest")
    if os.path.isdir(d):
        shutil.rmtree(d)
    os.makedirs(d)
    p = os.path.join(d, "t.asm")

    def expect_fail(name, fn, want):
        nonlocal ok
        try:
            fn()
        except Fail as e:
            hit = want in str(e)
            log("CONTROL %s: %s (%s)"
                % (name, "PASSED, gate fired" if hit else "FAILED, wrong reason",
                   str(e).split("\n")[0][:110]))
            ok = ok and hit
            return
        log("CONTROL %s: FAILED, the gate did not fire" % name)
        ok = False

    # C1: an edit that writes back the value already there must be refused.
    open(p, "w").write("Revision = 1\n;\t| If 0, x\n")
    expect_fail("C1 no-op-edit",
                lambda: apply_edit(p, 1, "Revision", 1, lambda s: None),
                "changed no text")
    log("   C1 mutation on disk: %r" % read_lines(p)[0])

    # C2: an edit aimed at a line that does not declare the switch.
    open(p, "w").write("; a comment\nRevision = 1\n;\t| If 0, x\n")
    expect_fail("C2 wrong-line",
                lambda: apply_edit(p, 1, "Revision", 0, lambda s: None),
                "does not declare")

    # C3: a second column-0 declaration of the same switch must be refused,
    # because the edit would be shadowed by the later one.
    open(p, "w").write("Revision = 1\n;\t| If 0, x\nRevision = 2\n")
    log("   C3 mutation on disk: %r" % read_lines(p)[2])
    expect_fail("C3 shadowed-declaration",
                lambda: apply_edit(p, 1, "Revision", 0, lambda s: None),
                "declared at column 0 on lines")

    # C4: the planted-byte control must itself be able to fail. A comparer that
    # reads one file twice cannot see a planted byte.
    a = os.path.join(d, "a.bin")
    open(a, "wb").write(bytes(1024))

    def blind():
        real = bytediff
        try:
            globals()["bytediff"] = lambda x, y: (0, [])
            compare_with_control(a, a, lambda s: None)
        finally:
            globals()["bytediff"] = real
    expect_fail("C4 blind-comparer", blind, "control failed")

    # C5: a documented declaration outside the ASSEMBLY OPTIONS block must be a
    # failure of the block cross-check, not a silent extra row.
    t2 = os.path.join(d, "tree")
    os.makedirs(t2)
    open(os.path.join(t2, "root.asm"), "w").write(
        "; ASSEMBLY OPTIONS:\nA = 1\n;\t| If 0, x\n; ==========\n")
    open(os.path.join(t2, "other.asm"), "w").write("B = 1\n;\t| If 0, y\n")
    log("   C5 mutation on disk: other.asm %r"
        % read_lines(os.path.join(t2, "other.asm"))[0])
    expect_fail("C5 option-outside-block",
                lambda: classify(t2, "selftest", "root.asm"),
                "disagree")

    # C6: a switch whose prose derives a one-element domain must be a failure,
    # not a switch that is counted as swept and then runs nothing. This is the
    # gate that caught `padToPowerOfTwo = 1`, whose prose says only `If 1,`.
    t3 = os.path.join(d, "tree6")
    os.makedirs(t3)
    open(os.path.join(t3, "root.asm"), "w").write(
        "; ASSEMBLY OPTIONS:\nA = 5\n;\t| If 5, x\n; ==========\n")
    log("   C6 mutation on disk: %r" % read_lines(os.path.join(t3, "root.asm"))[1])
    expect_fail("C6 one-element-domain",
                lambda: classify(t3, "selftest", "root.asm"),
                "execute no leg")

    # C8: the warning-parity normaliser, on the two real diagnostic shapes
    # observed in these corpora. It needs its own control because the shape
    # that matters most, a source `warning` directive both toolchains fire,
    # occurs only at corners sigil refuses for an unrelated reason, so no leg
    # in a passing run ever exercises it: an untested normaliser would report
    # parity it never checked.
    #
    # The coded half is the same shape one step further: asl's `#180` and
    # sigil's `[as.odd-address]` pair by LOCATION, and the pairing must be able
    # to report each way it can go wrong, not only agreement. The fixtures are
    # the real lines, asl's from a Sonic 2 build log and sigil's rendering of
    # the same line, plus a macro-trail location from probe `p14_macro` and the
    # comma-and-space file name Sonic 1 really has.
    asl_src = ("> > > sonic.asm(139): warning: 'Revision = 2' is unnecessary "
               "with 'FixBugs' enabled (use 'Revision = 1' instead).\n"
               "> > > s2.asm(30438): warning #180: address is not properly "
               "aligned\n"
               "> > > p14_macro.asm(7) rd(1): warning #180: address is not "
               "properly aligned\n"
               "> > > 87, 88, 89 Ending Sequence Sonic, Emeralds, Logo.asm(270): "
               "warning #180: address is not properly aligned\n")
    odd = ("warning: [as.odd-address] word access at odd address $1: on a 68000 "
           "a word or long read or write at an odd address is an address error "
           "and crashes the machine (asl #180: address is not properly aligned)\n")
    sig_src = ("sonic.asm(139):2: warning: [as.warning] 'Revision = 2' is "
               "unnecessary with 'FixBugs' enabled (use 'Revision = 1' "
               "instead).\n"
               "s2.asm(91275):2: warning: `shared` is ignored: sigil writes no "
               "share file\n"
               "s2.asm(30438):2: " + odd +
               "p14_macro.asm(7) rd(1):2: " + odd +
               "87, 88, 89 Ending Sequence Sonic, Emeralds, Logo.asm(270):2: " + odd)
    ac, at, al = asl_warnings(asl_src)
    st, so, sl = sigil_warnings(sig_src)
    want_locs = {"s2.asm(30438)", "p14_macro.asm(7) rd(1)",
                 "87, 88, 89 Ending Sequence Sonic, Emeralds, Logo.asm(270)"}
    shapes = (ac == {"180"} and at == st and at and so == {"`shared` is ignored"}
              and al == {"180": want_locs} and sl == {"180": want_locs})

    def keys(asl_text, sig_text):
        c, t, l = asl_warnings(asl_text)
        s_t, s_o, s_l = sigil_warnings(sig_text)
        return warning_parity_keys({"asl_coded": c, "asl_text": t, "asl_coded_locs": l,
                                    "sig_text": s_t, "sig_own": s_o,
                                    "sig_counterpart_locs": s_l})
    asl180 = "> > > s2.asm(30438): warning #180: address is not properly aligned\n"
    pair_cases = [
        ("same line", asl180, "s2.asm(30438):2: " + odd, set()),
        ("sigil silent", asl180, "", {"asl#180"}),
        ("sigil on another line", asl180, "s2.asm(30439):2: " + odd,
         {"asl#180", "sigil-only:[as.odd-address]"}),
        ("asl silent", "", "s2.asm(30438):2: " + odd, {"sigil-only:[as.odd-address]"}),
        ("asl line unreadable", "> > > warning #180: address is not properly aligned\n",
         "s2.asm(30438):2: " + odd, {"asl#180", "sigil-only:[as.odd-address]"}),
    ]
    pair_bad = [(name, want, got) for name, a_t, s_t, want in pair_cases
                for got in [keys(a_t, s_t)] if got != want]
    # Row level: a same-source leg where both warn at the same line is a
    # pairing and no key; the end-to-end control leg, where sigil warns on
    # source asl never saw, is not compared at all; a same-source leg where
    # only sigil warns is still a key.
    def row(tag, a_t, s_t, **extra):
        c, t, l = asl_warnings(a_t)
        s_t2, s_o, s_l = sigil_warnings(s_t)
        return dict(tag=tag, corpus="s2disasm", lua_wrote=True, sigil_wrote=True,
                    asl_coded=c, asl_text=t, asl_coded_locs=l, sig_text=s_t2,
                    sig_own=s_o, sig_counterpart_locs=s_l, **extra)
    sig180 = "s2.asm(30438):2: " + odd
    n_both, row_gaps, row_pairs = diagnostic_parity([
        row("paired", asl180, sig180),
        row("CONTROL-divergent-source", "", sig180, control=True),
    ])
    rows_ok = (n_both == 1 and row_gaps == {}
               and set(row_pairs) == {("s2disasm", "asl#180 = [as.odd-address]")})
    _, row_gaps2, _ = diagnostic_parity([row("sigil-only", "", sig180)])
    rows_ok = rows_ok and set(row_gaps2) == {("s2disasm", "sigil-only:[as.odd-address]")}
    if not rows_ok:
        pair_bad.append(("row level", (n_both, row_gaps, row_pairs), row_gaps2))
    c8 = shapes and not pair_bad
    log("CONTROL C8 warning-normaliser: %s"
        % ("PASSED, the source warning both toolchains fire normalises to one "
           "string, the coded and sigil-only ones stay apart, and asl#180 pairs "
           "with [as.odd-address] by location in all %d cases" % len(pair_cases)
           if c8 else "FAILED: asl coded=%s asl text=%s asl locs=%s sigil text=%s "
                      "sigil own=%s sigil locs=%s pairing failures=%s"
                      % (ac, at, al, st, so, sl, pair_bad)))
    ok = ok and c8

    # C9: the stock-decline adjudicator. A corner only sigil builds has no
    # reference image, and the whole risk this runner exists to catch is an
    # image nothing checked being read as agreement. It needs its own control
    # because in a passing run the observed set is EMPTY, and a function that
    # can only ever return an empty set is indistinguishable from a working one.
    c9_rows = [
        {"tag": "both-built", "lua_wrote": True, "sigil_wrote": True},
        {"tag": "stock-declined-sigil-declined", "lua_wrote": False,
         "sigil_wrote": False, "sigil_stderr": ["bra.w out of range"]},
        {"tag": "stock-declined-sigil-built", "lua_wrote": False,
         "sigil_wrote": True},
    ]
    ag, un, pr = reconcile_stock_declines(c9_rows, set())
    c9_finds = (ag == ["stock-declined-sigil-declined"]
                and un == ["stock-declined-sigil-built"]
                and len(pr) == 1 and "nothing can check" in pr[0])
    # ... and the OTHER direction: an acknowledgement matching no corner.
    _, _, pr2 = reconcile_stock_declines(c9_rows[:2], {"gone"})
    c9_stale = len(pr2) == 1 and "stale" in pr2[0]
    log("CONTROL C9 stock-decline-adjudicator: %s"
        % ("PASSED, an image only sigil produced is a finding and an "
           "acknowledgement matching no corner is stale"
           if c9_finds and c9_stale
           else "FAILED: agreed=%s unverifiable=%s problems=%s stale=%s"
                % (ag, un, pr, pr2)))
    ok = ok and c9_finds and c9_stale

    # C10: the build-script derivation. A boolean local outside the Settings
    # block must be a failure of the L2 cross-check, a statement in the block
    # that is not a literal local must be refused, and a number literal in the
    # block must derive UNREADABLE rather than be swept or dropped.
    t10 = os.path.join(d, "tree10")
    os.makedirs(os.path.join(t10, "tools"))
    settings = ("-- Settings --\nlocal fast = false\nlocal addr = 0x10\n"
                "-- End of settings --\n")
    open(os.path.join(t10, "build.lua"), "w").write(settings)
    open(os.path.join(t10, "tools", "x.lua"), "w").write("local extra = true\n")
    log("   C10 mutation on disk: tools/x.lua %r"
        % read_lines(os.path.join(t10, "tools", "x.lua"))[0])
    expect_fail("C10a lua-toggle-outside-block",
                lambda: build_script_settings(t10, "selftest"),
                "outside build.lua's Settings")
    os.remove(os.path.join(t10, "tools", "x.lua"))
    rows10, _ = build_script_settings(t10, "selftest")
    c10b = ([(x["name"], x["kind"], x.get("arms")) for x in rows10]
            == [("build.lua:fast", "swept", [1]), ("build.lua:addr", "unreadable", None)])
    log("CONTROL C10b lua-domains: %s" % ("PASSED, a boolean sweeps its other arm "
                                         "and a number literal is unreadable"
                                         if c10b else "FAILED: %s" % rows10))
    ok = ok and c10b
    open(os.path.join(t10, "build.lua"), "w").write(
        settings.replace("local addr = 0x10\n", "local addr = 0x10\nfast2 = 1\n"))
    expect_fail("C10c lua-block-statement",
                lambda: build_script_settings(t10, "selftest"),
                "is not a column-0")

    # C11: the Lua edit's own refusals, on disk.
    p11 = os.path.join(d, "t.lua")
    open(p11, "w").write("local fast = true\n")
    expect_fail("C11a lua-no-op-edit",
                lambda: apply_lua_edit(p11, 1, "fast", 1, lambda s: None),
                "changed no text")
    open(p11, "w").write("local fast = false\nfast = true\n")
    log("   C11b mutation on disk: %r" % read_lines(p11)[1])
    expect_fail("C11b lua-overriding-assignment",
                lambda: apply_lua_edit(p11, 1, "fast", 1, lambda s: None),
                "would override the edit")
    # ... and a table field that only READS the setting, the shape s2disasm's
    # build.lua has at its line 53, must not be mistaken for an assignment.
    open(p11, "w").write("local fast = false\nlocal t = {\n\tfast = fast,\n}\n")
    try:
        apply_lua_edit(p11, 1, "fast", 1, lambda s: None)
        c11c = read_lines(p11)[0] == "local fast = true"
    except Fail as e:
        c11c = False
        log("   C11c refused: %s" % e)
    log("CONTROL C11c lua-table-field-is-a-read: %s"
        % ("PASSED" if c11c else "FAILED, a table field blocked the edit"))
    ok = ok and c11c

    # C12: an acknowledgement carrying a diagnostic text must stop covering a leg
    # that refuses for a different reason, while the class alone still matches.
    row12 = {"klass": "SIGIL-DECLINED", "sigil_stderr": ["error: something else"]}
    c12 = (ack_mismatch(row12, ("SIGIL-DECLINED", "format sigil does not implement"))
           and ack_mismatch(row12, "SIGIL-DECLINED") is None
           and ack_mismatch(dict(row12, sigil_stderr=[
               "x: `kosinski-optimised` is a p2bin format sigil does not implement"]),
               ("SIGIL-DECLINED", "format sigil does not implement")) is None)
    log("CONTROL C12 ack-diagnostic-text: %s"
        % ("PASSED, a refusal for a neighbouring reason is a mismatch"
           if c12 else "FAILED"))
    ok = ok and bool(c12)

    # C12b: the same for a DIFFER, whose text is the byte count, the run count
    # and the offsets. An acknowledged difference that grows a byte, moves, or
    # spreads to a second run must stop being covered; the class alone must not
    # be enough to keep covering it.
    row12b = {"klass": "DIFFER", "diff": 2, "runs": 2,
              "offsets": ["0x18F", "0xEC051"]}
    pin = ("DIFFER", "2 bytes in 2 runs at 0x18F 0xEC051")
    c12b = (ack_mismatch(row12b, pin) is None
            and ack_mismatch(dict(row12b, diff=3), pin)
            and ack_mismatch(dict(row12b, runs=3), pin)
            and ack_mismatch(dict(row12b, offsets=["0x18F", "0xEC052"]), pin)
            and ack_mismatch(row12b, ("SIGIL-DECLINED", "anything")))
    log("CONTROL C12b ack-difference-text: %s"
        % ("PASSED, a difference of another size, shape or place is a mismatch"
           if c12b else "FAILED"))
    ok = ok and bool(c12b)

    # C13: the cross-corner acknowledgement, over synthetic corners. A rule
    # covers a family only where its partial assignment holds; a corner outside
    # every rule is uncovered; a covered corner whose difference is not the
    # pinned one is a mismatch, not a pass; and a rule that covers nothing has
    # zero hits, which the caller reads as stale.
    def xrow(diff, runs, offs):
        return {"klass": "DIFFER", "diff": diff, "runs": runs, "offsets": offs}
    xr = [({"opt": 1}, "2 bytes in 2 runs at 0x18F 0xEC051", "why")]
    xset = {"a": {"opt": 1, "rev": 0}, "b": {"opt": 1, "rev": 1},
            "c": {"opt": 0, "rev": 1}}
    two = xrow(2, 2, ["0x18F", "0xEC051"])
    hits, unc, bad = reconcile_cross_disagreements(
        xr, ["a", "b"], xset, {"a": two, "b": dict(two)})
    c13a = (hits == [2] and unc == [] and bad == [])
    # a corner the rule does not reach
    hits, unc, bad = reconcile_cross_disagreements(
        xr, ["a", "c"], xset, {"a": two, "c": two})
    c13b = (hits == [1] and unc == ["c"] and bad == [])
    # a covered corner that differs by a different amount
    three = xrow(3, 2, ["0x18F", "0xEC051"])
    hits, unc, bad = reconcile_cross_disagreements(
        xr, ["a", "b"], xset, {"a": two, "b": three})
    c13c = (hits == [2] and unc == [] and len(bad) == 1 and bad[0][0] == "b")
    # a rule that covers nothing at all
    hits, unc, bad = reconcile_cross_disagreements(
        xr, ["c"], xset, {"c": two})
    c13d = (hits == [0] and unc == ["c"])
    c13 = c13a and c13b and c13c and c13d
    log("CONTROL C13 cross-corner-acknowledgement: %s"
        % ("PASSED, a corner outside the rule, a corner differing by another "
           "amount, and a rule covering nothing are each caught" if c13
           else "FAILED (a=%s b=%s c=%s d=%s)" % (c13a, c13b, c13c, c13d)))
    ok = ok and bool(c13)

    shutil.rmtree(d)
    log("SELF_TEST %s" % ("PASSED" if ok else "FAILED"))
    return ok


# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sigil", required=True)
    ap.add_argument("--scratch",
                    default="/home/volence/sonic_hacks/.scratch/switch-matrix-sweep")
    ap.add_argument("--corpus", action="append", default=None,
                    help="NAME=PATH[@REV] of a corpus checkout; the corpus is "
                         "read out of commit REV (default HEAD); repeatable")
    ap.add_argument("--only", action="append", default=None,
                    help="run only these leg tags")
    ap.add_argument("--cross", action="store_true",
                    help="also run every corner of the switch space, and test "
                         "whether a refusal is caused by one setting and composes")
    ap.add_argument("--skip-self-test", action="store_true",
                    help="derivation only; no figure may be reported from a run "
                         "that used this")
    ap.add_argument("--derive-only", action="store_true",
                    help="stop after the derivation and its acknowledgement "
                         "reconciliation; launch no leg")
    a = ap.parse_args()

    corpora = []
    for spec in (a.corpus or ["s1disasm=/home/volence/sonic_hacks/s1disasm",
                              "s2disasm=/home/volence/sonic_hacks/s2disasm"]):
        name, _, rest = spec.partition("=")
        path, _, rev = rest.partition("@")
        corpora.append((name, path, rev or "HEAD"))

    lines = []

    def log(s):
        print(s, flush=True)
        lines.append(s)

    os.makedirs(os.path.join(a.scratch, "trees"), exist_ok=True)
    os.makedirs(os.path.join(a.scratch, "images"), exist_ok=True)
    os.makedirs(os.path.join(a.scratch, "logs"), exist_ok=True)

    ver = subprocess.run([a.sigil, "--version"], capture_output=True, text=True)
    log("SIGIL %s" % ver.stdout.split("\n")[0])
    log("SIGIL md5 %s" % subprocess.run(
        ["md5sum", a.sigil], capture_output=True, text=True).stdout.split()[0])

    failures = []
    if not a.skip_self_test:
        if not self_test(a.scratch, log):
            log("ABORT: this program's own controls did not pass, so no figure "
                "it prints is admissible")
            return 3
    else:
        failures.append("self-test skipped")

    all_rows, all_rescue, all_cross = [], [], []
    launched, reported = 0, 0
    unreadable_found = {}

    for corpus, src, rev in corpora:
        log("")
        log("=" * 78)
        rp = subprocess.run(["git", "-C", src, "rev-parse", "--verify",
                             rev + "^{commit}"], capture_output=True, text=True)
        head = rp.stdout.strip()
        if rp.returncode != 0 or not re.fullmatch(r"[0-9a-f]{40}", head):
            raise Fail("%s: `%s` does not name a commit in %s: %s"
                       % (corpus, rev, src, rp.stderr.strip()))
        log("CORPUS %s at %s (%s)" % (corpus, head, rev))
        # Extracted afresh from the named commit on every run. A tree kept from
        # an earlier run would be measured under this run's revision line while
        # holding whatever commit that earlier run extracted.
        pristine = os.path.join(a.scratch, "trees", corpus + "-pristine")
        if os.path.isdir(pristine):
            shutil.rmtree(pristine)
        os.makedirs(pristine)
        tar = subprocess.Popen(["git", "-C", src, "archive", "--format=tar", head],
                               stdout=subprocess.PIPE)
        untar = subprocess.run(["tar", "-x", "-C", pristine], stdin=tar.stdout)
        tar.stdout.close()
        if tar.wait() != 0 or untar.returncode != 0:
            raise Fail("%s: extracting %s from %s failed (git archive exit %s, "
                       "tar exit %s)" % (corpus, head, src, tar.returncode,
                                         untar.returncode))
        log("  extracted read-only by git archive of %s into %s" % (head, pristine))

        root_asm, out_bin, p2bin_args = derive_tool_args(pristine)
        log("  build.lua says: root=%s output=%s p2bin/sigil args=%s"
            % (root_asm, out_bin, " ".join(p2bin_args)))

        asm_switches = classify(pristine, corpus, root_asm)
        log("  DERIVED %d documented switches, cross-checked against the "
            "ASSEMBLY OPTIONS block" % len(asm_switches))
        lua_switches, n_lua_files = build_script_settings(pristine, corpus)
        log("  DERIVED %d build-script settings from build.lua's Settings block, "
            "cross-checked against every column-0 boolean local in %d .lua file(s)"
            % (len(lua_switches), n_lua_files))
        switches = asm_switches + lua_switches
        for s in switches:
            if s["kind"] == "swept":
                log("    %-40s = %-3d domain %-12s arms %s"
                    % (s["name"], s["current"], s["domain"], s["arms"]))
            else:
                log("    %-40s   %-8s %s" % (s["name"], s["kind"].upper(), s["why"]))
                if s["kind"] == "unreadable":
                    unreadable_found[(corpus, s["name"])] = s["why"]

        def population(rows):
            return (len(rows), sum(1 for x in rows if x["kind"] == "swept"),
                    sum(len(x["arms"]) for x in rows if x["kind"] == "swept"))
        pa, pl = population(asm_switches), population(lua_switches)
        corners_n = 1
        for x in switches:
            if x["kind"] == "swept":
                corners_n *= len(x["domain"])
        log("POPULATION %s asm_options=%d asm_swept=%d asm_arms=%d "
            "build_script_settings=%d build_script_swept=%d build_script_arms=%d "
            "lua_files=%d corners=%d"
            % ((corpus,) + pa + pl + (n_lua_files, corners_n)))
        if a.derive_only:
            continue

        cfg = {"scratch": a.scratch, "pristine": pristine, "corpus": corpus,
               "sigil": a.sigil, "root_asm": root_asm, "out_bin": out_bin,
               "p2bin_args": p2bin_args}

        # The shipped-settings baseline, which is also the vacuity yardstick.
        plan = [("%s-shipped" % corpus, [])]
        for s in switches:
            if s["kind"] != "swept":
                continue
            for v in s["arms"]:
                plan.append(("%s-%s-%d" % (corpus, s["name"], v), [(s, v)]))

        n_swept = sum(1 for s in switches if s["kind"] == "swept")
        n_arms = sum(len(s["arms"]) for s in switches if s["kind"] == "swept")
        n_excluded = len(switches) - n_swept
        log("  RECONCILE switches: %d derived = %d swept + %d excluded"
            % (len(switches), n_swept, n_excluded))
        if n_swept + n_excluded != len(switches):
            failures.append("%s: switch reconciliation does not add up" % corpus)
        if len(plan) - 1 != n_arms:
            failures.append("%s: planned %d flip legs for %d arms"
                            % (corpus, len(plan) - 1, n_arms))
        log("  RECONCILE legs: %d arms -> %d flip legs + 1 baseline"
            % (n_arms, len(plan) - 1))

        by_tag = {}

        def do_leg(tag, edits, vacuity_ref):
            nonlocal launched, reported
            log("")
            log("-- LEG %s   %s" % (tag, edits and
                                    ", ".join("%s = %d" % (s["name"], v)
                                              for s, v in edits) or "shipped"))
            launched += 1
            try:
                r = run_leg(cfg, tag, edits, log, vacuity_ref=vacuity_ref)
                log("   RESULT %s" % r["klass"])
            except Fail as e:
                r = {"tag": tag, "corpus": corpus, "klass": "LEG-ERROR",
                     "err": str(e)}
                log("   LEG FAILED: %s" % e)
                log("   LEG_ERROR %s" % tag)
                failures.append("%s: %s" % (tag, e))
            reported += 1
            all_rows.append(r)
            by_tag[tag] = r
            return r

        # Phase 1: the shipped baseline, then one leg per arm of one switch.
        for tag, edits in plan:
            if a.only and tag not in a.only:
                continue
            do_leg(tag, edits, cfg.get("baseline_ref"))
            if not edits and by_tag.get(tag, {}).get("ref"):
                cfg["baseline_ref"] = by_tag[tag]["ref"]

        # C7, the end-to-end control, once per corpus. The reference is built
        # from the shipped tree and then the source is flipped underneath
        # sigil, so a run that reports agreement here is a run whose two builds
        # are not independent. The arm used is chosen by measurement, not by
        # name: the first phase-1 arm whose own stock image differs from the
        # shipped one, so the control cannot be run on a flip that moves
        # nothing.
        if not a.only:
            base_crc = by_tag["%s-shipped" % corpus].get("ref_crc")
            pick = None
            for s in switches:
                if s["kind"] != "swept":
                    continue
                for v in s["arms"]:
                    row = by_tag.get("%s-%s-%d" % (corpus, s["name"], v))
                    if row and row.get("ref") and \
                            ident(row["ref"])[1] != base_crc:
                        pick = (s, v)
                        break
                if pick:
                    break
            log("")
            if not pick:
                failures.append("%s: no arm moves the stock image, so the "
                                "end-to-end control cannot be run" % corpus)
                log("== CONTROL C7 %s: NOT RUNNABLE, no arm moves the stock "
                    "image" % corpus)
            else:
                ctag = "%s-CONTROL-divergent-source" % corpus
                log("== CONTROL C7 %s: reference built from the shipped tree, "
                    "then %s = %d applied before sigil runs; the sweep must "
                    "report a difference"
                    % (corpus, pick[0]["name"], pick[1]))
                launched += 1
                cr = run_leg(cfg, ctag, [], log, vacuity_ref=None,
                             post_lua_edits=[pick])
                reported += 1
                cr["control"] = True
                all_rows.append(cr)
                # Only DIFFER passes. A refusal would leave the compare itself
                # unexercised, which is the thing this control exists to
                # exercise, so it is not a substitute for a difference.
                good = cr["klass"] == "DIFFER"
                log("   CONTROL C7 %s: %s (%s)"
                    % (corpus, "PASSED" if good else "FAILED", cr["klass"]))
                if not good:
                    failures.append(
                        "%s: the end-to-end control reported %s. The two builds "
                        "are not independent and no agreement in this run means "
                        "anything" % (corpus, cr["klass"]))

        # Phase 2: rescue the arms phase 1 could not measure. A vacuous leg is
        # one whose flip moved no byte of the stock image, which happens when a
        # switch is only reachable while another switch is set: Sonic 1's
        # BackupSRAM and AddressSRAM are read only where EnableSRAM opens the
        # code that reads them. Reporting those as agreement would be reporting
        # a comparison of two builds of source neither toolchain assembled
        # differently. So each vacuous arm is retried against one companion arm
        # at a time, in declaration order, and the yardstick becomes the stock
        # build of the companion ALONE, so what is still under test is the one
        # switch. This is a targeted rescue, not a cross product: it runs only
        # for arms phase 1 could not measure, and stops at the first companion
        # that makes the stock image move.
        rescue = []
        if not a.only:
            arms = [(s, v) for s in switches if s["kind"] == "swept"
                    for v in s["arms"]]
            for r in [x for x in all_rows
                      if x.get("vacuous") and x["corpus"] == corpus
                      and "+" not in x["tag"]]:
                sw = next(s for s in switches if s["name"] == r["edits"][0][0])
                val = r["edits"][0][1]
                log("")
                log("== RESCUE %s: phase 1 could not measure it; trying "
                    "companions in declaration order" % r["tag"])
                verdict, tried = None, []
                for (cs, cv) in arms:
                    if cs["name"] == sw["name"]:
                        continue
                    crow = by_tag.get("%s-%s-%d" % (corpus, cs["name"], cv))
                    if not crow or not crow.get("ref"):
                        tried.append("%s=%d(no stock image)" % (cs["name"], cv))
                        continue
                    ptag = "%s-%s-%d+%s-%d" % (corpus, sw["name"], val,
                                               cs["name"], cv)
                    pr = do_leg(ptag, [(sw, val), (cs, cv)], crow["ref"])
                    tried.append("%s=%d%s" % (cs["name"], cv,
                                              "" if pr.get("vacuous") else " MOVED"))
                    if not pr.get("vacuous") and pr["klass"] != "LEG-ERROR":
                        verdict = ptag
                        break
                rescue.append({"corpus": corpus, "arm": r["tag"],
                               "rescued_by": verdict, "tried": tried})
                log("   RESCUE %s: %s   (companions tried: %s)"
                    % (r["tag"],
                       "measured as %s" % verdict if verdict
                       else "UNMEASURABLE by any single companion",
                       ", ".join(tried)))
        all_rescue.extend(rescue)

        # Phase 3, optional: every corner of the switch space, not one arm at
        # a time. This was assumed infeasible and it is not: the two corpora
        # have 288 and 96 corners, and a leg costs about two and a half
        # seconds, so the whole product is a quarter of an hour.
        #
        # It tests two things phase 1 structurally cannot.
        #
        # COMPOSITION, on the sigil side. Phase 1 shows which single settings
        # sigil refuses; the prediction is that a refusal is caused by one
        # setting and composes, so whether sigil builds a corner is decided by
        # whether the corner contains such a setting and by nothing else. The
        # causes are read off this same run's phase 1, never from a table, so
        # the prediction cannot be tuned to the answer it is tested against.
        #
        # THE STOCK TOOLCHAIN'S OWN REACH. A corner is a combination of options
        # the corpus documents, and there is no guarantee the corpus can build
        # all of them. Where the stock build fails, sigil has nothing to agree
        # with, and calling that a sigil result would be reading a corpus
        # defect as ours. Those corners are their own category, and they must
        # be covered by a rule in ACK_STOCK_DECLINE that states the partial
        # assignment responsible: the covered set and the observed set are
        # asserted equal in both directions, and a rule that covers no corner
        # is as loud as a corner no rule covers.
        #
        # And where BOTH toolchains built, the corner must agree byte for byte.
        # That is the whole point of the product and it is asserted, not read
        # off a summary line.
        if a.cross and not a.only:
            swept = [s for s in switches if s["kind"] == "swept"]
            domains = [s["domain"] for s in swept]
            corners = list(itertools.product(*domains))
            want = 1
            for d in domains:
                want *= len(d)
            log("")
            log("== CROSS %s: %d swept switches with domains %s -> %d corners"
                % (corpus, len(swept), [len(d) for d in domains], want))
            if len(corners) != want:
                failures.append("%s: enumerated %d corners, the domains give %d"
                                % (corpus, len(corners), want))
            causes = {}
            for s in swept:
                for v in s["arms"]:
                    row = by_tag.get("%s-%s-%d" % (corpus, s["name"], v))
                    if row and not row.get("vacuous") \
                            and not row.get("sigil_wrote"):
                        causes[(s["name"], v)] = (row["sigil_stderr"] or
                                                  ["no message"])[0][:70]
            log("   sigil-refusing settings, read off this run's phase 1: %s"
                % ({"%s=%d" % k: v for k, v in causes.items()} or "none"))
            rules = [(d, why) for (c, d, why) in ACK_STOCK_DECLINE if c == corpus]
            log("   acknowledged stock-decline rules: %d" % len(rules))

            comp_ok, comp_bad, comp_skipped = 0, [], 0
            corner_rows, corner_settings = [], {}
            stock_bad, rule_hits = [], [0] * len(rules)
            agreed, disagreed, crcs = 0, [], {}
            for corner in corners:
                setting = {s["name"]: v for s, v in zip(swept, corner)}
                edits = [(s, v) for s, v in zip(swept, corner)
                         if v != s["current"]]
                tag = "%s-X-%s" % (corpus, "".join(str(v) for v in corner))
                pred = "DECLINED" if any(k in causes for k in setting.items()) \
                    else "BUILT"
                covered = [i for i, (d, w) in enumerate(rules)
                           if all(setting.get(k) == v for k, v in d.items())]
                log("")
                log("-- CORNER %s   %s   predict sigil %s%s"
                    % (tag, " ".join("%s=%d" % (s["name"], v)
                                     for s, v in zip(swept, corner)), pred,
                       ", stock decline acknowledged" if covered else ""))
                launched += 1
                try:
                    r = run_leg(cfg, tag, edits, log)
                except Fail as e:
                    r = {"tag": tag, "corpus": corpus, "klass": "LEG-ERROR",
                         "err": str(e)}
                    log("   LEG FAILED: %s" % e)
                    log("   LEG_ERROR %s" % tag)
                    failures.append("%s: %s" % (tag, e))
                reported += 1
                r["cross"] = True
                all_rows.append(r)
                corner_rows.append(r)
                if r.get("ref_crc"):
                    crcs.setdefault(r["ref_crc"], tag)
                if r.get("ref") and os.path.isfile(r["ref"]):
                    os.remove(r["ref"])

                got = "BUILT" if r.get("sigil_wrote") else "DECLINED"
                # THE COMPOSITION PREDICTION IS ONLY ASKED WHERE THE STOCK
                # TOOLCHAIN BUILT. It predicts sigil's own front-end refusals
                # from the single settings phase 1 saw sigil refuse. At a
                # corner the corpus's OWN toolchain cannot assemble, sigil
                # declining is agreement with the reference rather than a
                # surprise on the sigil side, and the interesting question is
                # the opposite one, handled by `reconcile_stock_declines`
                # below. Scoring those corners against the prediction would
                # make the runner permanently red on a Sonic 1 defect it
                # already reports through its own rule.
                if not r.get("lua_wrote"):
                    comp_skipped += 1
                elif got == pred:
                    comp_ok += 1
                else:
                    comp_bad.append((tag, pred, got))
                for i in covered:
                    rule_hits[i] += 1
                if not r.get("lua_wrote"):
                    stock_bad.append((tag, bool(covered)))
                elif r.get("sigil_wrote"):
                    if r["klass"] == "AGREE":
                        agreed += 1
                    else:
                        disagreed.append((tag, r["klass"]))
                corner_settings[tag] = setting
                log("   RESULT %s   sigil %s (predicted %s)%s"
                    % (r["klass"], got, pred,
                       "   STOCK DECLINED" if not r.get("lua_wrote") else ""))

            log("")
            log("== CROSS %s: %d corners" % (corpus, len(corners)))
            log("   composition prediction on the sigil side: %d held, %d "
                "broke, %d not asked (the stock toolchain declined the corner, "
                "so there is no reference and a sigil refusal there agrees "
                "with it)" % (comp_ok, len(comp_bad), comp_skipped))
            for t, pd, g in comp_bad:
                log("     BROKE %s: predicted sigil %s, sigil %s" % (t, pd, g))
            log("   both toolchains built: %d corners, %d agreed byte for "
                "byte, %d did not" % (agreed + len(disagreed), agreed,
                                      len(disagreed)))
            for t, k in disagreed:
                log("     DISAGREED %s: %s" % (t, k))
            log("   stock toolchain declined: %d corners, %d covered by an "
                "acknowledged rule" % (len(stock_bad),
                                       sum(1 for t, c in stock_bad if c)))
            for t, c in stock_bad:
                if not c:
                    log("     UNACKNOWLEDGED STOCK DECLINE %s" % t)
            sd_agreed, sd_unver, sd_problems = reconcile_stock_declines(
                corner_rows, {t for (c, t) in ACK_STOCK_DECLINE_SIGIL_BUILT
                              if c == corpus})
            log("   ... of those, sigil ALSO declined %d and BUILT %d. A "
                "corner only sigil builds has no reference image, so it is a "
                "finding and not a pass." % (len(sd_agreed), len(sd_unver)))
            for t in sd_unver:
                log("     UNVERIFIABLE IMAGE %s: sigil wrote a ROM at a corner "
                    "the corpus's own toolchain refuses" % t)
            for t in sd_agreed[:2]:
                row = next(r for r in corner_rows if r["tag"] == t)
                log("     sigil's refusal at %s: %s"
                    % (t, (row.get("sigil_stderr") or ["no message"])[0][:110]))
            for pr in sd_problems:
                failures.append("%s: %s" % (corpus, pr))
            for i, (d, w) in enumerate(rules):
                log("   rule %s covered %d corner(s)" % (d, rule_hits[i]))
            log("   distinct stock images across the corners: %d" % len(crcs))

            if comp_bad:
                failures.append("%s: %d corner(s) broke the composition "
                                "prediction" % (corpus, len(comp_bad)))
            xrules = [(d, pin, why) for (c, d, pin, why) in ACK_CROSS_DISAGREE
                      if c == corpus]
            xhits, xuncovered, xwrong = reconcile_cross_disagreements(
                xrules, [t for t, k in disagreed], corner_settings,
                {r["tag"]: r for r in corner_rows})
            log("   of the %d that did not agree, %d are covered by an "
                "acknowledged cross rule and %d are not"
                % (len(disagreed), len(disagreed) - len(xuncovered),
                   len(xuncovered)))
            for i, (d, pin, w) in enumerate(xrules):
                log("   cross rule %s pinned %r covered %d corner(s)"
                    % (d, pin, xhits[i]))
            for t in xuncovered:
                log("     UNACKNOWLEDGED DISAGREEMENT %s" % t)
            for t, want, got in xwrong:
                log("     CROSS PIN MISMATCH %s: acknowledged %r, reported %r"
                    % (t, want, got))
            if xuncovered:
                failures.append("%s: %d corner(s) built by both toolchains "
                                "disagree and no cross rule covers them: %s"
                                % (corpus, len(xuncovered), xuncovered[:4]))
            if xwrong:
                failures.append("%s: %d corner(s) differ in a way their cross "
                                "rule does not pin" % (corpus, len(xwrong)))
            xdead = [xrules[i][0] for i in range(len(xrules)) if xhits[i] == 0]
            if xdead:
                failures.append("%s: %d cross rule(s) cover no disagreeing "
                                "corner and are stale: %s"
                                % (corpus, len(xdead), xdead))
            uncov = [t for t, c in stock_bad if not c]
            if uncov:
                failures.append("%s: %d corner(s) the stock toolchain cannot "
                                "build are covered by no rule: %s"
                                % (corpus, len(uncov), uncov[:4]))
            dead = [rules[i][0] for i in range(len(rules)) if rule_hits[i] == 0]
            if dead:
                failures.append("%s: %d stock-decline rule(s) cover no corner "
                                "and are stale: %s" % (corpus, len(dead), dead))
            if len(crcs) < 2:
                failures.append("%s: the cross product produced %d distinct "
                                "stock image(s); it measured nothing"
                                % (corpus, len(crcs)))
            all_cross.append({"corpus": corpus, "corners": len(corners),
                              "comp_ok": comp_ok, "comp_bad": len(comp_bad),
                              "agreed": agreed, "disagreed": len(disagreed),
                              "stock_declined": len(stock_bad),
                              "distinct": len(crcs)})

    ack_keys = set(ACK_UNREADABLE)
    got_keys = set(unreadable_found)
    if a.derive_only:
        log("")
        log("RECONCILE unreadable-domain: found=%d acknowledged=%d %s"
            % (len(got_keys), len(ack_keys),
               "MATCH" if ack_keys == got_keys else "MISMATCH"))
        if ack_keys != got_keys:
            failures.append("unreadable-domain acknowledgements are stale: "
                            "found-not-acknowledged=%s acknowledged-not-found=%s"
                            % (sorted(got_keys - ack_keys),
                               sorted(ack_keys - got_keys)))
        log("DERIVE-ONLY: no leg was launched. This run is a probe of the "
            "derivation; it is not a sweep result.")
        if failures:
            log("SWEEP FAILED, %d reason(s):" % len(failures))
            for f in failures:
                log("  - %s" % f)
        log("SWEEP_END")
        return 1 if failures else 0

    log("")
    log("=" * 78)
    log("RECONCILE legs launched=%d reported=%d" % (launched, reported))
    if launched != reported:
        failures.append("%d legs launched but %d reported; %d produced no row "
                        "at all" % (launched, reported, launched - reported))

    if ack_keys != got_keys and not a.only:
        failures.append("unreadable-domain acknowledgements are stale: "
                        "found-not-acknowledged=%s acknowledged-not-found=%s"
                        % (sorted(got_keys - ack_keys), sorted(ack_keys - got_keys)))
    both, gaps, parity_pairs = diagnostic_parity(all_rows)
    log("RECONCILE diagnostics: %d leg(s) where both toolchains ran, %d "
        "warning-parity key(s) seen, %d acknowledged"
        % (both, len(gaps), len(ACK_WARNING_GAP)))
    for k in sorted(gaps):
        log("  %s %s on %d leg(s)%s"
            % (k[0], k[1], gaps[k],
               "" if k in ACK_WARNING_GAP else "   UNACKNOWLEDGED"))
    if set(gaps) != set(ACK_WARNING_GAP) and not a.only:
        failures.append("warning-parity acknowledgements are stale: "
                        "found-not-acknowledged=%s acknowledged-not-found=%s"
                        % (sorted(set(gaps) - set(ACK_WARNING_GAP)),
                           sorted(set(ACK_WARNING_GAP) - set(gaps))))
    log("RECONCILE coded-warning parity: %d pairing(s) seen, %d expected"
        % (len(parity_pairs), len(EXPECT_WARNING_PARITY)))
    for k in sorted(parity_pairs):
        log("  %s %s at the same locations on %d leg(s): %s%s"
            % (k[0], k[1], parity_pairs[k][0], ", ".join(sorted(parity_pairs[k][1])),
               "" if k in EXPECT_WARNING_PARITY else "   UNEXPECTED"))
    if set(parity_pairs) != set(EXPECT_WARNING_PARITY) and not a.only:
        failures.append("coded-warning parity expectations are stale: "
                        "seen-not-expected=%s expected-not-seen=%s"
                        % (sorted(set(parity_pairs) - set(EXPECT_WARNING_PARITY)),
                           sorted(set(EXPECT_WARNING_PARITY) - set(parity_pairs))))

    log("RECONCILE unreadable-domain: found=%d acknowledged=%d %s"
        % (len(got_keys), len(ack_keys),
           "MATCH" if ack_keys == got_keys else "MISMATCH"))

    log("")
    log("%-42s %-30s %s" % ("LEG", "RESULT", "DETAIL"))
    for r in [x for x in all_rows if not x.get("cross")]:
        detail = leg_report(r).split("\n")[0][:90]
        # The RESULT column never says AGREE for a leg that exercised nothing.
        # "The build agreed" and "the build could not disagree" must not be the
        # same word in a table anyone reads.
        klass = r["klass"]
        if r.get("vacuous"):
            klass = "NOT-MEASURED"
            detail += "   [stock image unchanged by this flip; the compare "
            detail += "reported %s and could not have reported otherwise]" \
                % r["klass"]
        if r.get("control"):
            klass = "CONTROL/" + klass
        log("%-42s %-30s %s" % (r["tag"], klass, detail))

    # Every arm phase 1 could not measure must end with a verdict, and an arm
    # that no single companion can reach is UNMEASURABLE and must be
    # acknowledged rather than counted anywhere as agreement.
    vac1 = [r for r in all_rows if r.get("vacuous") and "+" not in r["tag"]]
    log("RECONCILE vacuity: %d phase-1 arm(s) exercised no changed source, "
        "%d rescue verdict(s)" % (len(vac1), len(all_rescue)))
    if len(vac1) != len(all_rescue) and not a.only:
        failures.append("%d vacuous arms but %d rescue verdicts"
                        % (len(vac1), len(all_rescue)))
    unmeasurable = {(x["corpus"], x["arm"]) for x in all_rescue
                    if not x["rescued_by"]}
    for x in all_rescue:
        log("  %s: %s" % (x["arm"], "measured as %s" % x["rescued_by"]
                          if x["rescued_by"] else "UNMEASURABLE"))
    if unmeasurable != set(ACK_UNMEASURABLE) and not a.only:
        failures.append("unmeasurable acknowledgements are stale: "
                        "found-not-acknowledged=%s acknowledged-not-found=%s"
                        % (sorted(unmeasurable - set(ACK_UNMEASURABLE)),
                           sorted(set(ACK_UNMEASURABLE) - unmeasurable)))

    # An outcome that is not agreement must be acknowledged, and an
    # acknowledgement that no longer describes an outcome must be removed. A
    # vacuous leg is left out of this bookkeeping entirely: its agreement is
    # not evidence of anything, so counting it as a pass is the exact error
    # this program is here to prevent.
    bad, bad_rows = {}, {}
    for r in all_rows:
        if r["klass"] == "AGREE" or r.get("vacuous") or r.get("control") \
                or r.get("cross"):
            continue
        key = (r["corpus"], r["tag"])
        bad[key] = r["klass"]
        bad_rows[key] = r
    ackd = dict(ACK_DISAGREE)
    unack = {k: v for k, v in bad.items() if k not in ackd}
    stale = {k: v for k, v in ackd.items() if k not in bad}
    wrong = {}
    for k in bad:
        if k in ackd:
            why = ack_mismatch(bad_rows[k], ackd[k])
            if why:
                wrong[k] = (bad[k], why)
    log("")
    log("RECONCILE outcomes: %d non-agreeing, %d acknowledged, %d unacknowledged, "
        "%d stale acknowledgements, %d class mismatches"
        % (len(bad), len(ackd), len(unack), len(stale), len(wrong)))
    for k, v in sorted(unack.items()):
        log("  UNACKNOWLEDGED %s %s: %s" % (k[0], k[1], v))
    for k, v in sorted(stale.items()):
        log("  STALE ACKNOWLEDGEMENT %s %s: expected %s, the leg agreed"
            % (k[0], k[1], v))
    for k, v in sorted(wrong.items()):
        log("  CLASS MISMATCH %s %s: ran %s, %s" % (k[0], k[1], v[0], v[1]))
    if not a.only:
        if unack:
            failures.append("%d unacknowledged non-agreeing legs" % len(unack))
        if stale:
            failures.append("%d stale acknowledgements" % len(stale))
        if wrong:
            failures.append("%d acknowledgements name the wrong class" % len(wrong))

    if a.only:
        log("")
        log("PARTIAL RUN: --only was given, so legs outside %s did not run and "
            "the outcome, unreadable-domain and leg-count reconciliations were "
            "not enforced. This run is a probe; it is not a sweep result and no "
            "table may be built from it." % sorted(a.only))
    log("")
    if failures:
        log("SWEEP FAILED, %d reason(s):" % len(failures))
        for f in failures:
            log("  - %s" % f)
    else:
        log("SWEEP PASSED")
    log("SWEEP_END")
    return 1 if failures else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Fail as e:
        print("SWEEP ABORTED: %s" % e)
        print("SWEEP_END")
        sys.exit(2)
    except Exception:
        # Python's own exit status for an uncaught exception is 1, which is this
        # program's status for a FINDING. A crash is a run that measured nothing,
        # so it is reported as an abort and never as a result.
        import traceback
        traceback.print_exc()
        print("SWEEP ABORTED: uncaught exception, see the traceback above")
        print("SWEEP_END")
        sys.exit(2)
