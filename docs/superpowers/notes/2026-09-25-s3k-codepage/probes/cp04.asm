	cpu 68000
	charset $41,$11
	save
	codepage PG1
	charset $42,$22
	dc.b "AB"
	restore
	dc.b "AB"
