	cpu 68000
	dc.b "AB"
	codepage PG1
	dc.b "AB"
	codepage STANDARD
	dc.w	fwd
	codepage PG1
	charset $41,$11
	codepage STANDARD
	charset $42,$22
fwd:
	dc.b "AB"
