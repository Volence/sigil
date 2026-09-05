; The corpus construct itself: Sonic 2's `s2.macrosetup.asm:280` function and
; the `:304` jump it feeds, reduced to two targets at two different addresses
; so a fix that resolved every call to the same symbol cannot pass.
	cpu	68000
	org	$1000
extractJmpToName function name,val(substr(name, strstr(name, "_") + 1, strlen(name)))
Foo:
	nop
	nop
	nop
	nop
Bar:
	jmp	(extractJmpToName("JmpTo_Foo")).l
	jmp	(extractJmpToName("JmpTo_Bar")).l
	end
