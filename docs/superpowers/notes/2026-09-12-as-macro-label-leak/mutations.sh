#!/usr/bin/env bash
# RED-FIRST for AS-MACRO-LABEL-LEAK. Each mutation removes ONE mechanism of the
# fix from the COMMITTED baseline, is shown landed on disk (`git diff`) before its
# run, runs the WHOLE sigil-frontend-as suite (not one file: a mutation reddening
# only some instruments says which one carries the cell), and is restored with
# `git checkout HEAD --` on a tree whose only dirt is that mutation, the restore
# verified empty before the next one starts.
#
# The patcher asserts its anchor matches EXACTLY ONCE and aborts the run
# otherwise: a patch that silently fails to apply runs the original code and
# prints green, which is indistinguishable from a mutation the tests miss.
#
#   ./mutations.sh <repo-root> <log-dir>
set -uo pipefail
ROOT="$1"; LOGS="$2"
mkdir -p "$LOGS"
cd "$ROOT" || exit 9
EVAL=crates/sigil-frontend-as/src/eval.rs
export CARGO_TARGET_DIR="$ROOT/target"
echo "MUTATIONS_START pwd=$(pwd) head=$(git rev-parse HEAD) branch=$(git branch --show-current)"
if [ -n "$(git status --porcelain)" ]; then
    echo "ABORT: tree is dirty before the first mutation"; git status --porcelain; exit 4
fi

patch() {  # $1 = anchor, $2 = replacement (python string literals, raw)
    python3 - "$EVAL" "$1" "$2" <<'PY'
import sys
path, anchor, repl = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
n = s.count(anchor)
if n != 1:
    print(f"ANCHOR_MISS count={n}: {anchor!r}")
    sys.exit(3)
open(path, "w").write(s.replace(anchor, repl))
PY
}

run_one() {
    local tag="$1" desc="$2"
    echo "=================================================================="
    echo "== $tag: $desc"
    echo "-- landed on disk:"
    git diff --stat
    git diff -U0 | /usr/bin/grep -E '^[-+][^-+]' | head -20
    cargo test --release -p sigil-frontend-as --no-fail-fast > "$LOGS/$tag.log" 2>&1
    echo "-- CARGO_EXIT=$?"
    /usr/bin/grep -E '^test result:' "$LOGS/$tag.log" | awk '{p+=$4; f+=$6} END {print "-- binaries="NR" passed="p" failed="f}'
    echo "-- red tests:"
    /usr/bin/grep -E '^test .* \.\.\. FAILED$' "$LOGS/$tag.log" | sed -E 's/^test (.*) \.\.\. FAILED$/   \1/' | sort -u
    /usr/bin/grep -E '^error(\[|:)' "$LOGS/$tag.log" | head -5
    git checkout HEAD -- crates/sigil-frontend-as/src
    if [ -n "$(git status --porcelain)" ]; then
        echo "ABORT: restore left dirt"; git status --porcelain; exit 5
    fi
    echo "-- restored, tree clean"
}

# ONLY="M12 M14" runs just those tags; unset runs every one.
selected() { [ -z "${ONLY:-}" ] || [[ " $ONLY " == *" $1 "* ]]; }

m() {  # tag desc anchor replacement
    local tag="$1" desc="$2"
    selected "$tag" || return 0
    patch "$3" "$4" || { echo "ABORT at $tag"; git checkout HEAD -- "$EVAL"; exit 3; }
    run_one "$tag" "$desc"
}

# M0: the whole fix, reverted to the parcel's base.
if selected M0; then
    git show 3bc0d81a:crates/sigil-frontend-as/src/eval.rs > "$EVAL"
    git show 3bc0d81a:crates/sigil-frontend-as/src/nameless.rs > crates/sigil-frontend-as/src/nameless.rs
    run_one M0 "eval.rs and nameless.rs at base 3bc0d81a (the whole fix reverted)"
fi

m M1 "file_in_innermost files nothing: every name global again" \
'    fn file_in_innermost(&mut self, name: &str) -> Option<String> {
' \
'    fn file_in_innermost(&mut self, name: &str) -> Option<String> {
        if std::hint::black_box(true) {
            let _ = name;
            return None;
        }
'

m M2 "nameless slots filed globally (labels and enums still filed)" \
'        let key = self.file_in_innermost(name).unwrap_or_else(|| name.to_string());' \
'        let key = name.to_string();'

m M3 "a plain label is filed only when the body scan claimed it (the old rule)" \
'            self.file_in_innermost(name).unwrap_or_else(|| name.to_string())
        };' \
'            if matches!(self.expansion_labels.last(), Some(e) if e.labels.contains(name)) {
                self.file_in_innermost(name).unwrap_or_else(|| name.to_string())
            } else {
                name.to_string()
            }
        };'

m M4 "enum members bound globally" \
'            let q = match self.file_in_innermost(&name) {' \
'            let q = match None::<String> {'

m M5 "include clears the namespace stack again" \
'                self.include_depth += 1;
                self.exec(&lines);
                self.include_depth -= 1;' \
'                let outer_labels = std::mem::take(&mut self.expansion_labels);
                self.include_depth += 1;
                self.exec(&lines);
                self.include_depth -= 1;
                self.expansion_labels = outer_labels;'

m M6 "no previous-pass owner index" \
'    asm.prev_owned = index_instance_owned(seed_env);' \
'    asm.prev_owned = { let _ = index_instance_owned(seed_env); Default::default() };'

m M7 "the reader ignores what the instance has written this pass" \
'                    || e.written.contains(name)' \
'                    || std::hint::black_box(false)'

# The raw patch is guarded too: applied without its run, it would never be
# restored and every later mutation would carry it.
if selected M6M7; then
    patch '    asm.prev_owned = index_instance_owned(seed_env);' \
          '    asm.prev_owned = { let _ = index_instance_owned(seed_env); Default::default() };' || exit 3
fi
m M6M7 "both: no previous-pass index AND the reader ignores written" \
'                    || e.written.contains(name)' \
'                    || std::hint::black_box(false)'

m M8 "owned_by_head disabled (Lp.x keyed globally)" \
'        if self.expansion_labels.is_empty() || q.starts_with('"' '"') {' \
'        if std::hint::black_box(true) || q.starts_with('"' '"') {'

m M9 "{GLOBALSYMBOLS} not recognised" \
'        let global_symbols = head_declares_option(&head.text, "globalsymbols");' \
'        let global_symbols = std::hint::black_box(false) && head_declares_option(&head.text, "globalsymbols");'

m M10 "class recording back to the depth rule (no front-end #1000 under GLOBALSYMBOLS)" \
'        if self.expansion_depth > 0 && !global_only {' \
'        if self.expansion_depth > 0 && std::hint::black_box(true) || std::hint::black_box(global_only) && false {'

m M11 "a GLOBALSYMBOLS frame restores the caller scope on exit" \
'        if global_symbols {
            // Transparent: whatever scope the body left is the caller' \
'        if global_symbols {
            self.scope = caller_scope.clone();
            // Transparent: whatever scope the body left is the caller'

m M12 "dot_scope has no transparent arm" \
'            Some(f) if f.transparent => Some(self.scope.as_deref().unwrap_or("")),' \
'            Some(f) if f.transparent && std::hint::black_box(false) => Some(self.scope.as_deref().unwrap_or("")),'

m M13 "real_scope ignores transparency" \
'        let s = if self.macro_frames.iter().all(|f| f.transparent) {' \
'        let s = if self.macro_frames.is_empty() {'

m M14 "outermost ignores transparency" \
'        let outermost = self.macro_frames.iter().all(|f| f.transparent);' \
'        let outermost = self.macro_frames.is_empty();'

m M15 "a GLOBALSYMBOLS frame still swaps in a private scope" \
'        if !global_symbols {
            if outermost {' \
'        if std::hint::black_box(true) {
            if outermost {'

echo "MUTATIONS_END_MARKER"
