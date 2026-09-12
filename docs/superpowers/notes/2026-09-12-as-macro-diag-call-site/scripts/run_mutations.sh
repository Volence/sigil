#!/usr/bin/env bash
# run_mutations.sh : red-first proofs. Each mutation is applied to the COMMITTED
# tree, quoted back from disk, its test targets run, and the file restored with
# `git show HEAD:<path>`; the tracked tree must be clean before and after each.
S=/home/volence/sonic_hacks/.scratch/as-macro-diag
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a46ef34b369e42312
cd "$W" || exit 1
export CARGO_TARGET_DIR="$S/target"
unset AEON_DIR EMPYREAN_SUITE_ROOT SIGIL_STRICT_GATE
export SIGIL_ALLOW_PARTIAL=1
mkdir -p "$S/logs/mutations"
SPAN=crates/sigil-span/src/lib.rs
EVAL=crates/sigil-frontend-as/src/eval.rs
TRAIL="-p sigil-frontend-as --test as_macro_call_site_trail"
FAILED="-p sigil-frontend-as --test failed_run_reports_everything"
CLI="-p sigil-cli --test cli_diagnostic_location"
SPANT="-p sigil-span --lib"

mutate() {  # id file old new -- cargo target groups...
  local id="$1" file="$2" old="$3" new="$4"; shift 4
  local log="$S/logs/mutations/red-$id.log"
  {
    echo "== $id   HEAD $(git rev-parse HEAD)   branch $(git branch --show-current)"
    echo "tracked-changes before: $(git status --porcelain --untracked-files=no | wc -l)"
  } > "$log"
  if [ "$(git status --porcelain --untracked-files=no | wc -l)" != 0 ]; then
    echo "REFUSED: dirty tree" >> "$log"; echo "$id REFUSED dirty"; return
  fi
  if ! python3 "$S/mutate.py" "$file" "$old" "$new" >> "$log" 2>&1; then
    echo "$id MUTATION NOT APPLIED" | tee -a "$log"; return
  fi
  git diff --stat >> "$log"
  local group
  for group in "$@"; do
    echo "---- cargo test --release $group" >> "$log"
    # shellcheck disable=SC2086
    timeout 900 cargo test --release $group --no-fail-fast >> "$log" 2>&1
    echo "exit=$?" >> "$log"
  done
  git show "HEAD:$file" > "$file"
  echo "restored $file from HEAD; tracked-changes after: $(git status --porcelain --untracked-files=no | wc -l)" >> "$log"
  local red
  red=$(grep -E '^test .* FAILED$' "$log" | sed -E 's/^test (.*) \.\.\. FAILED$/\1/' | sort -u | tr '\n' ' ')
  local tot
  tot=$(awk '/^test result:/{p+=$4; f+=$6} END{print "passed " p ", failed " f}' "$log")
  echo "$id :: $tot :: RED: ${red:-<none>}" | tee -a "$log"
}

# m1: the feature absent: no run ever gets an id of its own.
mutate m1 "$EVAL" \
  "        let Some(first) = body.first() else {
            return;
        };
        let id = self.sources.add_expansion" \
  "        let Some(first) = body.first().filter(|_| false) else {
            return;
        };
        let id = self.sources.add_expansion" \
  "$TRAIL" "$FAILED" "$CLI"

# m2: loops not framed: `rept` runs its body without a run of its own.
mutate m2 "$EVAL" \
  "            self.enter_expansion(&mut body, entry, Frame::Rept(n));" \
  "            let _ = (entry, n);" \
  "$TRAIL"

# m3: a loop entered from its OPENING line instead of its closing line.
mutate m3 "$EVAL" \
  "        let mut body: Vec<SrcLine> = captured.unwrap_or_else(|| lines[start + 1..end].to_vec());
        let entry = loop_entry(&lines[end]);
        // A \`rept\` IS" \
  "        let mut body: Vec<SrcLine> = captured.unwrap_or_else(|| lines[start + 1..end].to_vec());
        let entry = loop_entry(&lines[start]);
        // A \`rept\` IS" \
  "$TRAIL"

# m4: every frame followed by a space, a loop frame included.
mutate m4 "$SPAN" \
  "        matches!(self, Frame::Macro(_))" \
  "        true" \
  "$SPANT" "$TRAIL"

# m5: only the innermost call named: the walk stops after one frame.
mutate m5 "$SPAN" \
  "            at = e.call;
        }" \
  "            at = e.call;
            break;
        }" \
  "$SPANT" "$TRAIL"

# m6: IRP named by the CURRENT item rather than the next.
mutate m6 "$EVAL" \
  "            let next = items.get(at + 1);" \
  "            let next = items.get(at);" \
  "$TRAIL"

# m7: WHILE spelled with parentheses like REPT.
mutate m7 "$SPAN" \
  "            Frame::While(n) => write!(out, \"WHILE {n}/{line}\")," \
  "            Frame::While(n) => write!(out, \"WHILE {n}({line})\")," \
  "$SPANT" "$TRAIL"

# m8: \"already reported here\" keyed on the run's own id, not the position.
mutate m8 "$EVAL" \
  "        let p = self.sources.physical(span);
        (p.source.0, p.start, p.end)" \
  "        let p = span;
        (p.source.0, p.start, p.end)" \
  "$TRAIL"

# m9: a carried warning matched to the returned report by raw span.
mutate m9 "$EVAL" \
  "        if diags.iter().any(|d| sources.physical(d.primary) == *physical) {" \
  "        if diags.iter().any(|d| d.primary == *span) {" \
  "$TRAIL"

# m10: the frame line counted from the FILE, not from the body's first line.
mutate m10 "$SPAN" \
  "            trail.push((&e.frame, line.saturating_sub(first) + 1));" \
  "            trail.push((&e.frame, line + 0 * first));" \
  "$SPANT" "$TRAIL"

echo "ALL MUTATIONS DONE; tracked-changes: $(git status --porcelain --untracked-files=no | wc -l)" | tee "$S/logs/mutations/END"
