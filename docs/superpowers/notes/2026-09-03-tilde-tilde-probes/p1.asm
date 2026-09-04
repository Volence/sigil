	cpu	68000
	padding off
	org	$1000

; --- 1: single tilde = bitwise not
	dc.l	~0
	dc.l	~1
	dc.l	~$0F
	dc.l	~-1

; --- 2: double tilde = ?
	dc.b	~~0
	dc.b	~~1
	dc.b	~~5
	dc.b	~~-1
	dc.b	~~$FF

; --- 3: triple tilde
	dc.b	~~~0
	dc.b	~~~1
	dc.b	~~~5

; --- 4: quadruple
	dc.b	~~~~0
	dc.b	~~~~1

; --- 5: with spaces between
	dc.l	~ ~ 0
	dc.l	~ ~ 1

; --- 6: parenthesised
	dc.b	~~(0)
	dc.b	~~(5)
	dc.l	~(~0)
	dc.l	~(~5)

; --- 7: precedence vs binary +
	dc.b	~~0+1
	dc.b	~~1+1
	dc.b	~~(0+1)

; --- 8: composition with || and &&
	dc.b	~~0||~~0
	dc.b	~~0||~~1
	dc.b	~~1||~~1
	dc.b	~~0&&~~0
	dc.b	~~0&&~~1
	dc.b	~~1&&~~1

; --- 9: applied to a comparison result
	dc.b	~~(1=1)
	dc.b	~~(1=2)
	dc.b	(1=1)
	dc.b	(1=2)

; --- 10: single ~ on a comparison
	dc.l	~(1=1)
	dc.l	~(1=2)

	end
