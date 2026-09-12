; body: / - then ref --
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
/	dc.w	$2222
-	dc.w	$3333
	dc.w	--	; REF
	endm
	mac
	dc.w	$4444
