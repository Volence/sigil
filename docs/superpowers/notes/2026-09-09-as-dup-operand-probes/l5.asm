; l5 - `listing` with TWO arguments. asl's refusal for a bare `listing` says
; "expected one argument", so the upper bound should be 1, but `page`'s says
; "between 1 and 2" and the two are not the same directive. Measured, not
; inferred from the neighbour.
	cpu	68000
	org	$1000
	listing	on,off
	dc.b	$11
	end
