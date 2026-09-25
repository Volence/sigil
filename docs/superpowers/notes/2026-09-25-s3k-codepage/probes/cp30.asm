	cpu 68000
	codepage PG1
	charset $41,$11
	charset $42,$22
	codepage STANDARD
chr	macro c,s
	save
	codepage PG1
	dc.b c,s
	restore
	dc.b c,s
	endm
	chr 'A',"AB"
	dc.b 'A',"AB"
