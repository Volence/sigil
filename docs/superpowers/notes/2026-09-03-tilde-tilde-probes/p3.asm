	cpu	68000
	padding off
	org	$1000
; every ~~ shape the S2 corpus actually uses, plus the semantics grid
	dc.b	~~0,~~1,~~5,~~-1,~~$FF
	dc.b	~~~0,~~~1,~~~5
	dc.l	~0,~1,~$0F
	dc.b	~~(0),~~(5)
	dc.l	~(~0),~(~5)
	dc.b	~~0+1,~~1+1,~~(0+1)
	dc.b	~~0||~~0,~~0||~~1,~~1||~~1
	dc.b	~~0&&~~0,~~0&&~~1,~~1&&~~1
	dc.b	~~(1=1),~~(1=2),(1=1),(1=2)
	dc.l	~(1=1),~(1=2)
	dc.b	1||0,2||0,5||3,2&&1,0&&5
	dc.b	~~0|2,~~0&3,~~0!1
	dc.b	-~~0,~~0*3,~~2*3
	dc.b	~~ 0,~~ 1
	dc.b	~~(-1)
	dc.l	~(-1)
	dc.b	~~0=1,~~1=0
	dc.b	~~(~~0),~~(~~5)
	dc.b	(~~0)=(1=1)
; the corpus's own composition shape (s2.sounddriver.asm:3253)
OptimiseDriver = 0
FixDriverBugs = 0
	if (~~OptimiseDriver)&&(~~FixDriverBugs)
	dc.b	$A5
	endif
; the corpus's own `if ~~FLAG` shape
	if ~~OptimiseDriver
	dc.b	$5A
	else
	dc.b	$33
	endif
	end
