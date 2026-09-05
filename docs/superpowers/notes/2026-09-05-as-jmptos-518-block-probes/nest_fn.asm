	cpu	68000
	org	$1000
extractJmpToName function name,val(substr(name, strstr(name, "_") + 1, strlen(name)))
Foo:
	dc.l	0,0,0
Bar:
	dc.l	extractJmpToName("JmpTo_Foo")
	dc.l	extractJmpToName("JmpTo_Bar")
	end
