#!/usr/bin/env python3
"""mutproof.py <id>... : red-first proofs for the half-register tests.

For each named mutation: read the committed baseline of every file it touches
(git show HEAD:<path>), apply each replacement (refused unless the old text
occurs exactly once in the file on disk), quote the mutated lines back FROM
DISK, run the named test binary, record pass/fail per test, then restore every
file from the committed baseline and require the tracked tree clean. Logs to
logs/mutations/<id>.log.
"""
import os
import re
import subprocess
import sys

W = "/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a735fcd77afe45035"
S = "/home/volence/sonic_hacks/.scratch/z80-half-registers"
EVAL = "crates/sigil-frontend-as/src/eval.rs"
STATE = "crates/sigil-frontend-as/src/state.rs"
HALF = ["-p", "sigil-frontend-as", "--test", "as_z80_half_registers"]
S2 = ["-p", "sigil-cli", "--test", "as_sonic2_whole_rom"]

MUT = {
    "m1-no-high-halves": ("ixl/iyl recognised, the high halves not", HALF, [
        (EVAL, '        "ixu" | "ixh" => Some((IndexReg::Ix, true)),\n', ""),
        (EVAL, '        "iyu" | "iyh" => Some((IndexReg::Iy, true)),\n', ""),
    ]),
    "m2-no-low-halves": ("the high halves recognised, ixl/iyl not", HALF, [
        (EVAL, '        "ixl" => Some((IndexReg::Ix, false)),\n', ""),
        (EVAL, '        "iyl" => Some((IndexReg::Iy, false)),\n', ""),
    ]),
    "m3-wrong-prefix": ("IY written, IX's prefix encoded", HALF, [
        (EVAL, "            IndexReg::Iy => 0xFD,\n        }))", "            IndexReg::Iy => 0xDD,\n        }))"),
    ]),
    "m4-h-l-mix-accepted": ("h and l accepted beside a half", HALF, [
        (EVAL, 'if atoms.iter().any(|a| reg_word(a, &["h", "l"])) {', 'if false && atoms.iter().any(|a| reg_word(a, &["h", "l"])) {'),
        (EVAL, 'const PLAIN: &[&str] = &["a", "b", "c", "d", "e"];', 'const PLAIN: &[&str] = &["a", "b", "c", "d", "e", "h", "l"];'),
    ]),
    "m5-ix-iy-mix-accepted": ("halves of IX and IY accepted together", HALF, [
        (EVAL, "halves.iter().find(|h| h.1 != reg)", "halves.iter().find(|h| false && h.1 != reg)"),
    ]),
    "m6-only-ld": ("only ld takes a half", HALF, [
        (EVAL, "            (Add | Adc | Sub | Sbc | And | Xor | Or | Cp, 2) => {\n"
               "                reg_word(&atoms[0], &[\"a\"]) && is_half(1)\n"
               "            }\n"
               "            (Sub | And | Xor | Or | Cp | Inc | Dec, 1) => is_half(0),\n", ""),
    ]),
    "m7-mode-ignored": ("the halves recognised under plain cpu z80 too", HALF, [
        (EVAL, "        let prefix = if self.state.z80_undoc {\n", "        let prefix = if true {\n"),
    ]),
    "m8-register-everywhere": ("the half is a register in every operand position", HALF, [
        (EVAL, "            Bit | Res | Set => 1,\n            _ => return Ok(None),\n", "            _ => 0, // mutation m8: every operand position\n"),
    ]),
    "m9-restore-loses-mode": ("restore does not bring the mode back", HALF, [
        (STATE, "        self.z80_undoc = s.z80_undoc;\n", ""),
    ]),
    "m10-momcpu-unchanged": ("MOMCPU still $80 under z80undoc", HALF, [
        (EVAL, "                Cpu::Z80 if self.state.z80_undoc => 0x80DC,\n", ""),
    ]),
    "m11-one-operand-add": ("one-operand add/adc/sbc accepted with a half", HALF, [
        (EVAL, "            (Sub | And | Xor | Or | Cp | Inc | Dec, 1) => is_half(0),",
               "            (Add | Adc | Sbc | Sub | And | Xor | Or | Cp | Inc | Dec, 1) => is_half(0),"),
    ]),
    "m12-feature-absent": ("no half registers at all", HALF, [
        (EVAL, "        let prefix = if self.state.z80_undoc {\n", "        let prefix = if false {\n"),
    ]),
    "m13-no-immediate": ("ld half,n refused", HALF, [
        (EVAL, "reg_word(&atoms[1], PLAIN) || matches!(atoms[1], OperandAtom::Value(_))", "reg_word(&atoms[1], PLAIN)"),
    ]),
    "s2a-wrong-prefix": ("Sonic 2 whole ROM: IY written, IX's prefix encoded", S2, [
        (EVAL, "            IndexReg::Iy => 0xFD,\n        }))", "            IndexReg::Iy => 0xDD,\n        }))"),
    ]),
    "s2b-feature-absent": ("Sonic 2 whole ROM: no half registers at all", S2, [
        (EVAL, "        let prefix = if self.state.z80_undoc {\n", "        let prefix = if false {\n"),
    ]),
}


def git(*args):
    return subprocess.run(["git", "-C", W, *args], capture_output=True, text=True)


def run(mid):
    what, test, edits = MUT[mid]
    os.makedirs(f"{S}/logs/mutations", exist_ok=True)
    log = open(f"{S}/logs/mutations/{mid}.log", "w")

    def out(s):
        print(s)
        log.write(s + "\n")
        log.flush()

    head = git("rev-parse", "HEAD").stdout.strip()
    out(f"== {mid}: {what}\nHEAD {head}")
    dirty = git("status", "--porcelain", "--untracked-files=no").stdout.strip()
    if dirty:
        out(f"REFUSED: tracked tree not clean before mutating:\n{dirty}")
        return False
    files = sorted({f for f, _, _ in edits})
    baseline = {f: git("show", f"HEAD:{f}").stdout for f in files}
    try:
        for f, old, new in edits:
            path = os.path.join(W, f)
            src = open(path).read()
            n = src.count(old)
            if n != 1:
                out(f"MUTATION REFUSED: old text occurs {n} times in {f}")
                return False
            src = src.replace(old, new)
            open(path, "w").write(src)
            back = open(path).read()
            if old in back:
                out(f"MUTATION DID NOT LAND in {f}")
                return False
            # Quote the mutated region from disk: the new text's lines, or for a
            # deletion, the lines around where the old text was.
            pos = back.find(new) if new else src.find(old[:0])
            if new:
                l0 = back[:pos].count("\n") + 1
                nl = max(new.count("\n") + (0 if new.endswith("\n") else 1), 1)
            else:
                base_src = open(path).read()
                # locate the deletion site in the baseline to derive the line
                bpos = baseline[f].find(old) if f in baseline else -1
                l0 = baseline[f][:bpos].count("\n") if bpos >= 0 else 1
                nl = 2
            lines = back.split("\n")
            out(f"APPLIED to {f}; read back from disk ({'deletion site' if not new else 'mutated lines'}):")
            for i in range(max(l0, 1), min(l0 + nl, len(lines)) + 1):
                out(f"  {f}:{i}  {lines[i - 1]}")
            out(f"  old text (now absent from the file): {old.strip()!r}")
        env = dict(os.environ, SIGIL_ALLOW_PARTIAL="1", CARGO_TARGET_DIR=f"{S}/target")
        r = subprocess.run(["cargo", "test", "--release", *test, "--no-fail-fast"], cwd=W, env=env,
                           capture_output=True, text=True)
        text = r.stdout + r.stderr
        for l in text.splitlines():
            if re.match(r"^test .* \.\.\. (ok|FAILED)$", l) or l.startswith("test result") \
                    or l.startswith("error: could not compile") or re.match(r"^error(\[E\d+\])?:", l):
                out(l)
        failed = re.findall(r"^test (\S+) \.\.\. FAILED$", text, re.M)
        compiled = "test result" in text
        out(f"cargo exit {r.returncode}; compiled={compiled}; FAILED tests: {len(failed)}")
        # the first panic message of each failed test, trimmed
        for m in re.finditer(r"panicked at [^\n]*\n([^\n]*)", text):
            out(f"  panic: {m.group(1)[:300]}")
        red = r.returncode != 0 and compiled and failed
        out("VERDICT: RED" if red else "VERDICT: NOT RED (the proof fails)")
        return bool(red)
    finally:
        for f in files:
            open(os.path.join(W, f), "w").write(baseline[f])
        dirty = git("status", "--porcelain", "--untracked-files=no").stdout.strip()
        out(f"restored from HEAD:{', HEAD:'.join(files)}; tracked tree {'CLEAN' if not dirty else 'DIRTY: ' + dirty}")
        log.close()


ids = sys.argv[1:] or list(MUT)
results = {mid: run(mid) for mid in ids}
print("SUMMARY " + " ".join(f"{k}={'RED' if v else 'NOT-RED'}" for k, v in results.items()))
print("MUTPROOF_END")
