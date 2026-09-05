; The decisive control for d1.asm: the comparison INVERTED. d1.asm asks
; `Undefined1=0` and asl takes TRUE; this asks `Undefined1=1`. No numeric
; placeholder can satisfy both, so if this also takes TRUE the condition is not
; being evaluated against a substituted value at all -- asl's #1820 refusal
; takes the THEN branch unconditionally.
; It takes TRUE and emits $1111.
	cpu	68000
	org	0
	if Undefined1=1
	dc.w	$1111
	else
	dc.w	$2222
	endif
	end
