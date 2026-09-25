	cpu 68000
PG1	equ	5
	charset $41,$11
	codepage PG1
	charset $42,$22
	dc.b "AB",PG1
	codepage STANDARD
	dc.b "AB"
