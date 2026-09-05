	cpu	68000
	org	$1000
Foo:
	dc.l	val(substr("JmpTo_Foo", strstr("JmpTo_Foo","_")+1, 3))
	dc.l	val(substr("JmpTo_Foo", 6, strlen("JmpTo_Foo")))
	end
