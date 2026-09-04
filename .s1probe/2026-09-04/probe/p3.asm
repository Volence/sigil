	cpu 68000
	padding off
	enumconf 1
	enum	zqA=$80,zqB,zqC
	nextenum zqD,zqE
	dc.b	zqA,zqB,zqC,zqD,zqE
	dc.b	zqNeverDefinedAnywhere
	end
