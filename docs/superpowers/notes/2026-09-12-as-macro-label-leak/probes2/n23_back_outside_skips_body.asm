; outer `-`; body `-`; after the call `dc.w --`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
-	dc.w	$3333
mac	macro
-	dc.w	$2222
	endm
	mac
	dc.w	--	; REF
	dc.w	$4444
