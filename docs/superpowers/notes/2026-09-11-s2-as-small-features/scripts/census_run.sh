#!/usr/bin/env bash
# census_run.sh : MEASUREMENT SCAFFOLD. Apply patch_census.py to a CLEAN tree,
# build into a private target dir, run all three corpora with the census on,
# then restore eval.rs from the committed HEAD and require a clean tree.
S=/home/volence/sonic_hacks/.scratch/s2-as-small-features
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a35527f94a0279a97
F=crates/sigil-frontend-as/src/eval.rs
cd "$WT" || exit 9
echo "HEAD $(git -C "$WT" rev-parse HEAD)"
if [ -n "$(git -C "$WT" status --porcelain --untracked-files=no)" ]; then echo "TREE NOT CLEAN"; exit 7; fi
python3 "$S/patch_census.py" || exit 5
/usr/bin/grep -n 'SIGIL_ARGTEXT_CENSUS' "$WT/$F"
CARGO_TARGET_DIR="$S/target-census" cargo build --release --bin sigil 2>&1 | tail -1
cp "$S/target-census/release/sigil" "$S/bin/sigil-census"
git -C "$WT" show "HEAD:$F" > "$WT/$F"
echo "restored; tracked changes now: $(git -C "$WT" status --porcelain --untracked-files=no | wc -l)"
echo "census binary md5 $(md5sum < "$S/bin/sigil-census" | cut -c1-32)"
mkdir -p "$S/runs/census"
(cd "$S/s1" && SIGIL_ARGTEXT_CENSUS=1 "$S/bin/sigil-census" sonic.asm -o /dev/null > /dev/null 2> "$S/runs/census/s1.err")
(cd "$S/corpus-gen" && SIGIL_ARGTEXT_CENSUS=1 "$S/bin/sigil-census" s2.asm -o /dev/null > /dev/null 2> "$S/runs/census/s2.err")
(cd "$S/s3k" && SIGIL_ARGTEXT_CENSUS=1 "$S/bin/sigil-census" sonic3k.asm -o /dev/null > /dev/null 2> "$S/runs/census/s3k.err")
for c in s1 s2 s3k; do
  echo "== $c: census lines $(/usr/bin/grep -c '^SIGIL-ARGTEXT' "$S/runs/census/$c.err"), distinct $(/usr/bin/grep '^SIGIL-ARGTEXT' "$S/runs/census/$c.err" | sort -u | wc -l)"
done
echo CENSUS_END
