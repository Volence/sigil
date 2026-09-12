#!/usr/bin/env bash
# run_mutations_review.sh : red-first for the review fix, on the COMMITTED tree.
S=/home/volence/sonic_hacks/.scratch/as-macro-diag
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a46ef34b369e42312
cd "$W" || exit 1
export CARGO_TARGET_DIR="$S/target"
mkdir -p "$S/logs/mutations"
SPAN=crates/sigil-span/src/lib.rs
DR=crates/sigil-harness/src/diag_render.rs
mutate() {  # id file old new -- cargo groups...
  local id="$1" file="$2" old="$3" new="$4"; shift 4
  local log="$S/logs/mutations/red-$id.log"
  {
    echo "== $id   HEAD $(git rev-parse HEAD)   branch $(git branch --show-current)"
    echo "tracked-changes before: $(git status --porcelain --untracked-files=no | wc -l)"
  } > "$log"
  [ "$(git status --porcelain --untracked-files=no | wc -l)" = 0 ] || { echo "$id REFUSED dirty" | tee -a "$log"; return; }
  python3 "$S/mutate.py" "$file" "$old" "$new" >> "$log" 2>&1 || { echo "$id NOT APPLIED" | tee -a "$log"; return; }
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
  local red tot
  red=$(grep -E '^test .* FAILED$' "$log" | sed -E 's/^test (.*) \.\.\. FAILED$/\1/' | sort -u | tr '\n' ' ')
  tot=$(awk '/^test result:/{p+=$4; f+=$6} END{print "passed " p ", failed " f}' "$log")
  echo "$id :: $tot :: RED: ${red:-<none>}" | tee -a "$log"
}
# r1: the file-count range check restored in the locator.
mutate r1 "$DR" \
  "        if !self.map.contains(span.source) {" \
  "        if span.source.0 as usize >= self.map.len() {" \
  "-p sigil-harness --lib diag_render"
# r2: `contains` that knows files only (the same trap, moved into the map).
mutate r2 "$SPAN" \
  "        (self.backing(id).0 as usize) < self.files.len()" \
  "        (id.0 as usize) < self.files.len()" \
  "-p sigil-span --lib" "-p sigil-harness --lib diag_render"
# r3: `contains` that answers yes for anything.
mutate r3 "$SPAN" \
  "        (self.backing(id).0 as usize) < self.files.len()" \
  "        let _ = id; true" \
  "-p sigil-span --lib" "-p sigil-harness --lib diag_render"
echo "REVIEW MUTATIONS DONE; tracked-changes: $(git status --porcelain --untracked-files=no | wc -l)" | tee "$S/logs/mutations/END-review"
