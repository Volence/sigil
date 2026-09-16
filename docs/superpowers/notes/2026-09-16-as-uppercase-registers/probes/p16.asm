	cpu	68000
	org	0
Tbl:	dc.w	1,2
	move.w	(Tbl,PC),d1
	move.w	(Tbl,PC,D0.W),d1
	move	D6,CCR
	move	#$2700,SR
	move	A6,USP
	end
