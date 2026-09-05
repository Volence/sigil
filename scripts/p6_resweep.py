#!/usr/bin/env python3
"""p6_resweep, re-measure how free the ROM packer is, at whatever layout a given
aeon tree currently has, for EVERY straddle subject rather than for `Art_Sonic`
alone.

WHY THIS EXISTS
---------------
`docs/superpowers/notes/2026-09-05-decouple-aeon-side-inventory.md` row P6 prices
the DECOUPLE precondition on one measurement, taken once, at a layout that has
since moved (the 2026-09-04 re-layout shifted the sound banks by 96 KB):

    sweeping Art_Sonic's base +/-64 KB, 2,773 of 131,073 one-byte positions
    FAIL, in 43 forbidden bands, worst peak 17 slots, current margin 5,188 B.

That figure is quoted in the note as the answer to "how free is the packer,
really". A number that is load-bearing for a go/no-go verdict and is a year of
layout churn out of date is not evidence, so this re-runs it.

WHAT IT DOES NOT DO
-------------------
It does not reimplement the measurement. Every constant, every subject, every
cost is aeon's `tools/dplc_straddle.py`, imported as a module from the tree under
test, this file only drives it over a range and reduces the result. Two
consequences worth stating:

  * A layout fact this reports is aeon's own tool's answer, not a second
    opinion about it. If the tool is wrong, this is wrong the same way.
  * The subject list is DERIVED, never typed here: it is whatever
    `dplc_straddle.SUBJECTS` names, cross-checked by the tool's own
    `subject_bindings()` against the `CharacterDef` literals and the appendage
    `equ` block. `--subjects` can narrow it for a quick run but the default is
    "all of them, whatever they are today".

THE FAST PATH, AND THE CONTROL OVER IT
--------------------------------------
`frame_costs` re-walks every frame of every entry per shift; 131,073 shifts x 4
subjects is minutes of interpreter. The straddle predicate is
`(src % B) + len > B`, so for a fixed entry it is a pure function of the shift
and vectorizes: this computes each entry's straddle mask over the whole shift
range with numpy and reduces to a per-shift peak.

That is a REWRITE OF THE PREDICATE, which is exactly the kind of speedup that
silently answers a slightly different question. So it is not trusted: `--control
N` re-computes N shifts (default 512: the two endpoints, zero, every band edge
found, and a seeded random sample) through `dplc_straddle.frame_costs` itself and
requires exact agreement on the peak-entry, peak-slot and reachable-split values.
A single disagreement is fatal and the run reports no numbers. The control is on
by default; `--control 0` turns it off and says so in the output.

WHAT "FAIL" MEANS HERE
----------------------
The same predicate P6's source doc used, which is `dplc_straddle`'s VERDICT A:

    peak SLOT cost over all frames  >  DMA_IMPORTANT_SLOTS - DPLC_ENTRY_RESERVE

Both constants are read from their defining `.emp` files by the tool. VERDICT B
(a REACHABLE frame splitting past the reserve) is reported alongside as a second
column, because it did not exist when P6's number was taken and a comparison
against P6 has to use P6's predicate. VERDICT C (the concurrent demand of all
resident sprite sets) is NOT swept: it is a property of every subject's base at
once, and this sweep moves one subject's base at a time. That is a stated limit,
not an omission.

USAGE
    scripts/p6_resweep.py --aeon-tree /path/to/detached/aeon/worktree
    scripts/p6_resweep.py --aeon-tree ... --range -65536:65536 --json out.json

The tree must already be built (both `s4.debug.lst` and `s4.debug.bin` present);
`scripts/provision-aeon-ref.sh <tree> <rev>` is how it gets that way.
"""

import argparse
import importlib.util
import json
import random
import statistics
import sys
import time
from pathlib import Path

import numpy as np


def load_tool(aeon_tree):
    """Import the tree's own dplc_straddle. Never a vendored copy: a stale copy
    of a tool whose whole job is deriving constants from source would report the
    old layout with total confidence."""
    p = Path(aeon_tree).resolve() / "tools" / "dplc_straddle.py"
    if not p.exists():
        raise SystemExit(f"p6_resweep: {p} does not exist — is --aeon-tree an aeon checkout?")
    spec = importlib.util.spec_from_file_location("dplc_straddle_under_test", p)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def straddle_mask(base, offset, length, boundary, shifts):
    """Boolean over `shifts`: does this entry cross a boundary at that shift?

    Mirrors dplc_straddle.straddles exactly, `(src % B) + len > B`, so touching
    a boundary is not crossing. The control re-derives this through the tool.
    """
    return ((base + offset + shifts) % boundary) + length > boundary


def sweep_subject(sub, tool, tile_size, boundary, reachable, lo, hi):
    """Per-shift peak entries / peak slots / worst reachable split, vectorized.

    Returns a dict of numpy arrays indexed by (shift - lo).
    """
    n = hi - lo + 1
    shifts = np.arange(lo, hi + 1, dtype=np.int64)
    base = sub["art_base"]

    peak_entries = 0
    peak_slots = np.zeros(n, dtype=np.int32)
    reach_split = np.zeros(n, dtype=np.int32)

    for fi, ents in enumerate(sub["frames"]):
        n_ent = len(ents)
        if n_ent == 0:
            continue
        peak_entries = max(peak_entries, n_ent)
        straddling = np.zeros(n, dtype=np.int32)
        for start, count in ents:
            straddling += straddle_mask(base, start * tile_size, count * tile_size,
                                        boundary, shifts)
        np.maximum(peak_slots, n_ent + straddling, out=peak_slots)
        if fi in reachable:
            np.maximum(reach_split, straddling, out=reach_split)

    return {"peak_entries": peak_entries, "peak_slots": peak_slots,
            "reach_split": reach_split, "shifts": shifts}


def control_check(sub, tool, tile_size, boundary, reachable, lo, hi, res, n_control, seed):
    """Re-run N shifts through the tool's own frame_costs and demand agreement.

    The sample is not purely random: it always includes the endpoints, zero and
    every band edge, because those are the positions a boundary-arithmetic bug
    would land on and a uniform sample is least likely to hit.
    """
    if n_control <= 0:
        return None, []
    ps = res["peak_slots"]
    edges = np.flatnonzero(np.diff(ps) != 0)
    picks = {0 - lo, hi - lo, min(max(-lo, 0), hi - lo)}
    for e in edges:
        picks.add(int(e))
        picks.add(int(e) + 1)
    rng = random.Random(seed)
    while len(picks) < n_control:
        picks.add(rng.randrange(0, hi - lo + 1))
    picks = sorted(i for i in picks if 0 <= i <= hi - lo)

    bad = []
    for i in picks:
        d = lo + i
        costs = tool.frame_costs(sub["frames"], sub["art_base"] + d, tile_size, boundary)
        want_slots = max(c[1] for c in costs)
        want_entries = max(c[0] for c in costs)
        in_range = [f for f in reachable if f < len(sub["frames"])]
        want_split = max((len(costs[f][2]) for f in in_range), default=0)
        if (want_slots != int(res["peak_slots"][i])
                or want_entries != res["peak_entries"]
                or want_split != int(res["reach_split"][i])):
            bad.append(f"shift {d}: fast path says slots={int(res['peak_slots'][i])} "
                       f"entries={res['peak_entries']} split={int(res['reach_split'][i])}, "
                       f"frame_costs says slots={want_slots} entries={want_entries} "
                       f"split={want_split}")
    return len(picks), bad


def runs_of(mask, lo):
    """Maximal contiguous runs of True, as (first_shift, last_shift)."""
    out = []
    idx = np.flatnonzero(mask)
    if idx.size == 0:
        return out
    breaks = np.flatnonzero(np.diff(idx) != 1)
    starts = np.concatenate(([idx[0]], idx[breaks + 1]))
    ends = np.concatenate((idx[breaks], [idx[-1]]))
    for a, b in zip(starts, ends):
        out.append((int(a) + lo, int(b) + lo))
    return out


def margin(mask, lo, hi):
    """Distance from shift 0 to the nearest failing shift, each direction.

    None means no failing shift exists in that direction WITHIN THE SWEPT RANGE,
    which is not the same as "there is none" and is reported as such.
    """
    zero = -lo
    if not (0 <= zero <= hi - lo):
        return None, None, None
    if mask[zero]:
        return 0, 0, True
    left = np.flatnonzero(mask[:zero])
    right = np.flatnonzero(mask[zero + 1:])
    earlier = int(zero - left[-1]) if left.size else None
    later = int(right[0] + 1) if right.size else None
    return earlier, later, False


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--aeon-tree", required=True,
                    help="a BUILT aeon checkout (needs s4.debug.lst + s4.debug.bin)")
    ap.add_argument("--lst", default="s4.debug.lst", help="listing, relative to --aeon-tree")
    ap.add_argument("--rom", help="ROM, relative to --aeon-tree (default: --lst with .bin)")
    ap.add_argument("--range", default="-65536:65536", help="sweep range LO:HI in bytes")
    ap.add_argument("--subjects", help="comma-separated art labels; default is every "
                                       "subject the tool knows about")
    ap.add_argument("--control", type=int, default=512,
                    help="shifts to re-check through the tool's own frame_costs (0 disables)")
    ap.add_argument("--seed", type=int, default=20260905)
    ap.add_argument("--json", help="write the full result here")
    a = ap.parse_args(argv)

    lo, hi = (int(x) for x in a.range.split(":"))
    tree = Path(a.aeon_tree).resolve()
    tool = load_tool(tree)

    t0 = time.time()
    try:
        tile_size = tool.const_from_emp("engine/system/constants.emp", "TILE_SIZE")
        slots = tool.const_from_emp("engine/system/constants.emp", "DMA_IMPORTANT_SLOTS")
        reserve = tool.const_from_emp("engine/objects/dplc.emp", "DPLC_ENTRY_RESERVE")
        boundary = tool.boundary_from_source()
        lst = str(tree / a.lst)
        labels = tool.lst_labels(lst)
        subs = tool.load_subjects(labels)
        rom = str(tree / a.rom) if a.rom else tool.default_rom_for(lst)
        reach = tool.reachable_sets(subs, tool.rom_bytes(rom), labels)
        ratchet, ratchet_prov = tool.ratchet_from_source(slots, reserve)
    except tool.Unmeasurable as e:
        print(f"p6_resweep: UNMEASURABLE — {e}", file=sys.stderr)
        return 2

    derived = [s[1] for s in tool.SUBJECTS]
    wanted = [x.strip() for x in a.subjects.split(",")] if a.subjects else derived
    unknown = [w for w in wanted if w not in derived]
    if unknown:
        print(f"p6_resweep: --subjects names no subject: {', '.join(unknown)}; "
              f"known: {', '.join(derived)}", file=sys.stderr)
        return 2

    n_shifts = hi - lo + 1
    print(f"p6_resweep [{lst}]")
    print(f"  aeon tree      : {tree}")
    print(f"  derived        : TILE_SIZE={tile_size}  DMA_IMPORTANT_SLOTS={slots}  "
          f"DPLC_ENTRY_RESERVE={reserve}  boundary=0x{boundary:X}")
    print(f"  FAIL predicate : peak SLOTS > {ratchet}  ({ratchet_prov})")
    print(f"  subjects       : {len(derived)} derived from dplc_straddle.SUBJECTS "
          f"+ subject_bindings(): {', '.join(derived)}")
    print(f"  sweep          : [{lo}, {hi}] B, {n_shifts} one-byte positions per subject")
    if a.control <= 0:
        print("  CONTROL        : DISABLED (--control 0), the fast path is unverified here")

    out = {"aeon_tree": str(tree), "lst": lst, "rom": rom, "range": [lo, hi],
           "n_shifts": n_shifts, "tile_size": tile_size, "slots": slots,
           "reserve": reserve, "boundary": boundary, "ratchet": ratchet,
           "ratchet_provenance": ratchet_prov, "subjects_derived": derived,
           "results": {}}

    widened = []
    for s in subs:
        if s["art_label"] not in wanted:
            continue
        r = reach[s["name"]]
        if r["undetermined"]:
            widened.extend(r["undetermined"])
        rf = {f for f in r["frames"] if f < len(s["frames"])}
        res = sweep_subject(s, tool, tile_size, boundary, rf, lo, hi)

        fail_a = res["peak_slots"] > ratchet
        fail_b = res["reach_split"] > reserve
        bands = runs_of(fail_a, lo)
        widths = [b - aa + 1 for aa, b in bands]
        earlier, later, at_zero = margin(fail_a, lo, hi)
        base_peak = int(res["peak_slots"][-lo]) if 0 <= -lo <= hi - lo else None
        base_split = int(res["reach_split"][-lo]) if 0 <= -lo <= hi - lo else None

        n_ctl, bad = control_check(s, tool, tile_size, boundary, rf, lo, hi,
                                   res, a.control, a.seed)
        if bad:
            print(f"\np6_resweep: CONTROL FAILED for {s['art_label']} — the fast path and "
                  f"dplc_straddle.frame_costs disagree on {len(bad)} of {n_ctl} shifts. "
                  f"No numbers are reported for this run.", file=sys.stderr)
            for b in bad[:10]:
                print(f"  ! {b}", file=sys.stderr)
            return 3

        print(f"\n  {s['name']}: {s['art_label']} 0x{s['art_base']:X} + {s['art_len']} B "
              f"= 0x{s['art_base'] + s['art_len']:X}  ({len(s['frames'])} frames, "
              f"{len(rf)} reachable)")
        print(f"    at the CURRENT base (shift 0): peak entries {res['peak_entries']}, "
              f"peak SLOTS {base_peak}, worst reachable split {base_split}")
        print(f"    VERDICT A (peak SLOTS > {ratchet}): "
              f"{int(fail_a.sum())} of {n_shifts} positions FAIL, in {len(bands)} band(s)")
        if widths:
            print(f"      band widths: min {min(widths)} B, median "
                  f"{int(statistics.median(widths))} B, max {max(widths)} B")
            print(f"      worst peak SLOTS anywhere in range: "
                  f"{int(res['peak_slots'].max())}")
        print(f"    VERDICT B (reachable split > {reserve}): "
              f"{int(fail_b.sum())} of {n_shifts} positions FAIL")
        if at_zero:
            print("    MARGIN: ZERO, the current base is itself inside a forbidden band")
        else:
            # BOTH conventions are printed because the difference is one byte and
            # P6's doc states its margins in the LAST-SAFE form without saying so.
            # Comparing a first-failing number against a last-safe one produces a
            # 2 B discrepancy in the round trip and invites the conclusion that
            # the two measurements disagree, which they do not.
            def _m(v):
                return "none in range" if v is None else f"first fail at {v} B (last safe {v - 1} B)"
            print(f"    MARGIN earlier (shrink upstream): {_m(earlier)}")
            print(f"    MARGIN later   (grow upstream):   {_m(later)}")
        near = [b for b in bands if b[1] < 0][-2:] + [b for b in bands if b[0] > 0][:2]
        if near:
            print(f"      nearest bands: "
                  f"{', '.join(f'[{x}, {y}]' for x, y in near)}")
        if n_ctl:
            print(f"    control: {n_ctl} shifts re-checked through frame_costs, all agree")

        out["results"][s["art_label"]] = {
            "name": s["name"], "art_base": s["art_base"], "art_len": s["art_len"],
            "frames": len(s["frames"]), "reachable_frames": len(rf),
            "peak_entries": res["peak_entries"],
            "base_peak_slots": base_peak, "base_reach_split": base_split,
            "fail_a": int(fail_a.sum()), "fail_b": int(fail_b.sum()),
            "bands": bands, "band_widths": widths,
            "worst_peak_slots": int(res["peak_slots"].max()),
            "margin_earlier": earlier, "margin_later": later, "at_zero": bool(at_zero),
            "control_shifts": n_ctl,
            "reachability_undetermined": list(dict.fromkeys(r["undetermined"])),
        }

    if widened:
        print(f"\np6_resweep: REACHABILITY WIDENED — {len(set(widened))} writer(s) could not "
              f"be classified, so the affected subjects were widened to ALL frames. The "
              f"reachable columns are an UPPER BOUND, never a narrowed set.")
        for why in dict.fromkeys(widened):
            print(f"  ! {why}")

    print(f"\n  positions swept: {n_shifts} per subject; positions UNMEASURABLE: 0 "
          f"(every shift is arithmetic over the loaded DPLC; a load failure raises "
          f"Unmeasurable and reports nothing at all)")
    print(f"  wall clock: {time.time() - t0:.1f} s")

    if a.json:
        Path(a.json).write_text(json.dumps(out, indent=2))
        print(f"  wrote {a.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
