#!/usr/bin/env bash
# mutrun.sh: apply each mutation of the SUBJECT on disk, show it, run the tests,
# report red/green with the first failure text, then restore from HEAD and prove
# the tree is clean again.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-adae49c7bddc9a0a8
export CARGO_TARGET_DIR=$S/target AEON_DIR=/home/volence/sonic_hacks/.aeon-bcd-pcindex
cd "$W" || exit 9
echo "pwd=$(pwd) HEAD=$(git rev-parse HEAD) branch=$(git branch --show-current)"
BASE=120be60971c58e43e712b285f411db2c605b388f

run_tests() {
  cargo test --release -p sigil-frontend-as --test as_bcd_pcindex --no-fail-fast "$@" > "$S/logs/mut-cur.log" 2>&1
  grep -E '^test result' "$S/logs/mut-cur.log"
  grep -E '^test .* FAILED$' "$S/logs/mut-cur.log" | sed 's/^/    RED /'
  grep -A3 -E '^---- ' "$S/logs/mut-cur.log" | grep -E 'panicked|got|want|left|right|refusal|expected' | head -4 | cut -c1-260 | sed 's/^/    TXT /'
}

restore() {
  for f in "$@"; do git show "HEAD:$f" > "$f"; done
  echo "  restored; diff stat after restore: [$(git diff --stat | tr '\n' ' ')]"
}

mutate() { # id file python-old python-new
  local id="$1" f="$2"
  python3 "$S/mut1.py" "$f" "$3" "$4" || { echo "$id: MUTATION DID NOT APPLY"; return 1; }
  echo "=== $id on $f"
  echo "  on disk: $(git diff --stat | tail -1)"
  git diff -U0 "$f" | grep -E '^[-+][^-+]' | head -4 | sed 's/^/  | /'
}

# M0: the implementation reverted to the base revision, the tests kept.
echo "=== M0: implementation files at base $BASE, test file kept"
for f in crates/sigil-frontend-as/src/eval.rs crates/sigil-isa/src/m68k.rs crates/sigil-isa/src/m68k_decode.rs crates/sigil-frontend-emp/src/flag_check.rs; do
  git show "$BASE:$f" > "$f"
done
echo "  on disk: $(git diff --stat | tail -1)"
run_tests
restore crates/sigil-frontend-as/src/eval.rs crates/sigil-isa/src/m68k.rs crates/sigil-isa/src/m68k_decode.rs crates/sigil-frontend-emp/src/flag_check.rs

F=crates/sigil-isa/src/m68k.rs
mutate M1 $F 'base | ((rx as u16) << 9) | (ss << 6) | (rm << 3) | (ry as u16)' 'base | ((ry as u16) << 9) | (ss << 6) | (rm << 3) | (rx as u16)' && { run_tests; restore $F; }
mutate M2 $F 'base | ((rx as u16) << 9) | (ss << 6) | (rm << 3) | (ry as u16)' 'base | ((rx as u16) << 9) | (ss << 6) | (ry as u16)' && { run_tests; restore $F; }
mutate M3 $F 'Mnemonic::Negx => 0x4000,' 'Mnemonic::Negx => 0x4400,' && { run_tests; restore $F; }
mutate M4 $F '        Mnemonic::Nbcd => {
            byte_only(inst)?;' '        Mnemonic::Nbcd => {' && { run_tests; restore $F; }
mutate M5 $F '        Mnemonic::Abcd => {
            byte_only(inst)?;
            (0xC100, 0)' '        Mnemonic::Abcd => {
            (0xC100, 0)' && { run_tests; restore $F; }

F=crates/sigil-frontend-as/src/eval.rs
mutate M6 $F 'M68kOperand::Pcd8Xn { d: 0, xn, long: *xlong },' 'M68kOperand::Pcd8Xn { d: 0, xn, long: false },' && { run_tests; restore $F; }
mutate M7 $F '            let frag = if indexed {
                self.m68k.lower_pcrel_idx_ea(&inst, target, span)' '            let frag = if !indexed {
                self.m68k.lower_pcrel_idx_ea(&inst, target, span)' && { run_tests; restore $F; }
mutate M8 $F '        Addx | Subx | Negx => Some(M68kSize::W),' '        Addx | Subx | Negx => Some(M68kSize::L),' && { run_tests; restore $F; }
mutate M9 $F '                vec![M68kOperand::RegList(mask), op]
            } else {
                vec![op, M68kOperand::RegList(mask)]' '                vec![op, M68kOperand::RegList(mask)]
            } else {
                vec![M68kOperand::RegList(mask), op]' && { run_tests; restore $F; }
mutate M10 $F '        "subx" => Subx,' '        "subx" => Addx,' && { run_tests; restore $F; }

F=crates/sigil-link/src/lib.rs
mutate M11 $F 'let disp = value - (site_vma as i64 - 1);' 'let disp = value - (site_vma as i64 - 3);' && { run_tests; restore $F; }

F=crates/sigil-isa/src/m68k_decode.rs
mutate M12 $F '        vec![Operand::PreDec(r0 as u8), Operand::PreDec(reg9 as u8)]' '        vec![Operand::PreDec(reg9 as u8), Operand::PreDec(r0 as u8)]' && {
  cargo test --release -p sigil-isa --no-fail-fast --lib --test m68k_capstone_differential --test m68k_opcode_sweep > "$S/logs/mut-cur.log" 2>&1
  grep -E '^test result' "$S/logs/mut-cur.log"; grep -E '^test .* FAILED$' "$S/logs/mut-cur.log" | sed 's/^/    RED /'
  grep -E 'panicked|disagreement|\[operands\]' "$S/logs/mut-cur.log" | head -3 | cut -c1-240 | sed 's/^/    TXT /'
  restore $F; }

echo "final diff stat: [$(git diff --stat | tr '\n' ' ')]"
echo MUT_END
