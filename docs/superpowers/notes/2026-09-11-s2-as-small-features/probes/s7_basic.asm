	cpu 68000
	padding off
	org 0
	dc.w $1234
Foo:	move.w #$0F64,d7
	shared Foo
	dc.b $EE
	end
