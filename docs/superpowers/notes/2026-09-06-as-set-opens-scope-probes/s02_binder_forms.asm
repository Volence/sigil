; s02: WHICH value-binding forms open a local-label scope in asl?
;
; Every local name is UNIQUE, so the symbol table's spelling of each names the
; parent it attached to with no ambiguity.  A name that existed under both
; candidates could not discriminate; none here does.
;
; Read the table, not the byte column: this run must exit 0, and it does.
	cpu	68000
	padding	off
	org	$1000
PlainL:
	nop
.aa:
	nop
Se	set	5
.bb:
	nop
Eq	equ	6
.cc:
	nop
Asg	=	7
.dd:
	nop
Cln	:=	8
.ee:
	nop
Evl	eval	9
.ff:
	nop
Lbl	label	*
.gg:
	nop
	end
