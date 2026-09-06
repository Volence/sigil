	cpu	68000
	padding off
	org	0
; Which float function names does this build know at all?
; One per file would be cleaner, but asl reports EVERY unknown function it
; meets on the same pass, so a single file names them all in one listing.
	dc.l	INT(LOG(100))
	dc.l	INT(LN(100))
	dc.l	INT(LOG10(100))
	dc.l	INT(EXP(1))
	dc.l	INT(SQRT(16))
	dc.l	INT(ABS(-3))
	dc.l	INT(TAN(0))
	dc.l	INT(ATAN(0))
	dc.l	INT(BOGUSFN(1))
	end
