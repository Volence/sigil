; phasez80.asm -- the same question asked of a Z80 section, because the real
; colliding symbol is described as "a Z80-space label".
;
; ⚠ FIRST VERSION OF THIS PROBE WAS REFUSED, and the refusal is the reason the
; addresses below are small. It used the 68000 probe's `org 0B8000h`, and asl
; answered `error #1925: address overflow` on the two UNPHASED `db` lines --
; the Z80 address space is 16 bits, so a physical B8000 does not exist there.
; That run exited 2, so by the standing rule no byte column and no symbol value
; from it is quotable, INCLUDING the phased line that did not error. It was
; re-shaped rather than read.
;
; Both candidates therefore live inside 16 bits, and are still unmistakable:
;   phase answer     Z80PhasedHead : 8000
;   physical answer  Z80PhasedHead : 1002
; Controls, outside the bracket: Z80CtrlBefore 1000, Z80CtrlAfter 1004.
	cpu	z80
	org	01000h
Z80CtrlBefore:
	db	11h,11h
	phase	08000h
Z80PhasedHead:
	db	22h,22h
	dephase
Z80CtrlAfter:
	db	33h,33h
	end
