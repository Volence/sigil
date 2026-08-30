#!/usr/bin/env python3
"""Report which lines of a lane's docs/decisions.jsonl the Dominion console can render.

Dominion's decision-card reader drops a line it cannot build a card from, and it drops
it silently as far as the lane is concerned: the owner sees fewer cards, not an error.
This tool is the lane's own view of that predicate, so a lane can know its number
without asking the console.

The predicate is a transcription of `parseDecisions`/`parseEntry` in Dominion's
`server/src/decisions.ts` at commit 796bc1e, including the timestamp rule it imports
from `contractTime.ts`. Reason strings are reproduced verbatim from that source so a
rejection here reads the same as the console's own.

COUNT BY LINE, NEVER BY ID. Contract rule 8c closes a decision by APPENDING a new entry
carrying the same question with `supersedes` set, so repeated ids are normal and a
report keyed on id silently collapses them. Every structure here is line-addressed.

Scope: accept/reject only. Dominion also attaches non-fatal notes to an accepted entry
(refs issues, detail type issues, prose lint flags, supersedes cycles); by that parser's
own stated asymmetry none of them can drop a line, so none are computed here.

Exit codes: 0 every line parses, 1 at least one line is rejected, 2 the file could not
be measured at all.
"""

from __future__ import annotations

import argparse
import calendar
import json
import re
import sys
from dataclasses import dataclass, field

# Dominion refuses to read a decisions file above this size, and every decision in it is
# unreadable until the lane trims it. That is an unmeasurable file, not a clean one.
MAX_BYTES = 1024 * 1024

AT_SHAPE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")

AT_HELP = (
    "expected the exact form that `date -u +%Y-%m-%dT%H:%M:%SZ` produces, "
    "e.g. 2026-08-23T09:14:00Z"
)


class Unmeasurable(Exception):
    """The file as a whole could not be read, so no count of any kind is honest."""


@dataclass
class LineResult:
    line: int
    id: str | None
    reasons: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.reasons


def is_obj(v: object) -> bool:
    """Dominion's isObj: an object that is neither null nor an array."""
    return isinstance(v, dict)


def str_field(v: object) -> str | None:
    """Dominion's str(): a string counts as content only once trimmed."""
    if isinstance(v, str) and v.strip():
        return v
    return None


def looks_like_a_timestamp(raw: str) -> bool:
    """Approximates `Number.isFinite(Date.parse(raw))` for the reason-wording split only.

    The two branches it chooses between are both rejections, so a divergence here can
    change how a bad line is described and never whether it parses. ISO-ish forms are
    covered, which is the case the source calls most likely; Date.parse also accepts
    legacy forms such as "Aug 24 2026" that land in the other branch here.
    """
    candidate = raw.strip()
    if candidate.endswith("Z"):
        candidate = candidate[:-1] + "+00:00"
    try:
        import datetime

        datetime.datetime.fromisoformat(candidate)
        return True
    except ValueError:
        return False


def parse_contract_at(raw: str, field_name: str) -> str | None:
    """Returns None when the timestamp is acceptable, else the rejection reason.

    Requiring the exact shape and then checking the calendar catches the case Date.parse
    waves through: February 30th parses and rolls forward to March 2nd, so a round trip
    is what exposes it. Field ranges are checked directly, which is the same verdict the
    round trip reaches and does not depend on a local library accepting second 60.
    """
    if AT_SHAPE.match(raw):
        year, month, day = int(raw[0:4]), int(raw[5:7]), int(raw[8:10])
        hour, minute, second = int(raw[11:13]), int(raw[14:16]), int(raw[17:19])
        real_date = (
            1 <= month <= 12
            and 1 <= day <= calendar.monthrange(year, month)[1]
            and hour <= 23
            and minute <= 59
            and second <= 59
        )
        if real_date:
            return None
        return f"{field_name} names a date that does not exist: {raw}"
    if looks_like_a_timestamp(raw):
        return f"{field_name} is not the shape this format requires: {raw} ({AT_HELP})"
    return f"{field_name} is not a parseable timestamp: {raw}"


def parse_options(raw: object) -> tuple[list[str] | None, str | None]:
    """Returns (option keys, error). Options are validated whole, never member by member.

    Dropping a bad member would tell the owner the lane offered fewer choices than it
    did, and could remove the very option the recommendation names.
    """
    if raw is None:
        return None, "options is missing"
    if not isinstance(raw, list):
        return None, "options must be an array"
    if len(raw) < 2:
        return None, "options must list two or more; a single option is not a decision"

    keys: list[str] = []
    for i, option in enumerate(raw):
        if not is_obj(option):
            return None, f"options[{i}] must be an object"
        for name in ("key", "name", "what", "costs"):
            if str_field(option.get(name)) is None:
                return None, f'options[{i}] is missing "{name}"'
        key = str_field(option.get("key"))
        assert key is not None
        # Duplicate keys make an answer ambiguous at the moment it is delivered, and the
        # delivered text is the one thing that cannot be taken back.
        if key in keys:
            return None, f'options[{i}] repeats the key "{key}"'
        keys.append(key)
    return keys, None


def parse_recommend(raw: object) -> tuple[str | None, str | None]:
    """Returns (recommended key, error)."""
    if raw is None:
        return None, "recommend is missing"
    if not is_obj(raw):
        return None, "recommend must be an object"
    key = str_field(raw.get("key"))
    because = str_field(raw.get("because"))
    if key is None:
        return None, 'recommend is missing "key"'
    # A recommendation is not a shrug: a bare pick with no reason is a lane declining to
    # do the work it is asking the owner to do.
    if because is None:
        return None, 'recommend is missing "because"'
    return key, None


def parse_entry(parsed: dict) -> LineResult:
    """Every applicable check runs rather than stopping at the first failure.

    The id is read before anything that can fail: a blocker pointing at a rejected entry
    is only distinguishable from a blocker pointing at nothing when the rejection still
    carries the id.
    """
    entry_id = str_field(parsed.get("id"))
    result = LineResult(line=0, id=entry_id)

    if entry_id is None:
        result.reasons.append("id is missing or empty")

    at_raw = str_field(parsed.get("at"))
    if at_raw is None:
        result.reasons.append("at is missing or empty")
    else:
        at_error = parse_contract_at(at_raw, "at")
        if at_error is not None:
            result.reasons.append(at_error)

    if str_field(parsed.get("question")) is None:
        result.reasons.append("question is missing or empty")

    option_keys, options_error = parse_options(parsed.get("options"))
    if options_error is not None:
        result.reasons.append(options_error)

    recommend_key, recommend_error = parse_recommend(parsed.get("recommend"))
    if recommend_error is not None:
        result.reasons.append(recommend_error)

    # Only checkable once both sides parsed. A recommendation pointing at nothing is
    # worse than no recommendation at all, because it looks like guidance.
    if option_keys is not None and recommend_key is not None:
        if recommend_key not in option_keys:
            result.reasons.append(
                f'recommend.key "{recommend_key}" names no option in this entry '
                f"(options are: {', '.join(option_keys)})"
            )

    return result


def parse_line(number: int, raw: str) -> LineResult:
    text = raw.strip()
    try:
        # JSON.parse rejects the NaN and Infinity literals that Python accepts by
        # default, so the constants are refused here to keep the two verdicts aligned.
        parsed = json.loads(text, parse_constant=_reject_constant)
    except ValueError as exc:
        return LineResult(line=number, id=None, reasons=[f"not valid JSON: {exc}"])
    if not is_obj(parsed):
        return LineResult(line=number, id=None, reasons=["not a JSON object"])
    result = parse_entry(parsed)
    result.line = number
    return result


def _reject_constant(name: str) -> object:
    raise ValueError(f"Unexpected token {name}")


def audit(path: str) -> list[LineResult]:
    """Reads one ledger and returns a result per non-blank line, in file order.

    Raises Unmeasurable when the file as a whole cannot be read, which the caller must
    surface as a failure. A file the reader refuses is not a file with zero problems.
    """
    try:
        with open(path, "rb") as handle:
            data = handle.read()
    except FileNotFoundError:
        raise Unmeasurable(f"no such file: {path}") from None
    except IsADirectoryError:
        raise Unmeasurable(f"not a regular file: {path}") from None
    except OSError as exc:
        raise Unmeasurable(f"cannot read {path}: {exc}") from None

    if len(data) > MAX_BYTES:
        raise Unmeasurable(
            f"too large: {len(data)} bytes, cap is {MAX_BYTES}; the console treats the "
            f"whole file as unreadable above the cap"
        )

    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise Unmeasurable(f"not valid UTF-8: {exc}") from None

    results: list[LineResult] = []
    for number, raw in enumerate(text.split("\n"), start=1):
        if not raw.strip():
            continue
        results.append(parse_line(number, raw))
    return results


def report(path: str, results: list[LineResult], out) -> None:
    parsing = [r for r in results if r.ok]
    rejected = [r for r in results if not r.ok]

    print(f"ledger: {path}", file=out)
    print(
        f"lines: {len(results)} total / {len(parsing)} parse / {len(rejected)} rejected",
        file=out,
    )

    histogram: dict[str, int] = {}
    for result in rejected:
        for reason in result.reasons:
            histogram[reason] = histogram.get(reason, 0) + 1

    if histogram:
        print("", file=out)
        print("reasons:", file=out)
        for reason, count in sorted(histogram.items(), key=lambda kv: (-kv[1], kv[0])):
            print(f"  {count:4d}  {reason}", file=out)

        print("", file=out)
        print("rejected lines:", file=out)
        for result in rejected:
            shown_id = result.id if result.id is not None else "(no id)"
            print(f"  line {result.line}  {shown_id}", file=out)
            for reason in result.reasons:
                print(f"           - {reason}", file=out)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Report which lines of a decisions ledger Dominion's card reader accepts."
    )
    parser.add_argument(
        "path",
        nargs="?",
        default="docs/decisions.jsonl",
        help="the ledger to audit (default: docs/decisions.jsonl)",
    )
    args = parser.parse_args(argv)

    try:
        results = audit(args.path)
    except Unmeasurable as exc:
        # An unmeasurable file gets its own exit code and its own words. Reporting it as
        # zero rejections would read as a clean ledger.
        print(f"UNMEASURABLE: {exc}", file=sys.stderr)
        print(f"ledger: {args.path}", file=sys.stderr)
        print("lines: not measured", file=sys.stderr)
        return 2

    report(args.path, results, sys.stdout)
    return 1 if any(not r.ok for r in results) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
