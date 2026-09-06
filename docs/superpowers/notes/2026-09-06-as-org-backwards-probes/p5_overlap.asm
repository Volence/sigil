; A backwards org onto ground a CLOSED region already owns.
	cpu	68000
	org	$1000
	dc.b	1,2,3,4,5,6,7,8
	org	$3000
	dc.b	$c0,$c1
	org	$1002
	dc.b	$e0,$e1,$e2,$e3
	end
