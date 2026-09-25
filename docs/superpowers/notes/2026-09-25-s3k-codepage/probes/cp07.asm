	cpu 68000
sel	macro
	codepage PG1
	endm
	charset $41,$11
	codepage PG1
	charset $42,$22
	codepage STANDARD
	dc.b "AB"
	sel
	dc.b "AB"
