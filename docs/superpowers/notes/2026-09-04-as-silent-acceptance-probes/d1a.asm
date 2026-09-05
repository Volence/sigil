; CONTROL for d1.asm: is asl's `=>TRUE` on a refused #1820 condition a
; SUBSTITUTED VALUE (the last thing asl computed, as in wrange.asm/d9.asm), or
; an unconditional branch choice? Identical to d1.asm except a non-zero $3333 is
; computed on the line above. If `Undefined1` took the last computed value the
; condition would be $3333=0 -> FALSE and this would emit $2222.
; It emits $1111. So the value theory is wrong here.
	cpu	68000
	org	0
	dc.w	$3333
	if Undefined1=0
	dc.w	$1111
	else
	dc.w	$2222
	endif
	end
