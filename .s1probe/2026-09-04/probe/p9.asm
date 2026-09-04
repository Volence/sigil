	cpu 68000
	padding off
	org $1000
Blob:
	save
	!org	0
	CPU Z80
zStart:
	ld	a,1
	ld	(zVar),a
zVar:	db	0
	restore
	dc.b	$AA
	dc.w	zVar
	end
