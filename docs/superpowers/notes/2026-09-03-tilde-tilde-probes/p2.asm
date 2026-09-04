	cpu	68000
	padding off
	org	$1000

; --- A: do || / && accept plain integers?
	dc.b	1||0
	dc.b	2||0
	dc.b	5||3
	dc.b	2&&1
	dc.b	0&&5

; --- B: does `if` accept a plain integer?
	if 5
	dc.b	$AA
	endif
	if 0
	dc.b	$BB
	endif
	if 1
	dc.b	$CC
	endif

; --- C: `if` on a ~~ result
	if ~~0
	dc.b	$D0
	endif
	if ~~1
	dc.b	$D1
	endif

; --- D: does ~~ produce a boolean usable with bitwise ops?
	dc.b	~~0|2
	dc.b	~~0&3
	dc.b	~~0!1

; --- E: ~~ of a string?
	dc.b	~~"a"

; --- F: ~~ of a float
	dc.b	~~0.0
	dc.b	~~1.5

; --- G: precedence vs unary minus and *
	dc.b	-~~0
	dc.b	~~0*3
	dc.b	~~2*3

; --- H: ~~ with space before operand
	dc.b	~~ 0
	dc.b	~~ 1

; --- I: ~~ of a parenthesised negative, and ~ of a parenthesised negative
	dc.b	~~(-1)
	dc.l	~(-1)
	dc.l	~ -1

; --- J: chained relational-ish
	dc.b	~~0=1
	dc.b	~~1=0

; --- K: ~~ applied twice via parens
	dc.b	~~(~~0)
	dc.b	~~(~~5)

; --- L: does ~~ of a boolean stay boolean (double compare)
	dc.b	(~~0)=(1=1)

	end
