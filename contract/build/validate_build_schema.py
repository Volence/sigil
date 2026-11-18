#!/usr/bin/env python3
"""Gate for contract/schema/build/empyrean.build-1.schema.json (the build-data export, G0a).

Run it from anywhere:  python3 contract/schema/tests/validate_build_schema.py

  B1  The schema is well-formed against the draft it declares, and carries the $id the
      contract names. An ill-formed authority authorises nothing.
  B2  Every vector in build/vectors/pass/ validates. A pass set that is empty is a failure,
      never a vacuous green.
  B3  Every vector in build/vectors/red/ is REJECTED, and for the reason it declares. A red
      vector's `expect` names the JSON pointer of the instance that failed ("" is the root)
      and the schema keyword that failed. The set of errors must be exactly that one error:
      a red vector that fails for a different reason, or for its declared reason plus a
      stray one, proves nothing about the rule it was written for, so it is itself a
      failure. This is the idea validate_contract_schema.py states as its anti-vacuity rule,
      tightened from "is rejected" to "is rejected for this".

Exit 0 green, 1 red, 2 cannot run (missing jsonschema, schema or vectors): never green.
"""

import json
import os
import sys

try:
    from jsonschema import Draft202012Validator as Validator
except ImportError:  # pragma: no cover
    sys.stderr.write("FATAL: python package `jsonschema` is not installed, so nothing was checked.\n")
    sys.exit(2)

HERE = os.path.dirname(os.path.abspath(__file__))
SCHEMA_NAME = "empyrean.build-1.schema.json"
# Two layouts, one runner: in empyrean it sits in schema/tests/ beside schema/build/; vendored
# into a consumer it sits in the same directory as the schema and its vectors/ folder.
_LAYOUTS = (os.path.join(os.path.dirname(HERE), "build"), HERE)
BUILD = next((d for d in _LAYOUTS if os.path.isfile(os.path.join(d, SCHEMA_NAME))), _LAYOUTS[0])
SCHEMA_PATH = os.path.join(BUILD, SCHEMA_NAME)
PASS_DIR = os.path.join(BUILD, "vectors", "pass")
RED_DIR = os.path.join(BUILD, "vectors", "red")
SCHEMA_ID = "https://empyrean.invalid/contract/build/1"

failures = []


def fail(msg):
    failures.append(msg)
    print("  FAIL  " + msg)


def pointer(err):
    return "".join("/" + str(p) for p in err.absolute_path)


def vectors(directory):
    if not os.path.isdir(directory):
        sys.stderr.write("FATAL: %s is missing; nothing was checked.\n" % directory)
        sys.exit(2)
    names = sorted(n for n in os.listdir(directory) if n.endswith(".json"))
    if not names:
        sys.stderr.write("FATAL: %s holds no vectors; a gate with nothing to check is not green.\n" % directory)
        sys.exit(2)
    for n in names:
        with open(os.path.join(directory, n)) as fh:
            yield n, json.load(fh)


def main():
    if not os.path.isfile(SCHEMA_PATH):
        sys.stderr.write("FATAL: %s does not exist; nothing was checked.\n" % SCHEMA_PATH)
        return 2
    with open(SCHEMA_PATH) as fh:
        schema = json.load(fh)

    print("== B1  schema is well-formed against %s" % schema.get("$schema"))
    try:
        Validator.check_schema(schema)
    except Exception as exc:  # noqa: BLE001
        fail("empyrean.build-1.schema.json is not a valid schema: %s" % exc)
    if schema.get("$id") != SCHEMA_ID:
        fail("$id is %r, the contract names %r" % (schema.get("$id"), SCHEMA_ID))
    validator = Validator(schema)
    print("  ok")

    print("== B2  pass vectors")
    n_pass = 0
    for name, v in vectors(PASS_DIR):
        errs = sorted(validator.iter_errors(v["doc"]), key=lambda e: pointer(e))
        if errs:
            fail("pass/%s: expected to validate, got %s at %r" % (name, errs[0].message[:160], pointer(errs[0])))
        else:
            n_pass += 1
    print("  %d pass vectors validated" % n_pass)

    print("== B3  red vectors fail for their declared reason")
    n_red = 0
    for name, v in vectors(RED_DIR):
        exp = v.get("expect")
        if not (isinstance(exp, dict) and isinstance(exp.get("path"), str) and isinstance(exp.get("keyword"), str)):
            fail("red/%s: no `expect` {path, keyword}; a red vector must name its reason" % name)
            continue
        errs = list(validator.iter_errors(v["doc"]))
        if not errs:
            fail("red/%s: expected REJECTION, the schema accepted it (the rule is vacuous)" % name)
            continue
        got = sorted({(pointer(e), e.validator) for e in errs})
        want = [(exp["path"], exp["keyword"])]
        if got != want:
            fail("red/%s: failed for the wrong reason: wanted %s, got %s" % (name, want, got))
        else:
            n_red += 1
    print("  %d red vectors rejected for their declared reason" % n_red)

    print("")
    if failures:
        print("RED: %d failure(s)." % len(failures))
        return 1
    print("GREEN: schema well-formed ($id %s); %d pass vectors, %d red vectors." % (SCHEMA_ID, n_pass, n_red))
    return 0


if __name__ == "__main__":
    sys.exit(main())
