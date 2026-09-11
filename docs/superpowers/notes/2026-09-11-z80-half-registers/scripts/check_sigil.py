#!/usr/bin/env python3
"""Run a sigil binary on every probe in a probe dir and compare with asl.

usage: check_sigil.py <probe-dir> <sigil-binary> <label>

asl's bytes are the concatenated byte columns of the probe's listing (every
source line, in order), read only from ASL_EXIT=0 runs with a complete
footer. sigil's bytes are its output image. Verdicts:
  MATCH            both accept, same bytes
  BOTH-REFUSE      both exit non-zero
  SIGIL-REFUSES    asl accepts, sigil refuses
  SIGIL-ACCEPTS    asl refuses, sigil accepts (the silent direction)
  DIFFER           both accept, different bytes
"""
import os
import re
import subprocess
import sys
import hashlib

d, binary, label = sys.argv[1], sys.argv[2], sys.argv[3]
md5 = hashlib.md5(open(binary, "rb").read()).hexdigest()
runlog = open(os.path.join(d, "_run.log")).read()
exits = dict(re.findall(r"== (\S+)\.asm ASL_EXIT=(\d+)", runlog))
counts = {}
lines = []
for name in sorted(exits):
    asm = os.path.join(d, name + ".asm")
    rc_asl = int(exits[name])
    asl_bytes = None
    if rc_asl == 0:
        txt = open(os.path.join(d, name + ".lst"), errors="replace").read()
        complete = bool(re.search(r"^ +\d+ passe?s?$", txt, re.M)) and \
            not re.search(r"^\s+Additional necessary passes", txt, re.M)
        if not complete:
            raise SystemExit(f"{name}: asl exit 0 without a complete footer")
        bs = []
        # A listing's byte column holds Z80 bytes (`FD 7D`) or 68000 words
        # (`303C 1234`); every even-length hex token is split into bytes.
        for m in re.finditer(r"^\s+\d+/\s*[0-9A-F]+ :((?: (?:[0-9A-F]{2})+(?= |\t|$))*)", txt, re.M):
            for tok in m.group(1).split():
                bs += [tok[k:k + 2] for k in range(0, len(tok), 2)]
        asl_bytes = " ".join(bs)
    out = os.path.join(d, f"{name}.{label}.bin")
    if os.path.exists(out):
        os.remove(out)
    r = subprocess.run([binary, name + ".asm", "-o", f"{name}.{label}.bin"], cwd=d,
                       capture_output=True, text=True)
    sig_bytes = None
    if r.returncode == 0 and os.path.exists(out):
        sig_bytes = " ".join(f"{b:02X}" for b in open(out, "rb").read())
    first_err = next((l for l in r.stderr.splitlines() if "error" in l), "")
    if rc_asl != 0 and r.returncode != 0:
        v = "BOTH-REFUSE"
    elif rc_asl != 0:
        v = "SIGIL-ACCEPTS"
    elif r.returncode != 0:
        v = "SIGIL-REFUSES"
    elif sig_bytes == asl_bytes:
        v = "MATCH"
    else:
        v = "DIFFER"
    counts[v] = counts.get(v, 0) + 1
    lines.append(f"{v}\t{name}\tasl={asl_bytes if asl_bytes is not None else 'REFUSED'}\t"
                 f"sigil={sig_bytes if sig_bytes is not None else 'REFUSED'}\t{first_err[:160]}")
report = os.path.join(os.path.dirname(os.path.abspath(d)), f"check-{os.path.basename(d)}-{label}.txt")
with open(report, "w") as fh:
    fh.write(f"# sigil {binary} md5 {md5}\n")
    fh.write("# " + " ".join(f"{k}={v}" for k, v in sorted(counts.items())) + "\n")
    fh.write("\n".join(lines) + "\n")
print(f"sigil md5 {md5}; " + " ".join(f"{k}={v}" for k, v in sorted(counts.items())) + f"; report {report}")
