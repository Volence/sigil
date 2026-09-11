#!/usr/bin/env python3
"""Copy this parcel's evidence from the scratch directory into the note's
evidence directory in the worktree, and write the digest list."""
import glob, hashlib, os, shutil, subprocess

S = "/home/volence/sonic_hacks/.scratch/s2-as-small-features"
E = "/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a35527f94a0279a97/docs/superpowers/notes/2026-09-11-s2-as-small-features"
for d in ("probes", "scripts", "logs", "logs/mutations", "rows"):
    os.makedirs(os.path.join(E, d), exist_ok=True)

for f in ("probe.sh", "gen_probes.py", "gen_probes2.py", "gen_probes3.py", "stub_h.py", "mutate.py",
          "mutproof.sh", "run_diag.sh", "run_s2_bytes.sh", "check_listings.py", "patch_census.py",
          "census_run.sh", "census_kinds.py", "suite_sum.py", "run_suite.sh", "copy_evidence.py"):
    shutil.copy(os.path.join(S, f), os.path.join(E, "scripts", f))
for f in glob.glob(os.path.join(S, "mut", "*")):
    shutil.copy(f, os.path.join(E, "logs", "mutations", os.path.basename(f)))
for f in ("probe-before.log", "probe-round2-before.log", "probe-round3-before.log", "probe-final.log",
          "check-final.txt", "s2-bytes-f7.log", "census.log", "stub-H.log"):
    shutil.copy(os.path.join(S, f), os.path.join(E, "logs", f))
for f in ("compare-f7-stubH.txt", "compare-f7-stubB.txt"):
    shutil.copy(os.path.join(S, "runs", f), os.path.join(E, "logs", f))
for c in ("s1", "s2", "s3k"):
    shutil.copy(os.path.join(S, "runs", "diag-%s-before" % c, "rows.sorted"), os.path.join(E, "rows", c + "-before.rows"))
    shutil.copy(os.path.join(S, "runs", "diag-%s-f7" % c, "rows.sorted"), os.path.join(E, "rows", c + "-final.rows"))
    for k in ("stdout",):
        shutil.copy(os.path.join(S, "runs", "diag-%s-before" % c, k), os.path.join(E, "rows", c + "-before." + k))
        shutil.copy(os.path.join(S, "runs", "diag-%s-f7" % c, k), os.path.join(E, "rows", c + "-final." + k))

with open(os.path.join(E, "logs", "census-kinds.txt"), "w") as out:
    subprocess.run(["python3", os.path.join(S, "census_kinds.py")] +
                   [os.path.join(S, "runs", "census", c + ".err") for c in ("s1", "s2", "s3k")],
                   stdout=out, check=True)
for f in ("f1", "f2b", "f3", "f4", "f5", "f6", "f7"):
    with open(os.path.join(E, "logs", "suite-%s.summary.txt" % f), "w") as out:
        subprocess.run(["python3", os.path.join(S, "suite_sum.py"), os.path.join(S, "suite-%s.log" % f)],
                       stdout=out, check=True)


def md5(p):
    return hashlib.md5(open(p, "rb").read()).hexdigest()


with open(os.path.join(E, "logs", "binary-md5s.txt"), "w") as out:
    for p in sorted(glob.glob(os.path.join(S, "bin", "sigil-*"))):
        out.write("%s  %s\n" % (md5(p), os.path.basename(p)))
    out.write("%s  asl (s1disasm/build_tools/Linux-x86_64/asl)\n" % md5("/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl"))
print(sum(len(fs) for _, _, fs in os.walk(E)), "files")
