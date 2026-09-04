#!/bin/bash
# Class = the diagnostic message with the file:line prefix stripped, backticked
# identifiers and bare numbers normalized.
f="$1"
sed -E 's/^.*\([0-9]+\): (error|warning|note): /\1: /' "$f" \
  | sed -E 's/`[^`]*`/`X`/g; s/-?[0-9]+/N/g' \
  | sort | uniq -c | sort -rn
