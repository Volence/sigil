| probe | cpu | source | asl (exit 0 bytes, or refusal) | sigil base `1e146771` | sigil tip | tip vs asl |
|---|---|---|---|---|---|---|
| b1 | 68k | `dc.b 'A'` | 41 | 41 | 41 | same |
| b2 | 68k | `dc.b 'AB'` | 4142 | refused | 4142 | fixed |
| b2d | 68k | `dc.b "AB"` | 4142 | 4142 | 4142 | same |
| b5 | 68k | `dc.b 'ABCDE'` | 4142434445 | refused | 4142434445 | fixed |
| b9 | 68k | `dc.b 'ABCDEFGHI'` | 414243444546474849 | refused | 414243444546474849 | fixed |
| w1 | 68k | `dc.w 'A'` | 0041 | 0041 | 0041 | same |
| w2 | 68k | `dc.w 'AB'` | 4142 | 4142 | 4142 | same |
| w2d | 68k | `dc.w "AB"` | 00410042 | refused | 00410042 | fixed |
| w3 | 68k | `dc.w 'ABC'` | 004100420043 | refused | 004100420043 | fixed |
| w4 | 68k | `dc.w 'ABCD'` | 0041004200430044 | refused | 0041004200430044 | fixed |
| w5 | 68k | `dc.w 'ABCDE'` | 00410042004300440045 | refused | 00410042004300440045 | fixed |
| l1 | 68k | `dc.l 'A'` | 00000041 | 00000041 | 00000041 | same |
| l3 | 68k | `dc.l 'ABC'` | 00414243 | 00414243 | 00414243 | same |
| l4 | 68k | `dc.l 'ABCD'` | 41424344 | 41424344 | 41424344 | same |
| l4d | 68k | `dc.l "ABCD"` | 00000041000000420000004300000044 | refused | 00000041000000420000004300000044 | fixed |
| l5 | 68k | `dc.l 'ABCDE'` | 0000004100000042000000430000004400000045 | refused | 0000004100000042000000430000004400000045 | fixed |
| l9 | 68k | `dc.l 'ABCDEFGHI'` | 000000410000004200000043000000440000004500000046000000470000004800000049 | refused | 000000410000004200000043000000440000004500000046000000470000004800000049 | fixed |
| bp1 | 68k | `dc.b 'AB'+1` | 4143 | refused | 4143 | fixed |
| bp1d | 68k | `dc.b "AB"+1` | 4143 | 4143 | 4143 | same |
| bp100 | 68k | `dc.b 'AB'+$100` | 4242 | refused | 4242 | fixed |
| b1p | 68k | `dc.b 1+'AB'` | 4143 | refused | 4143 | fixed |
| bneg | 68k | `dc.b -'AB'` | refused #1320 | refused | refused | same |
| bpar | 68k | `dc.b ('AB')` | 4142 | refused | 4142 | fixed |
| bmul | 68k | `dc.b 'AB'*2` | refused #1320 | refused | refused | same |
| bsub | 68k | `dc.b 'AB'-1` | refused #1320 | refused | refused | same |
| bss | 68k | `dc.b 'AB'+'CD'` | 41424344 | refused | 41424344 | fixed |
| bssd | 68k | `dc.b "AB"+"CD"` | 41424344 | 41424344 | 41424344 | same |
| b1s | 68k | `dc.b 'A'+'B'` | 4142 | 83 | 4142 | fixed |
| bint | 68k | `dc.b $4143` | refused #1320 | refused | refused | same |
| wp1 | 68k | `dc.w 'AB'+1` | 00410043 | 4143 | 00410043 | fixed |
| wp1d | 68k | `dc.w "AB"+1` | 00410043 | refused | 00410043 | fixed |
| w3p1 | 68k | `dc.w 'ABC'+1` | 004100420044 | refused | 004100420044 | fixed |
| lp1 | 68k | `dc.l 'ABCD'+1` | 00000041000000420000004300000045 | 41424345 | 00000041000000420000004300000045 | fixed |
| b5p1 | 68k | `dc.b 'ABCDE'+1` | (nothing) | refused | refused | differs |
| imm4 | 68k | `move.l #'ABCD',d0` | 203c41424344 | 203c41424344 | 203c41424344 | same |
| imm5 | 68k | `move.l #'ABCDE',d0` | refused #1141 | refused | refused | same |
| imm8 | 68k | `move.l #'ABCDEFGH'>>32,d0` | 203c00000000 | 203c41424344 | refused | differs |
| immw | 68k | `move.w #'AB'+1,d0` | 303c4143 | 303c4143 | 303c4143 | same |
| equ2 | 68k | `X equ 'AB' / dc.w X` | 4142 | 4142 | 4142 | same |
| equ2b | 68k | `X equ 'AB' / dc.b X` | 4142 | refused | 4142 | fixed |
| equ2bd | 68k | `X equ "AB" / dc.b X` | 4142 | 4142 | 4142 | same |
| set2b | 68k | `X set 'AB' / dc.b X` | 4142 | refused | 4142 | fixed |
| equ5 | 68k | `X equ 'ABCDE' / dc.b X` | 4142434445 | refused | 4142434445 | fixed |
| if2 | 68k | `if 'AB'=$4142 / dc.b 1 / else / dc.b 2 / endif` | 01 | 01 | 01 | same |
| if2s | 68k | `if 'AB'="AB" / dc.b 1 / else / dc.b 2 / endif` | 01 | 01 | 01 | same |
| if5 | 68k | `if 'ABCDE'="ABCDE" / dc.b 1 / else / dc.b 2 / endif` | 01 | refused | 01 | fixed |
| be | 68k | `dc.b ''` | 00 | 00 | 00 | same |
| bed | 68k | `dc.b ""` | (nothing) | (nothing) | (nothing) | same |
| we | 68k | `dc.w ''` | 0000 | 0000 | 0000 | same |
| imme | 68k | `move.w #'',d0` | refused #1141 | 303c0000 | refused | fixed |
| bdq | 68k | `dc.b 'A"B'` | 412242 | refused | 412242 | fixed |
| bsq2 | 68k | `dc.b 'A''B'` | refused #1020 | refused | refused | same |
| besc | 68k | `dc.b 'A\x42C'` | 414243 | refused | 414243 | fixed |
| besq | 68k | `dc.b 'A\'B'` | 412742 | refused | 412742 | fixed |
| bescn | 68k | `dc.b 'A\nB'` | 410a42 | refused | 410a42 | fixed |
| wesc | 68k | `dc.w 'A\x42'` | 4142 | 4142 | 4142 | same |
| blist | 68k | `dc.b 'J',0,'UE'` | 4a005545 | refused | 4a005545 | fixed |
| bmix | 68k | `dc.b 'AB',"CD",'E'` | 4142434445 | refused | 4142434445 | fixed |
| mac | 68k | `m macro a / dc.b a / endm / m 'AB'` | 4142 | refused | 4142 | fixed |
| strlen | 68k | `dc.b strlen('ABC')` | 03 | refused | 03 | fixed |
| upstr | 68k | `dc.b upstring('abc')` | 414243 | refused | refused | differs |
| zdb1 | z80 | `db 'A'` | 41 | 41 | 41 | same |
| zdb2 | z80 | `db 'AB'` | 4142 | refused | 4142 | fixed |
| zdb5 | z80 | `db 'ABCDE'` | 4142434445 | refused | 4142434445 | fixed |
| zdbp | z80 | `db 'AB'+1` | 4143 | refused | 4143 | fixed |
| zdw2 | z80 | `dw 'AB'` | 4241 | 4241 | 4241 | same |
| zdw2d | z80 | `dw "AB"` | 41004200 | refused | 41004200 | fixed |
| zdw3 | z80 | `dw 'ABC'` | 410042004300 | refused | 410042004300 | fixed |
| zdwp | z80 | `dw 'AB'+1` | 41004300 | 4341 | 41004300 | fixed |
| zld | z80 | `ld hl,'AB'` | 214241 | 214241 | 214241 | same |
| zlda | z80 | `ld a,'A'` | 3e41 | 3e41 | 3e41 | same |
| zdefb | z80 | `defb 'AB'` | 4142 | refused | refused | differs |
| zdcb | z80 | `dc.b 'AB'` | refused #1200 | refused | 4142 | differs |
| pw1 | 68k | `dc.w ('AB')` | 4142 | 4142 | 4142 | same |
| pw2 | 68k | `dc.w ('ABC')` | 004100420043 | refused | 004100420043 | fixed |
| pw3 | 68k | `dc.w (('AB'))` | 4142 | 4142 | 4142 | same |
| sw1 | 68k | `X equ "AB" / dc.w X` | 00410042 | refused | 00410042 | fixed |
| sw2 | 68k | `X equ 'ABC' / dc.w X` | 004100420043 | refused | 004100420043 | fixed |
| sw3 | 68k | `X set 'AB' / dc.w X` | 4142 | 4142 | 4142 | same |
| sw4 | 68k | `X equ 'AB' / Y equ X / dc.w Y` | 4142 | 4142 | 4142 | same |
| sw5 | 68k | `X := 'AB' / dc.w X` | 4142 | 4142 | 4142 | same |
| sw6 | 68k | `X equ 'AB' / dc.l X` | 00004142 | 00004142 | 00004142 | same |
| sw7 | 68k | `X equ 'A' / dc.w X+1` | 0042 | 0042 | 0042 | same |
| fn1 | 68k | `dc.w substr('ABC',0,2)` | 00410042 | refused | 00410042 | fixed |
| fn2 | 68k | `dc.w lowstring('AB')` | 00610062 | refused | 00610062 | fixed |
| fn3 | 68k | `dc.w upstring('ab')` | 00410042 | refused | refused | differs |
| fn4 | 68k | `dc.w strlen('ABC')` | 0003 | refused | 0003 | fixed |
| cc1 | 68k | `dc.w 'A'+'B'` | 4142 | 0083 | 4142 | fixed |
| cc2 | 68k | `move.w #'A'+'B',d0` | 303c4142 | 303c0083 | 303c4142 | fixed |
| cc3 | 68k | `dc.l 'A'+'B'` | 00004142 | 00000083 | 00004142 | fixed |
| ar1 | 68k | `dc.b 'A'+$100` | 0141 | refused | 0141 | fixed |
| ar2 | 68k | `dc.b 'AB'+(0-$4142)` | (nothing) | 00 | (nothing) | fixed |
| ar3 | 68k | `dc.b 'A'+1` | 42 | 42 | 42 | same |
| ar4 | 68k | `dc.w 'A'+1` | 0042 | 0042 | 0042 | same |
| ar5 | 68k | `dc.w 'AB'-1` | 4141 | 4141 | 4141 | same |
| ar6 | 68k | `dc.l 'ABCD'-1` | 41424343 | 41424343 | 41424343 | same |
| ar7 | 68k | `dc.w 'A'*2` | 0082 | 0082 | 0082 | same |
| ar8 | 68k | `dc.w 'A'+$100` | 00010041 | 0141 | 00010041 | fixed |
| ar9 | 68k | `dc.w 'AB'+(1-1)` | 00410042 | 4142 | 00410042 | fixed |
| ar10 | 68k | `dc.w 'AB'+1-1` | 4142 | 4142 | 4142 | same |
| lw1 | 68k | `dc.l 'AB'` | 00004142 | 00004142 | 00004142 | same |
| lw2 | 68k | `dc.l ''` | 00000000 | 00000000 | 00000000 | same |
| lw3 | 68k | `dc.b '','A'` | 0041 | 0041 | 0041 | same |
| lw4 | 68k | `dc.b 'A',''` | 4100 | 4100 | 4100 | same |
| big1 | 68k | `dc.l 'ABCDE'>>0` | 00000005 | refused | refused | differs |
| big2 | 68k | `move.l #'ABCDE'\|0,d0` | 203c00000005 | refused | refused | differs |
| big3 | 68k | `move.l #"ABCDE"\|0,d0` | 203c00000005 | refused | refused | differs |
| big4 | 68k | `dc.l 'ABCDE'-1` | (nothing) | refused | refused | differs |
| big5 | 68k | `X equ 'ABCDE' / move.l #X,d0` | refused #1141 | refused | refused | same |
| big6 | 68k | `dc.b 'ABCDE'\|0` | 05 | refused | refused | differs |
| cs1 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.b 'ABC'` | 112299 | refused | 112299 | fixed |
| cs2 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.w 'AB'` | 1122 | 1122 | 1122 | same |
| cs3 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.w 'ABC'` | 001100220099 | refused | 001100220099 | fixed |
| cs4 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.l 'CAB'` | 00991122 | 00991122 | 00991122 | same |
| cs5 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / move.l #'CAB',d0` | 203c00991122 | 203c00991122 | 203c00991122 | same |
| cs6 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.b 'AB'+1` | 11 | refused | 11 | fixed |
| cs7 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.b 'CBA',0,'B'` | 9922110022 | refused | 9922110022 | fixed |
| cs8 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / X equ 'CA' / charset / dc.b X / dc.w X` | 43414341 | refused | 43414341 | fixed |
| cs9 | 68k | `X equ 'CA' / charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.b X / dc.w X` | 99119911 | refused | 99119911 | fixed |
| cs10 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.b 'A\x42C'` | 112299 | refused | 112299 | fixed |
| cs11 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.b ''` | 00 | 00 | 00 | same |
| cs12 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.w 'A'+'B'` | 1122 | 0033 | 1122 | fixed |
| cs13 | z80 | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / db 'CAB' / dw 'CA' / ld hl,'BC'` | refused #1020 | refused | refused | same |
| cs14 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.b 'AB'+'C'` | 112299 | refused | 112299 | fixed |
| q1 | 68k | `dc.b 'A;B'` | 413b42 | refused | 413b42 | fixed |
| q2 | 68k | `dc.b 'A,B',1` | 412c4201 | refused | 412c4201 | fixed |
| q3 | 68k | `dc.b '"'` | 22 | 22 | 22 | same |
| q4 | 68k | `dc.b "'"` | 27 | 27 | 27 | same |
| q5 | 68k | `dc.b '\''` | 27 | 27 | 27 | same |
| q6 | 68k | `dc.b 'A''` | refused #1020 | refused | refused | same |
| q7 | 68k | `dc.b 'AB` | refused #1020 | refused | refused | same |
| esc1 | 68k | `dc.b '\x41\x42'` | 4142 | refused | 4142 | fixed |
| esc2 | 68k | `dc.w '\x41\x42\x43'` | 004100420043 | refused | 004100420043 | fixed |
| esc3 | 68k | `dc.b '\q'` | refused #2010 | refused | refused | same |
| esc4 | 68k | `dc.b 'A\{1+1}B'` | 413242 | refused | 413242 | fixed |
| mw1 | 68k | `m macro a / dc.w a / endm / m 'AB'` | 4142 | 4142 | 4142 | same |
| mw2 | 68k | `m macro a / dc.b a / endm / m 'ABC'` | 414243 | refused | 414243 | fixed |
| irp1 | 68k | `irpc c,"AB" / dc.w 'c' / endm` | 00410042 | 00410042 | 00410042 | same |
| irp2 | 68k | `irpc c,"AB" / dc.b 'c' / endm` | 4142 | 4142 | 4142 | same |
| rep1 | 68k | `rept 2 / dc.b 'AB' / endm` | 41424142 | refused | 41424142 | fixed |
| dup1 | 68k | `dc.b [2]'AB'` | 41424142 | refused | 41424142 | fixed |
| dup2 | 68k | `dc.w [2]'AB'` | 41424142 | 41424142 | 41424142 | same |
| cmp1 | 68k | `if 'A'<'B' / dc.b 1 / else / dc.b 2 / endif` | 01 | 01 | 01 | same |
| cmp2 | 68k | `if 'AB'<>'AC' / dc.b 1 / else / dc.b 2 / endif` | 01 | 01 | 01 | same |
| cmp3 | 68k | `dc.b 'AB'='AB'` | 01 | 01 | 01 | same |
| z1 | z80 | `dw 'A'` | 4100 | 4100 | 4100 | same |
| z2 | z80 | `dw ''` | 0000 | 0000 | 0000 | same |
| z3 | z80 | `defw 'AB'` | 4241 | refused | refused | differs |
| z4 | z80 | `db ''` | 00 | 00 | 00 | same |
| z5 | z80 | `db 'AB',0,'C'` | 41420043 | refused | 41420043 | fixed |
| z6 | z80 | `ld a,'AB'` | refused #1320 | refused | refused | same |
| z7 | z80 | `ld hl,'ABC'` | refused #1320 | refused | refused | same |
| z8 | z80 | `dw 'AB'-1` | 4141 | 4141 | 4141 | same |
| z9 | z80 | `cp 'A'` | fe41 | fe41 | fe41 | same |
| dcbs | 68k | `dcb.b 2,'A'` | refused #1200 | refused | refused | same |
| dcbs2 | 68k | `dcb.w 2,'AB'` | refused #1200 | refused | refused | same |
| dcbs3 | 68k | `dcb.b 2,'AB'` | refused #1200 | refused | refused | same |
| lbl1 | 68k | `L: / dc.b 'A'+L` | 41 | 41 | 41 | same |
| lbl2 | 68k | `dc.w 'A'+L / L:` | 0043 | 0043 | 0043 | same |
| lbl3 | 68k | `dc.b 'A'+L / L:` | 42 | 42 | 42 | same |
| cp1 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / dc.b 'AB'+1` | 1123 | refused | 1123 | fixed |
| cp2 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.b 'AB'+2` | 1124 | refused | 1124 | fixed |
| cp3 | 68k | `charset $23,$77 / dc.b 'AB'+1` | 4143 | refused | 4143 | fixed |
| cp4 | 68k | `charset $43,$77 / dc.b 'AB'+1` | 41 | refused | 41 | fixed |
| cp5 | 68k | `charset 'A',$11 / dc.b 'AB'+1` | 1143 | refused | 1143 | fixed |
| cp6 | 68k | `charset 'B',$22 / dc.b 'AB'+1` | 4123 | refused | 4123 | fixed |
| cp7 | 68k | `charset 'A',$51 / dc.b 'AB'+1` | 5143 | refused | 5143 | fixed |
| cp8 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / dc.b "AB"+1` | 1123 | refused | 1123 | fixed |
| zcs1 | z80 | `charset 'A',11h / charset 'B',22h / charset 'C',99h / db 'CAB'` | 991122 | refused | 991122 | fixed |
| zcs2 | z80 | `charset 'A',11h / charset 'B',22h / charset 'C',99h / dw 'CA'` | 1199 | 1199 | 1199 | same |
| zcs3 | z80 | `charset 'A',11h / charset 'B',22h / charset 'C',99h / ld hl,'BC'` | 219922 | 219922 | 219922 | same |
| zcs4 | z80 | `charset 'A',11h / charset 'B',22h / charset 'C',99h / dw 'CAB'` | 99ff11002200 | refused | 99ff11002200 | fixed |
| ff1 | 68k | `dc.w 'A'+"B"` | 00410042 | refused | 00410042 | fixed |
| ff2 | 68k | `dc.w "A"+'B'` | 00410042 | refused | 00410042 | fixed |
| ff3 | 68k | `dc.w ''+'A'` | 0041 | 0041 | 0041 | same |
| ff4 | 68k | `dc.w 'AB'+''` | 4142 | 4142 | 4142 | same |
| ff5 | 68k | `dc.w 'ABC'+''` | 004100420043 | refused | 004100420043 | fixed |
| ff6 | 68k | `dc.w ('A'+'B')` | 4142 | 0083 | 4142 | fixed |
| ff7 | 68k | `X equ 'A'+'B' / dc.w X` | 4142 | 0083 | 4142 | fixed |
| ff8 | 68k | `X equ 'AB'+1 / dc.w X` | 00410043 | 4143 | 00410043 | fixed |
| ff9 | 68k | `dc.l 'AB'+'CD'` | 41424344 | 00008486 | 41424344 | fixed |
| ff10 | 68k | `dc.l 'AB'+'CDE'` | 0000004100000042000000430000004400000045 | 00438587 | 0000004100000042000000430000004400000045 | fixed |
| ff11 | 68k | `dc.w "A"+"B"` | 00410042 | refused | 00410042 | fixed |
| big7 | 68k | `move.l #'ABCDEF'\|0,d0` | 203c00000006 | refused | refused | differs |
| big8 | 68k | `move.l #'ABCDEFGHIJ'\|0,d0` | 203c0000000a | refused | refused | differs |
| big9 | 68k | `move.l #'XYZWVU'\|0,d0` | 203c00000006 | refused | refused | differs |
| zdb | z80 | `db 'A'+'B' / dw 'A'+'B'` | 41424241 | 838300 | 41424241 | fixed |
| mw3 | 68k | `m macro a / dc.w a+1 / endm / m 'AB'` | 00410043 | 4143 | 00410043 | fixed |
| subs | 68k | `X equ 'AB' / dc.w substr(X,0,2)` | 00410042 | refused | 00410042 | fixed |
| sym3 | 68k | `X set 'A' / X set X+'B' / dc.w X` | 00410042 | 0083 | 00410042 | fixed |
| h1 | 68k | `dc.w "\x99\x41"` | 00990041 | refused | 00990041 | fixed |
| h2 | 68k | `dc.l "\x99\x41"` | 0000009900000041 | refused | 0000009900000041 | fixed |
| h3 | z80 | `dw "\x99\x41"` | 99ff4100 | refused | 99ff4100 | fixed |
| h4 | z80 | `dw 'C\x99\x41'` | 430099ff4100 | refused | 430099ff4100 | fixed |
| h5 | 68k | `dc.w 'A\x99\x41'` | 004100990041 | refused | 004100990041 | fixed |
| h6 | z80 | `dw "\x7f\x80"` | 7f0080ff | refused | 7f0080ff | fixed |
| h7 | 68k | `dc.l 'A\x99\x41\x42\x43'` | 0000004100000099000000410000004200000043 | refused | 0000004100000099000000410000004200000043 | fixed |
| h8 | z80 | `dw 'AB'+1` | 41004300 | 4341 | 41004300 | fixed |
| h9 | z80 | `dw 'A'+80h` | c1ff | c100 | c1ff | fixed |
| h10 | 68k | `dc.w 'A'+$80` | 00c1 | 00c1 | 00c1 | same |
| h11 | z80 | `db 'AB'+80h` | 41c2 | refused | 41c2 | fixed |
| h12 | z80 | `charset 'A',99h / dw 'AB'+1` | 99ff4300 | 4399 | 99ff4300 | fixed |
| h13 | 68k | `dc.w "A",'BCD',$1234` | 00410042004300441234 | refused | 00410042004300441234 | fixed |
| h14 | 68k | `dc.b 'A'+$FF` | 0140 | refused | 0140 | fixed |
| h15 | 68k | `dc.w 'A'-$42+$80` | 007f | 007f | 007f | same |
| h16 | 68k | `dc.w 'AB'+(-$4000)` | 00010042 | 0142 | 00010042 | fixed |
| h17 | z80 | `dw 'AB'+(-4000h)` | 01004200 | 4201 | 01004200 | fixed |
| h18 | 68k | `dc.w [2]'ABC'` | 004100420043004100420043 | refused | 004100420043004100420043 | fixed |
| h19 | 68k | `dc.w ''+''` | 0000 | 0000 | 0000 | same |
| h20 | 68k | `dc.b ''+''` | 00 | 00 | 00 | same |
| h21 | 68k | `dc.b ''+'',1` | 0001 | 0001 | 0001 | same |
| f1 | 68k | `X equ 'A' / dc.w X+'B'` | 00410042 | 0083 | 00410042 | fixed |
| f2 | 68k | `X equ 'B' / dc.w 'A'+X` | 00410042 | 0083 | 00410042 | fixed |
| f3 | 68k | `X equ 'A' / Y equ 'B' / dc.w X+Y` | 00410042 | 0083 | 00410042 | fixed |
| f4 | 68k | `X equ 'AB' / dc.w (X)` | 4142 | 4142 | 4142 | same |
| f5 | 68k | `X equ 'A' / Y equ X+'B' / dc.w Y` | 00410042 | 0083 | 00410042 | fixed |
| f6 | 68k | `dc.w ('A')+'B'` | 00410042 | 0083 | 00410042 | fixed |
| f7 | 68k | `dc.w 'A'+('B')` | 00410042 | 0083 | 00410042 | fixed |
| f8 | 68k | `dc.l 'A'+'B'+'C'` | 00414243 | 000000c6 | 00414243 | fixed |
| f9 | 68k | `dc.l ('A'+'B')+'C'` | 000000410000004200000043 | 000000c6 | 000000410000004200000043 | fixed |
| f10 | 68k | `dc.w substr('AB',0,2)+''` | 00410042 | refused | 00410042 | fixed |
| f11 | 68k | `X equ 'AB' / dc.w X / dc.w X+''` | 414200410042 | 41424142 | 414200410042 | fixed |
| f12 | 68k | `X set 'A' / dc.w X / dc.w X+'B'` | 004100410042 | 00410083 | 004100410042 | fixed |
| f13 | 68k | `X equ 'AB' / dc.w X+0` | 00410042 | 4142 | 00410042 | fixed |
| f14 | 68k | `dc.w 'AB'+0` | 00410042 | 4142 | 00410042 | fixed |
| f15 | 68k | `X equ "A" / dc.w X+'B'` | 00410042 | refused | 00410042 | fixed |
| f16 | 68k | `dc.w (('A')+('B'))` | 00410042 | 0083 | 00410042 | fixed |
| t1 | 68k | `dc.l 'A'+substr("BC",0,1)+'D'` | 00414244 | refused | 00414244 | fixed |
| t2 | 68k | `dc.l 'A'+"B"+'C'` | 00414243 | refused | 00414243 | fixed |
| t3 | 68k | `X equ 'B' / dc.l 'A'+X+'C'` | 00414243 | 000000c6 | 00414243 | fixed |
| t4 | 68k | `X equ 'A'+"B"+'C' / dc.l X` | 00414243 | refused | 00414243 | fixed |
| t5 | 68k | `X equ ('AB') / dc.w X` | 4142 | 4142 | 4142 | same |
| t6 | 68k | `X equ 'AB' / Y equ (X) / dc.w Y` | 4142 | 4142 | 4142 | same |
| t7 | 68k | `X equ 'A' / dc.w 'B'+X` | 00420041 | 0083 | 00420041 | fixed |
| t8 | 68k | `dc.l 'A'+1+'C'` | 00004243 | 00000085 | 00004243 | fixed |
| t9 | 68k | `dc.l 'A'+('B')+'C'` | 00414243 | 000000c6 | 00414243 | fixed |
| t10 | 68k | `dc.l ''+'A'+''` | 00000041 | 00000041 | 00000041 | same |
| t11 | 68k | `dc.w 'A' + 'B'` | 4142 | 0083 | 4142 | fixed |
| t12 | 68k | `dc.w  'AB'` | 4142 | 4142 | 4142 | same |
| t13 | 68k | `dc.w ('A'+'B')` | 4142 | 0083 | 4142 | fixed |
| t14 | 68k | `dc.w ('A')` | 0041 | 0041 | 0041 | same |
| t15 | 68k | `dc.w (('A')+'B')` | 00410042 | 0083 | 00410042 | fixed |
| t16 | 68k | `dc.w ('A'+('B'))` | 00410042 | 0083 | 00410042 | fixed |
| t17 | 68k | `dc.l lowstring('AB')+'C'` | 000000610000006200000043 | refused | 000000610000006200000043 | fixed |
| t18 | 68k | `dc.l 'A'+lowstring('BC')` | 000000410000006200000063 | refused | 000000410000006200000063 | fixed |
| t19 | 68k | `dc.l 'A'+lowstring('b')+'C'` | 00416243 | refused | 00416243 | fixed |
| cq1 | 68k | `charset 'B',$77 / charset 'A','B' / dc.b "A"` | 77 | 77 | 77 | same |
| cq2 | 68k | `charset $41,'BCD' / dc.b "ABC"` | refused #1320 | refused | refused | same |
| cq3 | 68k | `charset 'A',$99 / charset 'a','c','A' / dc.b "abc"` | 999a9b | 999a9b | 999a9b | same |
| cq4 | 68k | `charset 'C',$55 / charset 'A','BC' / dc.b "AB"` | refused #1320 | refused | refused | same |
| cq5 | 68k | `charset 'B',$77 / charset 'A',('B') / dc.b "A"` | 77 | 77 | 77 | same |
| cq6 | 68k | `charset 'B',$77 / charset 'A','B'+0 / dc.b "A"` | 42 | 77 | 42 | fixed |
| cq7 | 68k | `charset 'B',$77 / charset 'B','A' / dc.b "AB"` | 4177 | 4177 | 4177 | same |
| cq8 | 68k | `charset $41,'BCDEF' / dc.b "ABCDE"` | 4142434445 | refused | refused | differs |
| cq9 | 68k | `charset $41,'' / dc.b "AB"` | 4142 | 0042 | refused | differs |
| cq10 | 68k | `charset $41,"" / dc.b "AB"` | 4142 | 4142 | 4142 | same |
| cq11 | 68k | `charset 'AB',$55 / dc.b "AB"` | refused #1320 | refused | refused | same |
| inc1 | 68k | `include 'p.inc'` | refused #10001 | refused | refused | same |
| bin1 | 68k | `binclude 'p.bin'` | refused #10001 | refused | refused | same |
| msg1 | 68k | `message 'hi' / dc.b 1` | 01 | 01 | 01 | same |
| msg2 | 68k | `warning 'hi' / dc.b 1` | 01 | refused | 01 | fixed |
| sw1c | 68k | `switch 'AB' / case "AB" / dc.b 1 / elsecase / dc.b 2 / endcase` | 01 | 02 | 01 | fixed |
| fn5 | 68k | `f function a,'a' / dc.b f(66)` | 28363629 | 61 | 28363629 | fixed |
| fn6 | 68k | `f function a,"a" / dc.b f(66)` | 28363629 | 28363629 | 28363629 | same |
| rv1 | 68k | `charset 'A',$11 / dc.b ('AB'+1)="AC",('AB'+1)="\x11C"` | 0001 | 0101 | 0001 | fixed |
| rv2 | 68k | `charset 'A',$11 / dc.b lowstring('AB'+1)` | 1163 | refused | 1163 | fixed |
| rv3 | 68k | `charset 'A',$11 / dc.b strlen('AB'+1)` | 02 | refused | 02 | fixed |
| rv4 | 68k | `charset 'A',$11 / move.w #'AB'+1,d0` | 303c1143 | 303c1143 | 303c1143 | same |
| rv5 | 68k | `charset 'A',$11 / charset $11,$55 / dc.b 'AB'+1` | 1143 | refused | 1143 | fixed |
| ifq1 | 68k | `if 65='A' / dc.b 1 / else / dc.b 2 / endif` | 01 | 01 | 01 | same |
| ifq2 | 68k | `if 'A'=65 / dc.b 1 / else / dc.b 2 / endif` | 01 | 01 | 01 | same |
| ifq3 | 68k | `X equ 65 / if X='A' / dc.b 1 / else / dc.b 2 / endif` | 01 | 01 | 01 | same |
| ifq4 | 68k | `dc.b 65='A','A'=65,'AB'=$4142` | 010101 | 010101 | 010101 | same |
| swq1 | 68k | `switch 'A' / case 65 / dc.b 1 / case "A" / dc.b 2 / elsecase / dc.b 3 / endcase` | 02 | 01 | 02 | fixed |
| zdcb2 | z80 | `dc.b 1,"AB"` | refused #1200 | 014142 | 014142 | differs |
| csx | 68k | `charset 'A','X',$11 / dc.b "AB" / dc.w "AB" / move.w #"AB",d0` | 111200110012303c1112 | refused | 111200110012303c1112 | fixed |
| sgn1 | 68k | `Base: / SZ:	equ 8 / dc.l +'A'` | refused #1110 | 00000041 | refused | fixed |
| sgn2 | 68k | `Base: / SZ:	equ 8 / dc.l +'AB'` | refused #1110 | 00004142 | refused | fixed |
| tdw68 | 68k | `dw 1` | refused #1200 | 0100 | 0100 | differs |
| tw0 | 68k | `S1 equ "a" / S2 equ "ab" / dc.w S2` | 00610062 | refused | 00610062 | fixed |
| tw1 | 68k | `S1 equ "a" / S2 equ "ab" / dc.w S2+1` | 00610063 | refused | 00610063 | fixed |
| tw2 | 68k | `S1 equ "a" / S2 equ "ab" / dc.w S1` | 0061 | refused | 0061 | fixed |
| tw3 | 68k | `S1 equ "a" / S2 equ "ab" / dc.l S2` | 0000006100000062 | refused | 0000006100000062 | fixed |
| tw4 | 68k | `S1 equ "a" / S2 equ "ab" / dc.l S2+1` | 0000006100000063 | refused | 0000006100000063 | fixed |
| tw5 | 68k | `S1 equ "a" / S2 equ "ab" / dc.l S1` | 00000061 | refused | 00000061 | fixed |
| tw6 | 68k | `S1 equ "a" / S2 equ "ab" / dw S2` | refused #1200 | refused | 61006200 | differs |
| tw7 | 68k | `S1 equ "a" / S2 equ "ab" / dw S2+1` | refused #1200 | refused | 61006300 | differs |
| tw8 | 68k | `S1 equ "a" / S2 equ "ab" / dw S1` | refused #1200 | refused | 6100 | differs |
| tw9 | 68k | `S2 equ "ab" / dc.w S2-1` | 6161 | 6161 | 6161 | same |
| tcs1 | 68k | `charset 'a',$11 / dc.b "ab"+1,$EE` | 1163ee | refused | 1163ee | fixed |
| tcs2 | 68k | `charset 'a',$11 / dc.b "ab",$EE` | 1162ee | 1162ee | 1162ee | same |
| tl0 | 68k | `dc.w "AB"` | 00410042 | refused | 00410042 | fixed |
| tl1 | 68k | `dc.w "AB"+0` | 00410042 | refused | 00410042 | fixed |
| tl2 | 68k | `dc.l "ABCD"` | 00000041000000420000004300000044 | refused | 00000041000000420000004300000044 | fixed |
| tz0 | z80 | `S1 equ "a" / S2 equ "ab" / dw S2` | 61006200 | refused | 61006200 | fixed |
| tz1 | z80 | `S1 equ "a" / S2 equ "ab" / dw S2+1` | 61006300 | refused | 61006300 | fixed |
| tz2 | z80 | `S1 equ "a" / S2 equ "ab" / dw S1` | 6100 | refused | 6100 | fixed |
| tz3 | z80 | `S2 equ "ab" / dw S2-1` | 6161 | 6161 | 6161 | same |
| tp0 | 68k | `charset 'a',$11 / dc.b "ab"+1,$EE` | 1163ee | refused | 1163ee | fixed |
| tp1 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / dc.b 'AB'+1` | 1123 | refused | 1123 | fixed |
| tp2 | 68k | `charset 'A',$11 / charset 'B',$22 / charset 'C',$99 / charset $23,$77 / dc.b 'AB'+1` | 11 | refused | 11 | fixed |
| tp3 | 68k | `charset $43,$77 / dc.b 'AB'+1` | 41 | refused | 41 | fixed |
| tp4 | 68k | `charset 'A',$11 / charset $11,$55 / dc.b 'AB'+1` | 1143 | refused | 1143 | fixed |
| tp5 | 68k | `charset 'A',$51 / dc.b 'AB'+1` | 5143 | refused | 5143 | fixed |
| tp6 | 68k | `charset 'A',$11 / dc.b ('AB'+1)="AC",('AB'+1)="\x11C"` | 0001 | 0101 | 0001 | fixed |
| tp7 | 68k | `charset 'A',$11 / dc.b lowstring('AB'+1)` | 1163 | refused | 1163 | fixed |
| tp8 | 68k | `charset 'a',$11 / dc.b "ab",$EE` | 1162ee | 1162ee | 1162ee | same |
