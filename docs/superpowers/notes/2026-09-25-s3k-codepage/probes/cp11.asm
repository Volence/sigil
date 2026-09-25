	cpu 68000
	charset $41,$11
	codepage STANDARD
	dc.b "AB"
	dc.w	fwd
	codepage PG1
	charset $42,$22
fwd:
	dc.b "AB"
