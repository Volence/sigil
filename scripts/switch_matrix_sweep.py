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

Every leg proves its own edit applied by printing the line before and after from
disk and failing if they are equal, every image compare carries a planted-byte
control, and the leg count launched is reconciled against the leg count reported.

Usage:
    python3 scripts/switch_matrix_sweep.py --sigil <path to sigil> [--scratch DIR]
                                           [--corpus NAME=PATH]... [--only TAG]...
                                           [--cross]
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
}

# (corpus, leg tag): an outcome other than byte agreement that is known,
# adjudicated and booked. The value must equal the class the run computes, so an
# acknowledgement cannot quietly cover a result that changed underneath it.
ACK_DISAGREE = {
    ("s1disasm", "s1disasm-FixBugs-1"): "SIGIL-DECLINED",
    # `_incObj/DebugMode.asm:245`, reachable only under FixBugs, reads
    # `move.w (v_limitright2),d0`: an absolute address operand with no width
    # suffix. asl selects a width; sigil refuses. Every other reference to that
    # variable in the corpus writes `.w`, so this is the one bare site, and it
    # is a front-end width-selection row, not a ROM-writing one. Booked by
    # `SWITCH-SETTING-SILENT-ROMS` as fault 3 and deliberately untouched there.

    ("s2disasm", "s2disasm-fixBugs-1"): "SIGIL-DECLINED",
    # Under fixBugs the Saxman stream grows to $F88 while the source declares
    # `Size_of_Snd_driver_guess = $F64`, so every byte computed from that name
    # is short by $24. build.lua repairs the reference afterwards, reading the
    # real size out of asl's share file and patching the image in
    # `amend_sound_driver_size`; sigil writes no share file and so cannot, and
    # refuses rather than write a ROM whose decompressor is told the wrong
    # length. Chosen and argued in `SWITCH-SETTING-SILENT-ROMS`, fault 2.
}

# (corpus, leg tag of a phase-1 arm): an arm no single companion switch could
# make visible in the stock image. An entry here is a declaration that the arm
# is UNMEASURED, never that it agreed.
ACK_UNMEASURABLE = {
}


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


# ---------------------------------------------------------------------------
# The compare, which carries its own positive control
# ---------------------------------------------------------------------------

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

    for sw, val in edits:
        apply_edit(os.path.join(tree, sw["file"]), sw["line"], sw["name"], val, log)

    r = {"tag": tag, "corpus": cfg["corpus"],
         "edits": [(s["name"], v) for s, v in edits]}

    lua = subprocess.run(["lua", "build.lua"], cwd=tree,
                         capture_output=True, text=True, timeout=1800)
    r["lua_exit"] = lua.returncode
    refbin = os.path.join(tree, cfg["out_bin"])
    r["lua_wrote"] = os.path.isfile(refbin)
    lualog = os.path.join(cfg["scratch"], "logs", tag + ".lua.log")
    open(lualog, "w").write(lua.stdout + lua.stderr)
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
        apply_edit(os.path.join(tree, sw["file"]), sw["line"], sw["name"], val, log)

    out = os.path.join(tree, "sigil.bin")
    sg = subprocess.run([cfg["sigil"], cfg["root_asm"], "-o", "sigil.bin"]
                        + cfg["p2bin_args"], cwd=tree,
                        capture_output=True, text=True, timeout=1800)
    r["sigil_exit"] = sg.returncode
    r["sigil_wrote"] = os.path.isfile(out)
    r["sigil_stderr"] = [l for l in sg.stderr.split("\n")
                         if l.strip() and "`shared` is ignored" not in l]
    siglog = os.path.join(cfg["scratch"], "logs", tag + ".sigil.err")
    open(siglog, "w").write(sg.stderr)
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
                    help="NAME=PATH of a corpus checkout; repeatable")
    ap.add_argument("--only", action="append", default=None,
                    help="run only these leg tags")
    ap.add_argument("--cross", action="store_true",
                    help="also run every corner of the switch space, and test "
                         "whether a refusal is caused by one setting and composes")
    ap.add_argument("--skip-self-test", action="store_true",
                    help="derivation only; no figure may be reported from a run "
                         "that used this")
    a = ap.parse_args()

    corpora = []
    for spec in (a.corpus or ["s1disasm=/home/volence/sonic_hacks/s1disasm",
                              "s2disasm=/home/volence/sonic_hacks/s2disasm"]):
        name, _, path = spec.partition("=")
        corpora.append((name, path))

    lines = []

    def log(s):
        print(s, flush=True)
        lines.append(s)

    os.makedirs(os.path.join(a.scratch, "trees"), exist_ok=True)
    os.makedirs(os.path.join(a.scratch, "images"), exist_ok=True)

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

    for corpus, src in corpora:
        log("")
        log("=" * 78)
        head = subprocess.run(["git", "-C", src, "rev-parse", "HEAD"],
                              capture_output=True, text=True).stdout.strip()
        log("CORPUS %s at %s" % (corpus, head))
        pristine = os.path.join(a.scratch, "trees", corpus + "-pristine")
        if not os.path.isdir(pristine):
            os.makedirs(pristine)
            tar = subprocess.Popen(["git", "-C", src, "archive",
                                    "--format=tar", "HEAD"],
                                   stdout=subprocess.PIPE)
            subprocess.run(["tar", "-x", "-C", pristine], stdin=tar.stdout)
            tar.wait()
        log("  extracted read-only by git archive into %s" % pristine)

        root_asm, out_bin, p2bin_args = derive_tool_args(pristine)
        log("  build.lua says: root=%s output=%s p2bin/sigil args=%s"
            % (root_asm, out_bin, " ".join(p2bin_args)))

        switches = classify(pristine, corpus, root_asm)
        log("  DERIVED %d documented switches, cross-checked against the "
            "ASSEMBLY OPTIONS block" % len(switches))
        for s in switches:
            if s["kind"] == "swept":
                log("    %-24s = %-3d domain %-12s arms %s"
                    % (s["name"], s["current"], s["domain"], s["arms"]))
            else:
                log("    %-24s   %-8s %s" % (s["name"], s["kind"].upper(), s["why"]))
                if s["kind"] == "unreadable":
                    unreadable_found[(corpus, s["name"])] = s["why"]

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

        # Phase 3, optional: every corner of the switch space, not one arm at a
        # time. This was assumed infeasible and it is not: the two corpora have
        # 288 and 96 corners, and a leg costs about two and a half seconds, so
        # the whole product is a quarter of an hour. What it tests that phase 1
        # cannot is COMPOSITION: phase 1 shows which single settings sigil
        # refuses, and the prediction under test here is that a refusal is
        # caused by one setting and composes, so a corner's class is decided by
        # whether it contains such a setting and by nothing else. The causes are
        # read off this same run's phase 1, never from a table, so the
        # prediction cannot be tuned to the answer.
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
                    if row and row["klass"] != "AGREE" and not row.get("vacuous"):
                        causes[(s["name"], v)] = row["klass"]
            log("   causes read off this run's phase 1: %s"
                % ({"%s=%d" % k: v for k, v in causes.items()} or "none"))

            hits, miss, crcs = 0, [], {}
            for corner in corners:
                edits = [(s, v) for s, v in zip(swept, corner)
                         if v != s["current"]]
                tag = "%s-X-%s" % (corpus, "".join(str(v) for v in corner))
                predicted = "AGREE"
                for s, v in zip(swept, corner):
                    if (s["name"], v) in causes:
                        predicted = causes[(s["name"], v)]
                        break
                log("")
                log("-- CORNER %s   %s   predict %s"
                    % (tag, " ".join("%s=%d" % (s["name"], v)
                                     for s, v in zip(swept, corner)), predicted))
                launched += 1
                try:
                    r = run_leg(cfg, tag, edits, log)
                except Fail as e:
                    r = {"tag": tag, "corpus": corpus, "klass": "LEG-ERROR",
                         "err": str(e)}
                    failures.append("%s: %s" % (tag, e))
                reported += 1
                r["cross"] = True
                r["predicted"] = predicted
                all_rows.append(r)
                if r.get("ref_crc"):
                    crcs.setdefault(r["ref_crc"], tag)
                if r.get("ref") and os.path.isfile(r["ref"]):
                    os.remove(r["ref"])
                if r["klass"] == predicted:
                    hits += 1
                else:
                    miss.append((tag, predicted, r["klass"]))
                log("   RESULT %s   prediction %s"
                    % (r["klass"], "held" if r["klass"] == predicted else "BROKE"))

            log("")
            log("== CROSS %s: %d corners, %d matched the composition "
                "prediction, %d did not" % (corpus, len(corners), hits, len(miss)))
            log("   distinct stock images across the corners: %d" % len(crcs))
            for t, p, g in miss:
                log("   PREDICTION BROKE %s: predicted %s, ran %s" % (t, p, g))
            if miss:
                failures.append("%s: %d corner(s) broke the composition "
                                "prediction" % (corpus, len(miss)))
            if len(crcs) < 2:
                failures.append("%s: the cross product produced %d distinct "
                                "stock image(s); it measured nothing"
                                % (corpus, len(crcs)))
            all_cross.append({"corpus": corpus, "corners": len(corners),
                              "hits": hits, "miss": len(miss),
                              "distinct": len(crcs), "causes": dict(causes)})

    log("")
    log("=" * 78)
    log("RECONCILE legs launched=%d reported=%d" % (launched, reported))
    if launched != reported:
        failures.append("%d legs launched but %d reported; %d produced no row "
                        "at all" % (launched, reported, launched - reported))

    ack_keys = set(ACK_UNREADABLE)
    got_keys = set(unreadable_found)
    if ack_keys != got_keys and not a.only:
        failures.append("unreadable-domain acknowledgements are stale: "
                        "found-not-acknowledged=%s acknowledged-not-found=%s"
                        % (sorted(got_keys - ack_keys), sorted(ack_keys - got_keys)))
    log("RECONCILE unreadable-domain: found=%d acknowledged=%d %s"
        % (len(got_keys), len(ack_keys),
           "MATCH" if ack_keys == got_keys else "MISMATCH"))

    log("")
    log("%-42s %-30s %s" % ("LEG", "RESULT", "DETAIL"))
    for r in [x for x in all_rows if not x.get("cross")]:
        detail = ""
        if r["klass"] == "AGREE":
            detail = "crc32 %s size %d" % (r["sig_crc"], r["sig_size"])
        elif r["klass"] == "DIFFER":
            detail = "%d bytes in %d runs at %s" % (r["diff"], r["runs"],
                                                    " ".join(r["offsets"]))
        elif r["klass"] == "SIGIL-DECLINED":
            detail = (r["sigil_stderr"] or ["no message"])[0][:90]
        elif r["klass"] == "LEG-ERROR":
            detail = r["err"].split("\n")[0][:90]
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
    bad = {}
    for r in all_rows:
        if r["klass"] == "AGREE" or r.get("vacuous") or r.get("control") \
                or r.get("cross"):
            continue
        key = (r["corpus"], r["tag"])
        bad[key] = r["klass"]
    ackd = dict(ACK_DISAGREE)
    unack = {k: v for k, v in bad.items() if k not in ackd}
    stale = {k: v for k, v in ackd.items() if k not in bad}
    wrong = {k: (bad[k], ackd[k]) for k in bad if k in ackd and bad[k] != ackd[k]}
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
        log("  CLASS MISMATCH %s %s: ran %s, acknowledged %s" % (k[0], k[1], v[0], v[1]))
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
