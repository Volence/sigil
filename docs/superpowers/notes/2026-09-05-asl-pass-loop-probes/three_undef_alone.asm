; THE CONTROL. Three undefined symbols, nothing else wrong.
; asl: 2 passes, 3 errors, exit 2, footer WITHOUT the "not started" warning.
;
; It fails. Its diagnostics are complete. Those two facts together are why the
; exit status cannot stand in for the pass-loop check.
	cpu	68000
	org	$1000
start:
	move.w	#UND_ONE,d0
	move.w	#UND_TWO,d1
	move.w	#UND_THREE,d2
	rts
	end
