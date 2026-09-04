	cpu	68000
	padding off
	org	$1000
; macro-body round trip: `~~` must survive being captured and re-rendered
gate macro flag,yes,no
	if ~~flag
	dc.b	yes
	else
	dc.b	no
	endif
	endm
Zero = 0
One = 1
	gate Zero,$AA,$BB
	gate One,$CC,$DD
; `~~` inside a macro body, used in an expression rather than an `if`
val macro x
	dc.b	~~x,~~~x
	endm
	val 0
	val 5
	end
