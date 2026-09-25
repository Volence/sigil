#!/usr/bin/env bash
set -u
P=/home/volence/sonic_hacks/.scratch/as-cli-define/probes
. "$P/probe_lib.sh" || exit $?
run "empty middle part"            u2.asm -D FOO=1,,BAR=2
run "space before ="               u1.asm -D 'FOO =5'
run "trailing space in value"      u1.asm -D 'FOO=5 '
run "empty argument"               u1.asm -D ''
run "value 1+ (incomplete)"        u1.asm -D 'FOO=1+'
run "value 0FFh"                   u1.asm -D 'FOO=0FFh'
run "value ff (bare hex letters)"  u1.asm -D 'FOO=ff'
run "value 0b101"                  u1.asm -D 'FOO=0b101'
run "value @17 octal"              u1.asm -D 'FOO=@17'
run "value 1=1 comparison"         u1.asm -D 'FOO=1=1'
run "char value, run 3"            u1.asm -D "FOO='A'"
run "char value, run 4"            u1.asm -D "FOO='A'"
echo PROBE3_END
