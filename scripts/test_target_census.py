#!/usr/bin/env python3
"""HOW MANY TEST BINARIES A FULL `cargo test --workspace` SHOULD LAUNCH.

THE POPULATION THE LANDING VERDICT COULD NOT OTHERWISE GET. `landing-run.sh` pairs cargo's
`Running`/`Doc-tests` launch lines against the children's `test result:` lines, which
catches a binary that started and went quiet. It is silent about a target that STOPPED
BEING BUILT, because such a target neither launches nor reports: it is absent from both
halves of the pairing, and both halves live in the log. That is the reconciliation
message's own first named cause ("something stopped being built"), and answering it needs
a population from OUTSIDE the log.

`cargo metadata --no-deps` is that outside source. It reads the workspace manifests and
builds nothing, so it costs a second and cannot be affected by whatever the run did.

WHAT CARGO LAUNCHES, derived rather than copied from a nearby pin:

  * one binary per `test` target (`tests/*.rs` and any `[[test]]`),
  * one per `lib`/`proc-macro` target and one per `bin` target, for their `#[cfg(test)]`
    units, unless the target sets `test = false`,
  * one per `example`/`bench` target whose `test` flag comes back true,
  * one DOCTEST binary per `lib`/`proc-macro` target unless it sets `doctest = false`.

EVERY FLAG IS THE ONE CARGO RESOLVED, never a default assumed here. `cargo metadata`
reports `test` and `doctest` already resolved per target, so this does not need to know
that `test` defaults false for an example and true for a bench, and cannot drift when
cargo's defaults do.

Checked against this workspace on 2026-09-10: 413 test + 13 lib + 10 bin = 436 runnable
and 13 doctest, matching exactly the 436 `Running` and 13 `Doc-tests` lines in the three
most recent landing logs. Checked against real cargo on a fixture workspace (lib, bin, two
integration tests, an example, a bench): the census says 5 runnable + 1 doctest and
`cargo test` launches exactly those five `Running` lines and one `Doc-tests`.

TARGETS WITH `required-features` ARE EXCLUDED AND COUNTED SEPARATELY. Whether cargo builds
one depends on the feature set the run selected, which this script cannot know from
manifests alone. Counting them would make the census red on correct work the first time
somebody adds one, so they are reported as an UNCOUNTED remainder and the consumer is told
the figure is a lower bound by that many. There are none in this workspace today.

OUTPUT, one `KEY VALUE` line per fact, so a shell can read it without parsing JSON:

    runnable <n>
    doctest <n>
    excluded-required-features <n>

Exit 0 on a census, 2 if `cargo metadata` could not be run or read: NEVER a zero census,
because a zero that means "could not measure" is the defect this whole area exists to
close.
"""

import json
import subprocess
import sys

RUNNABLE_KINDS = {"test", "lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro", "bin"}
DOCTEST_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"}
# Whether `cargo test` launches one of these is decided by the target's own resolved
# `test` flag, not by its kind: false for an example by default, true for a bench.
FLAG_DECIDED_KINDS = {"example", "bench"}


def main(argv: list[str]) -> int:
    manifest = argv[1] if len(argv) > 1 else "Cargo.toml"
    cmd = ["cargo", "metadata", "--no-deps", "--format-version", "1", "--manifest-path", manifest]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    except OSError as exc:
        print(f"census: could not run cargo metadata: {exc}", file=sys.stderr)
        return 2
    if proc.returncode != 0:
        print(
            f"census: `{' '.join(cmd)}` exited {proc.returncode}:\n{proc.stderr.strip()}",
            file=sys.stderr,
        )
        return 2
    try:
        meta = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        print(f"census: cargo metadata did not return JSON: {exc}", file=sys.stderr)
        return 2

    runnable = doctest = excluded = 0
    for package in meta.get("packages", []):
        for target in package.get("targets", []):
            kinds = set(target.get("kind", []))
            if target.get("required-features"):
                # Only counted as excluded if it would otherwise have been counted.
                if kinds & RUNNABLE_KINDS or (kinds & FLAG_DECIDED_KINDS and target.get("test")):
                    excluded += 1
                continue
            if kinds & RUNNABLE_KINDS:
                if target.get("test", True):
                    runnable += 1
            elif kinds & FLAG_DECIDED_KINDS and target.get("test"):
                runnable += 1
            if kinds & DOCTEST_KINDS and target.get("doctest", True):
                doctest += 1

    print(f"runnable {runnable}")
    print(f"doctest {doctest}")
    print(f"excluded-required-features {excluded}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
