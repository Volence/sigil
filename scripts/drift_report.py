#!/usr/bin/env python3
"""Classify and report aeon-reference drift observations.

This is the reporting half of `scripts/nightly_ref_drift.sh`, kept apart from the
shell so the state machine can be exercised without building a ROM.

THE ASYMMETRY THIS TOOL EXISTS TO ENCODE. The job's evidence does not accumulate
symmetrically, and leaving that to a reader's memory is how a quiet log turns into a
conclusion nobody drew:

  * A RED settles the question in a SINGLE observation. The byte-identity gate caught
    something a landing would have caught, so it is load-bearing. N is irrelevant to
    this state — one red at k=1 says exactly what one red at k=N says.
  * QUIET ACCUMULATES AND NEVER CONCLUDES. N quiet chains are "no drift was observed in
    N chains", which is not "there is none" and not "the gate is spent". This tool
    therefore reports `N reached, quiet` and `N reached, verdict` as DISTINCT states and
    REFUSES to render the first as the second. `FORBIDDEN_ON_QUIET` below is that
    refusal made checkable: no quiet rendering may contain any of those phrasings, and
    `selftest` proves it.

ANYTHING UNMEASURED IS NAMED. A shape that was not built, a record that does not exist,
a reader that could not answer — each is its own state with its own non-zero exit. None
of them is rendered as 0, green, or clean, and none of them advances N.

Verbs:
    observe   classify one build against the record and append it to the ledger
    report    render the accumulated ledger
    selftest  construct every state, including the quiet-at-N refusal, and check it

N is never defaulted. `--n` is required and has no fallback anywhere in this file.
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile

# Exit codes, in the precedence the job applies when several apply at once.
EXIT_QUIET = 0
EXIT_DRIFT = 1
EXIT_NOTHING_MEASURED = 2
EXIT_UNVERIFIED = 3

# Per-shape verdicts. The first four carry a real expectation from the record; the rest
# do not, and the distinction is what keeps N honest.
V_QUIET = "quiet"                                  # exact pair hit, bytes match
V_QUIET_SIGIL_MOVED = "quiet-sigil-moved"          # aeon rev known, assembler moved, bytes held
V_DRIFT_SAME_PAIR = "drift-same-pair"              # identical inputs, different bytes
V_DRIFT_SIGIL_MOVED = "drift-sigil-moved"          # assembler moved bytes under identical engine source
V_UNVERIFIED_AEON_MOVED = "unverified-aeon-moved"  # engine source the record has never seen
V_UNATTRIBUTABLE = "unattributable-both-moved"     # both coordinates moved; cause not derivable
V_RECORD_DISAGREES = "record-disagrees"            # the record itself holds two CRCs for one aeon rev
V_UNMEASURED = "unmeasured"                        # not built, or not covered by the record

DRIFT_VERDICTS = {V_DRIFT_SAME_PAIR, V_DRIFT_SIGIL_MOVED}
QUIET_VERDICTS = {V_QUIET, V_QUIET_SIGIL_MOVED}

# Observation states, worst first. A red dominates because it settles the question;
# an unmeasured shape outranks quiet because quiet must never absorb it.
S_DRIFT = "DRIFT"
S_NOTHING_MEASURED = "NOTHING MEASURED"
S_UNVERIFIED = "UNVERIFIED"
S_QUIET = "QUIET"
STATE_ORDER = [S_DRIFT, S_NOTHING_MEASURED, S_UNVERIFIED, S_QUIET]
STATE_EXIT = {
    S_DRIFT: EXIT_DRIFT,
    S_NOTHING_MEASURED: EXIT_NOTHING_MEASURED,
    S_UNVERIFIED: EXIT_UNVERIFIED,
    S_QUIET: EXIT_QUIET,
}

# THE REFUSAL, made checkable. No rendering of a quiet accumulation may contain any of
# these, whatever k and N are. They are the readings a person would otherwise take away
# from a page of clean rows, and every one of them is a conclusion this job cannot draw.
FORBIDDEN_ON_QUIET = [
    "gate was spent",
    "gate is spent",
    "the gate is spent",
    "safe to archive",
    "safe to retire",
    "no drift exists",
    "there is no drift",
    "byte identity is unnecessary",
    "verdict: spent",
    "VERDICT AVAILABLE",
]

REFUSAL = (
    "  THIS JOB CANNOT CONCLUDE FROM QUIET. Quiet is the absence of evidence and no\n"
    "  number of quiet chains becomes evidence: the reading it would support — that the\n"
    "  coupling has stopped catching anything — and its opposite — that the engine is\n"
    "  clean for reasons of its own — produce the same clean rows, which is why aeon's\n"
    "  own plan says neither lane can distinguish them from inside. Only a RED settles\n"
    "  this, and a red settles it in one observation.\n"
    "  WHAT THIS REPORTS: no drift was OBSERVED in the chains below.\n"
    "  WHAT IT DOES NOT REPORT: that there is none.\n"
)


# ── the record seam ──────────────────────────────────────────────────────────────
# Every expectation enters here and nowhere else. The protocol is in
# docs/DRIFT_RECORD_SEAM.md; the format behind it is the aeon lane's.

class ReaderUnavailable(Exception):
    """The reader could not answer. Never treated as `no drift`."""


def _reader(cmd, *args):
    """Run one reader verb. Returns (hit, lines); raises ReaderUnavailable on exit 2."""
    try:
        p = subprocess.run(
            cmd + list(args), capture_output=True, text=True, timeout=120
        )
    except (OSError, subprocess.SubprocessError) as e:
        raise ReaderUnavailable(f"`{' '.join(cmd + list(args))}` could not be run: {e}")
    if p.returncode == 3:
        return False, []
    if p.returncode != 0:
        raise ReaderUnavailable(
            f"`{' '.join(cmd + list(args))}` exited {p.returncode}: "
            f"{(p.stderr or p.stdout).strip()[:400]}"
        )
    return True, [ln for ln in p.stdout.splitlines() if ln.strip()]


def record_lookup_pair(cmd, aeon_rev, sigil_rev):
    hit, lines = _reader(cmd, "lookup", aeon_rev, sigil_rev)
    if not hit:
        return None
    out = {}
    for ln in lines:
        shape, crc, size = ln.split()
        out[shape] = (crc.lower(), int(size))
    return out


def record_lookup_aeon(cmd, aeon_rev):
    """Every entry recorded at this aeon revision, grouped by shape.

    Returns {shape: {crc/size: [sigil_rev, ...]}} so a record holding two different
    CRCs for one engine revision is visible as a disagreement rather than collapsed
    into whichever row happened to come first.
    """
    hit, lines = _reader(cmd, "lookup-aeon", aeon_rev)
    if not hit:
        return None
    out = {}
    for ln in lines:
        srev, shape, crc, size = ln.split()
        out.setdefault(shape, {}).setdefault((crc.lower(), int(size)), []).append(srev)
    return out


def record_has_sigil(cmd, sigil_rev):
    hit, _ = _reader(cmd, "has-sigil", sigil_rev)
    return hit


def record_shapes(cmd):
    _, lines = _reader(cmd, "shapes")
    return [ln.strip() for ln in lines]


# ── classification ───────────────────────────────────────────────────────────────

def classify(cmd, aeon_rev, sigil_rev, measured):
    """Classify each measured shape against the record.

    `measured` is {shape: (crc, size)} or {shape: None} for a shape that could not be
    built. Returns (shapes_dict, notes).
    """
    notes = []
    pair = record_lookup_pair(cmd, aeon_rev, sigil_rev)
    at_aeon = None if pair is not None else record_lookup_aeon(cmd, aeon_rev)
    sigil_known = None
    if pair is None and at_aeon is None:
        sigil_known = record_has_sigil(cmd, sigil_rev)

    covered = set(record_shapes(cmd))
    for shape in sorted(covered - set(measured)):
        notes.append(f"the record covers `{shape}` and this job did not build it")
    for shape in sorted(set(measured) - covered):
        notes.append(f"this job built `{shape}` and the record does not cover it")

    out = {}
    for shape, got in sorted(measured.items()):
        if got is None:
            out[shape] = {"verdict": V_UNMEASURED, "why": "the shape was not built"}
            continue
        crc, size = got[0].lower(), int(got[1])
        entry = {"crc": crc, "size": size}
        if shape not in covered:
            entry.update(verdict=V_UNMEASURED, why="the record does not cover this shape")
            out[shape] = entry
            continue
        if pair is not None:
            want = pair.get(shape)
            if want is None:
                entry.update(verdict=V_UNMEASURED,
                             why="the record's entry for this pair omits this shape")
            elif want == (crc, size):
                entry.update(verdict=V_QUIET, expected=f"{want[0]}/{want[1]}")
            else:
                entry.update(verdict=V_DRIFT_SAME_PAIR, expected=f"{want[0]}/{want[1]}",
                             why="identical revisions on both sides produced different "
                                 "bytes: nondeterminism or an environment leak")
            out[shape] = entry
            continue
        if at_aeon is not None:
            wants = at_aeon.get(shape)
            if not wants:
                entry.update(verdict=V_UNMEASURED,
                             why="the record's entries at this engine revision omit this shape")
            elif len(wants) > 1:
                entry.update(verdict=V_RECORD_DISAGREES,
                             expected="; ".join(f"{c}/{s} at {','.join(r)[:8]}"
                                                for (c, s), r in sorted(wants.items())),
                             why="the record holds more than one CRC for this engine "
                                 "revision, so it has already recorded an assembler-caused "
                                 "move; this job does not pick one")
            else:
                want = next(iter(wants))
                if want == (crc, size):
                    entry.update(verdict=V_QUIET_SIGIL_MOVED, expected=f"{want[0]}/{want[1]}")
                else:
                    entry.update(verdict=V_DRIFT_SIGIL_MOVED, expected=f"{want[0]}/{want[1]}",
                                 why="the assembler moved bytes under engine source the "
                                     "record already covers — the drift a landing would "
                                     "have caught")
            out[shape] = entry
            continue
        entry.update(
            verdict=V_UNVERIFIED_AEON_MOVED if sigil_known else V_UNATTRIBUTABLE,
            why=("the engine source is newer than anything the record covers, so no "
                 "expectation exists for it")
            if sigil_known else
            ("both coordinates moved past the record, so a difference here is not "
             "attributable to either side and this job does not pick one"),
        )
        out[shape] = entry
    return out, notes


def observation_state(shapes, tree_state):
    if not shapes:
        return S_NOTHING_MEASURED
    verdicts = {s["verdict"] for s in shapes.values()}
    if verdicts & DRIFT_VERDICTS:
        return S_DRIFT
    if V_UNMEASURED in verdicts:
        return S_NOTHING_MEASURED
    if verdicts & {V_UNVERIFIED_AEON_MOVED, V_UNATTRIBUTABLE, V_RECORD_DISAGREES}:
        return S_UNVERIFIED
    # A dirty tree makes the key non-identifying: the bytes correspond to no committed
    # revision, so a match against a record entry cannot be attributed to one either.
    if tree_state != "clean":
        return S_UNVERIFIED
    return S_QUIET


def worst(states):
    for s in STATE_ORDER:
        if s in states:
            return s
    return S_NOTHING_MEASURED


# ── the ledger ───────────────────────────────────────────────────────────────────

def read_ledger(path):
    """Returns (observations, malformed_line_numbers). Malformed lines are NAMED."""
    obs, bad = [], []
    if not os.path.exists(path):
        return obs, bad
    with open(path) as f:
        for i, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                obs.append(json.loads(line))
            except ValueError:
                bad.append(i)
    return obs, bad


def chain_key(o):
    """A CHAIN is a distinct revision pair, not a night.

    The provenance chain counts one entry per paired landing, which is the unit the N
    ruling was about. Keying on the closure revision rather than HEAD is deliberate:
    HEAD moves on commits no compilation can see, and counting those would mint chains
    that carry no evidence.
    """
    return (o.get("aeon_rev", "?"), o.get("sigil_closure_rev") or o.get("sigil_linked_rev", "?"))


# ── rendering ────────────────────────────────────────────────────────────────────

def render(obs, bad_lines, n, n_source, ledger_path):
    lines = []
    w = lines.append
    w("sigil <-> aeon byte-identity drift — nightly, NON-BLOCKING")
    w("")
    w(f"  ledger:       {ledger_path}")
    w(f"                {len(obs)} observation(s); MACHINE-LOCAL and outside every repo, so")
    w("                this evidence does not survive this machine")
    if bad_lines:
        w(f"  UNREADABLE:   {len(bad_lines)} ledger line(s) did not parse "
          f"(line {', '.join(str(b) for b in bad_lines[:10])}) — those observations are")
        w("                LOST, not clean")
    w(f"  N:            {n}  (from {n_source})")
    w("                provisional, ruled by the hub in the owner's place, overturnable")
    w("")

    chains = {}
    for o in obs:
        k = chain_key(o)
        chains.setdefault(k, []).append(o)

    quiet, quiet_evidence, red, unverified, unmeasured = [], [], [], [], []
    for k, group in chains.items():
        st = worst([o.get("state", S_NOTHING_MEASURED) for o in group])
        if st == S_DRIFT:
            red.append((k, group))
        elif st == S_NOTHING_MEASURED:
            unmeasured.append((k, group))
        elif st == S_UNVERIFIED:
            unverified.append((k, group))
        else:
            quiet.append((k, group))
            if any(s.get("verdict") == V_QUIET_SIGIL_MOVED
                   for o in group for s in o.get("shapes", {}).values()):
                quiet_evidence.append((k, group))

    w("CHAINS (distinct revision pairs — the unit N counts; a repeated night is not a chain)")
    # A count of reds is only a number when something was measured. With no chain
    # carrying an expectation, `red 0` is not a fact about the engine, and printing it
    # as one is the render-the-unmeasured-as-zero failure this job is built to refuse.
    measured_any = bool(quiet or red or unverified)
    zero = (lambda n: str(n)) if measured_any else (lambda n: "— nothing measured")
    w(f"  red, drift observed .................... {zero(len(red))}")
    w(f"  quiet ................................. {zero(len(quiet))}")
    w(f"  quiet AND evidence-bearing ............ {zero(len(quiet_evidence))}"
      "   (the assembler moved and the bytes did not)")
    w(f"  unverified ............................ {zero(len(unverified))}"
      "   (no expectation existed; N unmoved)")
    w(f"  NOTHING MEASURED ...................... {len(unmeasured)}"
      "   (named, never counted as clean)")
    w("")

    k_evidence = len(quiet_evidence)
    k_quiet = len(quiet)

    if red:
        w("STATUS: VERDICT AVAILABLE — DRIFT OBSERVED")
        w("  verdict: available")
        w(f"  {len(red)} chain(s) drifted. A red settles this question in a SINGLE")
        w("  observation and N IS IRRELEVANT TO IT: the byte-identity gate caught")
        w("  something a landing would have caught, which is the LOAD-BEARING reading.")
        w("  Step 4 archives that, and this job earns a promotion rather than a retirement.")
        for k, group in red:
            w(f"    aeon {k[0][:8]} / sigil {k[1][:8]}")
            for o in group:
                for shape, s in sorted(o.get("shapes", {}).items()):
                    if s.get("verdict") in DRIFT_VERDICTS:
                        w(f"      {shape}: got {s.get('crc')}/{s.get('size')} "
                          f"expected {s.get('expected')} [{s['verdict']}]")
                        if s.get("why"):
                            w(f"        {s['why']}")
        state = S_DRIFT
    elif not chains or (not quiet and not unverified):
        w("STATUS: NOTHING MEASURED")
        w("  verdict: none")
        w("  No chain has been measured against a real expectation. This is not a pass,")
        w("  not a zero and not green — it is the absence of a measurement.")
        if unmeasured:
            for k, group in unmeasured[-5:]:
                why = "; ".join(sorted({o.get("note", "") for o in group if o.get("note")}))
                w(f"    aeon {k[0][:8]} / sigil {k[1][:8]}: {why or 'nothing recorded'}")
        state = S_NOTHING_MEASURED
    elif k_evidence >= n:
        w("STATUS: N REACHED, QUIET — THIS IS NOT A VERDICT")
        w("  verdict: none")
        w(f"  {k_evidence} of {n} evidence-bearing chains carried a real expectation and none")
        w("  drifted.")
        w(REFUSAL.rstrip("\n"))
        state = S_QUIET
    elif k_quiet >= n:
        w("STATUS: N REACHED ON THE WEAK POPULATION ONLY — NOT A VERDICT")
        w("  verdict: none")
        w(f"  {k_quiet} of {n} chains are quiet, but only {k_evidence} of them are")
        w("  EVIDENCE-BEARING. A chain in which the assembler did not move says nothing")
        w("  about the assembler, so counting it toward N would answer step 4's question")
        w("  with chains that cannot address it.")
        w("  THE N RULING DID NOT SETTLE WHICH POPULATION IT COUNTS. This is the owner's")
        w("  to decide, and until he does the report shows both rather than picking.")
        w(REFUSAL.rstrip("\n"))
        state = S_QUIET
    else:
        w("STATUS: N NOT REACHED, QUIET — NO EVIDENCE YET")
        w("  verdict: none")
        w(f"  {k_evidence} of {n} evidence-bearing chains "
          f"({k_quiet} quiet in total). Quiet accumulates and never concludes.")
        w(REFUSAL.rstrip("\n"))
        state = S_UNVERIFIED if unverified and not quiet else S_QUIET

    if unmeasured and state != S_NOTHING_MEASURED:
        w("")
        w(f"  ALSO: {len(unmeasured)} chain(s) measured NOTHING and are excluded from every")
        w("  count above. They are not clean rows.")
    return "\n".join(lines), state


# ── verbs ────────────────────────────────────────────────────────────────────────

def cmd_observe(a):
    measured = {}
    for spec in a.shape:
        name, _, val = spec.partition("=")
        if not val or val == "unmeasured":
            measured[name] = None
        else:
            crc, _, size = val.partition("/")
            measured[name] = (crc, size)

    rec = {
        "observed_at": a.observed_at,
        "aeon_rev": a.aeon_rev,
        "sigil_linked_rev": a.sigil_linked_rev,
        "sigil_closure_rev": a.sigil_closure_rev,
        "sigil_tree_state": a.sigil_tree_state,
        "expectation_source": a.record_reader or "none",
    }

    if not a.record_reader:
        # NO RECORD. The measurements are still written down, tagged with an expectation
        # source of `none` so a later session cannot mistake this job's own past output
        # for an expectation — which is the interim oracle this project exists to refuse.
        rec["shapes"] = {
            k: {"crc": v[0], "size": int(v[1]), "verdict": V_UNMEASURED,
                "why": "no drift record is configured, so nothing declared what these "
                       "bytes should be"} if v else
               {"verdict": V_UNMEASURED, "why": "the shape was not built"}
            for k, v in measured.items()
        }
        rec["state"] = S_NOTHING_MEASURED
        rec["note"] = "no drift record is configured (DRIFT_RECORD_READER is empty)"
    else:
        cmd = a.record_reader.split()
        try:
            shapes, notes = classify(cmd, a.aeon_rev, a.sigil_closure_rev, measured)
            rec["shapes"] = shapes
            rec["state"] = observation_state(shapes, a.sigil_tree_state)
            if notes:
                rec["note"] = "; ".join(notes)
        except ReaderUnavailable as e:
            rec["shapes"] = {}
            rec["state"] = S_NOTHING_MEASURED
            rec["note"] = f"the drift record reader could not answer: {e}"

    os.makedirs(os.path.dirname(os.path.abspath(a.ledger)), exist_ok=True)
    with open(a.ledger, "a") as f:
        f.write(json.dumps(rec, sort_keys=True) + "\n")
    print(f"observation: {rec['state']}" + (f" — {rec['note']}" if rec.get("note") else ""))
    return STATE_EXIT[rec["state"]]


def cmd_report(a):
    obs, bad = read_ledger(a.ledger)
    text, state = render(obs, bad, a.n, a.n_source, a.ledger)
    print(text)
    return STATE_EXIT[state]


# ── selftest ─────────────────────────────────────────────────────────────────────

_CLOCK = [0]


def _obs(aeon, sigil, state, shapes=None, note=None):
    # Each fixture observation gets its OWN timestamp. Nights are what a naive counter
    # would count, so a fixture that shares one timestamp cannot tell a per-night
    # counter apart from a per-chain one.
    _CLOCK[0] += 1
    o = {"observed_at": f"2026-09-01T05:17:{_CLOCK[0]:02d}Z",
         "aeon_rev": aeon, "sigil_closure_rev": sigil, "sigil_linked_rev": sigil,
         "sigil_tree_state": "clean", "state": state, "shapes": shapes or {},
         "expectation_source": "selftest"}
    if note:
        o["note"] = note
    return o


def norm(text):
    """Collapse whitespace before scanning for a forbidden phrase.

    Line wrapping is not a defence. A rendering that says `the gate\\n  is spent`
    reads to a person exactly as the unwrapped phrase does, and a raw substring scan
    would clear it — an accidental pass that depends on where a line happened to
    break.
    """
    return " ".join(text.split())


def cmd_selftest(_a):
    failures = []
    try:
        return _selftest_body(failures)
    except Exception as e:  # noqa: BLE001 - a crash must be a NAMED failure
        print(f"  FAIL  {_CURRENT[0]}: raised {type(e).__name__}: {e}")
        print("")
        print(f"SELFTEST FAILED: execution STOPPED inside `{_CURRENT[0]}`, so every check "
              "after it is UNRUN, not passing")
        return 1


_CURRENT = [""]


def _selftest_body(failures):

    def block(name):
        # An exception inside one block must not truncate the report. A selftest that
        # stops at its first crash reports a tail, and a tail is where failures hide.
        _CURRENT[0] = name
        print(name)

    def check(name, cond, detail=""):
        if cond:
            print(f"  ok    {name}")
        else:
            print(f"  FAIL  {name}{': ' + detail if detail else ''}")
            failures.append(name)

    quiet_shape = {"s4": {"verdict": V_QUIET_SIGIL_MOVED, "crc": "aaaaaaaa", "size": 10}}
    drift_shape = {"s4": {"verdict": V_DRIFT_SIGIL_MOVED, "crc": "bbbbbbbb", "size": 10,
                          "expected": "aaaaaaaa/10", "why": "the assembler moved bytes"}}

    block("the phrase scan itself")
    # The matcher must be live before its silence means anything. A raw substring scan
    # clears a wrapped phrase, so this asserts the scan sees through a line break —
    # and that a forbidden phrase in a rendering is actually detected.
    check("the scan sees through line wrapping",
          "gate is spent" in norm("... the reading it would support (the gate\n  is spent) ..."))
    check("the scan detects a forbidden phrase at all",
          any(p in norm("STATUS: quiet. the gate was spent.") for p in FORBIDDEN_ON_QUIET))

    block("state: N reached with no reds")
    at_n = [_obs("a" * 40, f"{i}" * 40, S_QUIET, quiet_shape) for i in range(5)]
    text, state = render(at_n, [], 5, "selftest", "/dev/null")
    check("N-reached quiet is its own state", "N REACHED, QUIET — THIS IS NOT A VERDICT" in text)
    check("N-reached quiet declares no verdict", "verdict: none" in text)
    check("N-reached quiet carries the refusal", "THIS JOB CANNOT CONCLUDE FROM QUIET" in text)
    for phrase in FORBIDDEN_ON_QUIET:
        check(f"quiet never renders `{phrase}`", phrase not in norm(text))
    check("N-reached quiet does not exit red", STATE_EXIT[state] == EXIT_QUIET)

    block("state: one red, k far below N")
    text, state = render([_obs("a" * 40, "1" * 40, S_DRIFT, drift_shape)], [], 5,
                         "selftest", "/dev/null")
    check("a single red is a verdict", "VERDICT AVAILABLE" in text)
    check("a single red says N is irrelevant", "N IS IRRELEVANT TO IT" in text)
    check("a single red exits drift", STATE_EXIT[state] == EXIT_DRIFT)
    check("a single red names the shape and both CRCs",
          "s4: got bbbbbbbb/10 expected aaaaaaaa/10" in text)

    block("state: no record")
    text, state = render([_obs("a" * 40, "1" * 40, S_NOTHING_MEASURED, {},
                               "no drift record is configured")], [], 5,
                         "selftest", "/dev/null")
    check("absence renders as NOTHING MEASURED", "STATUS: NOTHING MEASURED" in text)
    check("absence is not a pass", "not a pass" in text and "not green" in text)
    check("absence exits non-zero", STATE_EXIT[state] == EXIT_NOTHING_MEASURED)
    check("absence draws no verdict", "verdict: none" in text)

    block("state: empty ledger")
    text, state = render([], [], 5, "selftest", "/dev/null")
    check("an empty ledger measures nothing", "STATUS: NOTHING MEASURED" in text)
    check("an empty ledger exits non-zero", STATE_EXIT[state] == EXIT_NOTHING_MEASURED)

    block("state: quiet below N")
    text, state = render([_obs("a" * 40, "1" * 40, S_QUIET, quiet_shape)], [], 5,
                         "selftest", "/dev/null")
    check("below N reads as no evidence yet", "N NOT REACHED, QUIET — NO EVIDENCE YET" in text)
    check("below N counts evidence-bearing chains", "1 of 5 evidence-bearing chains" in text)
    for phrase in FORBIDDEN_ON_QUIET:
        check(f"below-N quiet never renders `{phrase}`", phrase not in norm(text))

    block("state: N reached on the weak population only")
    weak = [_obs("a" * 40, f"{i}" * 40, S_QUIET,
                 {"s4": {"verdict": V_QUIET, "crc": "aaaaaaaa", "size": 10}})
            for i in range(5)]
    text, state = render(weak, [], 5, "selftest", "/dev/null")
    check("weak-population N is its own state", "WEAK POPULATION ONLY" in text)
    check("weak-population N draws no verdict", "verdict: none" in text)
    for phrase in FORBIDDEN_ON_QUIET:
        check(f"weak-population quiet never renders `{phrase}`", phrase not in norm(text))

    block("state: a repeated night is not a chain")
    same = [_obs("a" * 40, "1" * 40, S_QUIET, quiet_shape) for _ in range(5)]
    text, state = render(same, [], 5, "selftest", "/dev/null")
    check("five nights on one pair are one chain", "N NOT REACHED" in text)

    block("state: an unmeasured shape is never absorbed into quiet")
    mixed = {"s4": {"verdict": V_QUIET_SIGIL_MOVED, "crc": "aaaaaaaa", "size": 10},
             "demo": {"verdict": V_UNMEASURED, "why": "the shape was not built"}}
    check("mixed measurement is NOTHING MEASURED",
          observation_state(mixed, "clean") == S_NOTHING_MEASURED)
    check("a dirty tree cannot be quiet",
          observation_state(quiet_shape, "dirty") == S_UNVERIFIED)
    check("a red outranks an unmeasured shape",
          observation_state({**mixed, **drift_shape}, "clean") == S_DRIFT)

    block("state: unreadable ledger lines are named, not dropped silently")
    text, _ = render(at_n, [3, 7], 5, "selftest", "/dev/null")
    check("unreadable lines are named", "UNREADABLE:   2 ledger line(s) did not parse" in text)
    check("unreadable lines are not clean", "LOST, not clean" in text)

    block("state: the reader's own failure is not silence")
    with tempfile.TemporaryDirectory() as d:
        broken = os.path.join(d, "broken.sh")
        with open(broken, "w") as f:
            f.write("#!/bin/sh\necho 'the record is corrupt' >&2\nexit 2\n")
        os.chmod(broken, 0o755)
        try:
            record_shapes([broken])
            check("a reader exiting 2 raises", False, "no exception")
        except ReaderUnavailable as e:
            check("a reader exiting 2 raises", True)
            check("the raise names the reader's own words", "the record is corrupt" in str(e))
        try:
            record_shapes([os.path.join(d, "nonexistent")])
            check("a missing reader raises", False, "no exception")
        except ReaderUnavailable:
            check("a missing reader raises", True)

    block("the four cases, against a throwaway reader")
    # A TEST DOUBLE, written into a temp dir and deleted with it. It exists to prove
    # the classifier is live — a matcher whose silence has never been contradicted
    # proves nothing — and it is deliberately not a file anything could configure as
    # the real reader. The real expectations are the aeon lane's and enter only
    # through DRIFT_RECORD_READER.
    with tempfile.TemporaryDirectory() as d:
        AEON_KNOWN = "e" * 40
        SIGIL_KNOWN = "5" * 40
        reader = os.path.join(d, "reader.sh")
        with open(reader, "w") as f:
            f.write(
                "#!/bin/sh\n"
                "case \"$1\" in\n"
                "  shapes) echo s4; exit 0;;\n"
                f"  lookup) [ \"$2\" = {AEON_KNOWN} ] && [ \"$3\" = {SIGIL_KNOWN} ] "
                "&& { echo 's4 aaaaaaaa 100'; exit 0; }; exit 3;;\n"
                f"  lookup-aeon) [ \"$2\" = {AEON_KNOWN} ] "
                f"&& {{ echo '{SIGIL_KNOWN} s4 aaaaaaaa 100'; exit 0; }}; exit 3;;\n"
                f"  has-sigil) [ \"$2\" = {SIGIL_KNOWN} ] && exit 0; exit 3;;\n"
                "esac\nexit 2\n"
            )
        os.chmod(reader, 0o755)
        cmd = [reader]

        def verdict(aeon, sigil, crc):
            shapes, _ = classify(cmd, aeon, sigil, {"s4": (crc, 100)})
            return shapes["s4"]["verdict"]

        check("case 1: same pair, same bytes -> quiet",
              verdict(AEON_KNOWN, SIGIL_KNOWN, "aaaaaaaa") == V_QUIET)
        check("case 1: same pair, different bytes -> the unambiguous defect",
              verdict(AEON_KNOWN, SIGIL_KNOWN, "bbbbbbbb") == V_DRIFT_SAME_PAIR)
        check("case 2: sigil moved, bytes held -> quiet AND evidence-bearing",
              verdict(AEON_KNOWN, "9" * 40, "aaaaaaaa") == V_QUIET_SIGIL_MOVED)
        check("case 2: sigil moved, bytes moved -> the red step 4 needs",
              verdict(AEON_KNOWN, "9" * 40, "bbbbbbbb") == V_DRIFT_SIGIL_MOVED)
        check("case 3: aeon moved -> unverified, not a red and not quiet",
              verdict("f" * 40, SIGIL_KNOWN, "bbbbbbbb") == V_UNVERIFIED_AEON_MOVED)
        check("case 4: both moved -> unattributable, and the job does not pick",
              verdict("f" * 40, "9" * 40, "bbbbbbbb") == V_UNATTRIBUTABLE)
        shapes, _ = classify(cmd, "f" * 40, "9" * 40, {"s4": ("bbbbbbbb", 100)})
        check("case 4 says so in words", "not attributable" in shapes["s4"]["why"])
        check("case 3 and 4 never advance N",
              observation_state(shapes, "clean") == S_UNVERIFIED)
        shapes, notes = classify(cmd, AEON_KNOWN, SIGIL_KNOWN,
                                 {"s4": ("aaaaaaaa", 100), "demo": ("cccccccc", 50)})
        check("a shape the record does not cover is unmeasured, not quiet",
              shapes["demo"]["verdict"] == V_UNMEASURED)
        check("a shape outside the record is NAMED",
              any("does not cover" in n for n in notes))

    block("state: N has no default anywhere")
    p = subprocess.run([sys.executable, os.path.abspath(__file__), "report",
                        "--ledger", "/dev/null"], capture_output=True, text=True)
    check("report without --n is refused", p.returncode != 0)
    check("the refusal names --n", "--n" in (p.stderr + p.stdout))

    print("")
    if failures:
        print(f"SELFTEST FAILED: {len(failures)} check(s): {', '.join(failures)}")
        return 1
    print("SELFTEST PASSED")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="verb", required=True)

    o = sub.add_parser("observe")
    o.add_argument("--ledger", required=True)
    o.add_argument("--aeon-rev", required=True)
    o.add_argument("--sigil-linked-rev", required=True)
    o.add_argument("--sigil-closure-rev", required=True)
    o.add_argument("--sigil-tree-state", required=True)
    o.add_argument("--observed-at", required=True)
    o.add_argument("--record-reader", default="")
    o.add_argument("--shape", action="append", default=[],
                   help="<name>=<crc8>/<size>, or <name>=unmeasured")
    o.set_defaults(fn=cmd_observe)

    r = sub.add_parser("report")
    r.add_argument("--ledger", required=True)
    # NO DEFAULT. N is the owner's number and lives in scripts/drift-nightly.conf; a
    # fallback here would be a compiled-in N wearing a config file's clothes.
    r.add_argument("--n", type=int, required=True)
    r.add_argument("--n-source", default="unnamed source")
    r.set_defaults(fn=cmd_report)

    s = sub.add_parser("selftest")
    s.set_defaults(fn=cmd_selftest)

    a = ap.parse_args()
    return a.fn(a)


if __name__ == "__main__":
    sys.exit(main())
