	cpu	68000
	padding off
	org	$1000
; `~~` inside a macro ARGUMENT, re-rendered through the parameter and ALLARGS
one macro v
	dc.b	v
	endm
all macro
	dc.b	ALLARGS
	endm
	one ~~0
	one ~~1
	all ~~0,~~5,~~~0
; and inside a %<...> style string embed
str macro v
	dc.b	"[v]"
	endm
	str ~~0
	end
