	cpu 68000
	charset $41,$11
	save
	codepage PG1
	charset $42,$22
	save
	codepage PG2
	charset $43,$33
	dc.b "ABC"
	restore
	dc.b "ABC"
	restore
	dc.b "ABC"
