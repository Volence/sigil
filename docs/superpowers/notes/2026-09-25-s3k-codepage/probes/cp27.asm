	cpu 68000
	charset $41,$11
first:
	codepage .loc
	charset $42,$22
	codepage STANDARD
second:
	codepage .loc
	dc.b "AB"
