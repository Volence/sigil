; phase68k_control.asm -- phase68k.asm with the PHASE/DEPHASE bracket DELETED
; and nothing else changed. This is the positive control that shows what the
; PHYSICAL answer looks like in this exact listing format: here PhasedHead can
; only be B8002, because there is no phase to report.
;
; So: if phase68k.lst prints PhasedHead : B8002 it agrees with this file, and
; asl reports the physical address. If it prints 8000 it disagrees, and asl
; reports the phase address.
	cpu	68000
	org	$B8000
CtrlBefore:
	dc.w	$1111
PhasedHead:
	dc.w	$2222
CtrlAfter:
	dc.w	$3333
	end
