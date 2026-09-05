; A builtin head used as an ordinary displacement SYMBOL. `val(a0)` is a
; displacement addressing mode, not a call, and must stay one.
	cpu	68000
	org	$1000
val:	equ	4
strlen:	equ	4
	move.l	val(a0),d0
	move.l	strlen(a0,a0.l),d0
	end
