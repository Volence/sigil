#!/usr/bin/env bash
set -u
P=/home/volence/sonic_hacks/.scratch/as-cli-define/probes
. "$P/probe_lib.sh" || exit $?
run "name with a dot, used"        udot.asm -D FOO.BAR=1
run "name starting with dot, used" uldot.asm -D .FOO=1
run "underscore name, used"        uus.asm -D _FOO=1
run "name FOO?, used"              uq.asm -D 'FOO?=1'
run "name @FOO, used"              uat.asm -D @FOO=1
run "string value, run 1"          strA.asm -D 'FOO="A"'
run "string value, run 2"          strA.asm -D 'FOO="A"'
run "char value, run 2"            u1.asm -D "FOO='A'"
run "trailing comma"               u1.asm -D FOO=5,
run "leading comma"                u1.asm -D ,FOO=5
run "double equals"                u1.asm -D FOO==5
run "bare dollar"                  u1.asm -D 'FOO=$'
run "value ~1"                     u1.asm -D 'FOO=~1'
run "value 1<<4"                   u1.asm -D 'FOO=1<<4'
run "value 64-bit, high half"      uhi.asm -D 'FOO=$123456789'
run "value > 64 bits"              uhi.asm -D 'FOO=$10000000000000000'
run "value \$FFFFFFFFFFFFFFFF"     uhi.asm -D 'FOO=$FFFFFFFFFFFFFFFF'
run "bare then valued, same name"  u1.asm -D FOO -D FOO=2
run "valued then bare, same name"  u1.asm -D FOO=5 -D FOO
run "-D MOMCPU (a builtin)"        umom.asm -D MOMCPU=5
run "comma list three"             u2.asm -D FOO=1,BAR=2,BAZ=3
run "value with a space inside"    u1.asm -D 'FOO=1 + 2'
run "lowercase name via -D, upper in source" u1.asm -D foo=5
echo PROBE2_END
